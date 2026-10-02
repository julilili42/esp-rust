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
use embassy_time::{Duration, Timer};
use embedded_websocket::WebSocketClient;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, main, rng::Rng};
use esp_println as _;
use esp_rust::{
    websocket::{connect_tcp, ws_handshake, ws_send},
    wifi,
};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let stack = match wifi::connect(spawner, peripherals, SSID, PASSWORD).await {
        Ok(stack) => stack,
        Err(e) => {
            error!("Wifi error: {}", defmt::Debug2Format(&e));
            loop {
                Timer::after(Duration::from_secs(1)).await;
            }
        }
    };

    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];
    let mut stream = connect_tcp(stack, &mut rx_buf, &mut tx_buf).await;

    let mut write_buf = [0; 4000];
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
        let send = ws_send(
            &mut websocket,
            &mut stream,
            &mut write_buf,
            "hello from esp",
        )
        .await;

        match send {
            Ok(_) => info!("Message send."),
            Err(e) => error!("Send failed: {}", defmt::Debug2Format(&e)),
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}
