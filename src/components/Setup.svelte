<script lang="ts">
  import { SafeStorage } from '../lib/utils'
  import { Keyring } from '../lib/api'
  import { serverList, addSavedServer, removeSavedServer, type SavedServer } from '../lib/stores'

  interface Props {
    error?: string
    doConnect: (server: string, username: string, password: string) => Promise<void>
  }

  // error is bindable so the parent can write the initial "Connecting…" message.
  let { error = $bindable(''), doConnect }: Props = $props()

  let serverUrl = $state((SafeStorage.getItem('firmium_server') ?? '').replace(/\/+$/, ''))
  let username = $state(SafeStorage.getItem('firmium_user') ?? '')
  let password = $state('')
  let savePassword = $state(SafeStorage.getItem('firmium_save_pass') === 'true')
  let connecting = $state(false)

  // Warn (non-blocking) when credentials would be sent in cleartext over a
  // non-local network — LAN/localhost http is fine, but http to a hostname
  // implies the auth token travels unencrypted.
  const isLocalHost = (host: string): boolean =>
    host === 'localhost' || host === '127.0.0.1' || host === '::1' ||
    /^10\.|^192\.168\.|^172\.(1[6-9]|2\d|3[01])\.|\.local$/.test(host)

  const cleartextWarning = $derived.by(() => {
    try {
      const url = new URL(serverUrl)
      if (url.protocol === 'http:' && !isLocalHost(url.hostname)) {
        return 'Connecting over plain HTTP to a non-local server sends your credentials unencrypted. Use HTTPS if possible.'
      }
    } catch (_) {}
    return ''
  })

  async function handleConnect() {
    if (!serverUrl || !username || !password) { error = 'Please fill out all fields'; return }
    connecting = true
    error = ''
    const normalizedUrl = serverUrl.replace(/\/+$/, '')
    try {
      await doConnect(normalizedUrl, username, password)

      SafeStorage.setItem('firmium_server', normalizedUrl)
      SafeStorage.setItem('firmium_user', username)
      addSavedServer(normalizedUrl, username)

      if (savePassword) {
        SafeStorage.setItem('firmium_save_pass', 'true')
        try { await Keyring.save(username, password, normalizedUrl) } catch (kErr) {
          console.warn('Keyring save failed — password will not be remembered:', kErr)
        }
      } else {
        SafeStorage.setItem('firmium_save_pass', 'false')
        Keyring.remove(username, normalizedUrl).catch(() => {})
      }
    } catch (err: any) {
      error = (typeof err === 'string' ? err : err?.message || (err instanceof Error ? err.toString() : null)) || 'Connection failed — check the server URL and try again'
    } finally {
      connecting = false
    }
  }

  async function connectSaved(server: SavedServer) {
    connecting = true
    error = ''
    try {
      const pass = await Keyring.load(server.username, server.url)
      await doConnect(server.url, server.username, pass as string)
      SafeStorage.setItem('firmium_server', server.url)
      SafeStorage.setItem('firmium_user', server.username)
      addSavedServer(server.url, server.username)
    } catch (err: any) {
      error = (typeof err === 'string' ? err : err?.message || (err instanceof Error ? err.toString() : null)) || 'Connection failed'
    } finally {
      connecting = false
    }
  }

  function deleteSaved(server: SavedServer) {
    Keyring.remove(server.username, server.url).catch(() => {})
    removeSavedServer(server.url, server.username)
  }
</script>

<div class="setup-box">
  <h1>Firmium</h1>

  {#if $serverList.length > 0}
    <div class="saved-servers">
      <div class="saved-servers-title">Saved Servers</div>
      {#each $serverList as server}
        <div class="saved-server-row">
          <div class="saved-server-info">
            <div class="saved-server-url">{server.url}</div>
            <div class="saved-server-user">{server.username}</div>
          </div>
          <button class="btn-connect-saved" onclick={() => connectSaved(server)} disabled={connecting}>Connect</button>
          <button class="btn-remove-saved" onclick={() => deleteSaved(server)} title="Remove server">&times;</button>
        </div>
      {/each}
    </div>
    <div class="saved-servers-divider">or add a new server</div>
  {/if}

  <div class="field">
    <label for="setup-server">Server URL</label>
    <input id="setup-server" type="url" bind:value={serverUrl} placeholder="https://navidrome.music:4533" />
    {#if cleartextWarning}
      <div class="warning-msg">{cleartextWarning}</div>
    {/if}
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
