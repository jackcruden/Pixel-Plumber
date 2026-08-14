// Pixel Plumber shell: wasm loader, fixed-timestep loop, input, overlay, HUD.
// The simulation runs at a fixed 60 Hz; the shell reads its output buffers
// zero-copy from WASM linear memory each rendered frame.

import { Renderer, Camera } from "./gl";

interface PP {
  memory: WebAssembly.Memory;
  pp_init(level: number): number;
  pp_step(moveX: number, aimX: number, aimY: number, buttons: number): void;
  pp_buy_upgrade(): number;
  pp_field_w(): number;
  pp_field_h(): number;
  pp_density_ptr(): number;
  pp_material_ptr(): number;
  pp_palette_ptr(): number;
  pp_palette_len(): number;
  pp_particle_count(): number;
  pp_particle_pos_ptr(): number;
  pp_particle_meta_ptr(): number;
  pp_shot_count(): number;
  pp_shot_ptr(): number;
  pp_shot_stride(): number;
  pp_events_ptr(): number;
  pp_stats_ptr(): number;
}

// Stats block layout (must match wasm/src/lib.rs).
const S_PX = 0, S_PY = 1, S_HEALTH = 4, S_CREDITS = 5, S_TIER = 6,
  S_DELIVERED = 7, S_GOAL_VOL = 8, S_PURITY = 9, S_GOAL_PURITY = 10,
  S_PHASE = 11, S_UPGRADE_COST = 13, S_N_STEAM = 14, S_N_IMPACTS = 15,
  S_INTAKE_X = 16, S_INTAKE_Y = 17, S_INTAKE_W = 18, S_INTAKE_H = 19;

const BTN_JUMP = 1, BTN_FIRE = 2;

// Material indices for water/contaminant/molten in materials.ron order.
const KIND_WATER = 7, KIND_CONTAMINANT = 8, KIND_MOLTEN = 9;

declare global {
  interface Window { PP_WASM_B64?: string }
}

async function loadWasm(): Promise<PP> {
  let bytes: ArrayBuffer;
  if (window.PP_WASM_B64) {
    const bin = atob(window.PP_WASM_B64);
    const arr = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
    bytes = arr.buffer;
  } else {
    bytes = await (await fetch("pp_wasm.wasm")).arrayBuffer();
  }
  const { instance } = await WebAssembly.instantiate(bytes, {});
  return instance.exports as unknown as PP;
}

interface Puff { x: number; y: number; age: number; kind: "steam" | "impact" }

