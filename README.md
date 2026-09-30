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

| Binary | Description |
|---|---|
| `external_led` | Blink an LED on GPIO23 |
| `smart_led` | Control the RGB LED on GPIO8 |
| `button_press` | Change RGB colors with the button on GPIO9 |
| `button_interrupt` | Handle the button on GPIO9 using an interrupt |
| `i2c_gyro` | Read the MPU6050: acceleration (g), angular velocity (rad/s), temperature (°C) |

To run another example, replace the name after `--bin`.

MPU6050 wiring: **VCC → 3V3, GND → G, SDA → GPIO2, SCL → GPIO3**.

Personal notes are in [src/notes](src/notes).
