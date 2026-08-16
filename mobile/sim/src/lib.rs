//! Pixel Plumber simulation core.
//!
//! Pure computation: no I/O, no time, no platform. `Sim::step` advances one
//! fixed timestep from explicit inputs. See AGENTS.md for the determinism
//! contract, update-order convention, and performance budget.

pub mod contour;
pub mod field;
pub mod fluid;
pub mod heat;
pub mod level;
pub mod materials;
pub mod player;
pub mod rng;

use field::{Brush, DensityField};
use fluid::{Fluid, FluidIds, FluidParams};
use glam::Vec2;
use heat::HeatField;
use level::Level;
use materials::MaterialTable;
use player::{InputFrame, Player, Shots};
use rng::Rng;

/// Fixed timestep. Render interpolates; simulation never sees variable dt.
pub const DT: f32 = 1.0 / 60.0;

pub const DIG_RADIUS: f32 = 3.4;
pub const DIG_STRENGTH: f32 = 2.4;
pub const SHOT_SPEED: f32 = 110.0;
pub const FIRE_INTERVAL: f32 = 0.11;
/// Water touching the player hurts if hot; molten always does.
pub const MOLTEN_DAMAGE_RADIUS: f32 = 4.0;
/// Credits for the first tool-tier upgrade; doubles each tier.
pub const TOOL_UPGRADE_BASE_COST: i64 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Playing,
    Won,
    Dead,
}

struct EmitterState {
    spec: level::Emitter,
    kind_idx: u8,
    accum: f32,
    emitted: u32,
}

pub struct Sim {
    pub level: Level,
    pub materials: MaterialTable,
    pub field: DensityField,
    pub heat: HeatField,
    pub fluid: Fluid,
    pub player: Player,
    pub shots: Shots,
    pub rng: Rng,
    pub tick: u64,
    pub phase: Phase,
    emitters: Vec<EmitterState>,
    pub ids: FluidIds,
    /// Mean purity of everything delivered so far, 0..1.
    pub delivered_purity: f32,
}

