//! Position Based Fluids (Macklin & Müller 2013), 2D.
//!
//! Fixed solver iteration count (predictable frame cost), fixed timestep,
//! spatial hash rebuilt per step via counting sort (deterministic order — no
//! HashMap iteration anywhere). All per-step scratch lives in reusable
//! buffers; steady-state stepping does not allocate.
//!
//! Update order per step (see AGENTS.md — do not reorder):
//!   1. external forces + predict positions
//!   2. build spatial hash on predicted positions
//!   3. N solver iterations: lambdas -> position deltas -> field collision
//!   4. velocity update from positions, XSPH viscosity
//!   5. material interactions (molten+water -> slag, purity mixing)
//!   6. thermal effects (heat deposit, evaporation)
//!   7. delivery / culling / compaction

use crate::field::{Brush, DensityField};
use crate::heat::HeatField;
use crate::materials::MaterialTable;
use crate::rng::Rng;
use glam::Vec2;

pub const MAX_NEIGHBORS: usize = 40;

/// Resolved material indices for the fluid kinds the interaction rules need.
#[derive(Clone, Copy, Debug)]
pub struct FluidIds {
    pub water: u8,
    pub contaminant: u8,
    pub molten: u8,
    pub slag: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct FluidParams {
    /// Smoothing radius (cells).
    pub h: f32,
    /// Rest particle spacing (cells).
    pub spacing: f32,
    pub gravity: f32,
    pub iterations: u32,
    pub max_velocity: f32,
    /// Artificial-pressure (surface tension) coefficient.
    pub s_corr_k: f32,
    /// Radius within which water+molten react.
    pub interact_radius: f32,
    /// Purity loss rate per unit contaminant kernel weight per second.
    pub purity_mix_rate: f32,
    /// Temperature above which water starts to evaporate.
    pub evap_threshold: f32,
    pub evap_rate: f32,
    /// Slag brush applied per consumed molten particle.
    pub slag_radius: f32,
    pub slag_strength: f32,
    pub particle_cap: usize,
}

impl Default for FluidParams {
    fn default() -> Self {
        Self {
            h: 2.8,
            spacing: 1.3,
            gravity: 240.0,
            iterations: 3,
            max_velocity: 130.0,
            s_corr_k: 0.0006,
            interact_radius: 2.0,
            purity_mix_rate: 3.0,
            evap_threshold: 0.55,
            evap_rate: 2.2,
            slag_radius: 2.4,
            slag_strength: 1.6,
            particle_cap: 4000,
        }
    }
}

/// Removal ledger — mass conservation tests reconcile against these.
#[derive(Clone, Copy, Debug, Default)]
pub struct FluidCounters {
    pub spawned: u64,
    pub delivered: u64,
    pub delivered_purity_sum: f64,
    pub evaporated: u64,
    pub consumed_molten: u64,
    pub consumed_water: u64,
    pub culled: u64,
    /// Cosmetic steam puffs emitted this step (position pairs for the shell).
    pub steam_this_step: u32,
}

pub struct Fluid {
    pub params: FluidParams,
    pub ids: FluidIds,
    pub rest_density: f32,

    // SoA particle state. `pos` is exported to the shell as a raw xy buffer.
    pub pos: Vec<Vec2>,
    pub vel: Vec<Vec2>,
    pub kind: Vec<u8>,
    pub purity: Vec<f32>,

    pub counters: FluidCounters,
    pub steam_events: Vec<Vec2>,

