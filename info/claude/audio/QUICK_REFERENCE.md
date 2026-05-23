# Native Audio Backend - Quick Reference Card

## 🚀 Setup Checklist (Copy & Paste)

```bash
# 1. Copy backend files
cp Cargo.toml src-tauri/Cargo.toml
cp audio.rs src-tauri/src/audio.rs
cp main.rs src-tauri/src/main.rs

# 2. Copy frontend file
cp audio-bridge.js src/audio-bridge.js

# 3. Build & test
npm run tauri dev

# 4. If errors, check
cargo check
```

---

## 📝 app.js Changes (Main Points)

### 1. Add at top (with Store definition):
```javascript
Store.Audio = (() => {
  let _bridge = null;
  return {
    init: () => {
      _bridge = new AudioBridge();
      _bridge.on('statechange', (state) => {
        DOM.el('playBtn').textContent = state === 'playing' ? '⏸' : '▶';
      });
      _bridge.on('finished', () => {
        // Handle track end...
      });
      return _bridge;
    },
    getBridge: () => _bridge,
  };
})();
```

### 2. Update playAt() function:
```javascript
const playAt = async (idx) => {
  const bridge = Store.Audio.getBridge();
  // ... setup code ...
  const playerId = await bridge.play(streamUrl, track.id);
  // ... done!
};
```

### 3. Update play toggle:
```javascript
case 'play-toggle': {
  const bridge = Store.Audio.getBridge();
  const state = await bridge.getState();
  await (state === 'paused' ? bridge.resume() : bridge.pause());
  break;
}
```

### 4. Update volume slider:
```javascript
DOM.el('volSlider')?.addEventListener('input', async (e) => {
  const bridge = Store.Audio.getBridge();
  await bridge.setVolume(e.target.value);
  Store.Playback.setVolume(e.target.value);
});
```

### 5. Update teardown:
```javascript
const teardownApp = () => {
  Store.Audio.getBridge()?.destroy();
  // ... rest ...
};
```

### 6. Delete these:
- `audio.addEventListener('play', ...)` 
- `audio.addEventListener('pause', ...)`
- `audio.addEventListener('ended', ...)`
- `audio.addEventListener('timeupdate', ...)`
- `DOM.el('seekBar')?.addEventListener('change', ...)`
- Old `<audio>` element creation code

---

## 🎯 Frontend AudioBridge API

```javascript
// Get bridge instance
const bridge = Store.Audio.getBridge();

// Play
await bridge.play(url, trackId)         // → playerId

// Control
await bridge.pause()
await bridge.resume()
await bridge.stop()

// Check state
const state = await bridge.getState()   // "playing" | "paused" | "stopped"
const finished = await bridge.isFinished()
const duration = await bridge.getDuration()  // seconds or null

// Volume
await bridge.setVolume(0.8)             // 0.0 to 1.0
const vol = await bridge.getVolume()

// Events
bridge.on('statechange', (state) => {})
bridge.on('finished', () => {})
bridge.on('volumechange', (vol) => {})
bridge.on('error', (msg) => {})

// Cleanup
bridge.destroy()
```

---

## 🔧 Backend Tauri Commands

```javascript
// All invoked via: window.__TAURI__.invoke('command', { args })

// Audio
await __TAURI__.invoke('play_stream', 
  { stream_url: '...', track_id: '...' })
await __TAURI__.invoke('pause_playback', { player_id: '...' })
await __TAURI__.invoke('resume_playback', { player_id: '...' })
await __TAURI__.invoke('stop_playback', { player_id: '...' })
await __TAURI__.invoke('set_volume', { player_id: '...', volume: 0.8 })
const vol = await __TAURI__.invoke('get_volume', { player_id: '...' })
const state = await __TAURI__.invoke('get_playback_state', { player_id: '...' })
const done = await __TAURI__.invoke('is_playback_finished', { player_id: '...' })
const dur = await __TAURI__.invoke('get_track_duration', { player_id: '...' })
const devices = await __TAURI__.invoke('list_audio_devices')

// Existing (unchanged)
await __TAURI__.invoke('save_password', { service, user, pass })
await __TAURI__.invoke('get_password', { service, user })
await __TAURI__.invoke('delete_password', { service, user })
const path = await __TAURI__.invoke('cache_cover', { id, server_url })
const info = await __TAURI__.invoke('get_machine_info')
```

