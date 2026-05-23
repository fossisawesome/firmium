// ============================================================================
// AUDIO BRIDGE INITIALIZATION
// ============================================================================
// Add this near the top of the Store object, alongside Playback:

Store.Audio = (() => {
  let _bridge = null;
  
  return {
    init: () => {
      _bridge = new AudioBridge();
      
      // Wire up event listeners
      _bridge.on('statechange', (state) => {
        // Update UI based on state
        const isPlaying = state === 'playing';
        const playBtn = DOM.el('playBtn');
        if (playBtn) playBtn.textContent = isPlaying ? '⏸' : '▶';
      });
      
      _bridge.on('finished', () => {
        // Handle track end - same as audio.ended event
        if (Store.Playback.getRepeatOne()) {
          const currentIdx = Store.Playback.getQueueIdx();
          playAt(currentIdx);
        } else if (Store.Playback.getQueueIdx() < Store.Playback.getQueue().length - 1) {
          playAt(Store.Playback.getQueueIdx() + 1);
        } else if (Store.Playback.getRepeatAll()) {
          playAt(0);
        } else {
          document.title = 'Firmium';
        }
      });
      
      _bridge.on('volumechange', (vol) => {
        // Update volume slider if needed
        const slider = DOM.el('volSlider');
        if (slider) slider.value = vol;
      });
      
      _bridge.on('error', (msg) => {
        console.error('Audio error:', msg);
        DOM.render('npArtist', `Audio Error: ${DOM.safeText(msg)}`);
      });
      
      return _bridge;
    },
    
    getBridge: () => _bridge,
  };
})();

// ============================================================================
// UPDATED PLAYBACK FUNCTION
// ============================================================================
// Replace the existing playAt function with this:

const playAt = async (idx) => {
  const bridge = Store.Audio.getBridge();
  if (!bridge || idx < 0 || idx >= Store.Playback.getQueue().length) return;

  Store.Playback.setQueueIdx(idx);
  const track = Store.Playback.getCurrentTrack();
  if (!track) return;

  const currentToken = Store.Playback.bumpToken();
  updateNowPlaying(track);
  highlightCurrentTrack();

  try {
    const streamUrl = SubsonicRouter.buildUrl('stream', { id: track.id });
    if (currentToken !== Store.Playback.getPlayToken()) return;

    // Use native audio bridge instead of <audio> element
    const playerId = await bridge.play(streamUrl, track.id);
    
    // Store player ID for volume sync
    Store.Playback._currentPlayerId = playerId;
    
    document.title = `▶ ${track.title} - Firmium`;
  } catch (e) {
    if (currentToken === Store.Playback.getPlayToken()) {
      console.error('Playback exception:', e);
      DOM.render('npArtist', `Playback Error: ${DOM.safeText(e.message)}`);
    }
  }
};

// ============================================================================
// UPDATED PLAY/PAUSE TOGGLE
// ============================================================================
// Update the play-toggle action in the click handler:

case 'play-toggle': {
  const bridge = Store.Audio.getBridge();
  if (!Store.Playback.getCurrentTrack() || !bridge) return;
  
  bridge.getState().then(state => {
    if (state === 'paused') {
      bridge.resume();
    } else {
      bridge.pause();
    }
  }).catch(err => {
    console.error('Toggle failed:', err);
  });
  break;
}

// ============================================================================
// UPDATED VOLUME CONTROL
// ============================================================================
// Replace the volSlider event listener:

DOM.el('volSlider')?.addEventListener('input', async (e) => {
  const volume = e.target.value;
  const bridge = Store.Audio.getBridge();
  
  if (bridge) {
    try {
      await bridge.setVolume(volume);
    } catch (err) {
      console.error('Volume change failed:', err);
    }
  }
  
  // Persist volume locally
  Store.Playback.setVolume(volume);
});

// ============================================================================
// INITIALIZATION IN DOMContentLoaded
// ============================================================================
// Add this to the DOMContentLoaded event handler, after SafeStorage initialization:

// Initialize audio bridge
const audioBridge = Store.Audio.init();

// Initialize volume from storage
const savedVolume = Number(SafeStorage.getItem('firmium_volume') ?? 0.8);
Store.Playback.setVolume(savedVolume);
DOM.el('volSlider').value = savedVolume;

// ============================================================================
// TEARDOWN UPDATES
// ============================================================================
// Update teardownApp to cleanup audio bridge:

