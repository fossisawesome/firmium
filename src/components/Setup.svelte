<script lang="ts">
  import { Api } from '../lib/api'
  import { auth, setAuthState, showAccountModal } from '../lib/stores'
  import { errorMessage } from '../lib/ipc'
  import type { SavedAccount } from '../lib/types'

  interface Props { error?: string }
  let { error = $bindable('') }: Props = $props()

  let serverUrl = $state($auth.server ?? '')
  let username = $state($auth.username ?? '')
  let password = $state('')
  let savePassword = $state(true)
  let connecting = $state(false)

  // Plain HTTP to a non-local host sends the auth token unencrypted.
  const isLocalHost = (host: string): boolean =>
    host === 'localhost' || host === '127.0.0.1' || host === '::1' ||
    /^10\.|^192\.168\.|^172\.(1[6-9]|2\d|3[01])\.|\.local$/.test(host)

  const cleartextWarning = $derived.by(() => {
    try {
      const url = new URL(serverUrl)
      if (url.protocol === 'http:' && !isLocalHost(url.hostname)) {
        return 'Connecting over plain HTTP to a non-local server sends your credentials unencrypted. Use HTTPS if possible.'
      }
    } catch { /* not a URL yet */ }
    return ''
  })

  async function finish(next: Promise<import('../lib/types').AuthState>) {
    connecting = true
    error = ''
    try {
      await setAuthState(await next)
      showAccountModal.set(false)
    } catch (e) {
      error = errorMessage(e)
    } finally {
      connecting = false
    }
  }

  const handleConnect = () => {
    if (!serverUrl || !username || !password) { error = 'Please fill out all fields'; return }
    finish(Api.connect(serverUrl, username, password, savePassword))
  }

  async function connectSaved(a: SavedAccount) {
    connecting = true
    error = ''
    try {
      await setAuthState(await Api.switchAccount(a.server, a.username))
      showAccountModal.set(false)
    } catch {
      // No stored password: prefill the form so only the password is needed.
      serverUrl = a.server
      username = a.username
      error = 'Password not saved. Enter it to connect.'
    } finally {
      connecting = false
    }
  }

  async function deleteSaved(a: SavedAccount) {
    auth.set(await Api.removeAccount(a.server, a.username))
  }
</script>

<div class="setup-box">
  <h1>Firmium</h1>

  {#if $auth.accounts.length > 0}
    <div class="saved-servers">
      <div class="saved-servers-title">Saved Servers</div>
      {#each $auth.accounts as a}
        <div class="saved-server-row">
          <div class="saved-server-info">
            <div class="saved-server-url">{a.server}</div>
            <div class="saved-server-user">{a.username}</div>
          </div>
          <button class="btn-connect-saved" onclick={() => connectSaved(a)} disabled={connecting}>Connect</button>
          <button class="btn-remove-saved" onclick={() => deleteSaved(a)} title="Remove server">&times;</button>
        </div>
      {/each}
    </div>
    <div class="saved-servers-divider">or add a new server</div>
  {/if}

  <div class="field">
    <label for="setup-server">Server URL</label>
    <input id="setup-server" type="url" bind:value={serverUrl} placeholder="https://navidrome.music:4533" />
    {#if cleartextWarning}<div class="warning-msg">{cleartextWarning}</div>{/if}
  </div>
  <div class="field">
    <label for="setup-username">Username</label>
    <input id="setup-username" type="text" bind:value={username} placeholder="admin" autocomplete="username" />
  </div>
  <div class="field">
    <label for="setup-password">Password</label>
    <input id="setup-password" type="password" bind:value={password} placeholder="••••••••" autocomplete="current-password"
      onkeydown={e => e.key === 'Enter' && handleConnect()} />
  </div>
  <div class="field-checkbox">
    <input type="checkbox" id="savePassword" bind:checked={savePassword} />
    <label for="savePassword">Save Password</label>
  </div>

  <button class="btn-primary" onclick={handleConnect} disabled={connecting}>{connecting ? 'Connecting…' : 'Connect'}</button>

  {#if error}<div class="error-msg">{error}</div>{/if}
</div>
