#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::{info, warn};
use embassy_net::{IpEndpoint, tcp::TcpSocket};
use embedded_io_async::Write;
use embedded_websocket::{
    Client, Error::HttpHeaderIncomplete, WebSocket, WebSocketClient, WebSocketOptions,
};
use esp_backtrace as _;
use esp_hal::rng::Rng;
use esp_println as _;
use heapless::{String, format};

pub async fn ws_connect<'a>(
    tcp_socket: &mut TcpSocket<'a>,
    endpoint: IpEndpoint,
    path: &str,
) -> Result<WebSocket<Rng, Client>, WsHandshakeError> {
    match reconnect_socket(tcp_socket, endpoint).await {
        Ok(_) => {
            info!("Connected successfully to TCP socket.");
            let mut ws_client = WebSocketClient::new_client(Rng::new());

            let mut write_buf = [0; 1024];
            let mut read_buf = [0; 1024];
            ws_handshake(
                endpoint,
                path,
                &mut ws_client,
                tcp_socket,
                &mut write_buf,
                &mut read_buf,
            )
            .await?;

            Ok(ws_client)
        }
        Err(e) => Err(WsHandshakeError::Tcp(e)),
    }
}

async fn reconnect_socket(
    tcp_socket: &mut TcpSocket<'_>,
    endpoint: IpEndpoint,
) -> Result<(), embassy_net::tcp::ConnectError> {
    tcp_socket.abort();

    info!("Attempting to connect to {}...", endpoint);
    tcp_socket.connect(endpoint).await
}

#[derive(Debug)]
pub enum WsHandshakeError {
    Socket(embedded_websocket::Error),
    SocketIO,
    ConnectionFailure(&'static str),
    Tcp(embassy_net::tcp::ConnectError),
    BufferOverflow,
}

#[warn(clippy::large_stack_frames)]
pub async fn ws_handshake(
    endpoint: IpEndpoint,
    path: &str,
    websocket: &mut WebSocket<Rng, Client>,
    tcp_socket: &mut TcpSocket<'_>,
    write_buf: &mut [u8],
    read_buf: &mut [u8],
) -> Result<(), WsHandshakeError> {
    let host: String<20> = format!("{}:{}", endpoint.addr, endpoint.port).unwrap();
    let origin: String<30> = format!("http://{}:{}", endpoint.addr, endpoint.port).unwrap();

    let websocket_options = WebSocketOptions {
        path,
        host: &host,
        origin: &origin,
        sub_protocols: None,
        additional_headers: None,
    };
    let (len, key) = websocket
        .client_connect(&websocket_options, write_buf)
        .map_err(|e| WsHandshakeError::Socket(e))?;

    tcp_socket
        .write_all(&write_buf[..len])
        .await
        .map_err(|_| WsHandshakeError::SocketIO)?;

    let mut received = 0;
    loop {
        if received >= read_buf.len() {
            return Err(WsHandshakeError::BufferOverflow);
        }

        let n = tcp_socket
            .read(&mut read_buf[received..])
            .await
            .map_err(|_| WsHandshakeError::SocketIO)?;

        if n == 0 {
            return Err(WsHandshakeError::ConnectionFailure("socket was closed"));
        }
        received += n;

        if read_buf[..received].windows(4).any(|w| w == b"\r\n\r\n") {
            websocket
                .client_accept(&key, &read_buf[..received])
                .map_err(|e| WsHandshakeError::Socket(e))?;
            break;
        }
    }
    Ok(())
}

pub async fn ws_manage_heartbeat(
    websocket: &mut WebSocket<Rng, Client>,
    tcp_socket: &mut TcpSocket<'_>,
    read_buf: &mut [u8],
    received: &mut usize,
) -> Result<(), WsHandshakeError> {
    let mut ping_payload_buf = [0u8; 64];

    loop {
        let n = tcp_socket
            .read(&mut read_buf[*received..])
            .await
            .map_err(|_| WsHandshakeError::SocketIO)?;

        if n == 0 {
            return Err(WsHandshakeError::ConnectionFailure("socket was closed"));
        }

        *received += n;

        match websocket.read(&read_buf[..*received], &mut ping_payload_buf) {
            Ok(parser) => {
                let bytes_written = parser.len_to;
                match parser.message_type {
                    embedded_websocket::WebSocketReceiveMessageType::Ping => {
                        let mut out_buf = [0u8; 128];
                        let len = websocket
                            .write(
                                embedded_websocket::WebSocketSendMessageType::Pong,
                                true,
                                &ping_payload_buf[..bytes_written],
                                &mut out_buf,
                            )
                            .map_err(|_| WsHandshakeError::SocketIO)?;

                        tcp_socket
                            .write_all(&out_buf[..len])
                            .await
                            .map_err(|_| WsHandshakeError::SocketIO)?;

                        info!("Ping received, Pong send");
                        *received = 0;
                        continue;
                    }
                    _ => {}
                }
            }
            Err(HttpHeaderIncomplete) => continue,
            Err(_) => return Err(WsHandshakeError::SocketIO),
        }
    }
}

pub async fn ws_send(
    websocket: &mut WebSocket<Rng, Client>,
    tcp_socket: &mut TcpSocket<'_>,
    write_buf: &mut [u8],
    message: &str,
) -> Result<(), embedded_websocket::Error> {
    let len = websocket.write(
        embedded_websocket::WebSocketSendMessageType::Text,
        true,
        message.as_bytes(),
        write_buf,
    )?;

    tcp_socket
        .write_all(&write_buf[..len])
        .await
        .map_err(|_| embedded_websocket::Error::Unknown)?;

    Ok(())
}
