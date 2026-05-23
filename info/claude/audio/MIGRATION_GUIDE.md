# Migration Guide: Web Audio API → Native Backend

## Overview

This guide shows before/after code for migrating from the web audio API to the native Rust backend.

---

## 1. Audio Element Setup

### BEFORE (Web Audio API)

```javascript
// In DOMContentLoaded:
let audio = DOM.el('audioEl');
if (!audio) {
  audio = document.createElement('audio');
  audio.id = 'audioEl';
  audio.preload = 'auto';
  document.body.appendChild(audio);
}
Store.Playback.initAudio(audio);

// In index.html:
// <audio id="audioEl" preload="auto"></audio>
```

### AFTER (Native Backend)

```javascript
// In DOMContentLoaded:
// Initialize audio bridge instead
const audioBridge = Store.Audio.init();

// In index.html:
// <audio id="audioEl" preload="none" style="display: none;"></audio>
// (Can be removed entirely)
```

---

## 2. Playing Audio

### BEFORE

```javascript
const playAt = async (idx) => {
  const audio = Store.Playback.getAudio();
  if (!audio || idx < 0 || idx >= Store.Playback.getQueue().length) return;

  Store.Playback.setQueueIdx(idx);
  const track = Store.Playback.getCurrentTrack();
  if (!track) return;

  const currentToken = Store.Playback.bumpToken();
  updateNowPlaying(track);
  highlightCurrentTrack();

  try {
    const streamUrl = SubsonicRouter.buildUrl('stream', { id: track.id });
    if (currentToken !== Store.Playback.getPlayToken()) return;

    audio.removeAttribute('src');
    audio.load();
    audio.src = streamUrl;
    await audio.play();
    document.title = `▶ ${track.title} - Firmium`;
  } catch (e) {
    if (currentToken === Store.Playback.getPlayToken()) {
      console.error('Core audio exception:', e);
      DOM.render('npArtist', `Playback Error: ${DOM.safeText(e.message)}`);
    }
  }
};
```

### AFTER

```javascript
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

    const playerId = await bridge.play(streamUrl, track.id);
    Store.Playback._currentPlayerId = playerId;
    document.title = `▶ ${track.title} - Firmium`;
  } catch (e) {
    if (currentToken === Store.Playback.getPlayToken()) {
      console.error('Playback exception:', e);
      DOM.render('npArtist', `Playback Error: ${DOM.safeText(e.message)}`);
    }
  }
};
```

**Changes:**
- Get bridge from Store instead of audio element
- Call `bridge.play()` instead of setting `audio.src` and `audio.play()`
- Store player ID for future control

---

## 3. Play/Pause Toggle

### BEFORE

```javascript
case 'play-toggle':
  if (!Store.Playback.getCurrentTrack()) return;
  if (audio.paused) audio.play().catch(() => {});
  else audio.pause();
  break;
```

### AFTER

```javascript
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
```

**Changes:**
- Use bridge methods instead of audio element methods
- Check state before resuming (audio might be stopped)

---

## 4. Volume Control

### BEFORE

```javascript
// Set volume
Store.Playback.setVolume(v);
if (audio) audio.volume = v;

// Get volume
DOM.el('volSlider').value = Store.Playback.getVolume();

// Event listener
DOM.el('volSlider')?.addEventListener('input', (e) => {
  Store.Playback.setVolume(e.target.value);
});
```

### AFTER

```javascript
// Set volume
async function setVolume(vol) {
  const bridge = Store.Audio.getBridge();
  if (bridge) {
    await bridge.setVolume(vol);
  }
  Store.Playback.setVolume(vol); // Also persist locally
}

// Get volume (on startup)
const savedVol = Number(SafeStorage.getItem('firmium_volume') ?? 0.8);
DOM.el('volSlider').value = savedVol;

// Event listener
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
  
  Store.Playback.setVolume(volume);
});
```

**Changes:**
- Use async bridge method
- Handle potential errors
- Keep local storage for persistence

---

## 5. Track End Handling

### BEFORE

```javascript
audio.addEventListener('ended', () => {
  if (Store.Playback.getRepeatOne()) {
    playAt(Store.Playback.getQueueIdx());
  } else if (Store.Playback.getQueueIdx() < Store.Playback.getQueue().length - 1) {
    playAt(Store.Playback.getQueueIdx() + 1);
  } else if (Store.Playback.getRepeatAll()) {
    playAt(0);
  } else {
    document.title = 'Firmium';
  }
});
```

### AFTER

```javascript
// In Store.Audio.init():
_bridge.on('finished', () => {
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
```

**Changes:**
- Move logic from `audio.ended` event to `bridge.on('finished')`
- Same logic, different event source

---

## 6. Pause State Tracking

### BEFORE

```javascript
audio.addEventListener('play', () => { 
  DOM.render('playBtn', '⏸'); 
});

audio.addEventListener('pause', () => { 
  DOM.render('playBtn', '▶'); 
});
```

### AFTER

```javascript
// In Store.Audio.init():
_bridge.on('statechange', (state) => {
  const isPlaying = state === 'playing';
  const playBtn = DOM.el('playBtn');
  if (playBtn) {
    playBtn.textContent = isPlaying ? '⏸' : '▶';
  }
});
```

**Changes:**
- Listen to state change events instead of play/pause events
- Update UI based on state

---

## 7. Seek / Time Update

### BEFORE

