<script lang="ts">
  import { IconMusic, IconDownload, IconLoading } from '../lib/icons'
  import { loadImage, Api } from '../lib/api'
  import { showPlaylistMenu } from '../lib/playlistMenu'
  import { lazyLoad } from '../lib/lazyLoad'
  import { navToAlbum, downloadFormat, isAuthed } from '../lib/stores'
  import type { Album } from '../lib/types/tauri-commands'

  let { album, signal = null }: { album: Album; signal?: AbortSignal | null } = $props()

  let downloadState = $state<'idle' | 'loading' | 'done' | 'error'>('idle')

  async function download(e: MouseEvent) {
    e.stopPropagation()
    if (downloadState === 'loading') return
    downloadState = 'loading'
    try {
      await Api.downloadAlbum(album.id, $downloadFormat)
      downloadState = 'done'
    } catch (err) {
      console.error('Album download failed:', err)
      downloadState = 'error'
    } finally {
      setTimeout(() => { downloadState = 'idle' }, 2000)
    }
  }
</script>

<div
  class="album-row"
  role="button"
  tabindex="0"
  onclick={() => navToAlbum(album.id)}
  onkeydown={e => (e.key === 'Enter' || e.key === ' ') && navToAlbum(album.id)}
>
  <div class="album-art-sm">
    {#if album.coverArtId}
      <img use:lazyLoad={img => loadImage(img, album.coverArtId, signal)} alt="" />
    {:else}
      <div class="no-art"><span class="icon" style="width:16px;height:16px;color:var(--muted)">{@html IconMusic}</span></div>
    {/if}
  </div>
  <div class="album-info">
    <div class="album-title">{album.name}</div>
    <div class="album-artist">{album.albumArtist}</div>
  </div>
  {#if $isAuthed}
    <button
      class="album-download-btn"
      class:download-done={downloadState === 'done'}
      class:download-error={downloadState === 'error'}
      title="Download album"
      disabled={downloadState === 'loading'}
      onclick={download}
    >
      <span class="icon" style="width:14px;height:14px">{@html downloadState === 'loading' ? IconLoading : IconDownload}</span>
    </button>
  {/if}
  <button
    class="album-add-btn"
    title="Add to playlist"
    onclick={e => { e.stopPropagation(); showPlaylistMenu(e.currentTarget, { type: 'album', albumId: album.id, albumName: album.name }) }}
  >+</button>
</div>
