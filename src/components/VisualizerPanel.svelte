<script lang="ts">
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import { Channel } from '@tauri-apps/api/core'
  import { tauriInvoke } from '../lib/tauri'
  import { visualizerOpen, visualizerMode, setVisualizerMode, currentTrack } from '../lib/stores'
  import { IconClose } from '../lib/icons'
  import { OpenSubsonicRouter } from '../lib/api'
  import type { OrbPalette, CoverColorsResult } from '../lib/types/tauri-commands'

  let canvas: HTMLCanvasElement | undefined = $state()

  const DEFAULT_PALETTE: OrbPalette = {
    primary:   { r: 124, g: 92,  b: 255 },
    secondary: { r: 170, g: 136, b: 255 },
    tertiary:  { r: 85,  g: 51,  b: 204 },
  }
  // Reactive so the palette $effect re-fires when cover-art extraction updates it.
  let palette = $state<OrbPalette>(DEFAULT_PALETTE)

  // Rendering happens in Rust (wgpu). The frontend is a passive display surface:
  // it receives raw RGBA frames over a Tauri Channel and blits them to a 2D canvas.
  let ctx: CanvasRenderingContext2D | null = null
  let frameChannel: Channel<ArrayBuffer> | null = null
  let curW = 0
  let curH = 0

  function applyFrame(buf: ArrayBuffer) {
    if (!ctx || curW === 0 || curH === 0) return
    const arr = new Uint8ClampedArray(buf)
    if (arr.length !== curW * curH * 4) return  // stale frame from a previous size
    ctx.putImageData(new ImageData(arr, curW, curH), 0, 0)
  }

  function startRenderer() {
    if (!canvas) return
    const rect = canvas.getBoundingClientRect()
    curW = Math.max(1, Math.round(rect.width))
    curH = Math.max(1, Math.round(rect.height))
    canvas.width = curW
    canvas.height = curH
    ctx = canvas.getContext('2d')

    frameChannel = new Channel<ArrayBuffer>()
    frameChannel.onmessage = applyFrame
    tauriInvoke('start_visualizer_renderer', { channel: frameChannel, width: curW, height: curH })
      .catch((e) => console.error('[visualizer] start_visualizer_renderer failed:', e))
  }

  function onResize() {
    if (get(visualizerOpen)) startRenderer()  // re-send size; backend swaps the channel + target
  }

  function close() { visualizerOpen.set(false) }

  const MODES = ['orb', 'bars', 'oscilloscope'] as const
  function cycleMode() {
    const i = MODES.indexOf(get(visualizerMode))
    setVisualizerMode(MODES[(i + 1) % MODES.length])
  }

  onMount(() => {
    window.addEventListener('resize', onResize)
    return () => window.removeEventListener('resize', onResize)
  })

  $effect(() => {
    if ($visualizerOpen) {
      tauriInvoke('set_visualizer_enabled', { enabled: true })
        .then(() => setTimeout(startRenderer, 260))  // let the decode task pre-fill the ring buffer
        .catch((e) => console.error('[visualizer] set_visualizer_enabled failed:', e))
    } else {
      tauriInvoke('stop_visualizer_renderer').catch(() => {})
      tauriInvoke('set_visualizer_enabled', { enabled: false }).catch(() => {})
    }
  })

  // Push the selected mode to the renderer (covers both the header buttons and tap-to-cycle).
  $effect(() => {
    const mode = $visualizerMode
    if ($visualizerOpen) tauriInvoke('set_visualizer_mode', { mode }).catch(() => {})
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

  // Forward palette changes to the renderer.
  $effect(() => {
    const p = palette
    if ($visualizerOpen) {
      tauriInvoke('set_visualizer_palette', {
        primary: p.primary, secondary: p.secondary, tertiary: p.tertiary,
      }).catch(() => {})
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
