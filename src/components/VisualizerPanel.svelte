<script lang="ts">
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import { listen } from '@tauri-apps/api/event'
  import { convertFileSrc } from '@tauri-apps/api/core'
  import { tauriInvoke } from '../lib/tauri'
  import { visualizerOpen, visualizerMode, setVisualizerMode, currentTrack, isAuthed } from '../lib/stores'
  import { IconClose } from '../lib/icons'
  import { getCoverArt } from '../lib/coverCache'
  import { OpenSubsonicRouter } from '../lib/api'
  import { extractOrbPalette, type OrbPalette } from '../lib/coverColor'

  let canvas: HTMLCanvasElement | undefined = $state()

  let bass = 0
  let smoothBass = 0
  let bars: number[] = new Array(24).fill(0)

  const DEFAULT_PALETTE: OrbPalette = {
    primary: { r: 124, g: 92, b: 255 },
    secondary: { r: 170, g: 136, b: 255 },
    tertiary: { r: 85, g: 51, b: 204 },
  }
  let palette: OrbPalette = DEFAULT_PALETTE

  // Continuous animation state (updated each rAF tick)
  let clock = 0       // 0..1, period = 8 s
  let breathe = 0     // 0..1, period = 2.4 s
  let lastTs = 0
  let rafId = 0

  const PARTICLE_COUNT = 28
  const particles = Array.from({ length: PARTICLE_COUNT }, (_, i) => ({
    baseAngle: (i / PARTICLE_COUNT) * 2 * Math.PI,
    speed: 0.3 + (i % 7) * 0.1,
    phase: i / PARTICLE_COUNT,
  }))

  function rgba(c: { r: number; g: number; b: number }, a: number): string {
    return `rgba(${c.r},${c.g},${c.b},${Math.max(0, Math.min(1, a))})`
  }

  function drawOrb(ctx: CanvasRenderingContext2D, w: number, h: number) {
    const cx = w / 2, cy = h / 2
    const maxR = Math.min(w, h) / 2
    const breatheFrac = Math.sin(breathe * 2 * Math.PI) * 0.5 + 0.5
    const baseR = maxR * (0.28 + breatheFrac * 0.08)
    const orbR = baseR * (1 + smoothBass * 0.55)

    // 4-layer glow bloom
    for (let layer = 3; layer >= 0; layer--) {
      const factor = layer / 3
      const alpha = 0.12 + factor * 0.25
      const r = orbR * (1.8 - factor * 0.8)
      const grad = ctx.createRadialGradient(cx, cy, 0, cx, cy, r)
      grad.addColorStop(0, rgba(palette.primary, alpha))
      grad.addColorStop(1, rgba(palette.primary, 0))
      ctx.fillStyle = grad
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fill()
    }

    // Bright core with white hotspot
    const coreGrad = ctx.createRadialGradient(cx, cy, 0, cx, cy, orbR)
    coreGrad.addColorStop(0, 'rgba(255,255,255,0.85)')
    coreGrad.addColorStop(0.5, rgba(palette.primary, 0.9))
    coreGrad.addColorStop(1, rgba(palette.primary, 0))
    ctx.fillStyle = coreGrad
    ctx.beginPath(); ctx.arc(cx, cy, orbR, 0, Math.PI * 2); ctx.fill()

    // 3 staggered expanding rings
    for (let i = 0; i < 3; i++) {
      const phase = (clock + i / 3) % 1
      const ringR = orbR * (1.1 + phase * 2.2)
      const ringAlpha = (1 - phase) * (0.4 + smoothBass * 0.4)
      ctx.strokeStyle = rgba(i % 2 === 0 ? palette.primary : palette.secondary, ringAlpha)
      ctx.lineWidth = Math.max(0.5, 3 - phase * 2.5)
      ctx.beginPath(); ctx.arc(cx, cy, ringR, 0, Math.PI * 2); ctx.stroke()
    }

    // 4 orbiting energy wisps
    for (let w2 = 0; w2 < 4; w2++) {
      const angle = clock * 2 * Math.PI + w2 * (Math.PI / 2)
      const orbitR = orbR * (1.35 + Math.sin(breathe * Math.PI + w2) * 0.15)
      const wx = cx + Math.cos(angle) * orbitR
      const wy = cy + Math.sin(angle) * orbitR
      const wispR = Math.max(1, orbR * (0.18 + smoothBass * 0.12))
      const wispColor = w2 % 2 === 0 ? palette.secondary : palette.tertiary
      const wGrad = ctx.createRadialGradient(wx, wy, 0, wx, wy, wispR)
      wGrad.addColorStop(0, rgba(wispColor, 0.7))
      wGrad.addColorStop(1, rgba(wispColor, 0))
      ctx.fillStyle = wGrad
      ctx.beginPath(); ctx.arc(wx, wy, wispR, 0, Math.PI * 2); ctx.fill()
    }

    // Particle field
    for (const { baseAngle, speed, phase } of particles) {
      const age = (clock + phase) % 1
      const angle = baseAngle + clock * 0.8
      const dist = orbR * (0.9 + age * 1.8 * (0.6 + smoothBass * 0.8) * speed)
      const pColor = age < 0.33 ? palette.primary : age < 0.66 ? palette.secondary : palette.tertiary
      ctx.fillStyle = rgba(pColor, Math.max(0, (1 - age) * 0.7))
      ctx.beginPath()
      ctx.arc(cx + Math.cos(angle) * dist, cy + Math.sin(angle) * dist, Math.max(0.5, 3 - age * 2.5), 0, Math.PI * 2)
      ctx.fill()
    }
  }

  function drawBars(ctx: CanvasRenderingContext2D, w: number, h: number) {
    const gap = 4
    const bw = (w - gap * (bars.length - 1)) / bars.length
    bars.forEach((v, i) => {
      const bh = Math.max(2, v * h)
      const alpha = 0.6 + v * 0.4
      ctx.fillStyle = rgba(palette.primary, alpha)
      ctx.fillRect(i * (bw + gap), h - bh, bw, bh)
    })
  }

  function animate(ts: number) {
    if (!get(visualizerOpen)) { rafId = 0; return }

    const dt = lastTs ? (ts - lastTs) / 1000 : 0
    lastTs = ts
    clock = (clock + dt / 8) % 1
    breathe = (breathe + dt / 2.4) % 1
    smoothBass += (bass - smoothBass) * 0.25

    if (canvas) {
      const ctx = canvas.getContext('2d')
      if (ctx) {
        ctx.clearRect(0, 0, canvas.width, canvas.height)
        if (get(visualizerMode) === 'orb') drawOrb(ctx, canvas.width, canvas.height)
        else drawBars(ctx, canvas.width, canvas.height)
      }
    }

    rafId = requestAnimationFrame(animate)
  }

  function resizeCanvas() {
    if (!canvas) return
    const rect = canvas.getBoundingClientRect()
    canvas.width = rect.width
    canvas.height = rect.height
  }

  function close() { visualizerOpen.set(false) }

  onMount(() => {
    const unlisten = listen<{ bass: number; bars: number[] }>('firmium:audio-analysis', e => {
      bass = e.payload.bass
      bars = e.payload.bars
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
      setTimeout(resizeCanvas, 260)
      if (!rafId) {
        lastTs = 0
        rafId = requestAnimationFrame(animate)
      }
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
          palette = await extractOrbPalette(convertFileSrc(path))
        } catch (_) { palette = DEFAULT_PALETTE }
      })()
    } else {
      ;(async () => {
        try {
          const url = await OpenSubsonicRouter.buildUrl('getCoverArt', { id: track.coverArtId! })
          const assetUrl = await getCoverArt(track.coverArtId!, url)
          palette = await extractOrbPalette(assetUrl)
        } catch (_) { palette = DEFAULT_PALETTE }
      })()
    }
  })
</script>

<div class="visualizer-panel" class:open={$visualizerOpen}>
  <div class="visualizer-safe-top"></div>
  <div class="visualizer-header">
    <span class="visualizer-header-title">Visualizer</span>
    <div class="visualizer-mode-toggle">
      <button class:active={$visualizerMode === 'orb'} onclick={() => setVisualizerMode('orb')}>Orb</button>
      <button class:active={$visualizerMode === 'bars'} onclick={() => setVisualizerMode('bars')}>Bars</button>
    </div>
    <button class="visualizer-close" onclick={close}>
      <span class="icon" style="width:13px;height:13px">{@html IconClose}</span>
    </button>
  </div>
  <hr class="divider" style="margin: 0 20px;">
  <div class="visualizer-body">
    <canvas bind:this={canvas}></canvas>
  </div>
</div>
