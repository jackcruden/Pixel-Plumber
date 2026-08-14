// WebGL2 renderer: terrain from the density/material textures, fluid as
// metaballs (splat -> threshold), all reading zero-copy views into WASM
// linear memory.
//
// Look: chunky PixelJunk-Shooter-style rendering — posterised material
// shading in big blocks, dark surface outlines with a lit top crust,
// ambient-occluded caves, and water with speed-driven foam.

export interface Camera {
  // World-cell offset of the top-left of the view and cells-per-pixel scale.
  x: number;
  y: number;
  scale: number; // screen px per world cell
}

function compile(gl: WebGL2RenderingContext, vsSrc: string, fsSrc: string): WebGLProgram {
  const vs = gl.createShader(gl.VERTEX_SHADER)!;
  gl.shaderSource(vs, vsSrc);
  gl.compileShader(vs);
  if (!gl.getShaderParameter(vs, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(vs) ?? "vs");
  const fs = gl.createShader(gl.FRAGMENT_SHADER)!;
  gl.shaderSource(fs, fsSrc);
  gl.compileShader(fs);
  if (!gl.getShaderParameter(fs, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(fs) ?? "fs");
  const p = gl.createProgram()!;
  gl.attachShader(p, vs);
  gl.attachShader(p, fs);
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p) ?? "link");
  return p;
}

const QUAD_VS = `#version 300 es
layout(location=0) in vec2 aPos;
out vec2 vUv;
void main() {
  vUv = aPos * 0.5 + 0.5;
  gl_Position = vec4(aPos, 0.0, 1.0);
}`;

const TERRAIN_FS = `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 frag;
uniform sampler2D uDensity;   // R8 LINEAR, 0.5 = surface
uniform sampler2D uMaterial;  // R8 NEAREST, material index
uniform vec3 uPalette[16];
uniform vec2 uView;           // view size in cells
uniform vec2 uCam;            // camera offset in cells
uniform vec2 uField;          // field size in cells
uniform float uTime;

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

void main() {
  vec2 world = vec2(uCam.x + vUv.x * uView.x, uCam.y + (1.0 - vUv.y) * uView.y);
  vec2 uv = world / uField;
  if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) {
    frag = vec4(0.075, 0.075, 0.10, 1.0);
    return;
  }
  float d = texture(uDensity, uv).r;
  float mat = texture(uMaterial, uv).r * 255.0;
  int mi = int(mat + 0.5);
  vec3 base = uPalette[mi > 15 ? 0 : mi];
  float depthFrac = world.y / uField.y;

  // Antialiased solid mask around the iso-surface.
  float aa = fwidth(d) * 1.2 + 0.004;
  float solid = smoothstep(0.5 - aa, 0.5 + aa, d);

  // ---- Rock interior: posterised chunky shading ----
  // Big 2-cell blocks quantised into three value bands, plus sparse larger
  // blotches — features read at phone size instead of fine speckle.
  float blocks = hash(floor(world / 2.0));
  float band = floor(blocks * 3.0) / 3.0;
  float shade = 0.80 + band * 0.30;
  float blotch = hash(floor(world / 6.0) + 13.0);
  if (blotch < 0.28) shade *= 0.82;
  shade *= 1.0 - 0.22 * depthFrac;
  vec3 rock = base * shade;

  // Pipework (index 4): machined horizontal banding, no mineral grain.
  if (mi == 4) {
    float stripe = step(0.55, fract(world.y / 3.0));
    rock = base * (0.85 + 0.18 * stripe) * (1.0 - 0.15 * depthFrac);
  }
  // Slag (index 5): cooled-magma flecks of ember.
  if (mi == 5) {
    float fleck = step(0.90, hash(floor(world * 1.2) + 3.0));
    rock += vec3(0.55, 0.22, 0.05) * fleck;
  }

  // Dark outline hugging the surface (the PixelJunk silhouette).
  float outline = smoothstep(0.50, 0.535, d) * (1.0 - smoothstep(0.56, 0.63, d));
  rock *= 1.0 - 0.62 * outline;

  // Lit crust on up-facing surfaces: compare against the density one cell up.
  float dUp = texture(uDensity, (world - vec2(0.0, 1.3)) / uField).r;
  float crust = smoothstep(0.05, 0.22, d - dUp)
              * smoothstep(0.50, 0.56, d) * (1.0 - smoothstep(0.60, 0.72, d));
  rock += base * crust * 0.85 + vec3(0.10) * crust;

  // ---- Cave: dark ground, wall occlusion, faint large-scale variation ----
  vec3 cave = mix(vec3(0.10, 0.09, 0.13), vec3(0.035, 0.035, 0.055), depthFrac);
  cave *= 0.85 + 0.15 * hash(floor(world / 5.0) + 31.0);
  // Ambient occlusion: empty space near a wall darkens — quantised into
  // bands so it reads as chunky shadow, not blur.
  float ao = smoothstep(0.34, 0.50, d);
  ao = floor(ao * 3.0 + 0.5) / 3.0;
  cave *= 1.0 - 0.42 * ao;

  frag = vec4(mix(cave, rock, solid), 1.0);
}`;

