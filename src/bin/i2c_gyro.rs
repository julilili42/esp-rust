#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::info;
use embedded_hal_compat::ReverseCompat;
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    i2c::master::{Config, I2c},
    main,
    time::{Duration, Rate},
};
use esp_println::{self as _, println};
use mpu6050::Mpu6050;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

struct MpuDelay(Delay);
impl embedded_hal_02::blocking::delay::DelayMs<u8> for MpuDelay {
    fn delay_ms(&mut self, ms: u8) {
        self.0.delay(Duration::from_millis(u64::from(ms)));
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    // generator version: 1.4.0
    // generator parameters: -o esp32c6 -o unstable-hal -o defmt -o esp-backtrace -o zed

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let i2c_config = Config::default().with_frequency(Rate::from_khz(100));

    let i2c_bus = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO2)
        .with_scl(peripherals.GPIO3);

    let duration = Duration::from_millis(500);
    let delay = Delay::new();
    let mut mpu_delay = MpuDelay(delay);
    let mut mpu = Mpu6050::new(i2c_bus.reverse());

    mpu.init(&mut mpu_delay).unwrap();
    println!("initialized");

    loop {
        if let Ok(acc) = mpu.get_acc() {
            info!("Acc - X: {}, Y: {}, Z: {}", acc.x, acc.y, acc.z);
        }

        if let Ok(gyro) = mpu.get_gyro() {
            info!("Gyro - X: {}, Y: {}, Z: {}", gyro.x, gyro.y, gyro.z);
        }

        if let Ok(temp) = mpu.get_temp() {
            info!("Temperatur: {} Grad Celsius", temp);
        }

        info!("-----------------------------------------");
        delay.delay(duration);
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.2.2/examples
}
