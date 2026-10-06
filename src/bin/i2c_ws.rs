#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::net::Ipv4Addr;

use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::{IpAddress, IpEndpoint, Stack, tcp::TcpSocket};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use embedded_hal_compat::Reverse;
use esp_backtrace as _;
use esp_hal::{Blocking, clock::CpuClock, i2c::master::I2c, main};
use esp_println::{self as _};
use esp_rust::{
    sensor::{bus_setup, initialize_mpu, start_rtos},
    vibration::{AccData, Ema, EmaFilter, RawData, Rms, calc_rms},
    websocket::{ws_connect, ws_manage_heartbeat, ws_send},
    wifi,
};
use heapless::{String, Vec};
use mpu6050::Mpu6050;

esp_bootloader_esp_idf::esp_app_desc!();

const BATCH_CAPACITY: usize = 4;
type Batch = Vec<AccData, BATCH_CAPACITY>;

static BATCHES: Channel<CriticalSectionRawMutex, Batch, 2> = Channel::new();

const XYZ_JSON_CAPACITY: usize = 16 + 3 * 24;
const ACC_JSON_CAPACITY: usize = 15 + 2 * XYZ_JSON_CAPACITY + 16 + 1 * 24;
const JSON_CAPACITY: usize = 2 + BATCH_CAPACITY * (ACC_JSON_CAPACITY + 1);

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

const WS_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 0, 221);
const WS_PORT: u16 = 8000;
const WS_PATH: &str = "/ws";

#[embassy_executor::task]
#[warn(clippy::large_stack_frames)]
async fn send_batch(endpoint: IpEndpoint, path: String<20>, stack: Stack<'static>) {
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];

    let mut read_buf = [0; 1024];
    let mut write_buf = [0; JSON_CAPACITY + 14];

    let mut tcp_socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);

    // connection loop
    loop {
        // data loop
        if let Ok(mut ws_client) = ws_connect(&mut tcp_socket, endpoint, path.as_str()).await {
            let mut received = 0;

            loop {
                let receive_batch = BATCHES.receive();
                let receive_ping = ws_manage_heartbeat(
                    &mut ws_client,
                    &mut tcp_socket,
                    &mut read_buf,
                    &mut received,
                );

                match select(receive_batch, receive_ping).await {
                    Either::First(batch) => {
                        let message =
                            serde_json_core::to_string::<_, JSON_CAPACITY>(batch.as_slice())
                                .unwrap();

                        match ws_send(&mut ws_client, &mut tcp_socket, &mut write_buf, &message)
                            .await
                        {
                            Ok(_) => info!("Message send."),
                            Err(e) => {
                                error!("Send failed: {}", defmt::Debug2Format(&e));
                                break;
                            }
                        }
                    }
                    Either::Second(ping) => match ping {
                        Ok(()) => {}
                        Err(e) => {
                            error!("Connection lost in heartbeat: {}", defmt::Debug2Format(&e));
                            break;
                        }
                    },
                }
            }
        } else {
            warn!("Reconnecting...");
            Timer::after_secs(5).await;
        }
    }
}

#[embassy_executor::task]
#[warn(clippy::large_stack_frames)]
async fn accumulate_batch(
    mut mpu: Mpu6050<Reverse<I2c<'static, Blocking>>>,
    mut buffer: Vec<AccData, BATCH_CAPACITY>,
    refresh_duration: Duration,
) {
    let mut ema_filter = EmaFilter::new(0.05);

    loop {
        match mpu.get_acc() {
            Ok(acc) => {
                let ema = &ema_filter.update(Ema {
                    x: acc.x,
                    y: acc.y,
                    z: acc.z,
                });

                let rms = calc_rms(acc.x, acc.y, acc.z);

                buffer
                    .push(AccData {
                        raw: Some(RawData {
                            x: acc.x,
                            y: acc.y,
                            z: acc.z,
                        }),
                        ema: Some(Ema {
                            x: ema.x,
                            y: ema.y,
                            z: ema.z,
                        }),
                        rms: Some(Rms { value: rms }),
                    })
                    .unwrap();

                if buffer.is_full() {
                    BATCHES.send(core::mem::take(&mut buffer)).await;
                }
            }
            Err(e) => {
                error!("Failed to read sensor data: {:?}", defmt::Debug2Format(&e))
            }
        }
        Timer::after(refresh_duration).await;
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
async fn main(spawner: Spawner) {
    esp_alloc::heap_allocator!(size: 32 * 1024);
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    start_rtos(peripherals.TIMG0, peripherals.FROM_CPU_INTR0);
    let i2c_bus = match bus_setup(peripherals.I2C0, peripherals.GPIO2, peripherals.GPIO3) {
        Ok(bus) => bus,
        Err(e) => {
            error!("Failed to set-up i2c bus: {}", defmt::Display2Format(&e));
            loop {
                Timer::after(Duration::from_secs(1)).await;
            }
        }
    };

    let mpu = match initialize_mpu(i2c_bus) {
        Ok(m) => {
            info!("Initialized mpu");
            m
        }
        Err(e) => {
            error!("Failed to initialize mpu: {}", defmt::Debug2Format(&e));
            loop {
                Timer::after(Duration::from_secs(1)).await;
            }
        }
    };

    let stack = match wifi::connect(spawner, peripherals.WIFI, SSID, PASSWORD).await {
        Ok(stack) => stack,
        Err(e) => {
            error!("Wifi error: {}", defmt::Debug2Format(&e));
            loop {
                Timer::after(Duration::from_secs(1)).await;
            }
        }
    };

    let buffer: Vec<AccData, BATCH_CAPACITY> = Vec::new();
    let refresh_duration = Duration::from_millis(10);

    let mut ws_path: String<20> = String::new();
    ws_path.push_str(WS_PATH).unwrap();

    let endpoint = IpEndpoint::new(IpAddress::Ipv4(WS_IP), WS_PORT);

    spawner.spawn(accumulate_batch(mpu, buffer, refresh_duration).unwrap());
    spawner.spawn(send_batch(endpoint, ws_path, stack).unwrap());
}