const teardownApp = () => {
  const bridge = Store.Audio.getBridge();
  if (bridge) {
    bridge.destroy();
  }

  // Remove old audio element if it exists
  const oldAudio = DOM.el('audioEl');
  if (oldAudio) oldAudio.remove();

  Store.Playback.abortActive();
  Store.Playback.abortSearch();
  Store.Playback.clearObserver();
  Store.Playback.clearAllCache();
  Store.Auth.clearAuth();
  Store.UI.clearNav();

  DOM.el('setup')?.classList.remove('hidden');
  DOM.el('app')?.classList.add('hidden');
  DOM.render('setupError', '');
  document.title = 'Firmium';
};

// ============================================================================
// REMOVE OLD AUDIO ELEMENT INITIALIZATION
// ============================================================================
// DELETE this code from DOMContentLoaded (the old <audio> element setup):
/*
let audio = DOM.el('audioEl');
if (!audio) {
  audio = document.createElement('audio');
  audio.id = 'audioEl';
  audio.preload = 'auto';
  document.body.appendChild(audio);
}
Store.Playback.initAudio(audio);
*/

// And remove all the audio event listeners:
/*
audio.addEventListener('play', () => { ... });
audio.addEventListener('pause', () => { ... });
audio.addEventListener('durationchange', () => { ... });
audio.addEventListener('error', () => { ... });
audio.addEventListener('stalled', () => { ... });
audio.addEventListener('timeupdate', () => { ... });
audio.addEventListener('ended', () => { ... });
*/

// ============================================================================
// UPDATE STORE.PLAYBACK 
// ============================================================================
// Simplify the Playback store (you can keep most of it, but remove audio-related code):

Store.Playback = (() => {
  let _queue = [], _queueIdx = -1, _playToken = 0;
  let _volume = Number(SafeStorage.getItem('firmium_volume') ?? 0.8);
  let _repeatOne = false, _repeatAll = false;
  let _abortCtrl = null, _searchCtrl = null, _observer = null;
  const _covers = new Map(), _pendingCovers = new Map();
  let _currentPlayerId = null;

  return {
    getQueue: () => _queue,
    getQueueIdx: () => _queueIdx,
    getCurrentTrack: () => _queue[_queueIdx] || null,
    setQueue: (items, idx = 0) => { _queue = Array.isArray(items) ? items : []; _queueIdx = _queue.length ? idx : -1; },
    setQueueIdx: (idx) => { if (idx >= 0 && idx < _queue.length) _queueIdx = idx; },
    getPlayToken: () => _playToken,
    bumpToken: () => ++_playToken,
    getVolume: () => _volume,
    setVolume: (v) => { _volume = Math.max(0, Math.min(1, Number.isFinite(Number(v)) ? Number(v) : 0.8)); SafeStorage.setItem('firmium_volume', String(_volume)); },
    getRepeatOne: () => _repeatOne,
    setRepeatOne: (v) => { _repeatOne = Boolean(v); if (v) _repeatAll = false; },
    getRepeatAll: () => _repeatAll,
    setRepeatAll: (v) => { _repeatAll = Boolean(v); if (v) _repeatOne = false; },

    abortActive: () => { if (_abortCtrl) { _abortCtrl.abort(); _abortCtrl = null; } },
    setActiveCtrl: (c) => { _abortCtrl = c; },
    getActiveCtrl: () => _abortCtrl,
    abortSearch: () => { if (_searchCtrl) { _searchCtrl.abort(); _searchCtrl = null; } },
    setSearchCtrl: (c) => { _searchCtrl = c; },
    clearObserver: () => { if (_observer) { _observer.disconnect(); _observer = null; } },
    setObserver: (o) => { _observer = o; },

    addCover: (id, url) => {
      if (!id || !url) return;
      if (_covers.has(id)) _covers.delete(id);
      _covers.set(id, url);
      while (_covers.size > MAX_COVER_CACHE_SIZE) {
        const oldest = _covers.keys().next().value;
        const oldUrl = _covers.get(oldest);
        if (oldUrl?.startsWith('blob:')) { try { URL.revokeObjectURL(oldUrl); } catch(e){} }
        _covers.delete(oldest);
      }
    },
    getCover: (id) => {
      const url = _covers.get(id) || null;
      if (url) { _covers.delete(id); _covers.set(id, url); }
      return url;
    },
    getPendingCover: (id) => _pendingCovers.get(id) || null,
    setPendingCover: (id, p) => { if (id && p) _pendingCovers.set(id, p); },
    clearPendingCover: (id) => { _pendingCovers.delete(id); },
    clearAllCache: () => {
      _covers.forEach(url => { if (url?.startsWith('blob:')) { try { URL.revokeObjectURL(url); } catch(e){} } });
      _covers.clear();
      _pendingCovers.clear();
    }
  };
})();
