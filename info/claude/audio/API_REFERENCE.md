# Audio API Quick Reference

## Frontend: AudioBridge Methods

```javascript
const bridge = new AudioBridge();

// Playback Control
await bridge.play(streamUrl, trackId)           // Start playback → Promise<playerId>
await bridge.pause()                             // Pause
await bridge.resume()                            // Resume
await bridge.stop()                              // Stop & cleanup
await bridge.isFinished()                        // Check if track ended → Promise<bool>
await bridge.getState()                          // Get state → Promise<"playing"|"paused"|"stopped">

// Volume
await bridge.setVolume(0.8)                      // Set volume (0.0-1.0)
await bridge.getVolume()                         // Get volume → Promise<float>

// Metadata
await bridge.getDuration()                       // Get track duration → Promise<float|null>

// Events
bridge.on('statechange', (state) => {})          // 'playing', 'paused', 'stopped'
bridge.on('finished', () => {})                  // Track ended
bridge.on('volumechange', (vol) => {})           // Volume changed
bridge.on('error', (msg) => {})                  // Error occurred
bridge.off(event, callback)                      // Unregister listener

// Lifecycle
bridge.destroy()                                 // Cleanup & stop monitoring
```

---

## Backend: Tauri Commands

All commands are invoked via:
```javascript
await window.__TAURI__.invoke('command_name', { arg1, arg2, ... })
```

### Playback Commands

#### `play_stream`
**Arguments:**
- `stream_url` (string): HTTP/HTTPS audio stream URL
- `track_id` (string): Application track identifier

**Returns:** `string` - Unique player ID

**Example:**
```javascript
const playerId = await window.__TAURI__.invoke('play_stream', {
  stream_url: 'https://server.com/rest/stream.view?id=123&u=user&t=token&s=salt&c=app&f=json',
  track_id: 'track-123'
});
```

---

#### `pause_playback`
**Arguments:**
- `player_id` (string): Player ID from `play_stream`

**Returns:** `null` on success

```javascript
await window.__TAURI__.invoke('pause_playback', { player_id: 'uuid-here' });
```

---

#### `resume_playback`
**Arguments:**
- `player_id` (string): Player ID

**Returns:** `null` on success

```javascript
await window.__TAURI__.invoke('resume_playback', { player_id: 'uuid-here' });
```

---

#### `stop_playback`
**Arguments:**
- `player_id` (string): Player ID

**Returns:** `null` on success (removes session)

```javascript
await window.__TAURI__.invoke('stop_playback', { player_id: 'uuid-here' });
```

---

#### `set_volume`
**Arguments:**
- `player_id` (string): Player ID
- `volume` (float): 0.0 to 1.0

**Returns:** `null` on success

```javascript
await window.__TAURI__.invoke('set_volume', {
  player_id: 'uuid-here',
  volume: 0.8
});
```

---

#### `get_volume`
**Arguments:**
- `player_id` (string): Player ID

**Returns:** `float` - Current volume (0.0-1.0)

```javascript
const volume = await window.__TAURI__.invoke('get_volume', {
  player_id: 'uuid-here'
});
```

---

#### `get_playback_state`
**Arguments:**
- `player_id` (string): Player ID

**Returns:** `string` - One of: `"playing"`, `"paused"`, `"stopped"`

```javascript
const state = await window.__TAURI__.invoke('get_playback_state', {
  player_id: 'uuid-here'
});
```

---

#### `is_playback_finished`
**Arguments:**
- `player_id` (string): Player ID

**Returns:** `boolean` - True if track finished playing

```javascript
const finished = await window.__TAURI__.invoke('is_playback_finished', {
  player_id: 'uuid-here'
});
```

---

#### `get_track_duration`
**Arguments:**
- `player_id` (string): Player ID

**Returns:** `float | null` - Duration in seconds, or null if unavailable

```javascript
const duration = await window.__TAURI__.invoke('get_track_duration', {
  player_id: 'uuid-here'
});
// duration: 180.5 or null
```

---

#### `list_audio_devices`
**Arguments:** None

**Returns:** `Array<{name: string, default: boolean}>`

```javascript
const devices = await window.__TAURI__.invoke('list_audio_devices');
// [{ name: 'Default Output', default: true }]
```

---

## Existing Commands (Unchanged)

These commands still work as before:

```javascript
// Credentials
await window.__TAURI__.invoke('save_password', { service, user, pass })
await window.__TAURI__.invoke('get_password', { service, user })
await window.__TAURI__.invoke('delete_password', { service, user })

// Cover art
await window.__TAURI__.invoke('cache_cover', { id, server_url })

// System info
const info = await window.__TAURI__.invoke('get_machine_info')
// { cpu, gpu, distro, version, package_manager }
```

---

## Error Handling Pattern

All audio commands can throw errors. Proper error handling:

```javascript
try {
  await bridge.play(url, trackId);
} catch (error) {
  console.error('Playback failed:', error);
  // Show user-friendly error message
}

// Or use the error event
bridge.on('error', (msg) => {
  console.error('Audio error:', msg);
  alert(`Playback error: ${msg}`);
});
```

---

## State Machine Example

Recommended flow for play/pause/stop:

```javascript
const bridge = new AudioBridge();
let currentPlayerId = null;

// Play track
async function playTrack(url, trackId) {
  if (currentPlayerId) {
    await bridge.stop(); // Stop previous
  }
  currentPlayerId = await bridge.play(url, trackId);
}

// Toggle play/pause
async function togglePlayPause() {
  if (!currentPlayerId) return;
  
  const state = await bridge.getState();
  if (state === 'playing') {
    await bridge.pause();
  } else if (state === 'paused') {
    await bridge.resume();
  }
}

// Skip to next
async function skipNext() {
  if (currentPlayerId) {
    await bridge.stop();
  }
  // Load and play next track...
}

// Listen for completion
bridge.on('finished', async () => {
  currentPlayerId = null;
  // Play next track or stop
});
```

---

## Performance Tips

1. **Reuse player**: Don't create new sessions for every action
2. **Status polling**: AudioBridge checks status every 500ms (configurable)
3. **Volume persistence**: Save locally, sync to backend per-session
4. **Stream caching**: Subsonic server handles caching; Rust backend doesn't buffer full files
5. **Error recovery**: Always stop → play, not toggle on errors

---

## Compatibility Notes

- **Platform**: Linux (ALSA/PulseAudio), macOS (CoreAudio), Windows (WASAPI)
- **Audio formats**: FLAC, MP3, OGG, WAV (via `symphonia` library)
- **Streaming**: HTTP/HTTPS only (Subsonic standard)
- **Subsonic API**: Tested with Subsonic 6.1+, Navidrome

---

## Debugging

Enable verbose logging:

```javascript
// Frontend
bridge.on('error', (msg) => console.error('Audio error:', msg));
console.log('Player ID:', await bridge.play(...));
console.log('State:', await bridge.getState());

// Backend (Rust stderr)
// Check cargo output for decode errors, format issues
```

