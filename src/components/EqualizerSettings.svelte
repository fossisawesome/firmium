<script lang="ts">
  import { onMount } from 'svelte'
  import { tauriInvoke } from '../lib/tauri'
  import { IconChevronDown } from '../lib/icons'
  import type { AudioDevice, EqState, EqMode, EqBandSpec } from '../lib/types/tauri-commands'

  // Fixed 10-band ISO graphic set — must match commands/equalizer.rs band ordering
  // (first = low shelf, last = high shelf, middle = peaking).
  const GRAPHIC_FREQS = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]
  const GAIN_RANGE = 12

  let enabled = $state(false)
  let profiles = $state<EqState['profiles']>([])
  let devices = $state<AudioDevice[]>([])
  let deviceProfiles = $state<Record<string, string>>({})
  let defaultDevice = $state<string | null>(null)
  let selectedDevice = $state<string>('')

  let mode = $state<EqMode>('graphic')
  let bands = $state<EqBandSpec[]>(graphicDefaults())

  let deviceOpen = $state(false)
  let profileOpen = $state(false)
  let newName = $state('')
  let saveDebounce: ReturnType<typeof setTimeout> | undefined

  const activeProfileName = $derived(deviceProfiles[selectedDevice] ?? '')

  function graphicDefaults(): EqBandSpec[] {
    return GRAPHIC_FREQS.map(freq => ({ freq, gain: 0 }))
  }

  function freqLabel(f: number): string {
    return f >= 1000 ? `${f / 1000}k` : `${f}`
  }

  async function load() {
    try {
      const [state, devs] = await Promise.all([
        tauriInvoke<EqState>('get_eq_state'),
        tauriInvoke<AudioDevice[]>('list_audio_devices'),
      ])
      enabled = state.enabled
      profiles = state.profiles
      deviceProfiles = state.deviceProfiles
      defaultDevice = state.defaultDevice
      devices = devs
      selectedDevice = defaultDevice ?? devs.find(d => d.default)?.name ?? devs[0]?.name ?? ''
      loadActiveProfileIntoEditor()
    } catch (e) {
      console.error('Failed to load EQ state:', e)
    }
  }

  function loadActiveProfileIntoEditor() {
    const name = deviceProfiles[selectedDevice]
    const profile = profiles.find(p => p.name === name)
    if (profile) {
      mode = profile.kind
      bands = profile.bands.map(b => ({ ...b }))
    } else {
      mode = 'graphic'
      bands = graphicDefaults()
    }
  }

  async function toggleEnabled(v: boolean) {
    enabled = v
    try { await tauriInvoke('set_eq_enabled', { enabled: v }) } catch (e) { console.error(e) }
  }

  function selectDevice(name: string) {
    selectedDevice = name
    deviceOpen = false
    loadActiveProfileIntoEditor()
  }

  async function selectProfile(name: string) {
    profileOpen = false
    try {
      await tauriInvoke('set_eq_active_profile', { device: selectedDevice, profile: name })
      deviceProfiles = { ...deviceProfiles, [selectedDevice]: name }
      loadActiveProfileIntoEditor()
    } catch (e) { console.error('Failed to set active profile:', e) }
  }

  // Persist band edits to the currently active profile (live-applies in the backend).
  function persistBands() {
    if (!activeProfileName) return
    clearTimeout(saveDebounce)
    saveDebounce = setTimeout(() => {
      tauriInvoke('set_eq_bands', { profile: activeProfileName, bands: $state.snapshot(bands) }).catch(e => console.error(e))
    }, 120)
  }

  function setGraphicGain(i: number, gain: number) {
    bands[i] = { ...bands[i], gain }
    persistBands()
  }

  function setParamField(i: number, field: 'freq' | 'gain' | 'q', value: number) {
    bands[i] = { ...bands[i], [field]: value }
    persistBands()
  }

  function addParamBand() {
    bands = [...bands, { freq: 1000, gain: 0, q: 1.0 }]
    persistBands()
  }

  function removeParamBand(i: number) {
    bands = bands.filter((_, idx) => idx !== i)
    persistBands()
  }

  async function changeMode(next: EqMode) {
    if (next === mode) return
    mode = next
    bands = next === 'graphic'
      ? graphicDefaults()
      : [{ freq: 100, gain: 0, q: 1.0 }, { freq: 1000, gain: 0, q: 1.0 }, { freq: 8000, gain: 0, q: 1.0 }]
    // Re-save the active profile with its new shape, if one is selected.
    if (activeProfileName) {
      try {
        await tauriInvoke('save_eq_profile', { name: activeProfileName, kind: mode, bands: $state.snapshot(bands) })
        profiles = profiles.map(p => p.name === activeProfileName ? { ...p, kind: mode, bands: $state.snapshot(bands) } : p)
      } catch (e) { console.error(e) }
    }
  }

  async function saveAs() {
    const name = newName.trim()
    if (!name) return
    try {
      await tauriInvoke('save_eq_profile', { name, kind: mode, bands: $state.snapshot(bands) })
      await tauriInvoke('set_eq_active_profile', { device: selectedDevice, profile: name })
      newName = ''
      await load()
    } catch (e) { console.error('Failed to save profile:', e) }
  }

  async function deleteActive() {
    if (!activeProfileName) return
    try {
      await tauriInvoke('delete_eq_profile', { name: activeProfileName })
      await load()
    } catch (e) { console.error('Failed to delete profile:', e) }
  }

  onMount(load)
