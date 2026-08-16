//! Thin WASM boundary. Pointers in, pointers out — the shell reads simulation
//! output directly from linear memory (zero-copy). No per-frame serialisation,
//! no per-frame allocation (all export buffers are grown once and reused).
//!
//! Built without wasm-bindgen: plain C-ABI exports keep the toolchain to
//! `cargo build --target wasm32-unknown-unknown` and the JS loader trivial.

use sim::player::InputFrame;
use sim::{Phase, Sim};

const MATERIALS_RON: &str = include_str!("../../materials/materials.ron");
const LEVELS: &[&str] = &[include_str!("../../levels/demo.ron")];

const BTN_JUMP: u32 = 1 << 0;
const BTN_FIRE: u32 = 1 << 1;

pub struct State {
    sim: Sim,
    /// Density texture: field density mapped to 0..255 (128 = surface).
    /// Rebuilt each frame in Rust, uploaded by the shell with LINEAR
    /// filtering; the material grid is read straight from the field with
    /// NEAREST filtering.
    density_u8: Vec<u8>,
    /// Per-particle metadata: [kind, purity*255, speed_norm*255, 0] quads.
    /// Speed drives foam/turbulence rendering in the shell.
    meta: Vec<u8>,
    /// Cosmetic events: steam positions then impact positions, xy pairs.
    events: Vec<f32>,
    stats: [f32; 24],
    /// Palette from the material table: RGBA per material index.
    palette: Vec<u8>,
}

static mut STATE: Option<State> = None;

fn state() -> &'static mut State {
    // Wasm is single-threaded; this static is only ever touched from the one
    // JS thread that owns the module.
    unsafe {
        #[allow(static_mut_refs)]
        STATE.as_mut().expect("pp_init not called")
    }
}

#[no_mangle]
pub extern "C" fn pp_init(level: u32) -> u32 {
    let src = LEVELS[(level as usize).min(LEVELS.len() - 1)];
    match Sim::new(src, MATERIALS_RON) {
        Ok(sim) => {
            let n = sim.field.width * sim.field.height;
            let palette = sim
                .materials
                .materials
                .iter()
                .flat_map(|m| {
                    [
                        (m.colour.0 * 255.0) as u8,
                        (m.colour.1 * 255.0) as u8,
                        (m.colour.2 * 255.0) as u8,
                        255,
                    ]
                })
                .collect();
            unsafe {
                STATE = Some(State {
                    sim,
                    density_u8: vec![0; n],
                    meta: Vec::new(),
                    events: Vec::new(),
                    stats: [0.0; 24],
                    palette,
                });
            }
            1
        }
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn pp_step(move_x: f32, aim_x: f32, aim_y: f32, buttons: u32) {
    let st = state();
    let input = InputFrame {
        move_x,
        jump: buttons & BTN_JUMP != 0,
        aim_x,
        aim_y,
        firing: buttons & BTN_FIRE != 0,
    };
    st.sim.step(&input);

    // Terrain density texture.
    let f = &st.sim.field;
    for i in 0..f.density.len() {
        let d = (f.density[i] * 127.0 + 128.0).clamp(0.0, 255.0);
        st.density_u8[i] = d as u8;
    }

    // Particle metadata.
    let fl = &st.sim.fluid;
    st.meta.clear();
    for i in 0..fl.len() {
        st.meta.push(fl.kind[i]);
        st.meta.push((fl.purity[i] * 255.0) as u8);
        // Agitation with a deadzone: settled pools carry residual solver
        // jitter (~<10 cells/s) that must NOT read as turbulence — only
        // genuinely moving fluid foams.
        let speed = ((fl.vel[i].length() - 10.0) / 28.0).clamp(0.0, 1.0);
        st.meta.push((speed * 255.0) as u8);
        st.meta.push(0);
    }

    // Cosmetic events: steam then shot impacts.
    st.events.clear();
    for s in &fl.steam_events {
        st.events.push(s.x);
        st.events.push(s.y);
    }
    let n_steam = fl.steam_events.len();
    for p in &st.sim.shots.impacts {
        st.events.push(p.x);
        st.events.push(p.y);
    }

    // Stats block.
    let s = &st.sim;
    st.stats = [
        s.player.pos.x,
        s.player.pos.y,
        s.player.vel.x,
        s.player.vel.y,
        s.player.health,
        s.player.credits as f32,
        s.player.tool_tier as f32,
        s.fluid.counters.delivered as f32,
        s.level.goal_volume as f32,
        s.delivered_purity,
        s.level.goal_purity,
        match s.phase {
            Phase::Playing => 0.0,
            Phase::Won => 1.0,
            Phase::Dead => 2.0,
        },
        s.tick as f32,
        s.next_upgrade_cost() as f32,
        n_steam as f32,
        s.shots.impacts.len() as f32,
        s.level.intake.x,
        s.level.intake.y,
        s.level.intake.w,
        s.level.intake.h,
        s.fluid.len() as f32,
        s.player.on_ground as u32 as f32,
        0.0,
        0.0,
    ];
}

#[no_mangle]
pub extern "C" fn pp_buy_upgrade() -> u32 {
    state().sim.buy_tool_upgrade() as u32
}

#[no_mangle]
pub extern "C" fn pp_field_w() -> u32 {
    state().sim.field.width as u32
}

#[no_mangle]
pub extern "C" fn pp_field_h() -> u32 {
    state().sim.field.height as u32
}

#[no_mangle]
pub extern "C" fn pp_density_ptr() -> *const u8 {
    state().density_u8.as_ptr()
}

#[no_mangle]
pub extern "C" fn pp_material_ptr() -> *const u8 {
    state().sim.field.material.as_ptr()
}

#[no_mangle]
pub extern "C" fn pp_palette_ptr() -> *const u8 {
    state().palette.as_ptr()
}

#[no_mangle]
pub extern "C" fn pp_palette_len() -> u32 {
    (state().palette.len() / 4) as u32
}

#[no_mangle]
pub extern "C" fn pp_particle_count() -> u32 {
    state().sim.fluid.len() as u32
}

#[no_mangle]
pub extern "C" fn pp_particle_pos_ptr() -> *const f32 {
    // Vec<Vec2> is a contiguous [x, y, x, y, ...] f32 buffer.
    state().sim.fluid.pos.as_ptr() as *const f32
}

#[no_mangle]
pub extern "C" fn pp_particle_meta_ptr() -> *const u8 {
    state().meta.as_ptr()
}

#[no_mangle]
pub extern "C" fn pp_shot_count() -> u32 {
    state().sim.shots.shots.len() as u32
}

#[no_mangle]
pub extern "C" fn pp_shot_ptr() -> *const f32 {
    state().sim.shots.shots.as_ptr() as *const f32
}

/// Shot struct stride in f32s (pos, vel, alive+padding).
#[no_mangle]
pub extern "C" fn pp_shot_stride() -> u32 {
    (core::mem::size_of::<sim::player::Shot>() / 4) as u32
}

#[no_mangle]
pub extern "C" fn pp_events_ptr() -> *const f32 {
    state().events.as_ptr()
}

#[no_mangle]
pub extern "C" fn pp_stats_ptr() -> *const f32 {
    state().stats.as_ptr()
}
