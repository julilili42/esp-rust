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
use embassy_time::{Duration, Instant, Ticker, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_hal_compat::Reverse;
use embedded_lora_rfm95::{
    lora::types::{Bandwidth, CodingRate, CrcMode, Frequency, SpreadingFactor},
    rfm95::Rfm95Driver,
};
use esp_backtrace as _;
use esp_hal::{
    Blocking,
    clock::CpuClock,
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    i2c::master::I2c,
    main,
    spi::master::{Config, Spi},
};
use esp_println as _;
use esp_rust::{
    sensor::{bus_setup, initialize_mpu, start_rtos},
    vibration::{Ema, MovementDetector},
};
use mpu6050::Mpu6050;

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
#[warn(clippy::large_stack_frames)]
async fn detect_movement(
    mut mpu: Mpu6050<Reverse<I2c<'static, Blocking>>>,
    mut radio: Rfm95Driver<ExclusiveDevice<Spi<'static, Blocking>, Output<'static>, Delay>>,
    refresh_duration: Duration,
) {
    let mut ticker = Ticker::every(refresh_duration);
    let mut movement = MovementDetector::default();
    let mut sequence = 0u32;
    let mut next_tx = Instant::now();

    loop {
        match mpu.get_acc() {
            Ok(acc) => {
                if movement.update(Ema {
                    x: acc.x,
                    y: acc.y,
                    z: acc.z,
                }) && Instant::now() >= next_tx
                {
                    sequence = sequence.wrapping_add(1);
                    radio.start_tx(&sequence.to_be_bytes()).unwrap();
                    let started = Instant::now();
                    while radio.complete_tx().unwrap().is_none() {
                        assert!(
                            started.elapsed() < Duration::from_secs(4),
                            "LoRa TX timeout"
                        );
                        Timer::after_millis(5).await;
                    }
                    info!("Detected movement! PING #{} sent", sequence);
                    // Keep the same TX interval as lora_tx on the range_test branch.
                    next_tx = Instant::now() + Duration::from_secs(30);
                    // Do not replay sensor ticks missed during transmission.
                    ticker.reset();
                }
            }
            Err(e) => {
                error!("Failed to read sensor data: {:?}", defmt::Debug2Format(&e))
            }
        }
        ticker.next().await;
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

    let spi = Spi::new(peripherals.SPI2, Config::default())
        .unwrap()
        .with_sck(peripherals.GPIO6)
        .with_miso(peripherals.GPIO20)
        .with_mosi(peripherals.GPIO7);
    let cs = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO11, Level::High, OutputConfig::default());
    let mut radio = Rfm95Driver::new_from_bus(spi, cs, rst, Delay::new()).unwrap();
    radio.set_frequency(Frequency::hz(869_525_000)).unwrap();
    radio.set_bandwidth(Bandwidth::B62_5).unwrap();
    radio.set_spreading_factor(SpreadingFactor::S12).unwrap();
    radio.set_coding_rate(CodingRate::C4_8).unwrap();
    radio.set_crc_mode(CrcMode::Enabled).unwrap();

    info!("Calibrating movement detection: keep the sensor still for 1 s");
    spawner.spawn(detect_movement(mpu, radio, Duration::from_millis(10)).unwrap());
}
