#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::net::Ipv4Addr;

use defmt::{error, info};
use embassy_net::{Stack, tcp::TcpSocket};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use embedded_websocket::{
    Client, Error::HttpHeaderIncomplete, WebSocket, WebSocketOptions, WebSocketReadResult,
};
use esp_backtrace as _;
use esp_hal::rng::Rng;
use esp_println as _;

pub async fn connect_tcp<'a>(
    stack: Stack<'a>,
    rx_buf: &'a mut [u8],
    tx_buf: &'a mut [u8],
) -> TcpSocket<'a> {
    let address = Ipv4Addr::new(192, 168, 0, 221);
    info!("Connecting to: {}", address);
    let mut stream = TcpSocket::new(stack, rx_buf, tx_buf);
    match stream.connect((address, 8000)).await {
        Ok(_) => info!("TCP Connected."),
        Err(e) => {
            error!(
                "Failed to establish TCP connection: {}",
                defmt::Debug2Format(&e)
            );
            Timer::after(Duration::from_secs(1)).await;
        }
    }
    stream
}

#[warn(clippy::large_stack_frames)]
pub async fn ws_handshake(
    websocket: &mut WebSocket<Rng, Client>,
    stream: &mut TcpSocket<'_>,
    write_buf: &mut [u8],
    read_buf: &mut [u8],
) -> Result<(), embedded_websocket::Error> {
    let websocket_options = WebSocketOptions {
        path: "/ws",
        host: "192.168.0.221:8000",
        origin: "http://192.168.0.221:8000",
        sub_protocols: None,
        additional_headers: None,
    };
    let (len, key) = websocket.client_connect(&websocket_options, write_buf)?;

    stream
        .write_all(&write_buf[..len])
        .await
        .map_err(|_| embedded_websocket::Error::Unknown)?;

    let mut received = 0;
    loop {
        assert!(received < read_buf.len(), "Handshake buffer full");

        let n = stream
            .read(&mut read_buf[received..])
            .await
            .map_err(|_| embedded_websocket::Error::Unknown)?;
        assert!(n > 0, "Server disconnected");
        received += n;

        if read_buf[..received].windows(4).any(|w| w == b"\r\n\r\n") {
            websocket.client_accept(&key, &read_buf[..received])?;
            break;
        }
    }
    Ok(())
}

pub async fn ws_manage_heartbeat(
    websocket: &mut WebSocket<Rng, Client>,
    stream: &mut TcpSocket<'_>,
    read_buf: &mut [u8],
) -> Result<(), embedded_websocket::Error> {
    let mut received = 0;

    let mut ping_payload_buf = [0u8; 64];

    loop {
        let n = stream
            .read(&mut read_buf[received..])
            .await
            .map_err(|_| embedded_websocket::Error::Unknown)?;

        if n == 0 {
            return Err(embedded_websocket::Error::Unknown);
        }

        received += n;

        match websocket.read(&read_buf[..received], &mut ping_payload_buf) {
            Ok(parser) => {
                let bytes_written = parser.len_to;
                match parser.message_type {
                    embedded_websocket::WebSocketReceiveMessageType::Ping => {
                        let mut out_buf = [0u8; 128];
                        if let Ok(len) = websocket.write(
                            embedded_websocket::WebSocketSendMessageType::Pong,
                            true,
                            &ping_payload_buf[..bytes_written],
                            &mut out_buf,
                        ) {
                            let _ = stream.write_all(&out_buf[..len]).await;
                        };
                        info!("Ping received, Pong send");
                        received = 0;
                        continue;
                    }
                    _ => {}
                }
            }
            Err(HttpHeaderIncomplete) => continue,
            Err(e) => return Err(e),
        }
    }
}

pub async fn ws_send(
    websocket: &mut WebSocket<Rng, Client>,
    stream: &mut TcpSocket<'_>,
    write_buf: &mut [u8],
    message: &str,
) -> Result<(), embedded_websocket::Error> {
    let len = websocket.write(
        embedded_websocket::WebSocketSendMessageType::Text,
        true,
        message.as_bytes(),
        write_buf,
    )?;

    stream
        .write_all(&write_buf[..len])
        .await
        .map_err(|_| embedded_websocket::Error::Unknown)?;

    Ok(())
}