function main(pp: PP) {
  if (!pp.pp_init(0)) throw new Error("sim init failed");
  const fieldW = pp.pp_field_w();
  const fieldH = pp.pp_field_h();

  const glCanvas = document.getElementById("gl") as HTMLCanvasElement;
  const ovCanvas = document.getElementById("overlay") as HTMLCanvasElement;
  const ctx = ovCanvas.getContext("2d")!;

  const mem = () => pp.memory.buffer;
  const palette = new Uint8Array(mem(), pp.pp_palette_ptr(), pp.pp_palette_len() * 4).slice();
  const renderer = new Renderer(glCanvas, fieldW, fieldH, palette, [KIND_WATER, KIND_CONTAMINANT, KIND_MOLTEN]);

  const cam: Camera = { x: 0, y: 0, scale: 3 };
  const dpr = Math.min(window.devicePixelRatio || 1, 2);

  // Camera shows ~54 cells across — tuned for the initial design target of
  // an iPhone 17 Pro portrait view (402x874 pt) over the 96-cell-wide
  // level — and never sees past the field edges.
  const VIEW_W_CELLS = 54;
  function resize() {
    const w = window.innerWidth, h = window.innerHeight;
    for (const c of [glCanvas, ovCanvas]) {
      c.width = Math.floor(w * dpr);
      c.height = Math.floor(h * dpr);
      c.style.width = w + "px";
      c.style.height = h + "px";
    }
    cam.scale = Math.max(
      glCanvas.width / Math.min(VIEW_W_CELLS, fieldW),
      glCanvas.width / fieldW,
      glCanvas.height / fieldH,
    );
  }
  window.addEventListener("resize", resize);
  resize();

  // ---- Input ----
  const keys = new Set<string>();
  let mouseX = 0, mouseY = 0, mouseDown = false;
  let touchMoveX = 0;      // -1..1 from joystick
  let touchAim: { x: number; y: number } | null = null;
  let touchJump = false;
  const joystick = { active: false, id: -1, startX: 0 };
  const aimTouch = { active: false, id: -1 };

  const screenToWorld = (sx: number, sy: number) => ({
    x: cam.x + (sx * dpr) / cam.scale,
    y: cam.y + (sy * dpr) / cam.scale,
  });

  window.addEventListener("keydown", (e) => {
    if (["ArrowLeft", "ArrowRight", "ArrowUp", " ", "a", "d", "w"].includes(e.key)) e.preventDefault();
    keys.add(e.key.toLowerCase());
    if (e.key.toLowerCase() === "r") restart();
    if (e.key.toLowerCase() === "u") tryUpgrade();
  });
  window.addEventListener("keyup", (e) => keys.delete(e.key.toLowerCase()));
  ovCanvas.addEventListener("mousemove", (e) => { mouseX = e.clientX; mouseY = e.clientY; });
  ovCanvas.addEventListener("mousedown", (e) => { mouseDown = true; mouseX = e.clientX; mouseY = e.clientY; e.preventDefault(); });
  window.addEventListener("mouseup", () => { mouseDown = false; });
  ovCanvas.addEventListener("contextmenu", (e) => e.preventDefault());

  const jumpBtn = document.getElementById("jump")!;
  jumpBtn.addEventListener("touchstart", (e) => { touchJump = true; e.preventDefault(); }, { passive: false });
  jumpBtn.addEventListener("touchend", (e) => { touchJump = false; e.preventDefault(); }, { passive: false });

  ovCanvas.addEventListener("touchstart", (e) => {
    for (const t of Array.from(e.changedTouches)) {
      if (t.clientX < window.innerWidth * 0.38 && !joystick.active) {
        joystick.active = true; joystick.id = t.identifier; joystick.startX = t.clientX;
      } else if (!aimTouch.active) {
        aimTouch.active = true; aimTouch.id = t.identifier;
        touchAim = screenToWorld(t.clientX, t.clientY);
      }
    }
    e.preventDefault();
  }, { passive: false });
  ovCanvas.addEventListener("touchmove", (e) => {
    for (const t of Array.from(e.changedTouches)) {
      if (joystick.active && t.identifier === joystick.id) {
        touchMoveX = Math.max(-1, Math.min(1, (t.clientX - joystick.startX) / 48));
      } else if (aimTouch.active && t.identifier === aimTouch.id) {
        touchAim = screenToWorld(t.clientX, t.clientY);
      }
    }
    e.preventDefault();
  }, { passive: false });
  const endTouch = (e: TouchEvent) => {
    for (const t of Array.from(e.changedTouches)) {
      if (t.identifier === joystick.id) { joystick.active = false; touchMoveX = 0; }
      if (t.identifier === aimTouch.id) { aimTouch.active = false; touchAim = null; }
    }
    e.preventDefault();
  };
  ovCanvas.addEventListener("touchend", endTouch, { passive: false });
  ovCanvas.addEventListener("touchcancel", endTouch, { passive: false });

  // ---- HUD ----
  const volBar = document.getElementById("volbar") as HTMLDivElement;
  const purBar = document.getElementById("purbar") as HTMLDivElement;
  const purMark = document.getElementById("purmark") as HTMLDivElement;
  const hpBar = document.getElementById("hpbar") as HTMLDivElement;
  const creditsEl = document.getElementById("credits")!;
  const upgradeBtn = document.getElementById("upgrade") as HTMLButtonElement;
  const banner = document.getElementById("banner")!;
  const bannerText = document.getElementById("bannertext")!;
  const restartBtn = document.getElementById("restart")!;

  function tryUpgrade() { pp.pp_buy_upgrade(); }
  upgradeBtn.addEventListener("click", tryUpgrade);
  upgradeBtn.addEventListener("touchstart", (e) => { tryUpgrade(); e.preventDefault(); }, { passive: false });
  function restart() {
    pp.pp_init(0);
    puffs.length = 0;
    banner.style.display = "none";
    bannerShown = false;
  }
  restartBtn.addEventListener("click", restart);
  restartBtn.addEventListener("touchstart", (e) => { restart(); e.preventDefault(); }, { passive: false });

  // ---- Loop ----
  const puffs: Puff[] = [];
  let last = performance.now();
  let acc = 0;
  let bannerShown = false;
  const DT_MS = 1000 / 60;

  function frame(now: number) {
    acc += Math.min(now - last, 100);
    last = now;

    // Input assembly.
    let moveX = touchMoveX;
    if (keys.has("a") || keys.has("arrowleft")) moveX -= 1;
    if (keys.has("d") || keys.has("arrowright")) moveX += 1;
    moveX = Math.max(-1, Math.min(1, moveX));
    let buttons = 0;
    if (keys.has("w") || keys.has(" ") || keys.has("arrowup") || touchJump) buttons |= BTN_JUMP;
    let aim = touchAim ?? screenToWorld(mouseX, mouseY);
    if (mouseDown || touchAim) buttons |= BTN_FIRE;

    let stepped = false;
    while (acc >= DT_MS) {
      pp.pp_step(moveX, aim.x, aim.y, buttons);
      acc -= DT_MS;
      stepped = true;
      if (stepped) {
        // Collect cosmetic events per sim step so none are missed.
        const stats = new Float32Array(mem(), pp.pp_stats_ptr(), 24);
        const nSteam = stats[S_N_STEAM], nImp = stats[S_N_IMPACTS];
        const ev = new Float32Array(mem(), pp.pp_events_ptr(), (nSteam + nImp) * 2);
        for (let i = 0; i < nSteam; i++) puffs.push({ x: ev[i * 2], y: ev[i * 2 + 1], age: 0, kind: "steam" });
        for (let i = 0; i < nImp; i++) puffs.push({ x: ev[(nSteam + i) * 2], y: ev[(nSteam + i) * 2 + 1], age: 0, kind: "impact" });
      }
    }

    const stats = new Float32Array(mem(), pp.pp_stats_ptr(), 24);
    const px = stats[S_PX], py = stats[S_PY];

    // Camera: follow the player, clamped to the field.
    const viewW = glCanvas.width / cam.scale;
    const viewH = glCanvas.height / cam.scale;
    const targetX = Math.max(0, Math.min(fieldW - viewW, px - viewW * 0.5));
    const targetY = Math.max(0, Math.min(fieldH - viewH, py - viewH * 0.45));
    // Smooth follow so digging downward doesn't jitter the view.
    cam.x += (targetX - cam.x) * 0.12;
    cam.y += (targetY - cam.y) * 0.12;

    // Render world.
    const count = pp.pp_particle_count();
    const density = new Uint8Array(mem(), pp.pp_density_ptr(), fieldW * fieldH);
    const material = new Uint8Array(mem(), pp.pp_material_ptr(), fieldW * fieldH);
    const pos = new Float32Array(mem(), pp.pp_particle_pos_ptr(), count * 2);
    const meta = new Uint8Array(mem(), pp.pp_particle_meta_ptr(), count * 4);
    renderer.render(glCanvas, cam, density, material, pos, meta, count, now / 1000);

    // ---- Overlay ----
    ctx.clearRect(0, 0, ovCanvas.width, ovCanvas.height);
    const w2s = (wx: number, wy: number): [number, number] => [(wx - cam.x) * cam.scale, (wy - cam.y) * cam.scale];

    // Intake marker.
    {
      const [ix, iy] = w2s(stats[S_INTAKE_X], stats[S_INTAKE_Y]);
      const iw = stats[S_INTAKE_W] * cam.scale, ih = stats[S_INTAKE_H] * cam.scale;
      ctx.strokeStyle = "rgba(90, 230, 255, 0.8)";
      ctx.lineWidth = 2 * dpr;
      ctx.setLineDash([6 * dpr, 4 * dpr]);
      ctx.strokeRect(ix, iy, iw, ih);
      ctx.setLineDash([]);
      ctx.fillStyle = "rgba(90, 230, 255, 0.9)";
      ctx.font = `${10 * dpr}px monospace`;
      ctx.textAlign = "center";
      ctx.fillText("CORE INTAKE", ix + iw / 2, iy + ih + 12 * dpr);
    }

    // Shots.
    const shotCount = pp.pp_shot_count();
    if (shotCount > 0) {
      const stride = pp.pp_shot_stride();
      const shots = new Float32Array(mem(), pp.pp_shot_ptr(), shotCount * stride);
      ctx.fillStyle = "#ffd27a";
      for (let i = 0; i < shotCount; i++) {
        const [sx, sy] = w2s(shots[i * stride], shots[i * stride + 1]);
        ctx.beginPath();
        ctx.arc(sx, sy, 1.2 * dpr * (cam.scale / dpr / 3), 0, Math.PI * 2);
        ctx.fill();
      }
    }

    // Player: hardhat plumber blob.
    {
      const [sx, sy] = w2s(px, py);
      const r = 2.6 * cam.scale;
      ctx.beginPath();
      ctx.arc(sx, sy, r, 0, Math.PI * 2);
      ctx.fillStyle = "#2e3f66";
      ctx.fill();
      // Helmet.
      ctx.beginPath();
      ctx.arc(sx, sy - r * 0.25, r * 0.78, Math.PI, 0);
      ctx.fillStyle = "#f2c94c";
      ctx.fill();
      // Visor toward aim.
      const dx = aim.x - px, dy = aim.y - py;
      const len = Math.hypot(dx, dy) || 1;
      ctx.beginPath();
      ctx.arc(sx + (dx / len) * r * 0.45, sy + (dy / len) * r * 0.35 - r * 0.05, r * 0.30, 0, Math.PI * 2);
      ctx.fillStyle = "#bfe8ff";
      ctx.fill();
    }

    // Puffs.
    for (let i = puffs.length - 1; i >= 0; i--) {
      const p = puffs[i];
      p.age += 1;
      const life = p.kind === "steam" ? 45 : 14;
      if (p.age > life) { puffs.splice(i, 1); continue; }
      const t = p.age / life;
      const [sx, sy] = w2s(p.x, p.y - (p.kind === "steam" ? t * 6 : 0));
      ctx.beginPath();
      if (p.kind === "steam") {
        ctx.arc(sx, sy, (2 + t * 5) * cam.scale * 0.8, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(230, 235, 245, ${0.28 * (1 - t)})`;
      } else {
        ctx.arc(sx, sy, (1 + t * 3.5) * cam.scale * 0.8, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(255, 205, 120, ${0.5 * (1 - t)})`;
      }
      ctx.fill();
    }

    // Aim reticle (touch).
    if (touchAim) {
      const [sx, sy] = w2s(touchAim.x, touchAim.y);
      ctx.strokeStyle = "rgba(255,255,255,0.7)";
      ctx.lineWidth = 1.5 * dpr;
      ctx.beginPath();
      ctx.arc(sx, sy, 9 * dpr, 0, Math.PI * 2);
      ctx.stroke();
    }

    // HUD update.
    const delivered = stats[S_DELIVERED], goalV = stats[S_GOAL_VOL];
    const purity = stats[S_PURITY], goalP = stats[S_GOAL_PURITY];
    volBar.style.width = `${Math.min(100, (delivered / goalV) * 100)}%`;
    purBar.style.width = `${purity * 100}%`;
    purBar.style.background = purity >= goalP ? "#54d1ff" : "#c9a84a";
    purMark.style.left = `${goalP * 100}%`;
    hpBar.style.width = `${stats[S_HEALTH] * 100}%`;
    creditsEl.textContent = `⬢ ${stats[S_CREDITS]} · tier ${stats[S_TIER]}`;
    const cost = stats[S_UPGRADE_COST];
    upgradeBtn.textContent = `UPGRADE TOOL (${cost})`;
    upgradeBtn.classList.toggle("afford", stats[S_CREDITS] >= cost);

    const phase = stats[S_PHASE];
    if (phase !== 0 && !bannerShown) {
      bannerShown = true;
      banner.style.display = "flex";
      bannerText.textContent = phase === 1 ? "FLOW RESTORED" : "PLUMBER DOWN";
    }

    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}

loadWasm().then(main).catch((e) => {
  document.body.innerHTML = `<pre style="color:#f66;padding:2em">${e}\n${e.stack ?? ""}</pre>`;
});
