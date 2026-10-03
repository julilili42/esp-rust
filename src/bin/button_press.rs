#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::info;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Input, InputConfig},
    main,
    rmt::Rmt,
    time::{Duration, Rate},
};
use esp_hal_smartled::{RmtSmartLeds, WS2812_TIMING, buffer_size, color_order::Grb};
use esp_println as _;
use smart_leds::{RGB8, SmartLedsWrite};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    // generator version: 1.4.0
    // generator parameters: -o esp32c6 -o unstable-hal -o defmt -o esp-backtrace -o zed

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let button = Input::new(peripherals.GPIO9, InputConfig::default());

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let mut led = RmtSmartLeds::<{ buffer_size::<RGB8>(1) }, _, RGB8, Grb>::new(
        WS2812_TIMING,
        rmt.channel0,
        peripherals.GPIO8,
        Rate::from_mhz(80),
    )
    .unwrap();

    let colors = [
        RGB8 { r: 0, g: 100, b: 0 },
        RGB8 { r: 100, g: 0, b: 0 },
        RGB8 {
            r: 100,
            g: 0,
            b: 100,
        },
        RGB8 { r: 0, g: 0, b: 100 },
    ];
    let n = colors.len();

    let mut i = 0;

    loop {
        let wait = Delay::new();
        let pressed = button.is_low();

        if pressed {
            //let color = colors[i % n];
            info!("Blink! {}", i % n);
            let current_color = colors[i % n];
            let next_color = colors[(i + 1) % n];

            for j in 0..=100 {
                let blended = blend(current_color, next_color, j, 100);
                led.write([blended]).unwrap();
                wait.delay_millis(10);
            }

            wait.delay_millis(30);
            while button.is_low() {}
            i += 1;
        }

        wait.delay(Duration::from_millis(50));
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.2.2/examples
}

fn blend(start: RGB8, end: RGB8, step: i32, max_steps: i32) -> RGB8 {
    let interpolate = |s: u8, e: u8| {
        let start = s as i32;
        let end = e as i32;
        let res = start + (end - start) * step / max_steps;
        return res as u8;
    };

    RGB8 {
        r: interpolate(start.r, end.r),
        g: interpolate(start.g, end.g),
        b: interpolate(start.b, end.b),
    }
}
