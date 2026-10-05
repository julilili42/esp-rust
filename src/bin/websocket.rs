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
use embassy_net::{IpAddress, IpEndpoint, tcp::TcpSocket};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, main};
use esp_println as _;
use esp_rust::{
    websocket::{ws_connect, ws_send},
    wifi,
};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();
const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const WS_PATH: &str = "/ws";

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let stack = match wifi::connect(spawner, peripherals.WIFI, SSID, PASSWORD).await {
        Ok(stack) => stack,
        Err(e) => {
            error!("Wifi error: {}", defmt::Debug2Format(&e));
            loop {
                Timer::after(Duration::from_secs(1)).await;
            }
        }
    };

    let ip = Ipv4Addr::new(192, 168, 0, 221);
    let port = 8000;
    let endpoint = IpEndpoint::new(IpAddress::Ipv4(ip), port);

    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];
    let mut write_buf = [0u8; 1024];

    let mut tcp_socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
    loop {
        if let Ok(mut websocket) = ws_connect(&mut tcp_socket, endpoint, WS_PATH).await {
            loop {
                let send = ws_send(
                    &mut websocket,
                    &mut tcp_socket,
                    &mut write_buf,
                    "hello from esp",
                )
                .await;

                match send {
                    Ok(_) => info!("Message send."),
                    Err(e) => {
                        error!("Send failed: {}", defmt::Debug2Format(&e));
                        break;
                    }
                }

                Timer::after(Duration::from_secs(1)).await;
            }
        } else {
            warn!("Reconnecting...");
            Timer::after_secs(5).await;
        }
    }
}
