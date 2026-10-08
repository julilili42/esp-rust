#![no_std]
#![no_main]

use core::time::Duration;
use embedded_lora_rfm95::{
    lora::types::{Bandwidth, CodingRate, CrcMode, Frequency, SpreadingFactor},
    rfm95::Rfm95Driver,
};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
    spi::master::{Config, Spi},
    time::Instant,
};
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());
    let spi = Spi::new(p.SPI2, Config::default())
        .unwrap()
        .with_sck(p.GPIO6)
        .with_miso(p.GPIO20)
        .with_mosi(p.GPIO7);
    let cs = Output::new(p.GPIO10, Level::High, OutputConfig::default());
    let rst = Output::new(p.GPIO11, Level::High, OutputConfig::default());
    let delay = Delay::new();
    let mut radio = Rfm95Driver::new_from_bus(spi, cs, rst, delay).unwrap();
    radio.set_frequency(Frequency::hz(869_525_000)).unwrap();
    // ponytail: use a TCXO before narrowing bandwidth below 62.5 kHz.
    radio.set_bandwidth(Bandwidth::B62_5).unwrap();
    radio.set_spreading_factor(SpreadingFactor::S12).unwrap();
    radio.set_coding_rate(CodingRate::C4_8).unwrap();
    radio.set_crc_mode(CrcMode::Enabled).unwrap();

    println!("LoRa TX: 869.525 MHz, SF12, BW62.5, CR4/8, CRC on");
    let mut sequence = 0u32;
    let mut reply = [0; 4];
    loop {
        sequence = sequence.wrapping_add(1);
        let ping = sequence.to_be_bytes();
        radio.start_tx(&ping).unwrap();
        let started = Instant::now();
        while radio.complete_tx().unwrap().is_none() {
            assert!(started.elapsed().as_millis() < 4000, "LoRa TX timeout");
            delay.delay_millis(5);
        }
        println!("PING #{} sent", sequence);

        radio.start_rx(Duration::from_secs(5)).unwrap();
        loop {
            match radio.complete_rx(&mut reply) {
                Ok(Some(len)) => {
                    if len == ping.len() && reply == ping {
                        println!(
                            "PONG #{}: RSSI {} dBm, SNR {} dB",
                            sequence,
                            radio.get_packet_rssi().unwrap(),
                            radio.get_packet_snr().unwrap()
                        );
                    } else {
                        println!("No matching PONG for #{}", sequence);
                    }
                    break;
                }
                Ok(None) => delay.delay_millis(5),
                Err(error) => {
                    println!("No PONG for #{}: {:?}", sequence, error);
                    break;
                }
            }
        }
        // A four-byte packet takes ~1.9 s; keep TX duty cycle below 10% on this band.
        delay.delay_millis(30_000);
    }
}
