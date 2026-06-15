<script lang="ts">
  import { get } from 'svelte/store'
  import { isAuthed, authServer, clearAuth, closeAccountModal, audioBridge } from '../lib/stores'
  import { stopPositionTracking } from '../lib/playback'
  import { clearAll } from '../lib/coverCache'
  import { clearAll as clearListCache } from '../lib/listCache'
  import { IconClose } from '../lib/icons'
  import Setup from './Setup.svelte'

  interface Props {
    error?: string
    doConnect: (server: string, username: string, password: string) => Promise<void>
  }

  let { error = $bindable(''), doConnect }: Props = $props()

  const serverLabel = $derived((() => {
    try { return new URL($authServer ?? '').hostname } catch (_) { return $authServer ?? '' }
  })())

  async function handleDisconnect() {
    const bridge = get(audioBridge)
    if (bridge) { bridge.destroy() }
    stopPositionTracking()
    await clearAll()
    clearListCache()
    clearAuth()
    document.title = 'Firmium'
    closeAccountModal()
  }

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) closeAccountModal()
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') closeAccountModal()
  }

  let closeBtn: HTMLButtonElement | undefined = $state()
  $effect(() => {
    closeBtn?.focus()
  })

  function handleContentKeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return
    const content = e.currentTarget as HTMLElement
    const focusable = content.querySelectorAll<HTMLElement>(
      'button, input, select, textarea, a[href], [tabindex]:not([tabindex="-1"])'
    )
    if (focusable.length === 0) return
    const first = focusable[0]
    const last = focusable[focusable.length - 1]
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault()
      last.focus()
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault()
      first.focus()
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="account-modal-overlay" onclick={handleOverlayClick}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="account-modal-content" role="dialog" aria-modal="true" aria-label="Account" tabindex="-1" onkeydown={handleContentKeydown}>
    <button bind:this={closeBtn} class="account-modal-close" onclick={closeAccountModal} title="Close" aria-label="Close">
      <span class="icon" style="width:14px;height:14px" aria-hidden="true">{@html IconClose}</span>
    </button>

    {#if $isAuthed}
      <div class="setup-box">
        <h1>Connected</h1>
        <div class="account-modal-server">{serverLabel}</div>
        <button class="btn-primary account-modal-disconnect" onclick={handleDisconnect}>Disconnect</button>
      </div>
    {:else}
      <Setup bind:error {doConnect} />
    {/if}
  </div>
</div>
