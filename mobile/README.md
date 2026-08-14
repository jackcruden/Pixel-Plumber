# Pixel Plumber (mobile)

Rust + WASM rebuild of [Pixel Plumber](https://github.com/jackcruden/Pixel-Plumber):
density-field destructible terrain, Position Based Fluids, and the additive
slag mechanic (molten + water → solid geometry you place by routing fluids).

## Layout

```
sim/       Pure Rust simulation. No web deps, no I/O. cargo test runs here.
wasm/      Thin C-ABI boundary. Pointers in, pointers out.
harness/   Native binary: headless runs, PNG/GIF dumps, criterion benches.
shell/     TypeScript + WebGL2: rendering, touch input, HUD.
levels/    Authored level data (RON).
materials/ Material behaviour tables (RON).
```

## Quick start

```sh
# Tests (property + snapshot) and benches — native, fast
cargo test -p sim --release
cargo bench -p harness

# Visual harness: run a scenario headless, dump PNGs/GIF
cargo build -p harness --release
./target/release/harness harness/scenarios/dig_route.ron --out /tmp/out

# Browser build
rustup target add wasm32-unknown-unknown
cd shell && npm install && node build.mjs --single
# open shell/dist/index.html via any static server, or shell/dist/single.html directly
```

## Controls

- **Touch:** drag on the left side to move, touch-and-hold anywhere else to
  blast, JUMP button bottom-left.
- **Keyboard:** A/D move, W/space jump, mouse aim + hold LMB to blast,
  U upgrade tool, R restart.

Goal: route water from the supply pipe to the core intake — 400 units at
≥ 65% purity. Contaminant (green) ruins purity; molten (orange) kills you but
solidifies into buildable slag when it meets water. Harvest green mineral
growths to afford the tool upgrade that cuts vitreous scale (the blue-grey
stratum).

See `AGENTS.md` for the determinism contract, solver update order, and
performance budgets before changing `sim/`.
