import { tauriInvoke } from './tauri.js'
import { isMobile } from './platform.js'
import { OpenSubsonicRouter } from './api.js'

let _mediaActionHandler = null

// Call once after the bridge is ready. Wires notification button events
// (prev / togglePlayPause / next) back into the app via a callback.
export function initNowPlaying(onMediaAction) {
  if (!isMobile) return
  _mediaActionHandler = onMediaAction
  // Plugin.trigger() sends through a Channel, not the global event bus.
  // addPluginListener registers the channel with the plugin's registerListener command.
  import('@tauri-apps/api/core').then(({ addPluginListener }) => {
    addPluginListener('now-playing', 'mediaAction', (payload) => {
      if (_mediaActionHandler) _mediaActionHandler(payload?.action)
    })
  }).catch(() => {})
}

// Post or update the notification. Called whenever the current track changes.
export async function updateNowPlaying(track, isPlaying) {
  if (!isMobile || !track) return
  try {
    const coverUrl = track.coverArtId
      ? await OpenSubsonicRouter.buildUrl('getCoverArt', { id: track.coverArtId, size: 300 })
      : ''
    await tauriInvoke('update_now_playing', {
      title: track.title ?? '',
      artist: track.artist ?? '',
      album: track.album ?? '',
      coverUrl,
      isPlaying: !!isPlaying,
    })
  } catch (e) {
    console.error('updateNowPlaying failed:', e)
  }
}

// Refresh just the play/pause icon without re-fetching cover art.
export async function updateNowPlayingState(isPlaying) {
  if (!isMobile) return
  try {
    await tauriInvoke('update_playback_state', { isPlaying: !!isPlaying })
  } catch (e) {
    console.error('updateNowPlayingState failed:', e)
  }
}

// Remove the notification (e.g. when the queue ends).
export async function clearNowPlaying() {
  if (!isMobile) return
  try {
    await tauriInvoke('clear_now_playing', {})
  } catch (e) {
    console.error('clearNowPlaying failed:', e)
  }
}
