<script>
  import { onMount } from 'svelte'
  import { mobileSettingsOpen } from '../lib/stores.js'
  import { tauriInvoke } from '../lib/tauri.js'
  import { SafeStorage } from '../lib/utils.js'
  import { Keyring } from '../lib/api.js'
  import {
    crossfadeEnabled, crossfadeDuration,
    setCrossfadeEnabled, setCrossfadeDuration,
    gaplessEnabled, setGaplessEnabled
  } from '../lib/stores.js'
  import { clearAll } from '../lib/coverCache.js'
  import {
    IconBack, IconChevronRight, IconChevronDown,
    IconPalette, IconPlay, IconGlobe, IconUser, IconInfo
  } from '../lib/icons.js'

  let { onapplyTheme, onapplyDecorations, themes = [] } = $props()

  // ── Navigation state ─────────────────────────────────────────────────────────
  // null = category list, string = open sub-panel
  let activeCategory = $state(null)
  // Controls the whole overlay closing animation
  let closing = $state(false)
  // Tracks whether the sub-panel is sliding in (true) or out (false) so we can
  // animate in both directions.
  let subSliding = $state(false)

  // ── Settings state ───────────────────────────────────────────────────────────
  let isDecorated = $state(SafeStorage.getItem('firmium_decorations') !== 'false')
  let isAutoLoginEnabled = $state(SafeStorage.getItem('firmium_auto_login') !== 'false')
  let isLrclibEnabled = $state(SafeStorage.getItem('firmium_lrclib') !== 'false')
  let isLastfmEnabled = $state(SafeStorage.getItem('firmium_lastfm') === 'true')
  let lastfmKey = $state('')
  let lastfmSecret = $state('')
  let currentTheme = $state(SafeStorage.getItem('firmium_theme') || 'firmium')
  let themeOpen = $state(false)

  let appVersion = $state('…')
  let wipeCacheLabel = $state('Wipe')
  let deleteSettingsLabel = $state('Delete')
  let deleteLogsLabel = $state('Delete')
  let deleteLogsDisabled = $state(false)

  const SETTINGS_KEYS = [
    'firmium_server', 'firmium_user', 'firmium_save_pass',
    'firmium_auto_login', 'firmium_lrclib', 'firmium_theme',
    'firmium_decorations', 'firmium_crossfade', 'firmium_crossfade_duration',
    'firmium_volume', 'firmium_gapless', 'firmium_lastfm',
  ]

  onMount(async () => {
    tauriInvoke('get_app_version').then(v => appVersion = `v${v}`).catch(() => appVersion = 'unavailable')
    Keyring.load('lastfm_api_key').then(k => { if (k) lastfmKey = k }).catch(() => {})
    Keyring.load('lastfm_secret').then(s => { if (s) lastfmSecret = s }).catch(() => {})
  })

  const themeName = $derived(themes.find(t => t.id === currentTheme)?.name ?? currentTheme)

  // ── Category definitions ──────────────────────────────────────────────────────
  const CATEGORIES = [
    { id: 'appearance', label: 'Appearance', icon: IconPalette },
    { id: 'playback',   label: 'Playback',   icon: IconPlay    },
    { id: 'services',   label: 'Services',   icon: IconGlobe   },
    { id: 'account',    label: 'Account',    icon: IconUser    },
    { id: 'about',      label: 'About',      icon: IconInfo    },
  ]

  // ── Navigation ────────────────────────────────────────────────────────────────
  function openCategory(id) {
    subSliding = true
    activeCategory = id
  }

  function goBack() {
    if (activeCategory !== null) {
      subSliding = false
      activeCategory = null
    } else {
      closeOverlay()
    }
  }

  function closeOverlay() {
    closing = true
    setTimeout(() => mobileSettingsOpen.set(false), 280)
  }

  // ── Setting handlers ──────────────────────────────────────────────────────────
  function selectTheme(val) {
    currentTheme = val; themeOpen = false
    SafeStorage.setItem('firmium_theme', val)
    onapplyTheme?.(val)
  }

  function handleDecorationsChange(e) {
    SafeStorage.setItem('firmium_decorations', e.target.checked ? 'true' : 'false')
    onapplyDecorations?.()
  }

  function handleAutoLogin(e) { SafeStorage.setItem('firmium_auto_login', e.target.checked ? 'true' : 'false') }
  function handleLrclib(e)    { SafeStorage.setItem('firmium_lrclib',     e.target.checked ? 'true' : 'false') }
  function handleLastfm(e) {
    isLastfmEnabled = e.target.checked
    SafeStorage.setItem('firmium_lastfm', isLastfmEnabled ? 'true' : 'false')
  }
  function handleLastfmKey(e)    { lastfmKey = e.target.value;    Keyring.save('lastfm_api_key', e.target.value).catch(() => {}) }
  function handleLastfmSecret(e) { lastfmSecret = e.target.value; Keyring.save('lastfm_secret',  e.target.value).catch(() => {}) }

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
    clearAll(); wipeCacheLabel = 'Wiped!'
    setTimeout(() => wipeCacheLabel = 'Wipe', 1500)
  }
  async function deleteLogs() {
    deleteLogsDisabled = true
    try { await tauriInvoke('delete_logs'); deleteLogsLabel = 'Deleted!' }
    catch { deleteLogsLabel = 'Failed' }
    setTimeout(() => { deleteLogsLabel = 'Delete'; deleteLogsDisabled = false }, 1500)
  }
  function deleteSettings() {
    SETTINGS_KEYS.forEach(k => SafeStorage.removeItem(k))
    deleteSettingsLabel = 'Deleted!'
    setTimeout(() => deleteSettingsLabel = 'Delete', 1500)
  }

  // Sub-panel title derived from the active category
  const subTitle = $derived(CATEGORIES.find(c => c.id === activeCategory)?.label ?? '')
