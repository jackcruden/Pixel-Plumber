//! Scalar density field — the source of truth for terrain.
//!
//! Positive = solid, negative = empty, zero = surface. Polygons are a derived
//! cache (see `contour`); destruction and accretion are pure arithmetic on
//! this field and are always valid. A parallel `u8` grid stores the material
//! index for each cell (meaningful only where density > 0).

pub const CHUNK: usize = 64;

#[derive(Clone)]
pub struct DensityField {
    pub width: usize,
    pub height: usize,
    pub density: Vec<f32>,
    pub material: Vec<u8>,
    pub chunks_x: usize,
    pub chunks_y: usize,
    pub dirty: Vec<bool>,
}

/// One radial brush application, shared by destruction and accretion (they are
/// the same operation with opposite sign).
#[derive(Clone, Copy, Debug)]
pub struct Brush {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    /// Signed strength: negative excavates, positive accretes.
    pub strength: f32,
    /// Material written where accretion turns a cell solid.
    pub material: u8,
}

impl DensityField {
    pub fn new(width: usize, height: usize) -> Self {
        let chunks_x = width.div_ceil(CHUNK);
        let chunks_y = height.div_ceil(CHUNK);
        Self {
            width,
            height,
            density: vec![-1.0; width * height],
            material: vec![0; width * height],
            chunks_x,
            chunks_y,
            dirty: vec![true; chunks_x * chunks_y],
        }
    }

    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> f32 {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            // Outside the level reads as solid so nothing escapes the bounds.
            1.0
        } else {
            self.density[y as usize * self.width + x as usize]
        }
    }

    #[inline]
    pub fn material_at(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            0
        } else {
            self.material[y as usize * self.width + x as usize]
        }
    }

    /// Bilinear sample of the density at a continuous position.
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        let xf = x - 0.5;
        let yf = y - 0.5;
        let x0 = xf.floor() as i32;
        let y0 = yf.floor() as i32;
        let tx = xf - x0 as f32;
        let ty = yf - y0 as f32;
        let a = self.get(x0, y0) + tx * (self.get(x0 + 1, y0) - self.get(x0, y0));
        let b = self.get(x0, y0 + 1) + tx * (self.get(x0 + 1, y0 + 1) - self.get(x0, y0 + 1));
        a + ty * (b - a)
    }

    /// Central-difference gradient of the sampled field (points into solid).
    pub fn gradient(&self, x: f32, y: f32) -> (f32, f32) {
        let e = 0.75;
        let gx = self.sample(x + e, y) - self.sample(x - e, y);
        let gy = self.sample(x, y + e) - self.sample(x, y - e);
        (gx, gy)
    }

    #[inline]
    pub fn solid_at(&self, x: f32, y: f32) -> bool {
        self.sample(x, y) > 0.0
    }

    pub fn mark_dirty_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let cx0 = (x0.max(0) as usize / CHUNK).min(self.chunks_x - 1);
        let cy0 = (y0.max(0) as usize / CHUNK).min(self.chunks_y - 1);
        let cx1 = (x1.max(0) as usize / CHUNK).min(self.chunks_x - 1);
        let cy1 = (y1.max(0) as usize / CHUNK).min(self.chunks_y - 1);
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                self.dirty[cy * self.chunks_x + cx] = true;
            }
        }
    }

    pub fn clear_dirty(&mut self) {
        self.dirty.iter_mut().for_each(|d| *d = false);
    }

    /// Apply a radial smoothstep-falloff brush. Soft edges give crumbling
    /// rather than cookie-cutter holes. Returns the total absolute density
    /// actually changed (used by accretion accounting tests).
    ///
    /// `protect` is a per-cell predicate: cells for which it returns true are
    /// untouched (e.g. indestructible pipework during excavation).
    pub fn apply_brush<F: Fn(u8) -> bool>(&mut self, b: Brush, protect: F) -> f32 {
        self.apply_brush_with(b, protect, |_, _| {})
    }

    /// As `apply_brush`, additionally invoking `on_remove(material, amount)`
    /// for every cell whose solid density decreased (mineral harvesting).
    pub fn apply_brush_with<F: Fn(u8) -> bool, G: FnMut(u8, f32)>(
        &mut self,
        b: Brush,
        protect: F,
        mut on_remove: G,
    ) -> f32 {
        let r = b.radius.max(0.01);
        let x0 = (b.x - r).floor() as i32;
        let y0 = (b.y - r).floor() as i32;
        let x1 = (b.x + r).ceil() as i32;
        let y1 = (b.y + r).ceil() as i32;
        let mut changed = 0.0;
        for cy in y0.max(0)..=y1.min(self.height as i32 - 1) {
            for cx in x0.max(0)..=x1.min(self.width as i32 - 1) {
                let dx = cx as f32 + 0.5 - b.x;
                let dy = cy as f32 + 0.5 - b.y;
                let d = (dx * dx + dy * dy).sqrt();
                if d >= r {
                    continue;
                }
                let i = cy as usize * self.width + cx as usize;
                if protect(self.material[i]) {
                    continue;
                }
                // smoothstep falloff: full strength at centre, 0 at radius.
                let t = 1.0 - d / r;
                let fall = t * t * (3.0 - 2.0 * t);
                let before = self.density[i];
                let after = (before + b.strength * fall).clamp(-1.0, 1.0);
                if b.strength > 0.0 && before <= 0.0 && after > 0.0 {
                    self.material[i] = b.material;
                }
                if after < before && before > 0.0 {
                    on_remove(self.material[i], before.min(1.0) - after.max(0.0).min(1.0));
                }
                self.density[i] = after;
                changed += (after - before).abs();
            }
        }
        if changed > 0.0 {
            self.mark_dirty_rect(x0, y0, x1, y1);
        }
        changed
    }

    /// Total positive (solid) density in the field. Terrain monotonicity tests
    /// assert this only decreases absent accretion.
    pub fn total_solid(&self) -> f64 {
        self.density.iter().filter(|d| **d > 0.0).map(|d| *d as f64).sum()
    }

    /// Order-independent content hash for exact snapshot tests of field ops.
    /// Field operations must be exact-hash testable (determinism contract).
    pub fn content_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for (i, d) in self.density.iter().enumerate() {
            h ^= (d.to_bits() as u64).wrapping_add(i as u64);
            h = h.wrapping_mul(0x100000001b3);
            h ^= self.material[i] as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}
