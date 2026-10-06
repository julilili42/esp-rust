#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

export SSID=esp-wifi-check PASSWORD=check12345
unset WIFI_MODE WS_IP
cargo build --offline --locked --bin i2c_ws --bin websocket
WIFI_MODE=station WS_IP=192.168.0.2 cargo build --offline --locked --bin i2c_ws --bin websocket
WIFI_MODE=ap cargo build --offline --locked --bin i2c_ws --bin websocket

# A misspelled mode must fail instead of silently joining the wrong network.
if WIFI_MODE=invalid cargo check --offline --locked --lib; then
    echo "Invalid WIFI_MODE was accepted" >&2
    exit 1
fi
echo "Wi-Fi mode build checks passed"
