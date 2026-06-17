<script lang="ts">
  import { onMount } from 'svelte'
  import { tauriInvoke } from '../lib/tauri'
  import { SafeStorage } from '../lib/utils'
  import { Keyring } from '../lib/api'
  import { crossfadeEnabled, crossfadeDuration, setCrossfadeEnabled, setCrossfadeDuration, gaplessEnabled, setGaplessEnabled, bitPerfectMode, setBitPerfectMode, clearAuth, navToView, isAuthed, authServer, openAccountModal, downloadFormat, setDownloadFormat, lyricsWordFillEnabled, setLyricsWordFillEnabled } from '../lib/stores'
  import { clearAll } from '../lib/coverCache'
  import { clearAll as clearListCache } from '../lib/listCache'
  import { checkForUpdate, installUpdate } from '../lib/updater'
  import { IconChevronDown, IconPalette, IconPlay, IconGlobe, IconUser, IconInfo, IconDownload } from '../lib/icons'
  import type { Theme } from '../lib/types/tauri-commands'

  interface Props {
    onapplyTheme?: (id: string) => void
    onapplyDecorations?: () => void
    themes?: Theme[]
  }

  let { onapplyTheme, onapplyDecorations, themes = [] }: Props = $props()

  const SETTINGS_KEYS = [
    'firmium_server', 'firmium_user', 'firmium_save_pass',
    'firmium_auto_login', 'firmium_lrclib', 'firmium_theme',
    'firmium_decorations', 'firmium_crossfade', 'firmium_crossfade_duration',
    'firmium_volume', 'firmium_gapless', 'firmium_lastfm',
    'firmium_download_format', 'firmium_lyrics_word_fill', 'firmium_bit_perfect_mode',
  ]

  // ── Active category ───────────────────────────────────────────────────────────
  let activeCategory = $state('appearance')

  const CATEGORIES = [
    { id: 'appearance', label: 'Appearance', icon: IconPalette  },
    { id: 'playback',   label: 'Playback',   icon: IconPlay     },
    { id: 'downloads',  label: 'Downloads',  icon: IconDownload },
    { id: 'services',   label: 'Services',   icon: IconGlobe    },
    { id: 'account',    label: 'Account',    icon: IconUser     },
    { id: 'debug',      label: 'Debug',      icon: IconInfo     },
  ]

  // ── Settings state ────────────────────────────────────────────────────────────
  let isDecorated = $state(SafeStorage.getItem('firmium_decorations') !== 'false')
  let isAutoLoginEnabled = $state(SafeStorage.getItem('firmium_auto_login') !== 'false')
  let isLrclibEnabled = $state(SafeStorage.getItem('firmium_lrclib') !== 'false')
  let isLastfmEnabled = $state(SafeStorage.getItem('firmium_lastfm') === 'true')
  let lastfmKey = $state('')
  let lastfmSecret = $state('')
  let currentTheme = $state(SafeStorage.getItem('firmium_theme') || 'firmium')
  let themeOpen = $state(false)
  let formatOpen = $state(false)

  const FORMAT_OPTIONS = [
    { id: 'original', name: 'Original' },
    { id: 'mp3',      name: 'MP3'      },
    { id: 'flac',     name: 'FLAC'     },
    { id: 'wav',      name: 'WAV'      },
    { id: 'opus',     name: 'Opus'     },
  ]

  let appVersion = $state('Loading…')
  let wipeCacheLabel = $state('Wipe')
  let deleteSettingsLabel = $state('Delete')

  let updateLabel = $state('Check for Updates')
  let updateDisabled = $state(false)
  let updateAvailable = $state<string | null>(null)

  onMount(async () => {
    tauriInvoke<string>('get_app_version').then(v => appVersion = `v${v}`).catch(() => appVersion = 'unavailable')
    Keyring.load('lastfm_api_key').then(k => { if (k) lastfmKey = k as string }).catch(() => {})
    Keyring.load('lastfm_secret').then(s => { if (s) lastfmSecret = s as string }).catch(() => {})
  })

  const themeName = $derived(themes.find(t => t.id === currentTheme)?.name ?? currentTheme)

  function selectTheme(val: string) {
    currentTheme = val; themeOpen = false
    SafeStorage.setItem('firmium_theme', val)
    onapplyTheme?.(val)
  }

  function selectFormat(val: string) {
    setDownloadFormat(val)
    formatOpen = false
  }

  function handleDecorationsChange(e: Event) {
    SafeStorage.setItem('firmium_decorations', (e.target as HTMLInputElement).checked ? 'true' : 'false')
    onapplyDecorations?.()
  }

  function handleAutoLogin(e: Event)    { SafeStorage.setItem('firmium_auto_login', (e.target as HTMLInputElement).checked ? 'true' : 'false') }
  function handleLrclib(e: Event)       { SafeStorage.setItem('firmium_lrclib',     (e.target as HTMLInputElement).checked ? 'true' : 'false') }
  function handleLastfm(e: Event) {
    isLastfmEnabled = (e.target as HTMLInputElement).checked
    SafeStorage.setItem('firmium_lastfm', isLastfmEnabled ? 'true' : 'false')
  }
  function handleLastfmKey(e: Event)    { lastfmKey = (e.target as HTMLInputElement).value;    Keyring.save('lastfm_api_key', (e.target as HTMLInputElement).value).catch(() => {}) }
  function handleLastfmSecret(e: Event) { lastfmSecret = (e.target as HTMLInputElement).value; Keyring.save('lastfm_secret',  (e.target as HTMLInputElement).value).catch(() => {}) }

  function handleCrossfadeToggle(e: Event) {
    const checked = (e.target as HTMLInputElement).checked
    setCrossfadeEnabled(checked)
    if (checked) setGaplessEnabled(false)
  }

  function handleBitPerfectMode(mode: string) {
    setBitPerfectMode(mode)
    if (mode === 'strict') setGaplessEnabled(false)
  }
  function handleCrossfadeDuration(e: Event) { setCrossfadeDuration(Number((e.target as HTMLInputElement).value)) }
  function handleGaplessToggle(e: Event) {
    const checked = (e.target as HTMLInputElement).checked
    setGaplessEnabled(checked)
    if (checked) setCrossfadeEnabled(false)
  }

  async function wipeCache() {
    await clearAll(); clearListCache(); wipeCacheLabel = 'Wiped!'
    setTimeout(() => wipeCacheLabel = 'Wipe', 1500)
  }
  function deleteSettings() {
    SETTINGS_KEYS.forEach(k => SafeStorage.removeItem(k))
    deleteSettingsLabel = 'Deleted!'
    setTimeout(() => deleteSettingsLabel = 'Delete', 1500)
  }

  async function checkForUpdates() {
    updateDisabled = true
    updateLabel = 'Checking…'
    try {
      const update = await checkForUpdate()
      if (update) {
        updateAvailable = update.version
        updateLabel = `Install v${update.version}`
      } else {
        updateLabel = 'Up to date'
        setTimeout(() => updateLabel = 'Check for Updates', 2000)
      }
    } catch {
      updateLabel = 'Check failed'
      setTimeout(() => updateLabel = 'Check for Updates', 2000)
    } finally {
      updateDisabled = false
    }
  }

  async function applyUpdate() {
    updateDisabled = true
    updateLabel = 'Installing…'
    try {
      await installUpdate()
    } catch {
      updateLabel = 'Install failed'
      updateDisabled = false
      setTimeout(() => updateLabel = `Install v${updateAvailable}`, 2000)
    }
  }

  async function logout() {
    await clearAll()
    clearListCache()
    clearAuth()
    navToView('settings')
  }
