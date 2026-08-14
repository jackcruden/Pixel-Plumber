// WebGL2 renderer: terrain from the density/material textures, fluid as
// metaballs (splat -> threshold), all reading zero-copy views into WASM
// linear memory.

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
uniform sampler2D uDensity;   // R8 LINEAR, 128 = surface
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
    // Outside the level: solid machine casing.
    frag = vec4(0.10, 0.10, 0.13, 1.0);
    return;
  }
  float d = texture(uDensity, uv).r;           // 0.5 = surface
  float mat = texture(uMaterial, uv).r * 255.0;
  int mi = int(mat + 0.5);
  vec3 base = uPalette[mi > 15 ? 0 : mi];

  // Antialiased solid mask around the iso-surface.
  float w = fwidth(d) * 1.2 + 0.004;
  float solid = smoothstep(0.5 - w, 0.5 + w, d);

  // Grain: cell-scale speckle so deposits read as mineral, not flat fill.
  float g = hash(floor(world * 2.0)) * 0.16 + hash(floor(world * 0.5)) * 0.10;
  float depthShade = 1.0 - 0.30 * (world.y / uField.y);
  vec3 rock = base * (0.82 + g) * depthShade;
  // Edge highlight just inside the surface.
  float edge = smoothstep(0.5, 0.56, d) * (1.0 - smoothstep(0.56, 0.70, d));
  rock += base * edge * 0.35;

  // Empty space: dark cavern with faint depth gradient and dust.
  vec3 cave = mix(vec3(0.055, 0.05, 0.075), vec3(0.02, 0.02, 0.03), world.y / uField.y);
  cave += hash(floor(world * 1.3)) * 0.012;

  frag = vec4(mix(cave, rock, solid), 1.0);
}`;

const SPLAT_VS = `#version 300 es
layout(location=0) in vec2 aPos;   // world cells
layout(location=1) in vec2 aMeta;  // kind, purity (0..255)
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
  vec3 c = uPalette[k > 15 ? 0 : k];
  if (k == int(uKinds.x + 0.5)) {
    // Water shifts toward murk as purity drops.
    c = mix(vec3(0.45, 0.50, 0.18), c, purity);
  }
  vGlow = (k == int(uKinds.z + 0.5)) ? 1.0 : 0.0;
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
void main() {
  vec4 s = texture(uSplat, vUv);
  float t = s.a;
  // Metaball threshold with a soft shoulder.
  float body = smoothstep(0.22, 0.38, t);
  if (body <= 0.0) discard;
  vec3 c = s.rgb / max(t, 1e-4);
  // Surface sheen where the field just crosses the threshold.
  float rim = smoothstep(0.22, 0.30, t) * (1.0 - smoothstep(0.30, 0.55, t));
  c += rim * 0.25;
  // Deep-body darkening gives volume.
  c *= 1.0 - smoothstep(0.6, 1.6, t) * 0.25;
  frag = vec4(c, body * 0.92);
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
    gl.vertexAttribPointer(1, 2, gl.UNSIGNED_BYTE, false, 0, 0);

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
      gl.bufferData(gl.ARRAY_BUFFER, particleMeta.subarray(0, particleCount * 2), gl.DYNAMIC_DRAW);
      const u = (n: string) => gl.getUniformLocation(this.splatProg, n);
      gl.uniform2f(u("uCam"), cam.x, cam.y);
      gl.uniform2f(u("uView"), viewW, viewH);
      // Metaball support radius ~2.2 cells at half-res.
      gl.uniform1f(u("uPointPx"), 2.2 * cam.scale);
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
    // Flip Y: world y grows down, clip y grows up — handled by uv math.
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
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.disable(gl.BLEND);
  }
}
