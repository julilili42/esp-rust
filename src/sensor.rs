#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embedded_hal_compat::{Reverse, ReverseCompat};
use esp_backtrace as _;
use esp_hal::{
    Blocking,
    delay::Delay,
    i2c::master::{Config, ConfigError, I2c},
    peripherals::{FROM_CPU_INTR0, TIMG0},
    time::{Duration, Rate},
    timer::timg::TimerGroup,
};
use esp_println::{self as _};
use mpu6050::Mpu6050;

#[derive(Debug)]
pub struct AccData {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

struct MpuDelay(Delay);
impl embedded_hal_02::blocking::delay::DelayMs<u8> for MpuDelay {
    fn delay_ms(&mut self, ms: u8) {
        self.0.delay(Duration::from_millis(u64::from(ms)));
    }
}

pub fn start_rtos(timg0: TIMG0<'static>, interrupt: FROM_CPU_INTR0<'static>) {
    let timg0 = TimerGroup::new(timg0);
    esp_rtos::start(timg0.timer0, interrupt);
}

pub fn bus_setup() -> Result<I2c<'static, Blocking>, ConfigError> {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let i2c_config = Config::default().with_frequency(Rate::from_khz(100));

    start_rtos(peripherals.TIMG0, peripherals.FROM_CPU_INTR0);

    let i2c_bus = I2c::new(peripherals.I2C0, i2c_config)?
        .with_sda(peripherals.GPIO2)
        .with_scl(peripherals.GPIO3);

    Ok(i2c_bus)
}

pub fn initialize_mpu(
    i2c_bus: I2c<'static, Blocking>,
) -> Result<
    Mpu6050<Reverse<I2c<'static, Blocking>>>,
    mpu6050::Mpu6050Error<esp_hal::i2c::master::Error>,
> {
    let delay = Delay::new();
    let mut mpu_delay = MpuDelay(delay);
    let mut mpu = Mpu6050::new(i2c_bus.reverse());
    mpu.init(&mut mpu_delay)?;

    Ok(mpu)
}