    // Scratch (reused every step).
    predicted: Vec<Vec2>,
    mass: Vec<f32>,
    lambdas: Vec<f32>,
    deltas: Vec<Vec2>,
    neighbors: Vec<u32>,
    neighbor_count: Vec<u32>,
    cell_of: Vec<u32>,
    cell_start: Vec<u32>,
    sorted: Vec<u32>,
    remove: Vec<bool>,
    grid_w: usize,
    grid_h: usize,
}

#[inline]
fn poly6(r2: f32, h: f32) -> f32 {
    let h2 = h * h;
    if r2 >= h2 {
        return 0.0;
    }
    let c = 4.0 / (core::f32::consts::PI * h2.powi(4));
    let d = h2 - r2;
    c * d * d * d
}

/// poly6 with the normalisation constant precomputed (inner-loop form).
#[inline(always)]
fn poly6_pre(r2: f32, h2: f32, c: f32) -> f32 {
    if r2 >= h2 {
        return 0.0;
    }
    let d = h2 - r2;
    c * d * d * d
}

/// spiky gradient with the normalisation constant precomputed.
#[inline(always)]
fn spiky_grad_pre(rv: Vec2, r: f32, h: f32, c: f32) -> Vec2 {
    if r >= h || r < 1e-6 {
        return Vec2::ZERO;
    }
    let d = h - r;
    rv * (c * d * d / r)
}

impl Fluid {
    pub fn new(params: FluidParams, ids: FluidIds, world_w: usize, world_h: usize) -> Self {
        // Numerical rest density: sample the kernel over a filled lattice at
        // rest spacing. Robust to any (h, spacing) pairing.
        let mut rest = 0.0;
        let s = params.spacing;
        let n = (params.h / s).ceil() as i32 + 1;
        for gy in -n..=n {
            for gx in -n..=n {
                let r2 = (gx as f32 * s).powi(2) + (gy as f32 * s).powi(2);
                rest += poly6(r2, params.h);
            }
        }
        let grid_w = (world_w as f32 / params.h).ceil() as usize + 2;
        let grid_h = (world_h as f32 / params.h).ceil() as usize + 2;
        Self {
            params,
            ids,
            rest_density: rest,
            pos: Vec::new(),
            vel: Vec::new(),
            kind: Vec::new(),
            purity: Vec::new(),
            counters: FluidCounters::default(),
            steam_events: Vec::new(),
            predicted: Vec::new(),
            mass: Vec::new(),
            lambdas: Vec::new(),
            deltas: Vec::new(),
            neighbors: Vec::new(),
            neighbor_count: Vec::new(),
            cell_of: Vec::new(),
            cell_start: vec![0; grid_w * grid_h + 1],
            sorted: Vec::new(),
            remove: Vec::new(),
            grid_w,
            grid_h,
        }
    }

    pub fn len(&self) -> usize {
        self.pos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pos.is_empty()
    }

    /// Spawn a particle; silently drops (and counts a cull) at the cap by
    /// removing the oldest non-critical particle first.
    pub fn spawn(&mut self, p: Vec2, v: Vec2, kind: u8, purity: f32) {
        if self.pos.len() >= self.params.particle_cap {
            // Cull the oldest particle (index 0 region) — swap_remove keeps
            // this O(1) and deterministic.
            self.remove_particle(0);
            self.counters.culled += 1;
        }
        self.pos.push(p);
        self.vel.push(v);
        self.kind.push(kind);
        self.purity.push(purity);
        self.counters.spawned += 1;
    }

    fn remove_particle(&mut self, i: usize) {
        self.pos.swap_remove(i);
        self.vel.swap_remove(i);
        self.kind.swap_remove(i);
        self.purity.swap_remove(i);
    }

    #[inline]
    fn cell_index(&self, p: Vec2) -> u32 {
        let h = self.params.h;
        let cx = ((p.x / h) as i32 + 1).clamp(0, self.grid_w as i32 - 1) as u32;
        let cy = ((p.y / h) as i32 + 1).clamp(0, self.grid_h as i32 - 1) as u32;
        cy * self.grid_w as u32 + cx
    }

    fn build_hash(&mut self) {
        let n = self.pos.len();
        self.cell_of.clear();
        self.cell_of.reserve(n);
        for i in 0..n {
            self.cell_of.push(self.cell_index(self.predicted[i]));
        }
        self.cell_start.iter_mut().for_each(|c| *c = 0);
        for &c in &self.cell_of {
            self.cell_start[c as usize + 1] += 1;
        }
        for i in 1..self.cell_start.len() {
            self.cell_start[i] += self.cell_start[i - 1];
        }
        self.sorted.clear();
        self.sorted.resize(n, 0);
        let mut cursor = self.cell_start.clone();
        for i in 0..n {
            let c = self.cell_of[i] as usize;
            self.sorted[cursor[c] as usize] = i as u32;
            cursor[c] += 1;
        }
    }

