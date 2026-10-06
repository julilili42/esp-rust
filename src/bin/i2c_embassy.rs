#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use embedded_hal_compat::Reverse;
use esp_backtrace as _;
use esp_hal::{Blocking, clock::CpuClock, i2c::master::I2c, main};
use esp_println::{self as _};
use esp_rust::sensor::{bus_setup, initialize_mpu};
use esp_rust::vibration::{AccData, RawData};
use heapless::Vec;
use mpu6050::Mpu6050;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

type Batch = Vec<AccData, 32>;
static BATCHES: Channel<CriticalSectionRawMutex, Batch, 2> = Channel::new();

#[embassy_executor::task]
async fn print_batch() {
    loop {
        let batch = BATCHES.receive().await;
        info!("{:?}", defmt::Debug2Format(&batch));
    }
}

#[embassy_executor::task]
async fn accumulate_batch(
    mut mpu: Mpu6050<Reverse<I2c<'static, Blocking>>>,
    mut buffer: Vec<AccData, 32>,
    duration: Duration,
) {
    loop {
        match mpu.get_acc() {
            Ok(acc) => {
                buffer
                    .push(AccData {
                        raw: Some(RawData {
                            x: acc.x,
                            y: acc.y,
                            z: acc.z,
                        }),
                        ema: None,
                        rms: None,
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
        Timer::after(duration).await;
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

    let buffer: Vec<AccData, 32> = Vec::new();
    let refresh_duration = Duration::from_millis(500);
    spawner.spawn(accumulate_batch(mpu, buffer, refresh_duration).unwrap());
    spawner.spawn(print_batch().unwrap());
}

// for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.2.2/examples
