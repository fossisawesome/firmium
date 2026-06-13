<script lang="ts">
  import type { Snippet } from 'svelte'

  let { loading, error, empty, loadingMessage = 'Loading…', emptyMessage = 'Nothing found.', children }: {
    loading: boolean
    error: string
    empty: boolean
    loadingMessage?: string
    emptyMessage?: string
    children: Snippet
  } = $props()
</script>

{#if loading}
  <div class="loading-msg">{loadingMessage}</div>
{:else if error}
  <div class="loading-msg error-msg">{error}</div>
{:else if empty}
  <div class="loading-msg">{emptyMessage}</div>
{:else}
  {@render children()}
{/if}
