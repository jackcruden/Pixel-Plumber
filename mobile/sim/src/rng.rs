//! Seeded, explicit PRNG. `thread_rng` and system entropy are banned in `sim`
//! (determinism contract) — every consumer receives an `&mut Rng` through the
//! call chain.

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(0x9E3779B97F4A7C15),
        }
    }

    /// SplitMix64 step.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform in [lo, hi).
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }
}

/// Stateless integer hash used by the value-noise generator so terrain is a
/// pure function of (seed, x, y) and never depends on evaluation order.
pub fn hash2(seed: u64, x: i32, y: i32) -> u64 {
    let mut z = seed
        ^ (x as u64).wrapping_mul(0x9E3779B97F4A7C15)
        ^ (y as u64).wrapping_mul(0xC2B2AE3D27D4EB4F);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Value noise in [-1, 1] with smoothstep interpolation.
pub fn value_noise(seed: u64, x: f32, y: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = x - xi as f32;
    let fy = y - yi as f32;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let v = |dx: i32, dy: i32| -> f32 {
        (hash2(seed, xi + dx, yi + dy) >> 40) as f32 / (1u64 << 24) as f32
    };
    let a = v(0, 0) + sx * (v(1, 0) - v(0, 0));
    let b = v(0, 1) + sx * (v(1, 1) - v(0, 1));
    (a + sy * (b - a)) * 2.0 - 1.0
}

/// Fractional Brownian motion over value noise, in roughly [-1, 1].
pub fn fbm(seed: u64, x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    let mut norm = 0.0;
    for o in 0..octaves {
        sum += amp * value_noise(seed.wrapping_add(o as u64 * 0x51ED2701), x * freq, y * freq);
        norm += amp;
        amp *= gain;
        freq *= lacunarity;
    }
    if norm > 0.0 {
        sum / norm
    } else {
        0.0
    }
}