    fn gather_neighbors(&mut self) {
        let n = self.pos.len();
        let h2 = self.params.h * self.params.h;
        self.neighbors.clear();
        self.neighbors.resize(n * MAX_NEIGHBORS, 0);
        self.neighbor_count.clear();
        self.neighbor_count.resize(n, 0);
        for i in 0..n {
            let p = self.predicted[i];
            let c = self.cell_of[i];
            let cx = (c % self.grid_w as u32) as i32;
            let cy = (c / self.grid_w as u32) as i32;
            let mut count = 0usize;
            'cells: for dy in -1..=1 {
                for dx in -1..=1 {
                    let nx = cx + dx;
                    let ny = cy + dy;
                    if nx < 0 || ny < 0 || nx >= self.grid_w as i32 || ny >= self.grid_h as i32 {
                        continue;
                    }
                    let cell = (ny as usize) * self.grid_w + nx as usize;
                    let start = self.cell_start[cell] as usize;
                    let end = self.cell_start[cell + 1] as usize;
                    for s in start..end {
                        let j = self.sorted[s] as usize;
                        if j == i {
                            continue;
                        }
                        if (self.predicted[j] - p).length_squared() < h2 {
                            self.neighbors[i * MAX_NEIGHBORS + count] = j as u32;
                            count += 1;
                            if count == MAX_NEIGHBORS {
                                break 'cells;
                            }
                        }
                    }
                }
            }
            self.neighbor_count[i] = count as u32;
        }
    }

    /// Push a predicted position out of solid terrain (and world bounds).
    fn collide_field(field: &DensityField, p: &mut Vec2) {
        p.x = p.x.clamp(1.0, field.width as f32 - 1.0);
        p.y = p.y.clamp(1.0, field.height as f32 - 1.0);
        for _ in 0..4 {
            if field.sample(p.x, p.y) <= 0.0 {
                return;
            }
            let (gx, gy) = field.gradient(p.x, p.y);
            let g = Vec2::new(gx, gy);
            let len = g.length();
            if len < 1e-5 {
                p.y -= 0.5; // Degenerate gradient: nudge up.
            } else {
                *p -= g * (0.6 / len);
            }
        }
    }

