<script>
  import { SafeStorage } from '../lib/utils.js'
  import { Keyring } from '../lib/api.js'

  // error is bindable so the parent can write the initial "Connecting…" message.
  let { error = $bindable(''), doConnect } = $props()

  let serverUrl = $state((SafeStorage.getItem('firmium_server') ?? '').replace(/\/+$/, ''))
  let username = $state(SafeStorage.getItem('firmium_user') ?? '')
  let password = $state('')
  let savePassword = $state(SafeStorage.getItem('firmium_save_pass') === 'true')
  let connecting = $state(false)

  async function handleConnect() {
    if (!serverUrl || !username || !password) { error = 'Please fill out all fields'; return }
    connecting = true
    error = ''
    // Strip trailing slashes so http://host/ and http://host/sub/ match the HTTP scope
    const normalizedUrl = serverUrl.replace(/\/+$/, '')
    try {
      await doConnect(normalizedUrl, username, password)

      SafeStorage.setItem('firmium_server', normalizedUrl)
      SafeStorage.setItem('firmium_user', username)

      if (savePassword) {
        SafeStorage.setItem('firmium_save_pass', 'true')
        try { await Keyring.save(username, password) } catch (kErr) {
          console.warn('Keyring save failed — password will not be remembered:', kErr)
        }
      } else {
        SafeStorage.setItem('firmium_save_pass', 'false')
        Keyring.remove(username).catch(() => {})
      }
    } catch (err) {
      error = (typeof err === 'string' ? err : err?.message || (err instanceof Error ? err.toString() : null)) || 'Connection failed — check the server URL and try again'
    } finally {
      connecting = false
    }
  }
</script>

<div class="setup-box">
  <h1>Firmium</h1>

  <div class="field">
    <label for="setup-server">Server URL</label>
    <input id="setup-server" type="url" bind:value={serverUrl} placeholder="https://navidrome.music:4533" />
  </div>
  <div class="field">
    <label for="setup-username">Username</label>
    <input id="setup-username" type="text" bind:value={username} placeholder="admin" autocomplete="username" />
  </div>
  <div class="field">
    <label for="setup-password">Password</label>
    <input
      id="setup-password"
      type="password"
      bind:value={password}
      placeholder="••••••••"
      autocomplete="current-password"
      onkeydown={e => e.key === 'Enter' && handleConnect()}
    />
  </div>
  <div class="field-checkbox">
    <input type="checkbox" id="savePassword" bind:checked={savePassword} />
    <label for="savePassword">Save Password</label>
  </div>

  <button class="btn-primary" onclick={handleConnect} disabled={connecting}>
    {connecting ? 'Connecting…' : 'Connect'}
  </button>

  {#if error}
    <div class="error-msg">{error}</div>
  {/if}
</div>
