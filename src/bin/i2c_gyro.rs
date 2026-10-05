#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::{error, info};
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, delay::Delay, main, time::Duration};
use esp_println::{self as _, println};
use esp_rust::sensor::{AccData, bus_setup, initialize_mpu};
use heapless::Vec;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    esp_alloc::heap_allocator!(size: 32 * 1024);
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let i2c_bus = match bus_setup(peripherals.I2C0, peripherals.GPIO2, peripherals.GPIO3) {
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
            error!("Failed to initialize mpu: {}", defmt::Debug2Format(&e));
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
