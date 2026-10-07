#![no_std]
#![no_main]

use core::{str::from_utf8, time::Duration};
use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_time::Timer;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_lora_rfm95::{
    lora::types::{Bandwidth, CrcMode, Frequency, SpreadingFactor},
    rfm95::Rfm95Driver,
};

use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
    peripherals::{GPIO6, GPIO7, GPIO20, SPI2},
    spi::master::{Config, ConfigError, Spi},
};
use esp_println as _;
use esp_rust::sensor::start_rtos;

esp_bootloader_esp_idf::esp_app_desc!();

pub fn bus_setup(
    spi: SPI2<'static>,
    sclk: GPIO6<'static>,
    miso: GPIO20<'static>,
    mosi: GPIO7<'static>,
) -> Result<Spi<'static, Blocking>, ConfigError> {
    Ok(Spi::new(spi, Config::default())?
        .with_sck(sclk)
        .with_miso(miso)
        .with_mosi(mosi))
}

#[embassy_executor::task]
#[warn(clippy::large_stack_frames)]
async fn driver(
    mut radio: Rfm95Driver<ExclusiveDevice<Spi<'static, Blocking>, Output<'static>, Delay>>,
) {
    loop {
        send_bytes(&mut radio).await;
        receive_bytes(&mut radio).await;
    }
}

#[warn(clippy::large_stack_frames)]
async fn send_bytes(
    radio: &mut Rfm95Driver<ExclusiveDevice<Spi<'static, Blocking>, Output<'static>, Delay>>,
) {
    let started = embassy_time::Instant::now();
    radio.start_tx(b"ping").unwrap();

    while radio.complete_tx().unwrap().is_none() {
        if started.elapsed() >= embassy_time::Duration::from_secs(1) {
            error!("LoRa TX timeout");
            return;
        }
        Timer::after(embassy_time::Duration::from_millis(1)).await;
    }

    info!("Sent: ping");
}

#[warn(clippy::large_stack_frames)]
async fn receive_bytes(
    radio: &mut Rfm95Driver<ExclusiveDevice<Spi<'static, Blocking>, Output<'static>, Delay>>,
) {
    let mut rx_buf = [0u8; 1024];
    radio.start_rx(Duration::from_millis(500)).unwrap();
    loop {
        match radio.complete_rx(&mut rx_buf) {
            Ok(Some(len)) => {
                if let Ok(text) = from_utf8(&rx_buf[..len]) {
                    info!("Received: {}", text);
                }
                break;
            }
            Ok(None) => {}
            Err(e) => {
                warn!("{}", defmt::Debug2Format(&e));
                break;
            }
        }
        Timer::after(embassy_time::Duration::from_millis(1)).await;
    }
}

#[main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    start_rtos(peripherals.TIMG0, peripherals.FROM_CPU_INTR0);

    info!("LoRa SPI setup: SCK=6, MISO=20, MOSI=7");
    let spi = match bus_setup(
        peripherals.SPI2,
        peripherals.GPIO6,
        peripherals.GPIO20,
        peripherals.GPIO7,
    ) {
        Ok(bus) => bus,
        Err(e) => {
            error!("Failed to set-up spi bus: {}", defmt::Display2Format(&e));
            loop {
                Timer::after(embassy_time::Duration::from_secs(1)).await;
            }
        }
    };
    let cs = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO11, Level::High, OutputConfig::default());

    let delay = Delay::new();
    // Read RegVersion after reset; the driver repeats the reset during initialization.
    let mut radio = Rfm95Driver::new_from_bus(spi, cs, rst, delay)
        .inspect_err(|e| error!("RFM95W initialization failed: {}", defmt::Debug2Format(e)))
        .unwrap();

    radio.set_frequency(Frequency::hz(869_525_000)).unwrap();
    radio.set_bandwidth(Bandwidth::B125).unwrap();
    radio.set_spreading_factor(SpreadingFactor::S12).unwrap();
    radio.set_crc_mode(CrcMode::Enabled).unwrap();
    info!("RFM95W ready: 869.525 MHz, SF7, BW125 kHz, CRC on");

    spawner.spawn(driver(radio).unwrap());
}
