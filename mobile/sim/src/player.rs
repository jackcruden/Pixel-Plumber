//! Player and shots. Placeholder character controller collides directly
//! against the density field by sampling (Rapier2D + contour colliders is the
//! upgrade path once M2 feel is signed off — see AGENTS.md).

use crate::field::{Brush, DensityField};
use crate::materials::MaterialTable;
use glam::Vec2;

pub const PLAYER_RADIUS: f32 = 2.6;

#[derive(Clone, Copy, Debug, Default)]
pub struct InputFrame {
    /// -1..1 horizontal move.
    pub move_x: f32,
    pub jump: bool,
    /// Aim point in world (cell) coordinates.
    pub aim_x: f32,
    pub aim_y: f32,
    pub firing: bool,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub on_ground: bool,
    pub health: f32,
    pub tool_tier: u32,
    pub credits: i64,
    pub fire_cooldown: f32,
    pub aim: Vec2,
}

impl Player {
    pub fn new(spawn: (f32, f32)) -> Self {
        Self {
            pos: Vec2::new(spawn.0, spawn.1),
            vel: Vec2::ZERO,
            on_ground: false,
            health: 1.0,
            tool_tier: 0,
            credits: 0,
            fire_cooldown: 0.0,
            aim: Vec2::ZERO,
        }
    }

    pub fn step(&mut self, dt: f32, input: &InputFrame, field: &DensityField) {
        const ACCEL: f32 = 340.0;
        const MAX_SPEED: f32 = 36.0;
        const FRICTION: f32 = 8.0;
        const GRAVITY: f32 = 240.0;
        const JUMP: f32 = 66.0;

        self.aim = Vec2::new(input.aim_x, input.aim_y);
        self.vel.x += input.move_x.clamp(-1.0, 1.0) * ACCEL * dt;
        self.vel.x -= self.vel.x * FRICTION * dt;
        self.vel.x = self.vel.x.clamp(-MAX_SPEED, MAX_SPEED);
        self.vel.y += GRAVITY * dt;
        if input.jump && self.on_ground {
            self.vel.y = -JUMP;
        }

        // Axis-separated movement in sub-cell steps against the sampled field.
        let r = PLAYER_RADIUS;
        let solid = |field: &DensityField, p: Vec2| -> bool {
            // Sample a small ring; cheap capsule-ish test.
            field.sample(p.x, p.y) > 0.0
                || field.sample(p.x - r, p.y) > 0.0
                || field.sample(p.x + r, p.y) > 0.0
                || field.sample(p.x, p.y - r) > 0.0
                || field.sample(p.x, p.y + r) > 0.0
                || field.sample(p.x - r * 0.7, p.y - r * 0.7) > 0.0
                || field.sample(p.x + r * 0.7, p.y - r * 0.7) > 0.0
                || field.sample(p.x - r * 0.7, p.y + r * 0.7) > 0.0
                || field.sample(p.x + r * 0.7, p.y + r * 0.7) > 0.0
        };

        let step = 0.4;
        // X axis, with step-up assist so shallow crumble edges don't stop you.
        let dx = self.vel.x * dt;
        let steps = (dx.abs() / step).ceil().max(1.0) as i32;
        let sx = dx / steps as f32;
        for _ in 0..steps {
            let next = self.pos + Vec2::new(sx, 0.0);
            if !solid(field, next) {
                self.pos = next;
            } else if !solid(field, next - Vec2::new(0.0, 1.0)) {
                self.pos = next - Vec2::new(0.0, 1.0);
            } else {
                self.vel.x = 0.0;
                break;
            }
        }
        // Y axis.
        let dy = self.vel.y * dt;
        let steps = (dy.abs() / step).ceil().max(1.0) as i32;
        let sy = dy / steps as f32;
        for _ in 0..steps {
            let next = self.pos + Vec2::new(0.0, sy);
            if !solid(field, next) {
                self.pos = next;
            } else {
                self.vel.y = 0.0;
                break;
            }
        }
        self.pos.x = self.pos.x.clamp(r + 1.0, field.width as f32 - r - 1.0);
        self.pos.y = self.pos.y.clamp(r + 1.0, field.height as f32 - r - 1.0);
        self.on_ground = solid(field, self.pos + Vec2::new(0.0, 0.6));

        self.fire_cooldown = (self.fire_cooldown - dt).max(0.0);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub pos: Vec2,
    pub vel: Vec2,
    pub alive: bool,
}

pub struct Shots {
    pub shots: Vec<Shot>,
    /// Impact positions this step (cosmetic hit feedback for the shell).
    pub impacts: Vec<Vec2>,
}

impl Shots {
    pub fn new() -> Self {
        Self {
            shots: Vec::new(),
            impacts: Vec::new(),
        }
    }

    pub fn fire(&mut self, from: Vec2, toward: Vec2, speed: f32) {
        let dir = (toward - from).normalize_or_zero();
        if dir == Vec2::ZERO {
            return;
        }
        self.shots.push(Shot {
            pos: from + dir * (PLAYER_RADIUS + 1.5),
            vel: dir * speed,
            alive: true,
        });
    }

    /// Advance shots; excavate on impact. Excavation strength scales with
    /// (1 - hardness); materials above the player's tool tier and
    /// indestructible materials are protected. Returns credits harvested.
    pub fn step(
        &mut self,
        dt: f32,
        field: &mut DensityField,
        materials: &MaterialTable,
        tool_tier: u32,
        dig_radius: f32,
        dig_strength: f32,
    ) -> i64 {
        self.impacts.clear();
        let mut harvested = 0i64;
        for s in self.shots.iter_mut() {
            if !s.alive {
                continue;
            }
            let dist = s.vel.length() * dt;
            let steps = (dist / 0.8).ceil().max(1.0) as i32;
            let inc = s.vel * (dt / steps as f32);
            for _ in 0..steps {
                s.pos += inc;
                if s.pos.x < 1.0
                    || s.pos.y < 1.0
                    || s.pos.x > field.width as f32 - 1.0
                    || s.pos.y > field.height as f32 - 1.0
                {
                    s.alive = false;
                    break;
                }
                if field.solid_at(s.pos.x, s.pos.y) {
                    let hit_mat = field.material_at(s.pos.x as i32, s.pos.y as i32);
                    let m = materials.get(hit_mat);
                    let power = dig_strength * (1.0 - m.hardness).max(0.15);
                    let mut removed_value = 0.0f32;
                    field.apply_brush_with(
                        Brush {
                            x: s.pos.x,
                            y: s.pos.y,
                            radius: dig_radius,
                            strength: -power,
                            material: 0,
                        },
                        |mat| {
                            let m = materials.get(mat);
                            m.indestructible || m.tool_tier_required > tool_tier
                        },
                        |mat, removed| {
                            let v = materials.get(mat).value;
                            if v > 0 {
                                removed_value += removed * v as f32;
                            }
                        },
                    );
                    harvested += removed_value.round() as i64;
                    self.impacts.push(s.pos);
                    s.alive = false;
                    break;
                }
            }
        }
        self.shots.retain(|s| s.alive);
        harvested
    }
}