---

## ⚙️ Build Commands

```bash
# Development with hot reload
npm run tauri dev

# Production build
npm run tauri build

# Arch Linux PKGBUILD
npm run build:arch

# Just check Rust code
cd src-tauri && cargo check

# Build just the Rust part
cargo build --release
```

---

## 🐛 Troubleshooting

| Problem | Fix |
|---------|-----|
| **No sound** | Check system volume, test speakers with OS sound |
| **Compilation error** | Run `cargo update`, check Rust: `rustc --version` (need 1.70+) |
| **"Player not found"** | Bridge not initialized, check Store.Audio.init() called |
| **High CPU** | Audio format incompatible, try transcode on Subsonic |
| **Volume not changing** | Restart app, check browser console for errors |
| **Frozen UI** | Shouldn't happen (async), check for sync code in frontend |

---

## 📊 Performance Goals

| Metric | Target | Your Result |
|--------|--------|------------|
| CPU (single track) | 3-5% | ___ % |
| Memory | ~5-10 MB | ___ MB |
| Latency | 50-100ms | ___ ms |
| UI Response | <100ms | ___ ms |

---

## 🔗 Important Files

| File | Purpose | Edit? |
|------|---------|-------|
| `Cargo.toml` | Rust dependencies | Replace entirely |
| `audio.rs` | Rust audio engine | Copy as-is |
| `main.rs` | Tauri app init | Replace entirely |
| `audio-bridge.js` | Frontend IPC wrapper | Copy as-is |
| `app.js` | Your app logic | Modify (see guide) |
| `index.html` | UI layout | Usually no change |
| `style.css` | Styling | No change |

---

## 📚 Documentation Files

| Doc | Use for | Read |
|-----|---------|------|
| `INTEGRATION_GUIDE.md` | Full setup walkthrough | 📖 Start here |
| `MIGRATION_GUIDE.md` | Before/after code examples | 🔍 Code review |
| `API_REFERENCE.md` | Method parameters & returns | 📋 Lookup |
| `app-js-updates.md` | app.js code snippets | ✏️ Paste code |

---

## ✨ Features Checklist

- [x] High-quality native audio playback
- [x] Low CPU usage
- [x] Event-driven frontend
- [x] Volume control
- [x] Play/pause/stop
- [x] State tracking
- [x] Error handling
- [ ] Seeking (future)
- [ ] Multiple devices UI (future)
- [ ] Equalizer (future)

---

## 🎯 Test Steps

1. **Start dev server**: `npm run tauri dev`
2. **Login**: Enter Subsonic credentials
3. **Play track**: Click album → click track → should hear audio
4. **Test controls**: 
   - Pause → Resume (button should toggle)
   - Volume slider → should change audio level
   - Skip (next/prev) → should change track
5. **Check performance**: 
   - Open system monitor
   - Note CPU/memory before
   - Play a few tracks
   - Note CPU/memory after
   - Should be lower! ✅

---

## 🆘 Getting Help

1. **Check logs**: Browser console (F12) + Tauri stderr
2. **Refer to**: `INTEGRATION_GUIDE.md` Troubleshooting
3. **Verify**: All files copied to right locations
4. **Re-read**: MIGRATION_GUIDE.md app.js changes
5. **Rebuild**: `cargo clean && npm run tauri dev`

---

## 🚀 Success Indicators

You'll know it's working when:

✅ Click track → hear audio  
✅ CPU usage drops 30-50%  
✅ No UI freezing during playback  
✅ Pause/resume responds instantly  
✅ Volume slider works smoothly  
✅ Memory stable during long sessions  

---

**Quick Tip**: If unsure, read `INTEGRATION_GUIDE.md` first. It has all the details and troubleshooting!

