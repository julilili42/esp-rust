#[derive(Debug, serde::Serialize)]
pub struct RawData {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Ema {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, serde::Serialize)]
pub struct Rms {
    pub value: f32,
}

pub fn calc_rms(x: f32, y: f32, z: f32) -> f32 {
    let x_hat = libm::powf(x, 2.0);
    let y_hat = libm::powf(y, 2.0);
    let z_hat = libm::powf(z, 2.0);

    libm::sqrtf(x_hat + y_hat + z_hat)
}

pub struct EmaFilter {
    pub value: Option<Ema>,
    pub alpha: f32,
}

impl EmaFilter {
    pub fn new(alpha: f32) -> Self {
        EmaFilter { value: None, alpha }
    }

    pub fn update(&mut self, input: Ema) -> Ema {
        let value = match self.value {
            Some(ref prev) => {
                let ema_x = (input.x * self.alpha) + (prev.x * (1.0 - self.alpha));
                let ema_y = (input.y * self.alpha) + (prev.y * (1.0 - self.alpha));
                let ema_z = (input.z * self.alpha) + (prev.z * (1.0 - self.alpha));
                Ema {
                    x: ema_x,
                    y: ema_y,
                    z: ema_z,
                }
            }
            None => input,
        };

        self.value = Some(value.clone());
        value
    }
}

pub struct MovementDetector {
    baseline: EmaFilter,
    samples: u8,
    energy: f32,
    threshold: Option<f32>,
    active_windows: u8,
}

impl Default for MovementDetector {
    fn default() -> Self {
        Self {
            baseline: EmaFilter::new(0.05),
            samples: 0,
            energy: 0.0,
            threshold: None,
            active_windows: 0,
        }
    }
}

impl MovementDetector {
    pub fn update(&mut self, input: Ema) -> bool {
        let baseline = self.baseline.update(input.clone());
        let delta = calc_rms(
            input.x - baseline.x,
            input.y - baseline.y,
            input.z - baseline.z,
        );
        self.energy += delta * delta;
        self.samples += 1;

        let window = if self.threshold.is_none() { 100 } else { 10 };
        if self.samples < window {
            return false;
        }
        let energy = self.energy / f32::from(self.samples);
        self.energy = 0.0;
        self.samples = 0;

        let Some(threshold) = self.threshold else {
            // ponytail: measured on this MPU6050; remeasure after changing sensor or mounting.
            self.threshold = Some((1.51814 * energy).max(9.375_026e-5));
            return false;
        };
        self.active_windows = if energy > threshold {
            self.active_windows.saturating_add(1)
        } else {
            0
        };
        self.active_windows >= 3
    }
}

#[derive(Debug, serde::Serialize)]
pub struct AccData {
    pub raw: Option<RawData>,
    pub ema: Option<Ema>,
    pub rms: Option<Rms>,
}

#[cfg(test)]
mod tests {
    use super::{Ema, MovementDetector};

    fn sample([x, y, z]: [f32; 3]) -> Ema {
        Ema { x, y, z }
    }

    #[test]
    fn ignores_gravity_noise_and_isolated_spikes() {
        for rest in [[0.0, 0.0, 1.0], [0.0, -1.0, 0.0]] {
            let mut detector = MovementDetector::default();
            for i in 0..300 {
                let noise = if i % 2 == 0 { 0.004 } else { -0.004 };
                assert!(!detector.update(sample([
                    rest[0] + noise,
                    rest[1] - noise,
                    rest[2] + noise,
                ])));
            }
            assert!(!detector.update(sample([rest[0] + 0.04, rest[1], rest[2]])));
            for _ in 0..50 {
                assert!(!detector.update(sample(rest)));
            }
            for i in 0..20 {
                let transient = if i % 2 == 0 { 0.03 } else { -0.03 };
                assert!(!detector.update(sample([rest[0] + transient, rest[1], rest[2]])));
            }
            for _ in 0..50 {
                assert!(!detector.update(sample(rest)));
            }
        }
    }

    #[test]
    fn calibrates_to_resting_noise() {
        let mut detector = MovementDetector::default();
        for i in 0..300 {
            let noise = if i % 2 == 0 { 0.015 } else { -0.015 };
            assert!(!detector.update(sample([noise, -noise, 1.0 + noise])));
        }
        let detected = (0..100).any(|i| {
            let noise = if i % 2 == 0 { 0.015 } else { -0.015 };
            let vibration = 0.03 * libm::sinf(core::f32::consts::TAU * 10.0 * i as f32 / 100.0);
            detector.update(sample([noise + vibration, -noise, 1.0 + noise]))
        });
        assert!(detected);
    }

    #[test]
    fn detects_small_vibrations_on_every_axis() {
        for axis in 0..3 {
            for frequency in [5.0, 10.0, 25.0, 40.0] {
                let mut detector = MovementDetector::default();
                let rest = [0.0, 0.0, 1.0];
                for _ in 0..100 {
                    assert!(!detector.update(sample(rest)));
                }
                let detected = (0..100).any(|i| {
                    let mut acc = rest;
                    acc[axis] +=
                        0.015 * libm::sinf(core::f32::consts::TAU * frequency * i as f32 / 100.0);
                    detector.update(sample(acc))
                });
                assert!(detected, "axis {axis}, frequency {frequency}");
            }
        }
    }
}
