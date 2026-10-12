<script lang="ts">
  import { onMount } from 'svelte'
  import { Api } from '../lib/api'
  import {
    settings, themes, updateSetting, resetSettings, isAuthed, auth, showAccountModal, setAuthState, navTo, toast, toastError, FONT_OPTIONS,
  } from '../lib/stores'
  import { IconChevronDown, IconPalette, IconPlay, IconGlobe, IconUser, IconInfo, IconDownload, IconEqualizer, IconRecap } from '../lib/icons'
  import EqualizerSettings from '../components/EqualizerSettings.svelte'
  import type { PlayHistorySummary } from '../lib/types'

  const CATEGORIES = [
    { id: 'appearance', label: 'Appearance', icon: IconPalette },
    { id: 'playback', label: 'Playback', icon: IconPlay },
    { id: 'equalizer', label: 'Equalizer', icon: IconEqualizer },
    { id: 'downloads', label: 'Downloads', icon: IconDownload },
    { id: 'services', label: 'Services', icon: IconGlobe },
    { id: 'stats', label: 'Stats', icon: IconRecap },
    { id: 'account', label: 'Account', icon: IconUser },
    { id: 'debug', label: 'Debug', icon: IconInfo },
  ]
  const FORMATS = [
    { id: 'raw', name: 'Original' }, { id: 'mp3', name: 'MP3' }, { id: 'flac', name: 'FLAC' },
    { id: 'wav', name: 'WAV' }, { id: 'opus', name: 'Opus' },
  ]
  const UI_THEMES = [{ id: 'default', name: 'Default' }, { id: 'spotify', name: 'Spotify' }]
  const BIT_PERFECT = [['off', 'Off'], ['relaxed', 'Relaxed'], ['strict', 'Strict']] as const

  let category = $state('appearance')
  let open = $state<'' | 'theme' | 'ui' | 'font' | 'format'>('')
  let version = $state('Loading…')
  let summary = $state<PlayHistorySummary | null>(null)
  let lastfmOn = $state(false)
  let listenbrainzOn = $state(false)
  let flash = $state({ wipe: 'Wipe', reset: 'Reset' })

  const s = $derived($settings)

  onMount(async () => {
    Api.appVersion().then(v => (version = `v${v}`)).catch(() => (version = 'unavailable'))
    Api.historySummary().then(r => (summary = r)).catch(() => {})
    const cur = $settings
    if (cur) { lastfmOn = !!cur.lastfmKey; listenbrainzOn = !!cur.listenbrainzToken }
  })

  const checked = (e: Event) => (e.target as HTMLInputElement).checked
  const value = (e: Event) => (e.target as HTMLInputElement).value
  const pick = (kind: typeof open, set: () => void) => { set(); open = '' ; void kind }

  function toggleLastfm(on: boolean) {
    lastfmOn = on
    if (!on) { updateSetting('lastfmKey', ''); updateSetting('lastfmSecret', '') }
  }
  function toggleListenbrainz(on: boolean) {
    listenbrainzOn = on
    if (!on) updateSetting('listenbrainzToken', '')
  }

  async function exportHistory(format: 'csv' | 'json') {
    try { if (await Api.exportHistory(format)) toast('History exported') } catch (e) { toastError(e) }
  }

  async function importFiles() {
    try {
      const n = await Api.pickAndImport()
      if (n != null) toast(`Imported ${n} file${n === 1 ? '' : 's'}`)
    } catch (e) { toastError(e) }
  }

  async function wipeCache() {
    await Api.clearCoverCache().catch(toastError)
    flash.wipe = 'Wiped!'
    setTimeout(() => (flash.wipe = 'Wipe'), 1500)
  }

  async function reset() {
    await resetSettings()
    flash.reset = 'Reset!'
    setTimeout(() => (flash.reset = 'Reset'), 1500)
  }

  async function disconnect() {
    await setAuthState(await Api.logout())
    navTo('home')
  }

  const hours = (secs: number) => `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`
