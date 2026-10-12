<script lang="ts">
  import { toasts } from '../lib/stores'
</script>

<div class="toast-stack" aria-live="polite">
  {#each $toasts as t (t.id)}
    <div class="toast">{t.text}</div>
  {/each}
</div>
