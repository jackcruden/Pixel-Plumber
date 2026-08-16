//! Headless rendering of simulation state to RGBA frames, with optional debug
//! overlays (contours, heat, chunk dirty state).

use image::{Rgba, RgbaImage};
use sim::contour::contour_full;
use sim::field::CHUNK;
use sim::player::PLAYER_RADIUS;
use sim::Sim;

pub const SCALE: u32 = 2;

#[derive(Clone, Copy, Debug, Default)]
pub struct Overlays {
    pub contours: bool,
    pub heat: bool,
    pub dirty_chunks: bool,
}

fn put(img: &mut RgbaImage, x: i32, y: i32, c: Rgba<u8>) {
    if x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
        img.put_pixel(x as u32, y as u32, c);
    }
}

fn blend(base: Rgba<u8>, over: (u8, u8, u8), alpha: f32) -> Rgba<u8> {
    let a = alpha.clamp(0.0, 1.0);
    Rgba([
        (base[0] as f32 * (1.0 - a) + over.0 as f32 * a) as u8,
        (base[1] as f32 * (1.0 - a) + over.1 as f32 * a) as u8,
        (base[2] as f32 * (1.0 - a) + over.2 as f32 * a) as u8,
        255,
    ])
}

pub fn render(sim: &Sim, overlays: Overlays) -> RgbaImage {
    let w = sim.field.width as u32;
    let h = sim.field.height as u32;
    let mut img = RgbaImage::new(w * SCALE, h * SCALE);

    // Terrain from the density field, coloured by material, edge-softened.
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let d = sim.field.density[i];
            let c = if d > 0.0 {
                let m = sim.materials.get(sim.field.material[i]);
                let depth_shade = 1.0 - 0.25 * (y as f32 / h as f32);
                let solidity = 0.75 + 0.25 * d.min(1.0);
                Rgba([
                    (m.colour.0 * 255.0 * depth_shade * solidity) as u8,
                    (m.colour.1 * 255.0 * depth_shade * solidity) as u8,
                    (m.colour.2 * 255.0 * depth_shade * solidity) as u8,
                    255,
                ])
            } else {
                Rgba([16, 14, 20, 255])
            };
            for sy in 0..SCALE {
                for sx in 0..SCALE {
                    img.put_pixel(x * SCALE + sx, y * SCALE + sy, c);
                }
            }
        }
    }

    if overlays.heat {
        for y in 0..h {
            for x in 0..w {
                let t = sim.heat.sample(x as f32 + 0.5, y as f32 + 0.5);
                if t > 0.05 {
                    for sy in 0..SCALE {
                        for sx in 0..SCALE {
                            let px = x * SCALE + sx;
                            let py = y * SCALE + sy;
                            let base = *img.get_pixel(px, py);
                            img.put_pixel(px, py, blend(base, (255, 60, 0), t * 0.35));
                        }
                    }
                }
            }
        }
    }

    if overlays.dirty_chunks {
        for cy in 0..sim.field.chunks_y {
            for cx in 0..sim.field.chunks_x {
                if sim.field.dirty[cy * sim.field.chunks_x + cx] {
                    let x0 = (cx * CHUNK) as i32 * SCALE as i32;
                    let y0 = (cy * CHUNK) as i32 * SCALE as i32;
                    let x1 = x0 + (CHUNK as i32 * SCALE as i32) - 1;
                    let y1 = y0 + (CHUNK as i32 * SCALE as i32) - 1;
                    for x in x0..=x1 {
                        put(&mut img, x, y0, Rgba([255, 0, 255, 255]));
                        put(&mut img, x, y1, Rgba([255, 0, 255, 255]));
                    }
                    for y in y0..=y1 {
                        put(&mut img, x0, y, Rgba([255, 0, 255, 255]));
                        put(&mut img, x1, y, Rgba([255, 0, 255, 255]));
                    }
                }
            }
        }
    }

    // Fluid particles as soft dots (the real shell renders metaballs; here we
    // just need to see where the fluid is).
    for i in 0..sim.fluid.len() {
        let p = sim.fluid.pos[i];
        let k = sim.fluid.kind[i];
        let m = sim.materials.get(k);
        let mut col = (
            (m.colour.0 * 255.0) as u8,
            (m.colour.1 * 255.0) as u8,
            (m.colour.2 * 255.0) as u8,
        );
        if k == sim.ids.water {
            // Tint toward contaminant green as purity drops.
            let pu = sim.fluid.purity[i];
            col = (
                (col.0 as f32 * pu + 110.0 * (1.0 - pu)) as u8,
                (col.1 as f32 * pu + 140.0 * (1.0 - pu)) as u8,
                (col.2 as f32 * pu + 40.0 * (1.0 - pu)) as u8,
            );
        }
        let cx = (p.x * SCALE as f32) as i32;
        let cy = (p.y * SCALE as f32) as i32;
        let r = SCALE as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    put(&mut img, cx + dx, cy + dy, Rgba([col.0, col.1, col.2, 255]));
                }
            }
        }
    }

    if overlays.contours {
        for l in contour_full(&sim.field) {
            for w in l.windows(2) {
                draw_line(&mut img, w[0], w[1], Rgba([255, 255, 0, 255]));
            }
            if l.len() > 2 {
                draw_line(&mut img, l[l.len() - 1], l[0], Rgba([255, 255, 0, 255]));
            }
        }
    }

    // Intake region.
    let it = sim.level.intake;
    for x in (it.x as i32)..((it.x + it.w) as i32) {
        for y in (it.y as i32)..((it.y + it.h) as i32) {
            let base = *img.get_pixel_checked(x as u32 * SCALE, y as u32 * SCALE)
                .unwrap_or(&Rgba([0, 0, 0, 255]));
            let c = blend(base, (80, 220, 255), 0.25);
            for sy in 0..SCALE {
                for sx in 0..SCALE {
                    put(&mut img, x * SCALE as i32 + sx as i32, y * SCALE as i32 + sy as i32, c);
                }
            }
        }
    }

    // Player.
    let pp = sim.player.pos;
    let cx = (pp.x * SCALE as f32) as i32;
    let cy = (pp.y * SCALE as f32) as i32;
    let r = (PLAYER_RADIUS * SCALE as f32) as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                put(&mut img, cx + dx, cy + dy, Rgba([255, 235, 200, 255]));
            }
        }
    }
    img
}

fn draw_line(img: &mut RgbaImage, a: (f32, f32), b: (f32, f32), c: Rgba<u8>) {
    let s = SCALE as f32;
    let (x0, y0) = (a.0 * s, a.1 * s);
    let (x1, y1) = (b.0 * s, b.1 * s);
    let n = ((x1 - x0).abs().max((y1 - y0).abs()).ceil() as i32).max(1);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        put(
            img,
            (x0 + (x1 - x0) * t) as i32,
            (y0 + (y1 - y0) * t) as i32,
            c,
        );
    }
}