</script>

<svelte:document onclick={() => (open = '')} />

{#snippet dropdown(kind: 'theme' | 'ui' | 'font' | 'format', label: string, options: { id: string; name: string }[], current: string, onPick: (id: string) => void)}
  <div class="theme-selector" class:open={open === kind} role="button" tabindex="0"
    onclick={e => { e.stopPropagation(); open = open === kind ? '' : kind }}
    onkeydown={e => { e.stopPropagation(); if (e.key === 'Enter' || e.key === ' ') open = open === kind ? '' : kind }}>
    <div class="theme-selector-value">{label}</div>
    <span class="theme-selector-arrow icon" style="width:14px;height:14px">{@html IconChevronDown}</span>
    <div class="theme-selector-dropdown">
      {#each options as o (o.id)}
        <div class="theme-option" class:selected={current === o.id} role="option" tabindex="0" aria-selected={current === o.id}
          onclick={e => { e.stopPropagation(); pick(kind, () => onPick(o.id)) }}
          onkeydown={e => { e.stopPropagation(); if (e.key === 'Enter' || e.key === ' ') pick(kind, () => onPick(o.id)) }}>{o.name}</div>
      {/each}
    </div>
  </div>
{/snippet}

{#snippet toggle(on: boolean, onChange: (v: boolean) => void)}
  <label class="toggle-switch">
    <input type="checkbox" checked={on} onchange={e => onChange(checked(e))} />
    <span class="toggle-slider"></span>
  </label>
{/snippet}

{#if s}
<div class="sett-layout">
  <nav class="sett-sidebar">
    <div class="sett-sidebar-label">Settings</div>
    {#each CATEGORIES as cat}
      <button class="sett-nav-btn" class:sett-nav-btn--active={category === cat.id} onclick={() => (category = cat.id)}>
        <span class="icon sett-nav-icon" style="width:16px;height:16px">{@html cat.icon}</span>{cat.label}
      </button>
    {/each}
  </nav>

  <div class="sett-panel">
    {#if category === 'appearance'}
      <div class="sett-panel-title">Appearance</div>

      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Window Decorations</div><div class="settings-desc">Show native title bar and borders</div></div>
        {@render toggle(s.windowDecorations, v => updateSetting('windowDecorations', v))}
      </div>

      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Color Theme</div><div class="settings-desc">Color scheme for the interface</div></div>
        {@render dropdown('theme', $themes.find(t => t.id === s.themeId)?.name ?? s.themeId, $themes.map(t => ({ id: t.id, name: t.name })), s.themeId, id => updateSetting('themeId', id))}
      </div>

      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">UI Theme</div><div class="settings-desc">Layout style for navigation, player bar and browsing screens</div></div>
        {@render dropdown('ui', UI_THEMES.find(t => t.id === s.uiThemeId)?.name ?? 'Default', UI_THEMES, s.uiThemeId, id => updateSetting('uiThemeId', id))}
      </div>

      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Font</div><div class="settings-desc">Interface font</div></div>
        {@render dropdown('font', s.fontFamily, FONT_OPTIONS.map(f => ({ id: f, name: f })), s.fontFamily, id => updateSetting('fontFamily', id))}
      </div>

      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Scrollbar Width</div><div class="settings-desc">Thickness of list scrollbars</div></div>
        <div class="crossfade-duration-control">
          <input type="range" min="6" max="20" step="1" value={s.scrollbarWidth} onchange={e => updateSetting('scrollbarWidth', Number(value(e)))} />
          <span>{s.scrollbarWidth}px</span>
        </div>
      </div>

    {:else if category === 'playback'}
      <div class="sett-panel-title">Playback</div>

      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Crossfade</div><div class="settings-desc">Smoothly blend between tracks</div></div>
        {@render toggle(s.crossfadeEnabled, v => updateSetting('crossfadeEnabled', v))}
      </div>
      {#if s.crossfadeEnabled}
        <div class="settings-row">
          <div class="settings-info"><div class="settings-title">Crossfade Duration</div><div class="settings-desc">Length of the blend in seconds</div></div>
          <div class="crossfade-duration-control">
            <input type="range" min="1" max="12" step="1" value={s.crossfadeDuration} onchange={e => updateSetting('crossfadeDuration', Number(value(e)))} />
            <span>{s.crossfadeDuration}s</span>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-info"><div class="settings-title">Crossfade Curve</div><div class="settings-desc">Linear, or logarithmic (equal-power) fade</div></div>
          <div class="bp-mode-selector">
            {#each [['linear', 'Linear'], ['logarithmic', 'Log']] as [id, label]}
              <button class="bp-mode-btn" class:bp-mode-btn--active={s.crossfadeCurve === id} onclick={() => updateSetting('crossfadeCurve', id as 'linear' | 'logarithmic')}>{label}</button>
            {/each}
          </div>
        </div>
      {/if}

      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Gapless Playback</div><div class="settings-desc">Pre-buffer the next track for seamless transitions</div></div>
        {@render toggle(s.gaplessEnabled, v => updateSetting('gaplessEnabled', v))}
      </div>
      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">ReplayGain</div><div class="settings-desc">Normalize track loudness using server-provided gain values</div></div>
        {@render toggle(s.replayGainEnabled, v => updateSetting('replayGainEnabled', v))}
      </div>
      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Continue playing after queue ends</div><div class="settings-desc">Smart Radio keeps the music going by adding similar tracks when the queue runs out</div></div>
        {@render toggle(s.autoContinue, v => updateSetting('autoContinue', v))}
      </div>

      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Bit-Perfect Audio</div>
          <div class="settings-desc">
            {#if s.bitPerfectMode === 'off'}Resamples all audio to the device default rate
            {:else if s.bitPerfectMode === 'strict'}Matches native sample rate; crossfade and gapless are disabled
            {:else}Tries to match each track's native sample rate; falls back to resampling{/if}
          </div>
        </div>
        <div class="bp-mode-selector">
          {#each BIT_PERFECT as [id, label]}
            <button class="bp-mode-btn" class:bp-mode-btn--active={s.bitPerfectMode === id} onclick={() => updateSetting('bitPerfectMode', id)}>{label}</button>
          {/each}
        </div>
      </div>

    {:else if category === 'equalizer'}
      <EqualizerSettings />

    {:else if category === 'downloads'}
      <div class="sett-panel-title">Downloads</div>
      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Download Format</div><div class="settings-desc">Format used when downloading tracks and albums. "Original" saves the file exactly as stored on the server.</div></div>
        {@render dropdown('format', FORMATS.find(f => f.id === s.downloadFormat)?.name ?? 'Original', FORMATS, s.downloadFormat, id => updateSetting('downloadFormat', id))}
      </div>
      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Import Local Files</div><div class="settings-desc">Copy audio files into your offline library (~/Music/Firmium). You can also drop files onto the window.</div></div>
        <button class="debug-btn" onclick={importFiles}>Choose files…</button>
      </div>

    {:else if category === 'services'}
      <div class="sett-panel-title">Services</div>

      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Last.fm Integration</div><div class="settings-desc">Fetch artist biography and similar artists from Last.fm using your own API key</div></div>
        {@render toggle(lastfmOn, toggleLastfm)}
      </div>
      {#if lastfmOn}
        <div class="settings-row">
          <div class="settings-info"><div class="settings-title">Last.fm API Key</div><div class="settings-desc">From your Last.fm API account</div></div>
          <input class="settings-text-input" type="text" value={s.lastfmKey} onchange={e => updateSetting('lastfmKey', value(e))} placeholder="API key…" />
        </div>
        <div class="settings-row">
          <div class="settings-info"><div class="settings-title">Last.fm Secret</div><div class="settings-desc">Shared secret for your API account</div></div>
          <input class="settings-text-input" type="password" value={s.lastfmSecret} onchange={e => updateSetting('lastfmSecret', value(e))} placeholder="Secret…" />
        </div>
      {/if}

      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">ListenBrainz Scrobbling</div><div class="settings-desc">Submit each completed track to ListenBrainz using your user token</div></div>
        {@render toggle(listenbrainzOn, toggleListenbrainz)}
      </div>
      {#if listenbrainzOn}
        <div class="settings-row">
          <div class="settings-info"><div class="settings-title">ListenBrainz Token</div><div class="settings-desc">From your ListenBrainz profile settings</div></div>
          <input class="settings-text-input" type="password" value={s.listenbrainzToken} onchange={e => updateSetting('listenbrainzToken', value(e))} placeholder="User token…" />
        </div>
      {/if}

      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">External Lyrics (LRCLIB)</div><div class="settings-desc">Fetch synced lyrics from lrclib.net when your server has none. Sends song title and artist name.</div></div>
        {@render toggle(s.lrclibEnabled, v => updateSetting('lrclibEnabled', v))}
      </div>
      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Word-by-Word Lyrics Animation</div><div class="settings-desc">Karaoke-style fill on the active lyric line. Disable for plain line-by-line highlighting.</div></div>
        {@render toggle(s.lyricsWordFill, v => updateSetting('lyricsWordFill', v))}
      </div>

    {:else if category === 'stats'}
      <div class="sett-panel-title">Stats</div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Listening History</div>
          <div class="settings-desc">
            {#if summary}{summary.totalPlays} plays · {hours(summary.totalSeconds)} · {summary.uniqueTracks} tracks · {summary.uniqueArtists} artists
            {:else}Every completed track is recorded on this device.{/if}
          </div>
        </div>
        <button class="debug-btn" onclick={() => navTo('recap')}>Open Recap</button>
      </div>
      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Export History</div><div class="settings-desc">Save your play history as a file</div></div>
        <div class="bp-mode-selector">
          <button class="bp-mode-btn" onclick={() => exportHistory('csv')}>CSV</button>
          <button class="bp-mode-btn" onclick={() => exportHistory('json')}>JSON</button>
        </div>
      </div>

    {:else if category === 'account'}
      <div class="sett-panel-title">Account</div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Connection</div>
          <div class="settings-desc">{$isAuthed ? `${$auth.username}@${$auth.server}` : 'Not connected. Browsing local files'}</div>
        </div>
        {#if $isAuthed}
          <button class="debug-btn debug-btn--danger" onclick={disconnect}>Disconnect</button>
        {:else}
          <button class="debug-btn" onclick={() => showAccountModal.set(true)}>Connect</button>
        {/if}
      </div>
      {#if $auth.accounts.length > 0}
        <div class="settings-row">
          <div class="settings-info"><div class="settings-title">Saved Accounts</div><div class="settings-desc">Switch accounts from the account button in the sidebar</div></div>
          <button class="debug-btn" onclick={() => showAccountModal.set(true)}>Manage</button>
        </div>
      {/if}

    {:else if category === 'debug'}
      <div class="sett-panel-title">Debug</div>
      <div class="settings-row"><div class="settings-info"><div class="settings-title">App Version</div><div class="settings-desc">{version}</div></div></div>
      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Wipe Cover Cache</div><div class="settings-desc">Delete cached album art from disk</div></div>
        <button class="debug-btn" onclick={wipeCache}>{flash.wipe}</button>
      </div>
      <div class="settings-row">
        <div class="settings-info"><div class="settings-title">Reset Settings</div><div class="settings-desc">Reset preferences to defaults (accounts are kept)</div></div>
        <button class="debug-btn debug-btn--danger" onclick={reset}>{flash.reset}</button>
      </div>
    {/if}
  </div>
</div>
{/if}
