<script lang="ts">
  import { Api } from '../lib/api'
  import { app, setAuthState, navTo } from '../lib/stores.svelte'
  import { IconClose } from '../lib/icons'
  import Setup from './Setup.svelte'

  let { error = $bindable('') }: { error?: string } = $props()

  const serverLabel = $derived.by(() => {
    try { return new URL(app.auth.server ?? '').hostname } catch { return app.auth.server ?? '' }
  })

  const close = () => (app.showAccountModal = false)

  async function disconnect() {
    await setAuthState(await Api.logout())
    document.title = 'Firmium'
    navTo('home')
    close()
  }

  async function switchTo(server: string, username: string) {
    try {
      await setAuthState(await Api.switchAccount(server, username))
      navTo('home')
      close()
    } catch {
      // Password gone from the keyring: fall back to the login form.
      await setAuthState(await Api.logout())
      error = 'Password not saved. Please log in again.'
    }
  }

  let closeBtn: HTMLButtonElement | undefined = $state()
  $effect(() => { closeBtn?.focus() })
</script>

<svelte:window onkeydown={e => e.key === 'Escape' && close()} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="account-modal-overlay" onclick={e => e.target === e.currentTarget && close()}>
  <div class="account-modal-content" role="dialog" aria-modal="true" aria-label="Account" tabindex="-1">
    <button bind:this={closeBtn} class="account-modal-close" onclick={close} title="Close" aria-label="Close">
      <span class="icon" style="width:14px;height:14px" aria-hidden="true">{@html IconClose}</span>
    </button>

    {#if app.isAuthed}
      <div class="setup-box">
        <h1>Connected</h1>
        <div class="account-modal-server">{app.auth.username}@{serverLabel}</div>
        {#each app.auth.accounts.filter(a => a.server !== app.auth.server || a.username !== app.auth.username) as a}
          <div class="saved-server-row">
            <div class="saved-server-info">
              <div class="saved-server-url">{a.server}</div>
              <div class="saved-server-user">{a.username}</div>
            </div>
            <button class="btn-connect-saved" onclick={() => switchTo(a.server, a.username)}>Switch</button>
          </div>
        {/each}
        <button class="btn-primary account-modal-disconnect" onclick={disconnect}>Disconnect</button>
        {#if error}<div class="error-msg">{error}</div>{/if}
      </div>
    {:else}
      <Setup bind:error />
    {/if}
  </div>
</div>
