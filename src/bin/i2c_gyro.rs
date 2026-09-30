#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use anyhow::Result;
use defmt::{error, info};
use embedded_hal_compat::{Reverse, ReverseCompat};
use esp_backtrace as _;
use esp_hal::{
    Blocking,
    delay::Delay,
    i2c::master::{Config, I2c},
    main,
    time::{Duration, Rate},
};
use esp_println::{self as _, println};
use heapless::Vec;
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

#[derive(Debug)]
struct AccData {
    x: f32,
    y: f32,
    z: f32,
}

fn bus_setup() -> Result<I2c<'static, Blocking>> {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let i2c_config = Config::default().with_frequency(Rate::from_khz(100));

    let i2c_bus = I2c::new(peripherals.I2C0, i2c_config)?
        .with_sda(peripherals.GPIO2)
        .with_scl(peripherals.GPIO3);

    Ok(i2c_bus)
}

fn initialize_mpu(
    i2c_bus: I2c<'static, Blocking>,
) -> Result<Mpu6050<Reverse<I2c<'static, Blocking>>>> {
    let delay = Delay::new();
    let mut mpu_delay = MpuDelay(delay);
    let mut mpu = Mpu6050::new(i2c_bus.reverse());
    mpu.init(&mut mpu_delay).unwrap();

    Ok(mpu)
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    esp_alloc::heap_allocator!(size: 32 * 1024);

    let i2c_bus = match bus_setup() {
        Ok(bus) => bus,
        Err(e) => {
            error!("Failed to set-up i2c bus: {}", defmt::Display2Format(&e));
            loop {
                Delay::new().delay(Duration::from_millis(1000));
            }
        }
    };

    let mut mpu = match initialize_mpu(i2c_bus) {
        Ok(m) => {
            info!("Initialized mpu");
            m
        }
        Err(e) => {
            error!("Failed to initialize mpu: {}", defmt::Display2Format(&e));
            loop {
                Delay::new().delay(Duration::from_millis(1000));
            }
        }
    };

    let delay = Delay::new();
    let mut buffer: Vec<AccData, 32> = Vec::new();
    let mut i = 0;
    loop {
        if let Ok(acc) = mpu.get_acc() {
            info!("Acc - X: {}, Y: {}, Z: {}", acc.x, acc.y, acc.z);
            if i == 31 {
                println!("{:?}", buffer);
                buffer.clear();
                i = 0;
            }
            buffer
                .push(AccData {
                    x: acc.x,
                    y: acc.y,
                    z: acc.z,
                })
                .unwrap();

            i += 1;
        }
        delay.delay(Duration::from_millis(500));
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.2.2/examples
}
