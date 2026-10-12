<script lang="ts">
  import { IconMusic, IconDownload, IconLoading } from '../lib/icons'
  import { Api } from '../lib/api'
  import { cover } from '../lib/cover'
  import { showMenu } from '../lib/menu'
  import { navTo, isAuthed, toastError } from '../lib/stores'
  import type { Album } from '../lib/types'

  let { album }: { album: Album } = $props()

  let dl = $state<'idle' | 'loading' | 'done' | 'error'>('idle')
  async function download(e: MouseEvent) {
    e.stopPropagation()
    if (dl === 'loading') return
    dl = 'loading'
    try { await Api.downloadAlbum(album.id); dl = 'done' }
    catch (err) { toastError(err); dl = 'error' }
    finally { setTimeout(() => (dl = 'idle'), 2000) }
  }
</script>

<div class="album-row" role="button" tabindex="0" onclick={() => navTo('album', album.id)}
  onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navTo('album', album.id)}>
  <div class="album-art-sm">
    {#if album.coverArtId}
      <img use:cover={album.coverArtId} alt="" />
    {:else}
      <div class="no-art"><span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconMusic}</span></div>
    {/if}
  </div>
  <div class="album-info">
    <div class="album-title">{album.name}</div>
    <div class="album-artist">{album.albumArtist}</div>
  </div>
  {#if $isAuthed}
    <button class="album-download-btn" class:download-done={dl === 'done'} class:download-error={dl === 'error'}
      title="Download album" disabled={dl === 'loading'} onclick={download}>
      <span class="icon" style="width:14px;height:14px">{@html dl === 'loading' ? IconLoading : IconDownload}</span>
    </button>
  {/if}
  <button class="album-add-btn" title="More" onclick={e => { e.stopPropagation(); showMenu(e.currentTarget, { type: 'album', albumId: album.id, albumName: album.name }) }}>+</button>
</div>