```javascript
audio.addEventListener('durationchange', () => { 
  DOM.render('durTime', formatDuration(audio.duration || 0)); 
});

audio.addEventListener('timeupdate', () => {
  if (Store.Playback.isSeeking()) return;
  const cur = audio.currentTime, dur = audio.duration || 0;
  DOM.el('seekBar').value = dur > 0 ? String((cur / dur) * 100) : '0';
  
  const currentSec = Math.floor(cur);
  if (currentSec !== Store.Playback.getLastSec()) {
    Store.Playback.setLastSec(currentSec);
    DOM.render('curTime', formatDuration(currentSec));
  }
});

DOM.el('seekBar')?.addEventListener('change', (e) => {
  audio.currentTime = (Number(e.target.value) / 100) * (audio.duration || 0);
  Store.Playback.setSeeking(false);
});
```

### AFTER

```javascript
// Get duration when starting playback
async function updateDuration(playerId) {
  const bridge = Store.Audio.getBridge();
  if (bridge) {
    const duration = await bridge.getDuration();
    if (duration) {
      DOM.render('durTime', formatDuration(duration));
    }
  }
}

// Note: Seeking is not supported in native backend yet
// This is a known limitation - can be added in future

// For now, disable seek bar or make it read-only
DOM.el('seekBar').disabled = true; // Or remove event listener

// Optional: Update time display periodically
setInterval(async () => {
  const bridge = Store.Audio.getBridge();
  if (bridge) {
    const duration = await bridge.getDuration();
    if (duration) {
      // Estimate current time based on state duration
      // (More complex without true currentTime access)
    }
  }
}, 1000);
```

**Changes:**
- Duration is fetched once, not from timeupdate event
- **Seeking is NOT YET SUPPORTED** (requires rodio source wrapper)
- Current time tracking would need a different approach

**Note:** Seeking can be added later by wrapping rodio sources with seek support. For now, it's a known limitation.

---

## 8. Error Handling

### BEFORE

```javascript
audio.addEventListener('error', () => {
  const err = audio.error;
  if (err && err.code === 4) {
    const currentTimeSave = audio.currentTime;
    audio.load(); 
    audio.currentTime = currentTimeSave;
    audio.play().catch(() => {});
  }
});
```

### AFTER

```javascript
// In Store.Audio.init():
_bridge.on('error', (msg) => {
  console.error('Audio error:', msg);
  DOM.render('npArtist', `Audio Error: ${DOM.safeText(msg)}`);
  
  // Could auto-retry here:
  // setTimeout(() => bridge.play(lastUrl, lastTrackId), 2000);
});
```

**Changes:**
- Errors are emitted as events
- Simpler error handling without error codes
- Can implement retry logic if desired

---

## 9. Teardown/Cleanup

### BEFORE

```javascript
const teardownApp = () => {
  const audio = Store.Playback.getAudio();
  if (audio) {
    try { audio.pause(); } catch(e){}
    audio.removeAttribute('src');
    audio.load();
  }
  // ... rest of cleanup
};
```

### AFTER

```javascript
const teardownApp = () => {
  const bridge = Store.Audio.getBridge();
  if (bridge) {
    bridge.destroy(); // Stops monitoring, cleans up sessions
  }
  
  // Remove old audio element if exists
  const oldAudio = DOM.el('audioEl');
  if (oldAudio) oldAudio.remove();
  
  // ... rest of cleanup
};
```

**Changes:**
- Call `bridge.destroy()` instead of manipulating audio element
- Cleaner abstraction

---

## 10. Store Updates

### BEFORE

```javascript
Store.Playback = (() => {
  let _audio = null;
  // ...
  return {
    initAudio: (el) => { _audio = el; _audio.volume = _volume; },
    getAudio: () => _audio,
    // ...
  };
})();
```

### AFTER

```javascript
Store.Audio = (() => {
  let _bridge = null;
  
  return {
    init: () => {
      _bridge = new AudioBridge();
      // Setup event listeners...
      return _bridge;
    },
    getBridge: () => _bridge,
  };
})();

// Simplified Store.Playback (remove audio-specific code)
Store.Playback = (() => {
  let _queue = [], _queueIdx = -1, _playToken = 0;
  let _volume = Number(SafeStorage.getItem('firmium_volume') ?? 0.8);
  // ... rest unchanged, but no _audio or _seeking
  
  return {
    getQueue: () => _queue,
    // ... everything else
  };
})();
```

**Changes:**
- Create separate `Store.Audio` namespace
- `Store.Playback` becomes simpler (no audio element)
- Better separation of concerns

---

## Summary of Changes

| Feature | Before | After |
|---------|--------|-------|
| **Audio element** | HTML5 `<audio>` | Tauri IPC commands |
| **Play** | `audio.play()` | `bridge.play(url, id)` |
| **Pause** | `audio.pause()` | `bridge.pause()` |
| **Volume** | `audio.volume = 0.8` | `bridge.setVolume(0.8)` |
| **State tracking** | Event listeners | Event emitter |
| **Seeking** | `audio.currentTime` | ❌ Not yet supported |
| **Duration** | `audio.duration` | `bridge.getDuration()` |
| **Cleanup** | Element manipulation | `bridge.destroy()` |

---

## Testing After Migration

1. **Play a track** - Audio should play through system speakers
2. **Pause/resume** - Button should respond
3. **Change volume** - System audio volume should change
4. **Skip tracks** - Next/previous should work
5. **Play from search** - Search → play should work
6. **Repeat modes** - Repeat one/all should work
7. **Memory** - RAM usage should be stable, lower than before
8. **CPU** - CPU usage should drop 30-50%

