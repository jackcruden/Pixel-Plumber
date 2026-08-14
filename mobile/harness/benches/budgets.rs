//! Performance gates (spec §10). Budgets are per-frame numbers on the target
//! mobile device; these benches track regressions on whatever machine runs
//! them. "Well optimised" must mean a number: cite these when claiming perf.
//!
//! Budgets (target device): fluid solve <= 6 ms, contouring (dirty chunks)
//! <= 2 ms, collider rebuild <= 1 ms, total frame <= 16.6 ms.

use criterion::{criterion_group, criterion_main, Criterion};
use sim::field::Brush;
use sim::player::InputFrame;
use sim::Sim;

const MATERIALS: &str = include_str!("../../materials/materials.ron");
const LEVEL: &str = include_str!("../../levels/demo.ron");

fn loaded_sim() -> Sim {
    let mut s = Sim::new(LEVEL, MATERIALS).expect("sim");
    // Fill toward the particle cap for a worst-case-ish frame.
    s.spawn_blob(64.0, 60.0, 16.0, s.ids.water, 1500);
    s.spawn_blob(40.0, 150.0, 12.0, s.ids.water, 800);
    for _ in 0..120 {
        s.step(&InputFrame::default());
    }
    s
}

fn bench_step(c: &mut Criterion) {
    let mut s = loaded_sim();
    c.bench_function("sim_step_full_frame", |b| {
        b.iter(|| s.step(&InputFrame::default()))
    });
}

fn bench_brush(c: &mut Criterion) {
    let mut s = loaded_sim();
    c.bench_function("field_brush_r6", |b| {
        b.iter(|| {
            s.field.apply_brush(
                Brush {
                    x: 64.0,
                    y: 100.0,
                    radius: 5.0,
                    strength: -0.01,
                    material: 0,
                },
                |_| false,
            )
        })
    });
}

fn bench_contour(c: &mut Criterion) {
    let s = loaded_sim();
    c.bench_function("contour_full_field", |b| {
        b.iter(|| sim::contour::contour_full(&s.field))
    });
}

criterion_group!(benches, bench_step, bench_brush, bench_contour);
criterion_main!(benches);
