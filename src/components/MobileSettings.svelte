<script>
  import { mobileSettingsOpen } from '../lib/stores.js'
  import Settings from '../views/Settings.svelte'
  import { IconBack } from '../lib/icons.js'

  // Props passed through to the underlying Settings component
  let { onapplyTheme, onapplyDecorations, themes = [] } = $props()

  let closing = $state(false)

  function closeSettings() {
    closing = true
    setTimeout(() => mobileSettingsOpen.set(false), 280)
  }
</script>

<div class="mset-overlay" class:mset-closing={closing}>
  <div class="mset-header">
    <button class="mset-back-btn" onclick={closeSettings} aria-label="Close settings">
      <span class="icon" style="width:24px;height:24px">{@html IconBack}</span>
    </button>
    <span class="mset-title">Settings</span>
  </div>
  <div class="mset-body">
    <Settings {onapplyTheme} {onapplyDecorations} {themes} />
  </div>
</div>
