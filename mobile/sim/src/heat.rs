//! Coarse temperature field — 1/4 the resolution of the density field.
//! Ambient baseline rises with depth (this *is* the difficulty curve; no
//! separate difficulty modifiers exist). Molten particles deposit heat, water
//! absorbs it, and the field relaxes toward ambient while diffusing.

pub const HEAT_SCALE: usize = 4;

#[derive(Clone)]
pub struct HeatField {
    pub width: usize,
    pub height: usize,
    pub temp: Vec<f32>,
    ambient: Vec<f32>,
    scratch: Vec<f32>,
}

impl HeatField {
    pub fn new(world_w: usize, world_h: usize, base: f32, per_depth: f32) -> Self {
        let width = world_w.div_ceil(HEAT_SCALE);
        let height = world_h.div_ceil(HEAT_SCALE);
        let mut ambient = vec![0.0; width * height];
        for y in 0..height {
            let a = base + per_depth * (y * HEAT_SCALE) as f32;
            for x in 0..width {
                ambient[y * width + x] = a;
            }
        }
        Self {
            width,
            height,
            temp: ambient.clone(),
            ambient,
            scratch: vec![0.0; width * height],
        }
    }

    /// Bilinear sample at a world (cell-space) position.
    pub fn sample(&self, wx: f32, wy: f32) -> f32 {
        let x = (wx / HEAT_SCALE as f32 - 0.5).clamp(0.0, self.width as f32 - 1.001);
        let y = (wy / HEAT_SCALE as f32 - 0.5).clamp(0.0, self.height as f32 - 1.001);
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let a = self.temp[y0 * self.width + x0] * (1.0 - tx) + self.temp[y0 * self.width + x1] * tx;
        let b = self.temp[y1 * self.width + x0] * (1.0 - tx) + self.temp[y1 * self.width + x1] * tx;
        a * (1.0 - ty) + b * ty
    }

    /// Add (or with negative `amount`, absorb) heat at a world position.
    pub fn deposit(&mut self, wx: f32, wy: f32, amount: f32) {
        let x = (wx / HEAT_SCALE as f32) as usize;
        let y = (wy / HEAT_SCALE as f32) as usize;
        if x < self.width && y < self.height {
            let t = &mut self.temp[y * self.width + x];
            *t = (*t + amount).clamp(0.0, 4.0);
        }
    }

    /// Diffuse and relax toward ambient. Fixed kernel, fixed order.
    pub fn step(&mut self, dt: f32) {
        let w = self.width;
        let h = self.height;
        let diff = 4.0 * dt; // diffusion rate
        let relax = 0.6 * dt; // pull toward ambient baseline
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let c = self.temp[i];
                let l = self.temp[y * w + x.saturating_sub(1)];
                let r = self.temp[y * w + (x + 1).min(w - 1)];
                let u = self.temp[y.saturating_sub(1) * w + x];
                let d = self.temp[(y + 1).min(h - 1) * w + x];
                let lap = (l + r + u + d) * 0.25 - c;
                let amb = self.ambient[i];
                self.scratch[i] = c + lap * diff + (amb - c) * relax;
            }
        }
        core::mem::swap(&mut self.temp, &mut self.scratch);
    }
}
