# esp-rust

Small Rust experiments on the **ESP32-C6-DevKitC-1** using `esp-hal` and `no_std`.

## Getting started

Requirements: Rust ≥ 1.95, `espflash`, and the board connected via USB.

```sh
cargo install espflash
cargo run --bin i2c_gyro
```

`cargo run` builds and flashes the example and opens the serial monitor.

## Examples

| Binary             | Description                                                                    |
| ------------------ | ------------------------------------------------------------------------------ |
| `external_led`     | Blink an LED on GPIO23                                                         |
| `smart_led`        | Control the RGB LED on GPIO8                                                   |
| `button_press`     | Change RGB colors with the button on GPIO9                                     |
| `button_interrupt` | Handle the button on GPIO9 using an interrupt                                  |
| `i2c_gyro`         | Read the MPU6050: acceleration (g), angular velocity (rad/s), temperature (°C) |
| `embassy`          | Run concurrent async tasks with Embassy                                        |
| `websocket`        | Send text over Wi-Fi/WebSocket with automatic reconnect                        |
| `i2c_ws`           | Stream MPU6050 acceleration in JSON batches, with ping/pong and reconnect      |
| `lora_tx`          | Send numbered LoRa pings and check the echoed reply                            |
| `lora_rx`          | Receive LoRa pings, print RSSI/SNR and echo the packet                          |

To run another example, replace the name after `--bin`.

MPU6050 wiring: **VCC → 3V3, GND → G, SDA → GPIO2, SCL → GPIO3**.

## Live sensor plot

Requires Python ≥ 3.13 and `uv`. Start the WebSocket server and live x/y/z plot:

```sh
cd server
uv run server
```

Received batches are appended to `measurements.jsonl` in the working directory
(`server/` with the command above), one WebSocket message per line.

Choose the Wi-Fi mode at build/flash time (`station` is the default). Both modes
connect to the WebSocket server on the laptop at port 8000, path `/ws`.

Join an existing network, using your laptop's LAN IPv4 as `WS_IP`:

```sh
WIFI_MODE=station SSID="your-network" PASSWORD="your-password" WS_IP=192.168.0.221 cargo run --bin i2c_ws
```

For a direct connection without a router or phone, let the ESP create the network:

```sh
WIFI_MODE=ap SSID="esp-washer" PASSWORD="test12345" cargo run --bin i2c_ws
```

Connect the Mac to `esp-washer`. In its Wi-Fi settings, under **Details → TCP/IP**,
set **Configure IPv4 → Manually**, IP `192.168.4.2`, subnet mask `255.255.255.0`;
leave router and DNS empty. The ESP uses `192.168.4.1`; `WS_IP` defaults to
`192.168.4.2` in AP mode. No DHCP server is included. `SSID` and `PASSWORD` name
and protect the ESP network in this mode; use a password of at least 8 characters.
Keep the Python server running on the Mac. Return IPv4 configuration to DHCP when
rejoining your normal network.

The same variables work with `--bin websocket`. Without `WS_IP`, station mode
keeps the previous default `192.168.0.221`. Check both firmware builds with
`sh scripts/check-wifi.sh` (no flashing).

## LoRa range test

Flash `cargo run --bin lora_tx` onto the basement ESP and
`cargo run --bin lora_rx` onto the upstairs ESP. Both use the existing RFM95W
wiring: **SCK → GPIO6, MISO → GPIO20, MOSI → GPIO7, CS → GPIO10, RST → GPIO11,
VCC → 3V3, GND → G**. Connect an appropriate 868 MHz antenna to each module.

The settings prioritize range: **869.525 MHz, SF12, 62.5 kHz bandwidth,
coding rate 4/8, CRC**. The existing driver uses about **17 dBm** TX power and
automatically enables low data rate optimization. Narrower bandwidth is more
sensitive to oscillator drift; [Semtech recommends a TCXO below 62.5 kHz](https://cdn.sparkfun.com/assets/7/7/3/2/2/SX1276_Datasheet.pdf).

TX sends a four-byte packet number and accepts only the identical reply. Both
boards print packet RSSI and SNR. Each packet takes about 1.9 seconds on air;
TX pauses for 30 seconds between attempts and RX pauses for 20 seconds after
replying, keeping each transmitter below the 10% duty-cycle limit for this band
in Germany ([Bundesnetzagentur, Vfg 91/2025](https://www.bundesnetzagentur.de/DE/Fachthemen/Telekommunikation/Frequenzen/Allgemeinzuteilungen/_DL/vfg91_2025.pdf?__blob=publicationFile&v=3)).
A `PONG` on TX confirms both directions. Reception through reinforced
concrete depends on the building and antenna placement; first test side by side.

Personal notes are in [notes](notes).
