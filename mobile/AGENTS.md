# AGENTS.md — rules that cannot be inferred from the code

Read this before touching `sim/`. The spec lives in the PR description /
design doc; this file is the operational contract.

## Determinism contract (verbatim from spec §8 — violations are build-breaking bugs)

- Fixed timestep for all simulation (`sim::DT`). Render interpolates; simulation
  never sees variable dt.
- Seeded PRNG (`sim::rng::Rng`) passed explicitly through the call chain.
  `thread_rng` is banned in `sim`.
- No `HashMap`/`HashSet` iteration in any update path (nondeterministic order).
  Use `BTreeMap`, or index-sorted `Vec`.
- No floating-point accumulation dependent on iteration order where avoidable.
- No wall-clock time, no system entropy, no threading nondeterminism in `sim`.

Known limitation — be honest about it: float physics is deterministic on a
fixed binary and platform, but not reliably across platforms or compiler
versions. Therefore:

- **Field operations** (destruction, accretion, contouring) must remain
  **exact-hash testable** (`DensityField::content_hash`, see
  `sim/tests/properties.rs::field_ops_bit_deterministic`).
- **Fluid and rigid-body state** is tested with **tolerance-based** snapshots —
  aggregate statistics (`Fluid::stats`), not bit-exact hashes.

Do not claim cross-platform bit determinism. Do not build features that depend
on it.

## Hard rules

- **Never add a dependency to `sim`** beyond pure computation (currently:
  `glam`, `serde`, `ron`). No wasm-bindgen, no web-sys, no I/O, no logging,
  no time. `sim` must not know the browser exists.
- Materials are data (`materials/*.ron`). Adding a material must never require
  touching simulation code. If you find yourself writing a `match` on a
  material id in `sim`, stop — you are about to break the content pipeline.
- Terrain is the density field. Do not represent terrain as polygons; do not
  perform boolean/CSG geometry ops. Contour output is a derived cache only.

## Solver update order (this is where simulation bugs live — do not reorder)

`Fluid::step` (see the module doc in `sim/src/fluid.rs`):

1. external forces + predict positions
2. build spatial hash (counting sort) on predicted positions
3. N solver iterations, each: lambdas → position deltas → field collision
4. hard displacement clamp → velocities from positions → XSPH viscosity
5. material interactions (molten+water→slag accretion, purity mixing)
6. thermal effects (heat deposit, evaporation)
7. delivery, then compaction (descending-index swap_remove)

`Sim::step` order: player → shots/excavation → emitters → fluid → heat →
hazard damage → win/lose. Changing this order changes replay hashes.

## Performance budget (spec §10 — target: mid-range Android, ~3 years old)

| Budget | Target |
|---|---|
| Total frame | 16.6 ms |
| Fluid solve | ≤ 6 ms |
| Contouring (dirty chunks) | ≤ 2 ms |
| Rigid body | ≤ 2 ms |
| Collider rebuild | ≤ 1 ms |
| Render + shell | ≤ 4 ms |

**A performance claim requires a criterion run** (`cargo bench -p harness`).
Current numbers live in the bench output; re-run before and after any change
you describe as an optimisation. Do not micro-optimise before property tests
pass.

## Harness — how to look at what you did

```
cargo build -p harness --release
./target/release/harness harness/scenarios/slag_dam.ron \
    --out /tmp/out --gif /tmp/out/run.gif --heat --contours --dirty
```

Scenarios are declarative RON (`harness/scenarios/*.ron`): timed Dig / Blob /
Fire / Move / Jump / Upgrade actions against a level. The harness prints the
fluid ledger (spawned/delivered/evaporated/slagged/culled), the field hash,
and dumps PNG frames. **Look at the PNGs.** Emergent-behaviour bugs (fluid
tunnelling, slag not sealing, water pooling in wrong places) are visible long
before any assertion catches them.

## Open questions — stop and ask, do not decide

See spec §2: (Q1) one continuous machine vs discrete levels — until resolved,
each level is an independent, self-contained scene; (Q2) does the player
return to the surface; (Q3) final art direction — placeholder rendering only.

## Non-goals (spec §3)

No cellular automata, no multiplayer/networking, no procedural level design
beyond terrain field noise, no in-game level editor, no 3D, no analytics/ads/
IAP/accounts, no non-pure dependency in `sim`.

## Current deviations from spec (deliberate, revisit later)

- **Rapier2D not yet integrated.** The M2 player/shots use direct field-sample
  collision (`sim/src/player.rs`). Contour → Rapier colliders is the upgrade
  path; the contouring stage exists and is tested.
- **Contouring is whole-field**, not per-dirty-chunk incremental. Fine at
  256×384; make it incremental when Rapier lands or levels grow.
- **Render interpolation** not implemented in the shell (renders latest sim
  state; sim is still strictly fixed-dt).
- The wasm layer uses raw C-ABI exports instead of wasm-bindgen — same
  "pointers in, pointers out" contract, smaller toolchain.