</script>

<svelte:document onclick={() => { deviceOpen = false; profileOpen = false }} />

<div class="sett-panel-title">Equalizer</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Enable Equalizer</div>
    <div class="settings-desc">Apply the active profile to playback. Bypassed in Strict bit-perfect mode.</div>
  </div>
  <label class="toggle-switch">
    <input type="checkbox" checked={enabled} onchange={(e) => toggleEnabled((e.target as HTMLInputElement).checked)} />
    <span class="toggle-slider"></span>
  </label>
</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Output Device</div>
    <div class="settings-desc">Choose which device this profile applies to. Only the system default device is audibly active.</div>
  </div>
  <div class="theme-selector" class:open={deviceOpen} role="button" tabindex="0"
    onclick={e => { e.stopPropagation(); deviceOpen = !deviceOpen }}
    onkeydown={e => { e.stopPropagation(); (e.key === 'Enter' || e.key === ' ') && (deviceOpen = !deviceOpen) }}>
    <div class="theme-selector-value">{selectedDevice || 'No device'}{selectedDevice === defaultDevice ? ' (default)' : ''}</div>
    <span class="theme-selector-arrow icon" style="width:14px;height:14px">{@html IconChevronDown}</span>
    <div class="theme-selector-dropdown">
      {#each devices as dev (dev.name)}
        <div class="theme-option" class:selected={selectedDevice === dev.name} role="option" tabindex="0" aria-selected={selectedDevice === dev.name}
          onclick={e => { e.stopPropagation(); selectDevice(dev.name) }}
          onkeydown={e => { e.stopPropagation(); (e.key === 'Enter' || e.key === ' ') && selectDevice(dev.name) }}>
          {dev.name}{dev.default ? ' (default)' : ''}
        </div>
      {/each}
    </div>
  </div>
</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Profile</div>
    <div class="settings-desc">Active profile for the selected device</div>
  </div>
  <div class="eq-profile-controls">
    <div class="theme-selector" class:open={profileOpen} role="button" tabindex="0"
      onclick={e => { e.stopPropagation(); profileOpen = !profileOpen }}
      onkeydown={e => { e.stopPropagation(); (e.key === 'Enter' || e.key === ' ') && (profileOpen = !profileOpen) }}>
      <div class="theme-selector-value">{activeProfileName || 'None'}</div>
      <span class="theme-selector-arrow icon" style="width:14px;height:14px">{@html IconChevronDown}</span>
      <div class="theme-selector-dropdown">
        {#if profiles.length === 0}
          <div class="theme-option" aria-disabled="true">No saved profiles</div>
        {/if}
        {#each profiles as p (p.name)}
          <div class="theme-option" class:selected={activeProfileName === p.name} role="option" tabindex="0" aria-selected={activeProfileName === p.name}
            onclick={e => { e.stopPropagation(); selectProfile(p.name) }}
            onkeydown={e => { e.stopPropagation(); (e.key === 'Enter' || e.key === ' ') && selectProfile(p.name) }}>
            {p.name}
          </div>
        {/each}
      </div>
    </div>
    <button class="debug-btn debug-btn--danger" disabled={!activeProfileName} onclick={deleteActive}>Delete</button>
  </div>
</div>

<div class="settings-row">
  <div class="settings-info">
    <div class="settings-title">Mode</div>
    <div class="settings-desc">
      {mode === 'graphic' ? 'Fixed 10-band graphic equalizer' : 'Custom bands with frequency, gain, and Q'}
    </div>
  </div>
  <div class="bp-mode-selector">
    {#each [['graphic', 'Graphic'], ['parametric', 'Parametric']] as [id, label]}
      <button class="bp-mode-btn" class:bp-mode-btn--active={mode === id} onclick={() => changeMode(id as EqMode)}>{label}</button>
    {/each}
  </div>
</div>

{#if mode === 'graphic'}
  <div class="eq-graphic">
    {#each bands as band, i (band.freq)}
      <div class="eq-band">
        <span class="eq-band-gain">{band.gain > 0 ? '+' : ''}{band.gain.toFixed(0)}</span>
        <input class="eq-slider" type="range" min={-GAIN_RANGE} max={GAIN_RANGE} step="1"
          value={band.gain} oninput={e => setGraphicGain(i, Number((e.target as HTMLInputElement).value))} />
        <span class="eq-band-freq">{freqLabel(band.freq)}</span>
      </div>
    {/each}
  </div>
{:else}
  <div class="eq-parametric">
    <div class="eq-param-head">
      <span>Freq (Hz)</span><span>Gain (dB)</span><span>Q</span><span></span>
    </div>
    {#each bands as band, i (i)}
      <div class="eq-param-row">
        <input type="number" min="20" max="20000" step="1" value={band.freq}
          oninput={e => setParamField(i, 'freq', Number((e.target as HTMLInputElement).value))} />
        <input type="number" min={-GAIN_RANGE} max={GAIN_RANGE} step="0.5" value={band.gain}
          oninput={e => setParamField(i, 'gain', Number((e.target as HTMLInputElement).value))} />
        <input type="number" min="0.1" max="10" step="0.1" value={band.q ?? 1.0}
          oninput={e => setParamField(i, 'q', Number((e.target as HTMLInputElement).value))} />
        <button class="debug-btn debug-btn--danger" onclick={() => removeParamBand(i)}>Remove</button>
      </div>
    {/each}
    <button class="debug-btn" onclick={addParamBand}>Add band</button>
  </div>
{/if}

<div class="settings-row eq-saveas">
  <div class="settings-info">
    <div class="settings-title">Save as profile</div>
    <div class="settings-desc">Store the current settings under a name</div>
  </div>
  <div class="eq-profile-controls">
    <input class="settings-text-input" type="text" bind:value={newName} placeholder="Profile name…" />
    <button class="debug-btn" disabled={!newName.trim()} onclick={saveAs}>Save</button>
  </div>
</div>

<style>
  .eq-profile-controls { display: flex; gap: 8px; align-items: center; }
  .eq-graphic {
    display: flex; justify-content: space-between; gap: 4px;
    padding: 16px 4px; margin-top: 4px;
  }
  .eq-band { display: flex; flex-direction: column; align-items: center; gap: 6px; flex: 1; }
  .eq-band-gain { font-size: 11px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .eq-band-freq { font-size: 11px; color: var(--muted); }
  .eq-slider {
    writing-mode: vertical-lr; direction: rtl;
    width: 6px; height: 120px; cursor: pointer; accent-color: var(--accent);
  }
  .eq-parametric { padding: 12px 0; display: flex; flex-direction: column; gap: 8px; }
  .eq-param-head, .eq-param-row {
    display: grid; grid-template-columns: 1fr 1fr 1fr auto; gap: 8px; align-items: center;
  }
  .eq-param-head { font-size: 11px; color: var(--muted); }
  .eq-param-row input {
    background: var(--surface2); border: 1px solid var(--border); border-radius: 6px;
    color: var(--text); padding: 6px 8px; width: 100%;
  }
  .eq-saveas { margin-top: 4px; }
</style>