impl Sim {
    pub fn new(level_ron: &str, materials_ron: &str) -> Result<Self, String> {
        let level = Level::from_ron(level_ron)?;
        let materials = MaterialTable::from_ron(materials_ron)?;
        let need = |id: &str| -> Result<u8, String> {
            materials
                .index_of(id)
                .ok_or_else(|| format!("material table missing '{id}'"))
        };
        let ids = FluidIds {
            water: need("water")?,
            contaminant: need("contaminant")?,
            molten: need("molten")?,
            slag: need("slag")?,
        };

        let mut rng = Rng::new(level.seed);
        let field = generate_field(&level, &materials)?;
        let heat = HeatField::new(
            level.width,
            level.height,
            level.ambient_heat_base,
            level.ambient_heat_per_depth,
        );
        let params = FluidParams::default();
        let mut fluid = Fluid::new(params, ids, level.width, level.height);

        // Pre-filled pools, spawned on a jittered lattice at rest spacing.
        let s = params.spacing;
        let mut fill_pool = |r: &level::Rect, kind: u8, purity: f32, rng: &mut Rng| {
            let nx = (r.w / s) as i32;
            let ny = (r.h / s) as i32;
            for gy in 0..ny {
                for gx in 0..nx {
                    let p = Vec2::new(
                        r.x + (gx as f32 + 0.5) * s + rng.range_f32(-0.1, 0.1),
                        r.y + (gy as f32 + 0.5) * s + rng.range_f32(-0.1, 0.1),
                    );
                    fluid.spawn(p, Vec2::ZERO, kind, purity);
                }
            }
        };
        for pool in &level.molten_pools {
            fill_pool(pool, ids.molten, 0.0, &mut rng);
        }
        for pool in &level.contaminant_pools {
            fill_pool(pool, ids.contaminant, 0.0, &mut rng);
        }

        let emitters = level
            .emitters
            .iter()
            .map(|e| {
                Ok(EmitterState {
                    kind_idx: need(&e.kind)?,
                    spec: e.clone(),
                    accum: 0.0,
                    emitted: 0,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        let player = Player::new(level.player_spawn);
        Ok(Self {
            level,
            materials,
            field,
            heat,
            fluid,
            player,
            shots: Shots::new(),
            rng,
            tick: 0,
            phase: Phase::Playing,
            emitters,
            ids,
            delivered_purity: 0.0,
        })
    }

    /// Advance exactly one fixed timestep.
    pub fn step(&mut self, input: &InputFrame) {
        if self.phase != Phase::Playing {
            return;
        }
        self.tick += 1;

        // Player + firing.
        self.player.step(DT, input, &self.field);
        if input.firing && self.player.fire_cooldown <= 0.0 {
            self.shots.fire(
                self.player.pos,
                Vec2::new(input.aim_x, input.aim_y),
                SHOT_SPEED,
            );
            self.player.fire_cooldown = FIRE_INTERVAL;
        }
        let harvested = self.shots.step(
            DT,
            &mut self.field,
            &self.materials,
            self.player.tool_tier,
            DIG_RADIUS,
            DIG_STRENGTH,
        );
        self.player.credits += harvested;

        // Emitters. Back-pressure: stop short of the particle cap so the
        // supply throttles instead of forcing oldest-particle culls.
        let headroom = self.fluid.params.particle_cap.saturating_sub(64);
        for e in self.emitters.iter_mut() {
            if e.spec.max > 0 && e.emitted >= e.spec.max {
                continue;
            }
            if self.fluid.len() >= headroom {
                break;
            }
            e.accum += e.spec.rate * DT;
            while e.accum >= 1.0 {
                e.accum -= 1.0;
                e.emitted += 1;
                let jx = self.rng.range_f32(-0.8, 0.8);
                self.fluid.spawn(
                    Vec2::new(e.spec.x + jx, e.spec.y),
                    Vec2::new(e.spec.vx, e.spec.vy),
                    e.kind_idx,
                    1.0,
                );
                if e.spec.max > 0 && e.emitted >= e.spec.max {
                    break;
                }
            }
        }

        // Fluids.
        let intake = (
            self.level.intake.x,
            self.level.intake.y,
            self.level.intake.x + self.level.intake.w,
            self.level.intake.y + self.level.intake.h,
        );
        self.fluid.step(
            DT,
            &mut self.field,
            &mut self.heat,
            &self.materials,
            intake,
            &mut self.rng,
        );
        self.heat.step(DT);

        // Hazard damage: molten proximity and contaminated water contact.
        let pp = self.player.pos;
        let r2 = MOLTEN_DAMAGE_RADIUS * MOLTEN_DAMAGE_RADIUS;
        let mut damage = 0.0;
        for i in 0..self.fluid.len() {
            let d2 = (self.fluid.pos[i] - pp).length_squared();
            if d2 < r2 {
                let k = self.fluid.kind[i];
                if k == self.ids.molten {
                    damage += 0.5 * DT;
                } else if k == self.ids.contaminant {
                    damage += 0.12 * DT;
                }
            }
        }
        // Ambient heat also chips health when standing in hot zones.
        let t_here = self.heat.sample(pp.x, pp.y);
        if t_here > 0.85 {
            damage += (t_here - 0.85) * 0.25 * DT;
        }
        self.player.health = (self.player.health - damage).clamp(0.0, 1.0);

        // Win/lose.
        let c = &self.fluid.counters;
        self.delivered_purity = if c.delivered > 0 {
            (c.delivered_purity_sum / c.delivered as f64) as f32
        } else {
            0.0
        };
        if self.player.health <= 0.0 {
            self.phase = Phase::Dead;
        } else if c.delivered >= self.level.goal_volume as u64
            && self.delivered_purity >= self.level.goal_purity
        {
            self.phase = Phase::Won;
        }
    }

    /// Cost of the next tool-tier upgrade.
    pub fn next_upgrade_cost(&self) -> i64 {
        TOOL_UPGRADE_BASE_COST << self.player.tool_tier.min(8)
    }

    /// Spend harvested minerals on the next tool tier. Returns success.
    pub fn buy_tool_upgrade(&mut self) -> bool {
        let cost = self.next_upgrade_cost();
        if self.player.credits >= cost {
            self.player.credits -= cost;
            self.player.tool_tier += 1;
            true
        } else {
            false
        }
    }

    /// Debug/scenario helper: excavate directly at a point (tool-tier gated
    /// like a shot impact).
    pub fn dig_at(&mut self, x: f32, y: f32, radius: f32, strength: f32) {
        let materials = &self.materials;
        let tier = self.player.tool_tier;
        self.field.apply_brush(
            Brush {
                x,
                y,
                radius,
                strength: -strength.abs(),
                material: 0,
            },
            |mat| {
                let m = materials.get(mat);
                m.indestructible || m.tool_tier_required > tier
            },
        );
    }

    /// Debug/scenario helper: spawn a blob of fluid.
    pub fn spawn_blob(&mut self, x: f32, y: f32, r: f32, kind: u8, count: u32) {
        for _ in 0..count {
            let a = self.rng.range_f32(0.0, core::f32::consts::TAU);
            let d = self.rng.range_f32(0.0, r);
            self.fluid.spawn(
                Vec2::new(x + a.cos() * d, y + a.sin() * d),
                Vec2::ZERO,
                kind,
                if kind == self.ids.water { 1.0 } else { 0.0 },
            );
        }
    }
}

/// Level generation: strata + fBm fill, carved by cave noise and authored
/// clears, then authored pipework stamped as hard geometry. The contrast
/// between noisy deposit and hard machine structure is the intended look.
fn generate_field(level: &Level, materials: &MaterialTable) -> Result<DensityField, String> {
    let mut field = DensityField::new(level.width, level.height);
    let strata: Vec<(u8, f32, f32)> = level
        .strata
        .iter()
        .map(|s| {
            materials
                .index_of(&s.material)
                .map(|i| (i, s.from, s.to))
                .ok_or_else(|| format!("unknown stratum material '{}'", s.material))
        })
        .collect::<Result<_, _>>()?;
    let pipework = materials
        .index_of("pipework")
        .ok_or("material table missing 'pipework'")?;
    let mineral = materials
        .index_of("mineral_growth")
        .ok_or("material table missing 'mineral_growth'")?;

    let fill = &level.fill_noise;
    let caves = &level.cave_noise;
    for y in 0..level.height {
        let depth_frac = y as f32 / level.height as f32;
        let mat = strata
            .iter()
            .find(|(_, from, to)| depth_frac >= *from && depth_frac < *to)
            .map(|(m, _, _)| *m)
            .unwrap_or(0);
        for x in 0..level.width {
            let i = y * level.width + x;
            let f = rng::fbm(
                level.seed,
                x as f32 * fill.scale,
                y as f32 * fill.scale,
                fill.octaves,
                2.0,
                0.5,
            );
            let c = rng::fbm(
                level.seed.wrapping_add(0xCAFE),
                x as f32 * caves.scale,
                y as f32 * caves.scale,
                caves.octaves,
                2.0,
                0.5,
            );
            // Steep transition: the field is mostly ±1 with a narrow surface
            // band, so edges render crisp and chunky rather than mushy.
            let mut d = ((f - fill.threshold) * 3.0).clamp(-1.0, 1.0);
            if c > caves.threshold {
                d = -((c - caves.threshold) * 3.0).clamp(0.3, 1.0);
            }
            field.density[i] = d;
            field.material[i] = mat;
        }
    }

    let stamp_rect = |r: &level::Rect, value: f32, mat: u8, field: &mut DensityField| {
        let x0 = r.x.max(0.0) as usize;
        let y0 = r.y.max(0.0) as usize;
        let x1 = ((r.x + r.w) as usize).min(level.width);
        let y1 = ((r.y + r.h) as usize).min(level.height);
        for y in y0..y1 {
            for x in x0..x1 {
                let i = y * level.width + x;
                field.density[i] = value;
                if value > 0.0 {
                    field.material[i] = mat;
                }
            }
        }
    };
    for r in &level.clears {
        stamp_rect(r, -1.0, 0, &mut field);
    }
    for r in &level.pipes {
        stamp_rect(r, 1.0, pipework, &mut field);
    }
    for v in &level.mineral_veins {
        field.apply_brush(
            Brush {
                x: v.x,
                y: v.y,
                radius: v.r,
                strength: 2.0,
                material: mineral,
            },
            |m| m == pipework,
        );
    }
    // Ensure pools and spawn areas are clear.
    for r in level
        .molten_pools
        .iter()
        .chain(level.contaminant_pools.iter())
    {
        stamp_rect(r, -1.0, 0, &mut field);
    }
    field.mark_dirty_rect(0, 0, level.width as i32 - 1, level.height as i32 - 1);
    Ok(field)
}
