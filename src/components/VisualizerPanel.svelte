<script lang="ts">
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import { listen } from '@tauri-apps/api/event'
  import { tauriInvoke } from '../lib/tauri'
  import { visualizerOpen, visualizerMode, setVisualizerMode, currentTrack, isAuthed } from '../lib/stores'
  import { IconClose } from '../lib/icons'
  import { OpenSubsonicRouter } from '../lib/api'
  import type { OrbPalette, CoverColorsResult } from '../lib/types/tauri-commands'

  let canvas: HTMLCanvasElement | undefined = $state()

  let bass = 0
  let smoothBass = 0
  let bars: number[] = new Array(32).fill(0)
  let wave: number[] = new Array(128).fill(0)

  const DEFAULT_PALETTE: OrbPalette = {
    primary:   { r: 124, g: 92,  b: 255 },
    secondary: { r: 170, g: 136, b: 255 },
    tertiary:  { r: 85,  g: 51,  b: 204 },
  }
  let palette: OrbPalette = DEFAULT_PALETTE
  let paletteFlat = new Float32Array([
    124/255, 92/255,  255/255,
    170/255, 136/255, 255/255,
    85/255,  51/255,  204/255,
  ])

  let clock = 0
  let breathe = 0
  let lastTs = 0
  let rafId = 0

  let gl: WebGL2RenderingContext | null = null
  let orbProg: WebGLProgram | null = null
  let barsProg: WebGLProgram | null = null
  let scopeProg: WebGLProgram | null = null

  // ── Shaders ───────────────────────────────────────────────────────────────
  //
  // Fullscreen triangle trick: 3 vertices, no buffers.
  // VertexID 0→(−1,−1), 1→(3,−1), 2→(−1,3) covers the whole clip quad.

  const VERT_FULL = `#version 300 es
void main() {
  vec2 p = vec2(float(gl_VertexID % 2), float(gl_VertexID / 2));
  gl_Position = vec4(p * 4.0 - 1.0, 0.0, 1.0);
}
`

  // Orb: fragment shader draws everything — glow bloom, rings, wisps, particles.
  // All geometry is computed analytically via distance functions.
  // Additive blending (ONE, ONE) on black background gives the glow look.
  const FRAG_ORB = `#version 300 es
precision highp float;
out vec4 fragColor;

uniform vec2  u_res;
uniform float u_bass;
uniform float u_clock;
uniform float u_breathe;
uniform vec3  u_pal[3];

const float TAU = 6.28318530718;

vec3 pal(float t) {
  t = fract(t);
  if (t < 0.333) return mix(u_pal[0], u_pal[1], t / 0.333);
  if (t < 0.666) return mix(u_pal[1], u_pal[2], (t - 0.333) / 0.333);
  return mix(u_pal[2], u_pal[0], (t - 0.666) / 0.334);
}

void main() {
  // Aspect-corrected UV, centered at (0,0)
  vec2 p = (gl_FragCoord.xy / u_res - 0.5);
  p.x *= u_res.x / u_res.y;

  float d    = length(p);
  float bf   = sin(u_breathe * TAU) * 0.5 + 0.5;
  float orbR = 0.075 + bf * 0.022 + u_bass * 0.04;

  vec3 col = vec3(0.0);

  // 4-layer glow bloom
  for (int i = 0; i < 4; i++) {
    float f = float(i) / 3.0;
    float r = orbR * (1.8 - f * 0.8);
    float a = 0.12 + f * 0.25;
    col += pal(u_clock) * a * exp(-d * d / (r * r) * 2.0);
  }

  // Bright core with white hotspot
  float cm = exp(-d * d / (orbR * orbR) * 3.0);
  vec3 ch = mix(vec3(1.0), pal(u_clock), smoothstep(0.0, orbR * 0.5, d));
  col += ch * cm * 0.9;

  // 3 expanding rings
  for (int i = 0; i < 3; i++) {
    float ph  = fract(u_clock + float(i) / 3.0);
    float rr  = orbR * (1.1 + ph * 2.2);
    float ra  = (1.0 - ph) * (0.4 + u_bass * 0.4);
    vec3  rc  = (i % 2 == 0) ? pal(u_clock + 0.33) : pal(u_clock + 0.55);
    float rw  = max(0.5, 3.0 - ph * 2.5) / min(u_res.x, u_res.y);
    col += rc * ra * smoothstep(rw, 0.0, abs(d - rr));
  }

  // 4 orbiting wisps — positions in same aspect-corrected space as p
  for (int w = 0; w < 4; w++) {
    float ang  = u_clock * TAU + float(w) * (TAU / 4.0);
    float oR   = orbR * (1.35 + sin(u_breathe * TAU + float(w)) * 0.15);
    vec2  wpos = vec2(cos(ang), sin(ang)) * oR;
    float wd   = length(p - wpos);
    float wR   = orbR * (0.18 + u_bass * 0.12);
    vec3  wc   = (w % 2 == 0) ? pal(u_clock + 0.17) : pal(u_clock + 0.50);
    col += wc * 0.7 * exp(-wd * wd / (wR * wR) * 4.0);
  }

  // 28 particles
  vec3 pa = pal(u_clock + 0.10);
  vec3 pb = pal(u_clock + 0.40);
  vec3 pc = pal(u_clock + 0.70);
  for (int k = 0; k < 28; k++) {
    float baseA = float(k) / 28.0 * TAU;
    float speed = 0.3 + float(k % 7) * 0.1;
    float age   = fract(u_clock + float(k) / 28.0);
    float ang   = baseA + u_clock * 0.8 * TAU;
    float pd    = orbR * (0.9 + age * 1.8 * (0.6 + u_bass * 0.8) * speed);
    vec2  pp    = vec2(cos(ang), sin(ang)) * pd;
    float dd    = length(p - pp);
    float pr    = max(0.004, (3.0 - age * 2.5) / min(u_res.x, u_res.y));
    vec3  pkc   = (age < 0.33) ? pa : (age < 0.66) ? pb : pc;
    col += pkc * max(0.0, (1.0 - age) * 0.7) * exp(-dd * dd / (pr * pr));
  }

  fragColor = vec4(col, 1.0);
}
`

  // Bars: vertex shader builds 32 quads (6 verts each) from u_bars uniform array.
  // No vertex buffers — positions derived entirely from gl_VertexID.
  const VERT_BARS = `#version 300 es
uniform float u_bars[32];
uniform vec2  u_res;

out float v_t;    // 0 = bottom of bar, 1 = top of bar
out float v_idx;  // bar index, 0..31

void main() {
  int bar     = gl_VertexID / 6;
  int corner  = gl_VertexID % 6;

  float gap  = 3.0;
  float barW = (u_res.x - gap * 31.0) / 32.0;
  float barH = u_bars[bar] * u_res.y;

  float xL = float(bar) * (barW + gap);
  float xR = xL + barW;

  // 2 triangles CW from bottom-left: BL BR TR  BL TR TL
  bool right = (corner == 1 || corner == 2 || corner == 4);
  bool top   = (corner == 2 || corner == 4 || corner == 5);

  float px = right ? xR : xL;
  float py = top   ? barH : 0.0;

  v_t   = (barH > 0.0) ? py / barH : 0.0;
  v_idx = float(bar);

  // pixel → clip (WebGL y=0 is bottom)
  gl_Position = vec4((px / u_res.x) * 2.0 - 1.0,
                     (py / u_res.y) * 2.0 - 1.0,
                     0.0, 1.0);
}
`

  const FRAG_BARS = `#version 300 es
precision highp float;
in  float v_t;
in  float v_idx;
out vec4  fragColor;

uniform float u_clock;
uniform vec3  u_pal[3];

vec3 pal(float t) {
  t = fract(t);
  if (t < 0.333) return mix(u_pal[0], u_pal[1], t / 0.333);
  if (t < 0.666) return mix(u_pal[1], u_pal[2], (t - 0.333) / 0.333);
  return mix(u_pal[2], u_pal[0], (t - 0.666) / 0.334);
}

void main() {
  vec3 col = pal(u_clock + v_idx / 32.0);
  // Brighter at top, dim at bottom, glow highlight at the peak
  float bright = 0.45 + v_t * 0.55 + smoothstep(0.85, 1.0, v_t) * 0.5;
  // Fade low bars more aggressively so noise doesn't glow
  float alpha = clamp(bright, 0.0, 1.0);
  fragColor = vec4(col * bright, alpha);
}
`

  // Oscilloscope: 128-point waveform wrapped into a ring via LINE_LOOP. Each vertex's
  // radius is modulated by the waveform sample; no vertex buffers (positions from gl_VertexID).
  const VERT_SCOPE = `#version 300 es
uniform float u_wave[128];
uniform vec2  u_res;
out float v_i;
const float TAU = 6.28318530718;
void main() {
  int i = gl_VertexID;
  float a = float(i) / 128.0 * TAU;
  float r = 0.34 + u_wave[i] * 0.20;
  vec2 p = vec2(cos(a), sin(a)) * r;
  // keep the ring circular regardless of canvas aspect
  if (u_res.x > u_res.y) p.x *= u_res.y / u_res.x; else p.y *= u_res.x / u_res.y;
  v_i = float(i) / 128.0;
  gl_Position = vec4(p, 0.0, 1.0);
}
`

  const FRAG_SCOPE = `#version 300 es
precision highp float;
in  float v_i;
out vec4  fragColor;
uniform float u_clock;
uniform vec3  u_pal[3];
vec3 pal(float t) {
  t = fract(t);
  if (t < 0.333) return mix(u_pal[0], u_pal[1], t / 0.333);
  if (t < 0.666) return mix(u_pal[1], u_pal[2], (t - 0.333) / 0.333);
  return mix(u_pal[2], u_pal[0], (t - 0.666) / 0.334);
}
void main() { fragColor = vec4(pal(u_clock + v_i), 1.0); }
`

  // ── WebGL init ────────────────────────────────────────────────────────────

  function shader(g: WebGL2RenderingContext, type: number, src: string): WebGLShader {
    const s = g.createShader(type)!
    g.shaderSource(s, src)
    g.compileShader(s)
    if (!g.getShaderParameter(s, g.COMPILE_STATUS)) {
      const log = g.getShaderInfoLog(s); g.deleteShader(s)
      throw new Error(`Shader error: ${log}`)
    }
    return s
  }

  function program(g: WebGL2RenderingContext, vs: string, fs: string): WebGLProgram {
    const p = g.createProgram()!
    g.attachShader(p, shader(g, g.VERTEX_SHADER, vs))
    g.attachShader(p, shader(g, g.FRAGMENT_SHADER, fs))
    g.linkProgram(p)
    if (!g.getProgramParameter(p, g.LINK_STATUS)) {
      const log = g.getProgramInfoLog(p); g.deleteProgram(p)
      throw new Error(`Link error: ${log}`)
    }
    return p
  }

  function initGL(c: HTMLCanvasElement): WebGL2RenderingContext | null {
    // alpha: false → opaque canvas, avoids compositing headaches with additive blend
    const g = c.getContext('webgl2', { antialias: false, alpha: false })
    if (!g) { console.error('[visualizer] WebGL2 unavailable'); return null }
    try {
      orbProg  = program(g, VERT_FULL, FRAG_ORB)
      barsProg = program(g, VERT_BARS, FRAG_BARS)
      scopeProg = program(g, VERT_SCOPE, FRAG_SCOPE)
    } catch (e) {
      console.error('[visualizer]', e); return null
    }
    g.enable(g.BLEND)
    return g
  }

  // ── Render ────────────────────────────────────────────────────────────────

  function renderOrb(g: WebGL2RenderingContext, w: number, h: number) {
    g.blendFunc(g.ONE, g.ONE)        // additive glow on black
    g.useProgram(orbProg)
    const u = (n: string) => g.getUniformLocation(orbProg!, n)
    g.uniform2f(u('u_res'),    w, h)
    g.uniform1f(u('u_bass'),   smoothBass)
    g.uniform1f(u('u_clock'),  clock)
    g.uniform1f(u('u_breathe'), breathe)
    g.uniform3fv(u('u_pal'),   paletteFlat)
    g.drawArrays(g.TRIANGLES, 0, 3)
  }

  function renderBars(g: WebGL2RenderingContext, w: number, h: number) {
    g.blendFunc(g.SRC_ALPHA, g.ONE_MINUS_SRC_ALPHA)
    g.useProgram(barsProg)
    const u = (n: string) => g.getUniformLocation(barsProg!, n)
    g.uniform2f(u('u_res'),   w, h)
    g.uniform1fv(u('u_bars'), new Float32Array(bars))
    g.uniform1f(u('u_clock'), clock)
    g.uniform3fv(u('u_pal'),  paletteFlat)
    g.drawArrays(g.TRIANGLES, 0, 32 * 6)
  }

  function renderScope(g: WebGL2RenderingContext, w: number, h: number) {
    g.blendFunc(g.SRC_ALPHA, g.ONE_MINUS_SRC_ALPHA)
    g.useProgram(scopeProg)
    const u = (n: string) => g.getUniformLocation(scopeProg!, n)
    g.uniform2f(u('u_res'),   w, h)
    g.uniform1fv(u('u_wave'), new Float32Array(wave))
    g.uniform1f(u('u_clock'), clock)
    g.uniform3fv(u('u_pal'),  paletteFlat)
    g.drawArrays(g.LINE_LOOP, 0, 128)
  }

  // ── Animation loop ────────────────────────────────────────────────────────

  function animate(ts: number) {
    if (!get(visualizerOpen)) { rafId = 0; return }

    const dt = lastTs ? (ts - lastTs) / 1000 : 0
    lastTs  = ts
    clock   = (clock   + dt / 8)   % 1
    breathe = (breathe + dt / 2.4) % 1
    smoothBass += (bass - smoothBass) * 0.25

    if (canvas && gl) {
      const w = canvas.width, h = canvas.height
      gl.viewport(0, 0, w, h)
      gl.clearColor(0, 0, 0, 1)
      gl.clear(gl.COLOR_BUFFER_BIT)
      const mode = get(visualizerMode)
      if (mode === 'orb') renderOrb(gl, w, h)
      else if (mode === 'oscilloscope') renderScope(gl, w, h)
      else renderBars(gl, w, h)
    }

    rafId = requestAnimationFrame(animate)
  }

  function resizeCanvas() {
    if (!canvas) return
    const rect = canvas.getBoundingClientRect()
    canvas.width  = Math.round(rect.width)
    canvas.height = Math.round(rect.height)
  }

  function close() { visualizerOpen.set(false) }

  const MODES = ['orb', 'bars', 'oscilloscope'] as const
  function cycleMode() {
    const i = MODES.indexOf(get(visualizerMode))
    setVisualizerMode(MODES[(i + 1) % MODES.length])
  }

  onMount(() => {
    const unlisten = listen<{ bass: number; bars: number[]; wave?: number[] }>('firmium:audio-analysis', e => {
      bass = e.payload.bass
      bars = e.payload.bars
      if (e.payload.wave) wave = e.payload.wave
    })
    window.addEventListener('resize', resizeCanvas)
    resizeCanvas()
    return () => {
      unlisten.then(f => f())
      window.removeEventListener('resize', resizeCanvas)
      if (rafId) cancelAnimationFrame(rafId)
    }
  })

  $effect(() => {
    tauriInvoke('set_visualizer_enabled', { enabled: $visualizerOpen }).catch(() => {})
    if ($visualizerOpen) {
      setTimeout(() => {
        resizeCanvas()
        if (canvas && !gl) gl = initGL(canvas)
        if (!rafId) { lastTs = 0; rafId = requestAnimationFrame(animate) }
      }, 260)
    } else {
      if (rafId) { cancelAnimationFrame(rafId); rafId = 0 }
    }
  })

  $effect(() => {
    const track = $currentTrack
    if (!track?.coverArtId) { palette = DEFAULT_PALETTE; return }
    if (track.id.startsWith('local:')) {
      ;(async () => {
        try {
          const path = await tauriInvoke<string>('get_local_cover_art', { id: track.coverArtId })
          const result = await tauriInvoke<CoverColorsResult>('extract_cover_colors_from_path', { path })
          palette = result?.orb ?? DEFAULT_PALETTE
        } catch { palette = DEFAULT_PALETTE }
      })()
    } else {
      ;(async () => {
        try {
          const url = await OpenSubsonicRouter.buildUrl('getCoverArt', { id: track.coverArtId! })
          const result = await tauriInvoke<CoverColorsResult>('extract_cover_colors', { coverId: track.coverArtId, url })
          palette = result?.orb ?? DEFAULT_PALETTE
        } catch { palette = DEFAULT_PALETTE }
      })()
    }
  })

  $effect(() => {
    paletteFlat = new Float32Array([
      palette.primary.r   / 255, palette.primary.g   / 255, palette.primary.b   / 255,
      palette.secondary.r / 255, palette.secondary.g / 255, palette.secondary.b / 255,
      palette.tertiary.r  / 255, palette.tertiary.g  / 255, palette.tertiary.b  / 255,
    ])
  })
</script>

<div class="visualizer-panel" class:open={$visualizerOpen}>
  <div class="visualizer-safe-top"></div>
  <div class="visualizer-header">
    <span class="visualizer-header-title">Visualizer</span>
    <div class="visualizer-mode-toggle">
      <button class:active={$visualizerMode === 'orb'} onclick={() => setVisualizerMode('orb')}>Orb</button>
      <button class:active={$visualizerMode === 'bars'} onclick={() => setVisualizerMode('bars')}>Bars</button>
      <button class:active={$visualizerMode === 'oscilloscope'} onclick={() => setVisualizerMode('oscilloscope')}>Scope</button>
    </div>
    <button class="visualizer-close" onclick={close}>
      <span class="icon" style="width:13px;height:13px">{@html IconClose}</span>
    </button>
  </div>
  <hr class="divider" style="margin: 0 20px;">
  <div class="visualizer-body">
    <canvas
      bind:this={canvas}
      role="button"
      tabindex="0"
      title="Tap to change visualizer"
      onclick={cycleMode}
      onkeydown={e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); cycleMode() } }}
    ></canvas>
  </div>
</div>
