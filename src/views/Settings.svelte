<script>
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import { tauriInvoke } from '../lib/tauri.js'
  import { SafeStorage } from '../lib/utils.js'
  import { Keyring } from '../lib/api.js'
  import { crossfadeEnabled, crossfadeDuration, setCrossfadeEnabled, setCrossfadeDuration, gaplessEnabled, setGaplessEnabled, isOpenSubsonic, openSubsonicExtensions } from '../lib/stores.js'
  import { clearAll } from '../lib/coverCache.js'

  // Callback props replace createEventDispatcher.
  let { onapplyTheme, onapplyDecorations } = $props()

  const THEMES = [
    ['firmium',             'Firmium'],
    ['gruvbox',             'Gruvbox'],
    ['tokyo-night',         'Tokyo Night'],
    ['dracula',             'Dracula'],
    ['catppuccin-mocha',    'Catppuccin Mocha'],
    ['catppuccin-macchiato','Catppuccin Macchiato'],
    ['catppuccin-frappe',   'Catppuccin Frappé'],
    ['catppuccin-latte',    'Catppuccin Latte'],
    ['nord',                'Nord'],
    ['monokai-classic',     'Monokai Classic'],
    ['monokai-pro',         'Monokai Pro'],
    ['adwaita',             'Adwaita'],
    ['adwaita-dark',        'Adwaita Dark'],
    ['ayu',                 'ayu'],
    ['ayu-light',           'ayu Light'],
    ['github-dark',         'GitHub Dark Default'],
    ['nordfox',             'Nordfox'],
    ['synthwave',           'Synthwave'],
  ]

  const SETTINGS_KEYS = [
    'firmium_server', 'firmium_user', 'firmium_save_pass',
    'firmium_auto_login', 'firmium_lrclib', 'firmium_theme',
    'firmium_decorations', 'firmium_crossfade', 'firmium_crossfade_duration', 'firmium_volume', 'firmium_gapless',
    'firmium_lastfm',
  ]

  let isDecorated = $state(SafeStorage.getItem('firmium_decorations') !== 'false')
let isAutoLoginEnabled = $state(SafeStorage.getItem('firmium_auto_login') !== 'false')
  let isLrclibEnabled = $state(SafeStorage.getItem('firmium_lrclib') !== 'false')
  let isLastfmEnabled = $state(SafeStorage.getItem('firmium_lastfm') === 'true')
  let lastfmKey = $state('')
  let lastfmSecret = $state('')
  let currentTheme = $state(SafeStorage.getItem('firmium_theme') || 'firmium')
  let themeOpen = $state(false)

  let appVersion = $state('Loading…')
  let systemInfo = $state('Loading…')
  let logPath = $state('Loading…')
  let wipeCacheLabel = $state('Wipe')
  let deleteLogsLabel = $state('Delete')
  let deleteLogsDisabled = $state(false)
  let deleteSettingsLabel = $state('Delete')

  onMount(async () => {
    tauriInvoke('get_app_version').then(v => appVersion = `v${v}`).catch(() => appVersion = 'unavailable')
    tauriInvoke('get_machine_info').then(info => systemInfo = `${info.distro} ${info.version}`).catch(() => systemInfo = 'unavailable')
    tauriInvoke('get_log_path').then(p => logPath = p).catch(() => logPath = 'unavailable')
    // Load Last.fm credentials from keyring.
    Keyring.load('lastfm_api_key').then(k => { if (k) lastfmKey = k }).catch(() => {})
    Keyring.load('lastfm_secret').then(s => { if (s) lastfmSecret = s }).catch(() => {})
  })

  const themeName = $derived(THEMES.find(([v]) => v === currentTheme)?.[1] ?? 'Firmium')

  function selectTheme(val) {
    currentTheme = val
    themeOpen = false
    SafeStorage.setItem('firmium_theme', val)
    onapplyTheme?.(val)
  }

  function handleDecorationsChange(e) {
    const show = e.target.checked
    SafeStorage.setItem('firmium_decorations', show ? 'true' : 'false')
    onapplyDecorations?.()
  }

  function handleAutoLogin(e) { SafeStorage.setItem('firmium_auto_login', e.target.checked ? 'true' : 'false') }
function handleLrclib(e) { SafeStorage.setItem('firmium_lrclib', e.target.checked ? 'true' : 'false') }
  function handleLastfm(e) {
    isLastfmEnabled = e.target.checked
    SafeStorage.setItem('firmium_lastfm', isLastfmEnabled ? 'true' : 'false')
  }
  function handleLastfmKey(e) { lastfmKey = e.target.value; Keyring.save('lastfm_api_key', e.target.value).catch(() => {}) }
  function handleLastfmSecret(e) { lastfmSecret = e.target.value; Keyring.save('lastfm_secret', e.target.value).catch(() => {}) }

  function handleCrossfadeToggle(e) {
    setCrossfadeEnabled(e.target.checked)
    if (e.target.checked) setGaplessEnabled(false)
  }
  function handleCrossfadeDuration(e) { setCrossfadeDuration(Number(e.target.value)) }
  function handleGaplessToggle(e) {
    setGaplessEnabled(e.target.checked)
    if (e.target.checked) setCrossfadeEnabled(false)
  }

  function wipeCache() {
    clearAll()
    wipeCacheLabel = 'Wiped!'
    setTimeout(() => wipeCacheLabel = 'Wipe', 1500)
  }

  async function deleteLogs() {
    deleteLogsDisabled = true
    try {
      await tauriInvoke('delete_logs')
      deleteLogsLabel = 'Deleted!'
    } catch {
      deleteLogsLabel = 'Failed'
    }
    setTimeout(() => { deleteLogsLabel = 'Delete'; deleteLogsDisabled = false }, 1500)
  }

  function deleteSettings() {
    SETTINGS_KEYS.forEach(k => SafeStorage.removeItem(k))
    deleteSettingsLabel = 'Deleted!'
    setTimeout(() => deleteSettingsLabel = 'Delete', 1500)
  }
