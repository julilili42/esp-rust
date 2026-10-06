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

#[derive(Debug, serde::Serialize)]
pub struct AccData {
    pub raw: Option<RawData>,
    pub ema: Option<Ema>,
    pub rms: Option<Rms>,
}