    /// One fixed-dt step. `intake` is the delivery region (x0,y0,x1,y1).
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        dt: f32,
        field: &mut DensityField,
        heat: &mut HeatField,
        materials: &MaterialTable,
        intake: (f32, f32, f32, f32),
        rng: &mut Rng,
    ) {
        let n = self.pos.len();
        self.counters.steam_this_step = 0;
        self.steam_events.clear();
        if n == 0 {
            return;
        }
        let pr = self.params;

        // Per-particle mass cache (material table lookups are too slow for
        // the inner solver loops).
        self.mass.clear();
        self.mass.reserve(n);
        for i in 0..n {
            self.mass.push(materials.get(self.kind[i]).particle_mass);
        }

        // 1. External forces + prediction. Heavier fluids sink because their
        // mass enters the density constraint; gravity itself is uniform.
        self.predicted.clear();
        self.predicted.reserve(n);
        for i in 0..n {
            let mut v = self.vel[i];
            v.y += pr.gravity * dt;
            let sp = v.length();
            if sp > pr.max_velocity {
                v *= pr.max_velocity / sp;
            }
            self.vel[i] = v;
            self.predicted.push(self.pos[i] + v * dt);
        }

        // 2. Spatial hash + neighbours on predicted positions.
        self.build_hash();
        self.gather_neighbors();

        // 3. Constraint solve.
        self.lambdas.clear();
        self.lambdas.resize(n, 0.0);
        self.deltas.clear();
        self.deltas.resize(n, Vec2::ZERO);
        let rho0 = self.rest_density;
        let h2 = pr.h * pr.h;
        let poly6_c = 4.0 / (core::f32::consts::PI * h2.powi(4));
        let spiky_c = -30.0 / (core::f32::consts::PI * pr.h.powi(5));
        let w_dq = poly6((0.3 * pr.h).powi(2), pr.h);
        for _ in 0..pr.iterations {
            for i in 0..n {
                let pi = self.predicted[i];
                let mut rho = poly6_pre(0.0, h2, poly6_c) * self.mass[i];
                let mut grad_i = Vec2::ZERO;
                let mut sum_grad2 = 0.0;
                for k in 0..self.neighbor_count[i] as usize {
                    let j = self.neighbors[i * MAX_NEIGHBORS + k] as usize;
                    let rv = pi - self.predicted[j];
                    let r2 = rv.length_squared();
                    let mj = self.mass[j];
                    rho += mj * poly6_pre(r2, h2, poly6_c);
                    let g = spiky_grad_pre(rv, r2.sqrt(), pr.h, spiky_c) * (mj / rho0);
                    grad_i += g;
                    sum_grad2 += g.length_squared();
                }
                sum_grad2 += grad_i.length_squared();
                let c = (rho / rho0 - 1.0).max(0.0);
                self.lambdas[i] = -c / (sum_grad2 + 1e-2);
            }
            for i in 0..n {
                let pi = self.predicted[i];
                let mut d = Vec2::ZERO;
                for k in 0..self.neighbor_count[i] as usize {
                    let j = self.neighbors[i * MAX_NEIGHBORS + k] as usize;
                    let rv = pi - self.predicted[j];
                    let r2 = rv.length_squared();
                    let w = poly6_pre(r2, h2, poly6_c);
                    let s_corr = if w_dq > 0.0 {
                        -pr.s_corr_k * (w / w_dq).powi(4)
                    } else {
                        0.0
                    };
                    d += spiky_grad_pre(rv, r2.sqrt(), pr.h, spiky_c)
                        * ((self.lambdas[i] + self.lambdas[j] + s_corr) / rho0);
                }
                self.deltas[i] = d;
            }
            for i in 0..n {
                let mut p = self.predicted[i] + self.deltas[i];
                Self::collide_field(field, &mut p);
                self.predicted[i] = p;
            }
        }

        // Hard per-step displacement clamp (the no-teleportation property):
        // solver projection + collision push-out may not carry a particle
        // further than max_velocity*dt plus one smoothing radius.
        let max_move = pr.max_velocity * dt + pr.h * 0.999;
        for i in 0..n {
            let d = self.predicted[i] - self.pos[i];
            let len = d.length();
            if len > max_move {
                self.predicted[i] = self.pos[i] + d * (max_move / len);
            }
        }

        // 4. Velocities from positions, then XSPH viscosity per material.
        for i in 0..n {
            let mut v = (self.predicted[i] - self.pos[i]) / dt;
            let sp = v.length();
            if sp > pr.max_velocity {
                v *= pr.max_velocity / sp;
            }
            self.vel[i] = v;
        }
        for i in 0..n {
            let visc = materials.get(self.kind[i]).viscosity;
            if visc <= 0.0 {
                continue;
            }
            let mut dv = Vec2::ZERO;
            let mut wsum = 0.0;
            for k in 0..self.neighbor_count[i] as usize {
                let j = self.neighbors[i * MAX_NEIGHBORS + k] as usize;
                let w = poly6((self.predicted[i] - self.predicted[j]).length_squared(), pr.h);
                dv += (self.vel[j] - self.vel[i]) * w;
                wsum += w;
            }
            if wsum > 0.0 {
                self.deltas[i] = dv * (visc / wsum.max(1e-6));
            } else {
                self.deltas[i] = Vec2::ZERO;
            }
        }
        for i in 0..n {
            let visc = materials.get(self.kind[i]).viscosity;
            if visc > 0.0 {
                self.vel[i] += self.deltas[i];
            }
            self.pos[i] = self.predicted[i];
        }

        // 5. Interactions.
        self.remove.clear();
        self.remove.resize(n, false);
        let ids = self.ids;
        let interact2 = pr.interact_radius * pr.interact_radius;
        let w0 = poly6(0.0, pr.h);
        for i in 0..n {
            if self.remove[i] {
                continue;
            }
            let ki = self.kind[i];
            if ki == ids.water {
                for k in 0..self.neighbor_count[i] as usize {
                    let j = self.neighbors[i * MAX_NEIGHBORS + k] as usize;
                    if self.remove[j] {
                        continue;
                    }
                    let kj = self.kind[j];
                    let r2 = (self.pos[i] - self.pos[j]).length_squared();
                    if kj == ids.contaminant {
                        // Purity degrades as a function of local contaminant
                        // proximity. Purity only ever decreases.
                        let w = poly6(r2, pr.h) / w0;
                        self.purity[i] = (self.purity[i] - pr.purity_mix_rate * w * dt).max(0.0);
                    } else if kj == ids.molten && r2 < interact2 {
                        // The additive mechanic: both particles consumed, slag
                        // accreted into the field at the contact point.
                        let mid = (self.pos[i] + self.pos[j]) * 0.5;
                        self.remove[i] = true;
                        self.remove[j] = true;
                        self.counters.consumed_water += 1;
                        self.counters.consumed_molten += 1;
                        field.apply_brush(
                            Brush {
                                x: mid.x,
                                y: mid.y,
                                radius: pr.slag_radius,
                                strength: pr.slag_strength,
                                material: ids.slag,
                            },
                            |_| false,
                        );
                        heat.deposit(mid.x, mid.y, -0.4);
                        self.steam_events.push(mid);
                        self.counters.steam_this_step += 1;
                        break;
                    }
                }
            }
        }

        // 6. Thermal effects.
        for i in 0..n {
            if self.remove[i] {
                continue;
            }
            let m = materials.get(self.kind[i]);
            if m.heat_emission > 0.0 {
                heat.deposit(self.pos[i].x, self.pos[i].y, m.heat_emission * dt);
            }
            if self.kind[i] == ids.water {
                let t = heat.sample(self.pos[i].x, self.pos[i].y);
                if t > pr.evap_threshold {
                    let p_evap = (t - pr.evap_threshold) * pr.evap_rate * dt;
                    if rng.next_f32() < p_evap {
                        self.remove[i] = true;
                        self.counters.evaporated += 1;
                        heat.deposit(self.pos[i].x, self.pos[i].y, -0.05);
                        self.steam_events.push(self.pos[i]);
                        self.counters.steam_this_step += 1;
                    }
                }
            }
        }

        // 7. Delivery.
        let (ix0, iy0, ix1, iy1) = intake;
        for i in 0..n {
            if self.remove[i] || self.kind[i] != ids.water {
                continue;
            }
            let p = self.pos[i];
            if p.x >= ix0 && p.x <= ix1 && p.y >= iy0 && p.y <= iy1 {
                self.remove[i] = true;
                self.counters.delivered += 1;
                self.counters.delivered_purity_sum += self.purity[i] as f64;
            }
        }

        // Compact (descending order keeps swap_remove indices valid).
        for i in (0..n).rev() {
            if self.remove[i] {
                self.remove_particle(i);
            }
        }
    }

    /// Aggregate statistics for tolerance-based snapshot tests (fluid state is
    /// not bit-exact across platforms — see the determinism contract).
    pub fn stats(&self) -> FluidStats {
        let n = self.pos.len();
        let mut centroid = Vec2::ZERO;
        let mut min = Vec2::splat(f32::MAX);
        let mut max = Vec2::splat(f32::MIN);
        let mut purity_sum = 0.0f64;
        for i in 0..n {
            centroid += self.pos[i];
            min = min.min(self.pos[i]);
            max = max.max(self.pos[i]);
            purity_sum += self.purity[i] as f64;
        }
        if n > 0 {
            centroid /= n as f32;
        }
        FluidStats {
            count: n,
            centroid,
            min,
            max,
            purity_mean: if n > 0 { purity_sum / n as f64 } else { 0.0 },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FluidStats {
    pub count: usize,
    pub centroid: Vec2,
    pub min: Vec2,
    pub max: Vec2,
    pub purity_mean: f64,
}