</script>

<!-- Close theme dropdown when clicking outside -->
<svelte:document onclick={() => themeOpen = false} />

<div class="mset-overlay" class:mset-closing={closing}>

  <!-- ── Header ───────────────────────────────────────────────────────────── -->
  <div class="mset-header">
    <button class="mset-back-btn" onclick={goBack} aria-label="Back">
      <span class="icon" style="width:24px;height:24px">{@html IconBack}</span>
    </button>
    <span class="mset-title">{activeCategory ? subTitle : 'Settings'}</span>
  </div>

  <!-- ── Body: category list or sub-panel ─────────────────────────────────── -->
  <div class="mset-body">

    {#if activeCategory === null}
      <!-- Category list — the "folder" view -->
      <div class="mset-cat-list">
        {#each CATEGORIES as cat}
          <button
            class="mset-cat-row"
            onclick={() => openCategory(cat.id)}
          >
            <span class="mset-cat-icon icon" style="width:20px;height:20px">{@html cat.icon}</span>
            <span class="mset-cat-label">{cat.label}</span>
            <span class="mset-cat-chevron icon" style="width:16px;height:16px">{@html IconChevronRight}</span>
          </button>
        {/each}
      </div>

    {:else}
      <!-- Sub-panel — slides in from the right -->
      <div class="mset-subpanel" class:mset-subpanel--in={subSliding}>

        {#if activeCategory === 'appearance'}
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

        {:else if activeCategory === 'services'}
          <div class="settings-row">
            <div class="settings-info">
              <div class="settings-title">Last.fm Integration</div>
              <div class="settings-desc">Fetch artist biography and photo via Last.fm using your own API key</div>
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
              <div class="settings-desc">Fetch synced lyrics from lrclib.net when your server has none</div>
            </div>
            <label class="toggle-switch">
              <input type="checkbox" bind:checked={isLrclibEnabled} onchange={handleLrclib} />
              <span class="toggle-slider"></span>
            </label>
          </div>

        {:else if activeCategory === 'account'}
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

        {:else if activeCategory === 'about'}
          <div class="settings-row">
            <div class="settings-info">
              <div class="settings-title">App Version</div>
              <div class="settings-desc">{appVersion}</div>
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
              <div class="settings-title">Reset Settings</div>
              <div class="settings-desc">Reset all preferences to defaults</div>
            </div>
            <button class="debug-btn debug-btn--danger" onclick={deleteSettings}>{deleteSettingsLabel}</button>
          </div>
        {/if}

      </div>
    {/if}
  </div>
</div>