const SPLAT_VS = `#version 300 es
layout(location=0) in vec2 aPos;   // world cells
layout(location=1) in vec4 aMeta;  // kind, purity, speed, _ (0..255)
uniform vec2 uCam;
uniform vec2 uView;
uniform float uPointPx;
out vec3 vColor;
out float vGlow;
uniform vec3 uPalette[16];
uniform vec3 uKinds; // water, contaminant, molten indices
void main() {
  vec2 clip = ((aPos - uCam) / uView) * 2.0 - 1.0;
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
  gl_PointSize = uPointPx;
  int k = int(aMeta.x + 0.5);
  float purity = aMeta.y / 255.0;
  float speed = aMeta.z / 255.0;
  vec3 c = uPalette[k > 15 ? 0 : k];
  vGlow = 0.0;
  if (k == int(uKinds.x + 0.5)) {
    // Water: murkier as purity drops, whitened by speed (turbulence foam).
    c = mix(vec3(0.42, 0.47, 0.16), c, purity);
    c = mix(c, vec3(0.93, 0.97, 1.0), speed * speed * 0.6);
  } else if (k == int(uKinds.z + 0.5)) {
    // Molten: fast flow is bright liquid fire, slow flow crusts dark.
    c = mix(c * 0.55, vec3(1.0, 0.78, 0.25), speed * 0.9 + 0.15);
    vGlow = 1.0;
  } else if (k == int(uKinds.y + 0.5)) {
    // Contaminant: dull sludge, slightly lighter when agitated.
    c = mix(c * 0.8, c * 1.15, speed);
  }
  vColor = c;
}`;

const SPLAT_FS = `#version 300 es
precision mediump float;
in vec3 vColor;
in float vGlow;
out vec4 frag;
void main() {
  vec2 d = gl_PointCoord * 2.0 - 1.0;
  float r2 = dot(d, d);
  if (r2 > 1.0) discard;
  float w = exp(-r2 * 3.0) * 0.30;
  frag = vec4(vColor * w, w) * (1.0 + vGlow * 0.6);
}`;

