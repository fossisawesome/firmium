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
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="account-modal-overlay" onclick={handleOverlayClick}>
  <div class="account-modal-content">
    <button class="account-modal-close" onclick={closeAccountModal} title="Close">
      <span class="icon" style="width:14px;height:14px">{@html IconClose}</span>
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
