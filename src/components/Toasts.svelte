<script lang="ts">
  import { app } from '../lib/stores.svelte'
</script>

<div class="toast-stack" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast">{t.text}</div>
  {/each}
</div>
