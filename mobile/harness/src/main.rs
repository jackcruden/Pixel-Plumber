//! Visual harness: load a level + declarative input script, run headless at
//! full speed, dump annotated PNG frames and stitch GIFs.
//!
//! Usage:
//!   harness <scenario.ron> [--out DIR] [--gif out.gif] [--contours] [--heat] [--dirty]
//!
//! Scenario format (RON): see `harness/scenarios/*.ron`.

mod render;

use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame};
use render::{render, Overlays};
use serde::Deserialize;
use sim::player::InputFrame;
use sim::Sim;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
enum Action {
    /// Fire the player's tool at a world position from `step` for `steps`.
    Fire { step: u64, steps: u64, x: f32, y: f32 },
    /// Direct excavation (no projectile travel time).
    Dig { step: u64, x: f32, y: f32, r: f32 },
    /// Spawn a fluid blob: kind is a material id string.
    Blob { step: u64, x: f32, y: f32, r: f32, kind: String, count: u32 },
    /// Hold horizontal movement from `step` for `steps` (-1..1).
    Move { step: u64, steps: u64, dir: f32 },
    Jump { step: u64 },
    /// Debug: grant a tool tier directly (scenario shorthand for the
    /// harvest-and-spend loop).
    Upgrade { step: u64 },
}

#[derive(Clone, Debug, Deserialize)]
struct Scenario {
    level: String,
    materials: String,
    /// Total steps to simulate.
    steps: u64,
    /// Dump a PNG every `snapshot_every` steps (0 = only final frame).
    snapshot_every: u64,
    #[serde(default)]
    actions: Vec<Action>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: harness <scenario.ron> [--out DIR] [--gif FILE] [--contours] [--heat] [--dirty]");
        std::process::exit(2);
    }
    let scenario_path = &args[1];
    let mut out_dir = PathBuf::from("harness-out");
    let mut gif_path: Option<PathBuf> = None;
    let mut overlays = Overlays::default();
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                out_dir = PathBuf::from(&args[i + 1]);
                i += 1;
            }
            "--gif" => {
                gif_path = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--contours" => overlays.contours = true,
            "--heat" => overlays.heat = true,
            "--dirty" => overlays.dirty_chunks = true,
            other => {
                eprintln!("unknown flag {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let scenario_src = fs::read_to_string(scenario_path).expect("read scenario");
    let scenario: Scenario = ron::from_str(&scenario_src).expect("parse scenario");
    let base = PathBuf::from(scenario_path).parent().unwrap().to_path_buf();
    let level_src = fs::read_to_string(base.join(&scenario.level))
        .or_else(|_| fs::read_to_string(&scenario.level))
        .expect("read level");
    let materials_src = fs::read_to_string(base.join(&scenario.materials))
        .or_else(|_| fs::read_to_string(&scenario.materials))
        .expect("read materials");

    let mut sim = Sim::new(&level_src, &materials_src).expect("init sim");
    fs::create_dir_all(&out_dir).expect("create out dir");

    let mut frames: Vec<Frame> = Vec::new();
    let started = std::time::Instant::now();
    for step in 0..scenario.steps {
        let mut input = InputFrame::default();
        for a in &scenario.actions {
            match a {
                Action::Fire { step: s, steps, x, y } => {
                    if step >= *s && step < s + steps {
                        input.firing = true;
                        input.aim_x = *x;
                        input.aim_y = *y;
                    }
                }
                Action::Move { step: s, steps, dir } => {
                    if step >= *s && step < s + steps {
                        input.move_x = *dir;
                    }
                }
                Action::Jump { step: s } => {
                    if step == *s {
                        input.jump = true;
                    }
                }
                Action::Upgrade { step: s } => {
                    if step == *s {
                        sim.player.tool_tier += 1;
                    }
                }
                Action::Dig { step: s, x, y, r } => {
                    if step == *s {
                        sim.dig_at(*x, *y, *r, 2.5);
                    }
                }
                Action::Blob { step: s, x, y, r, kind, count } => {
                    if step == *s {
                        let k = sim.materials.index_of(kind).expect("blob material");
                        sim.spawn_blob(*x, *y, *r, k, *count);
                    }
                }
            }
        }
        sim.step(&input);

        let snap = scenario.snapshot_every > 0 && step % scenario.snapshot_every == 0;
        if snap || step == scenario.steps - 1 {
            let img = render(&sim, overlays);
            if snap {
                img.save(out_dir.join(format!("frame_{step:05}.png"))).expect("save png");
            }
            if step == scenario.steps - 1 {
                img.save(out_dir.join("final.png")).expect("save png");
            }
            if gif_path.is_some() {
                frames.push(Frame::from_parts(
                    img,
                    0,
                    0,
                    Delay::from_numer_denom_ms(
                        (scenario.snapshot_every.max(1) as u32) * 1000 / 60,
                        1,
                    ),
                ));
            }
        }
    }
    let elapsed = started.elapsed();

    if let Some(gp) = gif_path {
        let file = fs::File::create(&gp).expect("create gif");
        let mut enc = GifEncoder::new_with_speed(file, 10);
        enc.set_repeat(Repeat::Infinite).unwrap();
        enc.encode_frames(frames).expect("encode gif");
        println!("gif: {}", gp.display());
    }

    let c = sim.fluid.counters;
    let stats = sim.fluid.stats();
    println!(
        "steps={} wall={:.2?} ({:.1} steps/ms)",
        scenario.steps,
        elapsed,
        scenario.steps as f64 / elapsed.as_millis().max(1) as f64
    );
    println!(
        "particles={} spawned={} delivered={} purity={:.3} evaporated={} slagged={} culled={}",
        sim.fluid.len(),
        c.spawned,
        c.delivered,
        sim.delivered_purity,
        c.evaporated,
        c.consumed_molten,
        c.culled
    );
    println!(
        "fluid centroid=({:.1},{:.1}) phase={:?} health={:.2} credits={}",
        stats.centroid.x, stats.centroid.y, sim.phase, sim.player.health, sim.player.credits
    );
    println!("field hash={:016x}", sim.field.content_hash());
}