const COMPOSITE_FS = `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 frag;
uniform sampler2D uSplat;
uniform vec2 uCam;
uniform vec2 uView;
uniform float uTime;

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

void main() {
  vec4 s = texture(uSplat, vUv);
  float t = s.a;
  float body = smoothstep(0.20, 0.34, t);
  if (body <= 0.003) discard;
  vec3 c = s.rgb / max(t, 1e-4);

  // Depth: thick fluid darkens and saturates.
  c *= 1.0 - smoothstep(0.55, 1.5, t) * 0.30;

  // Surface band near the threshold.
  float rim = smoothstep(0.20, 0.29, t) * (1.0 - smoothstep(0.29, 0.52, t));

  // Foam: animated chunky dither on the surface band. Fast (already
  // whitened) fluid foams hard; calm surfaces get only a thin bright line.
  vec2 world = vec2(uCam.x + vUv.x * uView.x, uCam.y + (1.0 - vUv.y) * uView.y);
  float sparkle = step(0.42, hash(floor(world * 1.6) + floor(uTime * 7.0) * 17.0));
  float whiteness = smoothstep(0.55, 0.9, max(c.r, max(c.g, c.b)));
  float foam = rim * (0.30 + 0.70 * max(sparkle * 0.8, whiteness));
  c = mix(c, vec3(0.94, 0.97, 1.0), clamp(foam, 0.0, 0.85));

  // Top-surface sheen: t falls off upward at an up-facing surface.
  float sheen = clamp(-dFdy(t) * 6.0, 0.0, 1.0) * rim;
  c += sheen * 0.20;

  frag = vec4(c, body * 0.95);
}`;

export class Renderer {
  gl: WebGL2RenderingContext;
  private terrainProg: WebGLProgram;
  private splatProg: WebGLProgram;
  private compositeProg: WebGLProgram;
  private quad: WebGLVertexArrayObject;
  private densityTex: WebGLTexture;
  private materialTex: WebGLTexture;
  private splatFbo: WebGLFramebuffer;
  private splatTex: WebGLTexture;
  private splatW = 0;
  private splatH = 0;
  private particleVao: WebGLVertexArrayObject;
  private posBuf: WebGLBuffer;
  private metaBuf: WebGLBuffer;
  private palette: Float32Array;
  fieldW: number;
  fieldH: number;
  kinds: [number, number, number];

  constructor(
    canvas: HTMLCanvasElement,
    fieldW: number,
    fieldH: number,
    palette: Uint8Array,
    kinds: [number, number, number],
  ) {
    const gl = canvas.getContext("webgl2", { alpha: false, antialias: false })!;
    if (!gl) throw new Error("WebGL2 required");
    this.gl = gl;
    this.fieldW = fieldW;
    this.fieldH = fieldH;
    this.kinds = kinds;

    this.palette = new Float32Array(16 * 3);
    for (let i = 0; i < Math.min(16, palette.length / 4); i++) {
      this.palette[i * 3] = palette[i * 4] / 255;
      this.palette[i * 3 + 1] = palette[i * 4 + 1] / 255;
      this.palette[i * 3 + 2] = palette[i * 4 + 2] / 255;
    }

    this.terrainProg = compile(gl, QUAD_VS, TERRAIN_FS);
    this.splatProg = compile(gl, SPLAT_VS, SPLAT_FS);
    this.compositeProg = compile(gl, QUAD_VS, COMPOSITE_FS);

    // Fullscreen quad.
    this.quad = gl.createVertexArray()!;
    gl.bindVertexArray(this.quad);
    const qb = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, qb);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

    const mkTex = (filter: number) => {
      const t = gl.createTexture()!;
      gl.bindTexture(gl.TEXTURE_2D, t);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      return t;
    };
    this.densityTex = mkTex(gl.LINEAR);
    this.materialTex = mkTex(gl.NEAREST);

