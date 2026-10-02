#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::fmt::Write as _;
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use embedded_hal_compat::Reverse;
use embedded_websocket::WebSocketClient;
use esp_backtrace as _;
use esp_hal::{Blocking, clock::CpuClock, i2c::master::I2c, main, rng::Rng};
use esp_println::{self as _};
use esp_rust::{
    sensor::{AccData, bus_setup, initialize_mpu, start_rtos},
    websocket::{connect_tcp, ws_handshake, ws_send},
    wifi,
};
use heapless::{String, Vec};
use mpu6050::Mpu6050;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

type Batch = Vec<AccData, 32>;
static BATCHES: Channel<CriticalSectionRawMutex, Batch, 2> = Channel::new();
const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

#[embassy_executor::task]
#[warn(clippy::large_stack_frames)]
async fn send_batch(stack: Stack<'static>, send_duration: Duration) {
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];
    let mut stream = connect_tcp(stack, &mut rx_buf, &mut tx_buf).await;

    let mut write_buf = [0; 4110];
    let mut read_buf = [0; 4000];
    let mut websocket = WebSocketClient::new_client(Rng::new());

    let handshake = ws_handshake(&mut websocket, &mut stream, &mut write_buf, &mut read_buf).await;
    match handshake {
        Ok(_) => info!("Succesful WS handshake."),
        Err(e) => {
            error!("Error during handshake: {}", defmt::Debug2Format(&e));
            loop {
                Timer::after(Duration::from_secs(1)).await;
            }
        }
    }

    loop {
        let batch = BATCHES.receive().await;
        let mut message = String::<4096>::new();
        write!(&mut message, "{batch:?}").unwrap();
        let send = ws_send(&mut websocket, &mut stream, &mut write_buf, &message).await;

        match send {
            Ok(_) => info!("Message send."),
            Err(e) => error!("Send failed: {}", defmt::Debug2Format(&e)),
        }

        Timer::after(send_duration).await;
    }
}

#[embassy_executor::task]
#[warn(clippy::large_stack_frames)]
async fn accumulate_batch(
    mut mpu: Mpu6050<Reverse<I2c<'static, Blocking>>>,
    mut buffer: Vec<AccData, 32>,
    refresh_duration: Duration,
) {
    loop {
        match mpu.get_acc() {
            Ok(acc) => {
                buffer
                    .push(AccData {
                        x: acc.x,
                        y: acc.y,
                        z: acc.z,
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
                Timer::after(Duration::from_millis(1_000)).await;
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
                Timer::after(Duration::from_millis(1_000)).await;
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

    let buffer: Vec<AccData, 32> = Vec::new();
    let refresh_duration = Duration::from_millis(50);
    let send_duration = Duration::from_millis(50);

    spawner.spawn(accumulate_batch(mpu, buffer, refresh_duration).unwrap());
    spawner.spawn(send_batch(stack, send_duration).unwrap());
}

// for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.2.2/examples
