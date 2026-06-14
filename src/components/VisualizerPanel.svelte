<script lang="ts">
  import { onMount } from 'svelte'
  import { listen } from '@tauri-apps/api/event'
  import { tauriInvoke } from '../lib/tauri'
  import { visualizerOpen, visualizerMode, setVisualizerMode } from '../lib/stores'
  import { IconClose } from '../lib/icons'

  let canvas: HTMLCanvasElement | undefined = $state()
  let bass = 0
  let bars: number[] = new Array(24).fill(0)

  function close() {
    visualizerOpen.set(false)
  }

  function draw() {
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const w = canvas.width
    const h = canvas.height
    ctx.clearRect(0, 0, w, h)

    const accent = getComputedStyle(canvas).getPropertyValue('--accent').trim() || '#7c5cff'

    if ($visualizerMode === 'orb') {
      const cx = w / 2
      const cy = h / 2
      const baseRadius = Math.min(w, h) * 0.18
      const radius = baseRadius * (1 + bass * 1.4)

      const gradient = ctx.createRadialGradient(cx, cy, 0, cx, cy, radius * 1.8)
      gradient.addColorStop(0, accent)
      gradient.addColorStop(1, 'transparent')

      ctx.save()
      ctx.shadowColor = accent
      ctx.shadowBlur = 30 + bass * 60
      ctx.fillStyle = gradient
      ctx.beginPath()
      ctx.arc(cx, cy, radius, 0, Math.PI * 2)
      ctx.fill()
      ctx.restore()
    } else {
      const gap = 4
      const barWidth = (w - gap * (bars.length - 1)) / bars.length
      ctx.fillStyle = accent
      bars.forEach((v, i) => {
        const barHeight = Math.max(2, v * h)
        const x = i * (barWidth + gap)
        const y = h - barHeight
        ctx.fillRect(x, y, barWidth, barHeight)
      })
    }
  }

  function resizeCanvas() {
    if (!canvas) return
    const rect = canvas.getBoundingClientRect()
    canvas.width = rect.width
    canvas.height = rect.height
    draw()
  }

  onMount(() => {
    const unlisten = listen<{ bass: number; bars: number[] }>('firmium:audio-analysis', e => {
      bass = e.payload.bass
      bars = e.payload.bars
      draw()
    })

    window.addEventListener('resize', resizeCanvas)
    resizeCanvas()

    return () => {
      unlisten.then(f => f())
      window.removeEventListener('resize', resizeCanvas)
    }
  })

  // Start/stop the backend analysis task only while the panel is open.
  $effect(() => {
    tauriInvoke('set_visualizer_enabled', { enabled: $visualizerOpen }).catch(() => {})
    if ($visualizerOpen) {
      // Wait for the panel's open/width transition to finish before measuring.
      setTimeout(resizeCanvas, 260)
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