</script>

<!-- Close dropdowns when clicking outside -->
<svelte:document onclick={() => { themeOpen = false; formatOpen = false }} />

<div class="sett-layout">

  <!-- ── Left sidebar: category list ─────────────────────────────────────── -->
  <nav class="sett-sidebar">
    <div class="sett-sidebar-label">Settings</div>
    {#each CATEGORIES as cat}
      <button
        class="sett-nav-btn"
        class:sett-nav-btn--active={activeCategory === cat.id}
        onclick={() => activeCategory = cat.id}
      >
        <span class="icon sett-nav-icon" style="width:16px;height:16px">{@html cat.icon}</span>
        {cat.label}
      </button>
    {/each}
  </nav>

  <!-- ── Right panel: settings for the active category ──────────────────── -->
  <div class="sett-panel">

    {#if activeCategory === 'appearance'}
      <div class="sett-panel-title">Appearance</div>

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
          <span class="theme-selector-arrow icon" style="width:14px;height:14px">{@html IconChevronDown}</span>
          <div class="theme-selector-dropdown">
            {#each themes as theme (theme.id)}
              <div
                class="theme-option"
                class:selected={currentTheme === theme.id}
                role="option"
                tabindex="0"
                aria-selected={currentTheme === theme.id}
                onclick={e => { e.stopPropagation(); selectTheme(theme.id) }}
                onkeydown={e => { e.stopPropagation(); (e.key === 'Enter' || e.key === ' ') && selectTheme(theme.id) }}
              >{theme.name}</div>
            {/each}
          </div>
        </div>
      </div>

    {:else if activeCategory === 'playback'}
      <div class="sett-panel-title">Playback</div>

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
            <input type="range" min="1" max="12" step="1" value={$crossfadeDuration} oninput={handleCrossfadeDuration} />
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

      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Bit-Perfect Audio</div>
          <div class="settings-desc">
            {#if $bitPerfectMode === 'off'}
              Resamples all audio to the device default rate
            {:else if $bitPerfectMode === 'strict'}
              Matches native sample rate; crossfade is disabled
            {:else}
              Tries to match each track's native sample rate; falls back to resampling
            {/if}
          </div>
        </div>
        <div class="bp-mode-selector">
          {#each [['off', 'Off'], ['relaxed', 'Relaxed'], ['strict', 'Strict']] as [id, label]}
            <button
              class="bp-mode-btn"
              class:bp-mode-btn--active={$bitPerfectMode === id}
              onclick={() => handleBitPerfectMode(id)}
            >{label}</button>
          {/each}
        </div>
      </div>

    {:else if activeCategory === 'downloads'}
      <div class="sett-panel-title">Downloads</div>

      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Download Format</div>
          <div class="settings-desc">Format used when downloading tracks and albums. "Original" saves the file exactly as stored on the server.</div>
        </div>
        <div
          class="theme-selector"
          class:open={formatOpen}
          role="button"
          tabindex="0"
          onclick={e => { e.stopPropagation(); formatOpen = !formatOpen }}
          onkeydown={e => { e.stopPropagation(); (e.key === 'Enter' || e.key === ' ') && (formatOpen = !formatOpen) }}
        >
          <div class="theme-selector-value">{FORMAT_OPTIONS.find(f => f.id === $downloadFormat)?.name ?? 'Original'}</div>
          <span class="theme-selector-arrow icon" style="width:14px;height:14px">{@html IconChevronDown}</span>
          <div class="theme-selector-dropdown">
            {#each FORMAT_OPTIONS as fmt (fmt.id)}
              <div
                class="theme-option"
                class:selected={$downloadFormat === fmt.id}
                role="option"
                tabindex="0"
                aria-selected={$downloadFormat === fmt.id}
                onclick={e => { e.stopPropagation(); selectFormat(fmt.id) }}
                onkeydown={e => { e.stopPropagation(); (e.key === 'Enter' || e.key === ' ') && selectFormat(fmt.id) }}
              >{fmt.name}</div>
            {/each}
          </div>
        </div>
      </div>

    {:else if activeCategory === 'services'}
      <div class="sett-panel-title">Services</div>

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
          <div class="settings-title">Word-by-Word Lyrics Animation</div>
          <div class="settings-desc">Karaoke-style fill on the active lyric line, with per-word timing estimated from the line's timestamps. Disable for plain line-by-line highlighting.</div>
        </div>
        <label class="toggle-switch">
          <input type="checkbox" bind:checked={$lyricsWordFillEnabled} onchange={(e) => setLyricsWordFillEnabled((e.target as HTMLInputElement).checked)} />
          <span class="toggle-slider"></span>
        </label>
      </div>

    {:else if activeCategory === 'account'}
      <div class="sett-panel-title">Account</div>

      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Connection</div>
          <div class="settings-desc">{$isAuthed ? $authServer : 'Not connected — browsing local files'}</div>
        </div>
        {#if $isAuthed}
          <button class="debug-btn debug-btn--danger" onclick={logout}>Disconnect</button>
        {:else}
          <button class="debug-btn" onclick={openAccountModal}>Connect</button>
        {/if}
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

    {:else if activeCategory === 'debug'}
      <div class="sett-panel-title">Debug</div>

      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">App Version</div>
          <div class="settings-desc">{appVersion}</div>
        </div>
      </div>

      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Software Update</div>
          <div class="settings-desc">Check for and install a newer version (Windows/Linux AppImage builds)</div>
        </div>
        <button class="debug-btn" disabled={updateDisabled} onclick={updateAvailable ? applyUpdate : checkForUpdates}>{updateLabel}</button>
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
          <div class="settings-title">Delete User Settings</div>
          <div class="settings-desc">Reset all preferences to defaults</div>
        </div>
        <button class="debug-btn debug-btn--danger" onclick={deleteSettings}>{deleteSettingsLabel}</button>
      </div>
    {/if}

  </div>
</div>