</script>

<!-- Close theme dropdown when clicking outside -->
<svelte:document onclick={() => themeOpen = false} />

<div class="section-header">Settings</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Window Decorations</div>
    <div class="settings-desc">Show native title bar and borders</div>
  </div>
  <label class="toggle-switch">
    <input type="checkbox" bind:checked={isDecorated} onchange={handleDecorationsChange} />
    <span class="toggle-slider"></span>
  </label>
</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Theme</div>
    <div class="settings-desc">Color scheme for the interface</div>
  </div>
  <div
    class="theme-selector"
    class:open={themeOpen}
    role="button"
    tabindex="0"
    onclick={e => { e.stopPropagation(); themeOpen = !themeOpen }}
    onkeydown={e => { e.stopPropagation(); (e.key === 'Enter' || e.key === ' ') && (themeOpen = !themeOpen) }}
  >
    <div class="theme-selector-value">{themeName}</div>
    <span class="theme-selector-arrow">▾</span>
    <div class="theme-selector-dropdown">
      {#each THEMES as [val, label]}
        <div
          class="theme-option"
          class:selected={currentTheme === val}
          role="option"
          tabindex="0"
          aria-selected={currentTheme === val}
          onclick={e => { e.stopPropagation(); selectTheme(val) }}
          onkeydown={e => { e.stopPropagation(); (e.key === 'Enter' || e.key === ' ') && selectTheme(val) }}
        >{label}</div>
      {/each}
    </div>
  </div>
</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Auto-Login</div>
    <div class="settings-desc">Automatically connect on startup when credentials are saved</div>
  </div>
  <label class="toggle-switch">
    <input type="checkbox" bind:checked={isAutoLoginEnabled} onchange={handleAutoLogin} />
    <span class="toggle-slider"></span>
  </label>
</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Last.fm Integration</div>
    <div class="settings-desc">Fetch artist biography and photo directly from Last.fm using your own API key</div>
  </div>
  <label class="toggle-switch">
    <input type="checkbox" bind:checked={isLastfmEnabled} onchange={handleLastfm} />
    <span class="toggle-slider"></span>
  </label>
</div>

{#if isLastfmEnabled}
  <div class="settings-row">
    <div class="settings-info">
      <div class="settings-title">Last.fm API Key</div>
      <div class="settings-desc">From your Last.fm API account</div>
    </div>
    <input class="settings-text-input" type="text" value={lastfmKey} oninput={handleLastfmKey} placeholder="API key…" />
  </div>
  <div class="settings-row">
    <div class="settings-info">
      <div class="settings-title">Last.fm Secret</div>
      <div class="settings-desc">Shared secret for your API account</div>
    </div>
    <input class="settings-text-input" type="password" value={lastfmSecret} oninput={handleLastfmSecret} placeholder="Secret…" />
  </div>
{/if}

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">External Lyrics (LRCLIB)</div>
    <div class="settings-desc">Fetch synced lyrics from lrclib.net when your server has none. Sends song title and artist name.</div>
  </div>
  <label class="toggle-switch">
    <input type="checkbox" bind:checked={isLrclibEnabled} onchange={handleLrclib} />
    <span class="toggle-slider"></span>
  </label>
</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Crossfade</div>
    <div class="settings-desc">Smoothly blend between tracks</div>
  </div>
  <label class="toggle-switch">
    <input type="checkbox" checked={$crossfadeEnabled} onchange={handleCrossfadeToggle} />
    <span class="toggle-slider"></span>
  </label>
</div>

{#if $crossfadeEnabled}
  <div class="settings-row">
    <div class="settings-info">
      <div class="settings-title">Crossfade Duration</div>
      <div class="settings-desc">Length of the blend in seconds</div>
    </div>
    <div class="crossfade-duration-control">
      <input
        type="range"
        min="1" max="12" step="1"
        value={$crossfadeDuration}
        oninput={handleCrossfadeDuration}
      />
      <span>{$crossfadeDuration}s</span>
    </div>
  </div>
{/if}

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Gapless Playback</div>
    <div class="settings-desc">Pre-buffer the next track for seamless transitions</div>
  </div>
  <label class="toggle-switch">
    <input type="checkbox" checked={$gaplessEnabled} onchange={handleGaplessToggle} />
    <span class="toggle-slider"></span>
  </label>
</div>

<div class="section-header">Debug</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">App Version</div>
    <div class="settings-desc">{appVersion}</div>
  </div>
</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">System</div>
    <div class="settings-desc">{systemInfo}</div>
  </div>
</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Log File</div>
    <div class="settings-desc debug-path">{logPath}</div>
  </div>
</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Wipe Cache</div>
    <div class="settings-desc">Clear in-memory cover art cache</div>
  </div>
  <button class="debug-btn" onclick={wipeCache}>{wipeCacheLabel}</button>
</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Delete Logs</div>
    <div class="settings-desc">Remove the app-logs.txt file from disk</div>
  </div>
  <button class="debug-btn debug-btn--danger" onclick={deleteLogs} disabled={deleteLogsDisabled}>{deleteLogsLabel}</button>
</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Delete User Settings</div>
    <div class="settings-desc">Reset all preferences to defaults</div>
  </div>
  <button class="debug-btn debug-btn--danger" onclick={deleteSettings}>{deleteSettingsLabel}</button>
</div>