    // Particle buffers + VAO.
    this.particleVao = gl.createVertexArray()!;
    gl.bindVertexArray(this.particleVao);
    this.posBuf = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.posBuf);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    this.metaBuf = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.metaBuf);
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 4, gl.UNSIGNED_BYTE, false, 0, 0);

    // Splat FBO (created on first resize).
    this.splatFbo = gl.createFramebuffer()!;
    this.splatTex = mkTex(gl.LINEAR);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  }

  resize(w: number, h: number) {
    const gl = this.gl;
    const sw = Math.max(2, Math.floor(w / 2));
    const sh = Math.max(2, Math.floor(h / 2));
    if (sw === this.splatW && sh === this.splatH) return;
    this.splatW = sw;
    this.splatH = sh;
    gl.bindTexture(gl.TEXTURE_2D, this.splatTex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, sw, sh, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.splatFbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, this.splatTex, 0);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }

  render(
    canvas: HTMLCanvasElement,
    cam: Camera,
    density: Uint8Array,
    material: Uint8Array,
    particlePos: Float32Array,
    particleMeta: Uint8Array,
    particleCount: number,
    time: number,
  ) {
    const gl = this.gl;
    this.resize(canvas.width, canvas.height);
    const viewW = canvas.width / cam.scale;
    const viewH = canvas.height / cam.scale;

    // Upload field textures.
    gl.bindTexture(gl.TEXTURE_2D, this.densityTex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, this.fieldW, this.fieldH, 0, gl.RED, gl.UNSIGNED_BYTE, density);
    gl.bindTexture(gl.TEXTURE_2D, this.materialTex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, this.fieldW, this.fieldH, 0, gl.RED, gl.UNSIGNED_BYTE, material);

    // --- Pass 1: particle splats into half-res FBO ---
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.splatFbo);
    gl.viewport(0, 0, this.splatW, this.splatH);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    if (particleCount > 0) {
      gl.useProgram(this.splatProg);
      gl.bindVertexArray(this.particleVao);
      gl.bindBuffer(gl.ARRAY_BUFFER, this.posBuf);
      gl.bufferData(gl.ARRAY_BUFFER, particlePos.subarray(0, particleCount * 2), gl.DYNAMIC_DRAW);
      gl.bindBuffer(gl.ARRAY_BUFFER, this.metaBuf);
      gl.bufferData(gl.ARRAY_BUFFER, particleMeta.subarray(0, particleCount * 4), gl.DYNAMIC_DRAW);
      const u = (n: string) => gl.getUniformLocation(this.splatProg, n);
      gl.uniform2f(u("uCam"), cam.x, cam.y);
      gl.uniform2f(u("uView"), viewW, viewH);
      // Splat support radius ~2.6 cells; the FBO is half-res, so a point's
      // pixel size there is cells * (scale/2) * 2 = cells * scale.
      gl.uniform1f(u("uPointPx"), 2.6 * cam.scale);
      gl.uniform3fv(u("uPalette"), this.palette);
      gl.uniform3f(u("uKinds"), this.kinds[0], this.kinds[1], this.kinds[2]);
      gl.enable(gl.BLEND);
      gl.blendFunc(gl.ONE, gl.ONE);
      gl.drawArrays(gl.POINTS, 0, particleCount);
      gl.disable(gl.BLEND);
    }

    // --- Pass 2: terrain to screen ---
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.useProgram(this.terrainProg);
    gl.bindVertexArray(this.quad);
    const t = (n: string) => gl.getUniformLocation(this.terrainProg, n);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.densityTex);
    gl.uniform1i(t("uDensity"), 0);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.materialTex);
    gl.uniform1i(t("uMaterial"), 1);
    gl.uniform3fv(t("uPalette"), this.palette);
    gl.uniform2f(t("uView"), viewW, viewH);
    gl.uniform2f(t("uCam"), cam.x, cam.y);
    gl.uniform2f(t("uField"), this.fieldW, this.fieldH);
    gl.uniform1f(t("uTime"), time);
    gl.drawArrays(gl.TRIANGLES, 0, 3);

    // --- Pass 3: composite metaballs over terrain ---
    gl.useProgram(this.compositeProg);
    const c = (n: string) => gl.getUniformLocation(this.compositeProg, n);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.splatTex);
    gl.uniform1i(c("uSplat"), 0);
    gl.uniform2f(c("uCam"), cam.x, cam.y);
    gl.uniform2f(c("uView"), viewW, viewH);
    gl.uniform1f(c("uTime"), time);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.disable(gl.BLEND);
  }
}
