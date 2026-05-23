/**
 * Audio Bridge — Frontend interface to the native Rust audio backend.
 *
 * Replaces the Web Audio API with Tauri IPC calls to the native rodio engine.
 * Provides an event-emitter pattern for playback state updates.
 *
 * Events emitted:
 *   'statechange' (state: string)  — 'loading' | 'playing' | 'paused' | 'stopped'
 *   'finished'    ()               — track played to completion
 *   'volumechange'(vol: number)    — volume was changed
 *   'error'       (msg: string)    — a playback error occurred
 *
 * Usage:
 *   const bridge = new AudioBridge();
 *   await bridge.play(streamUrl, trackId);
 *   bridge.on('finished', () => playNextTrack());
 */

// Safe global invocation extractor supporting Tauri v2 namespaces.
const tauriInvoke = (cmd, args) => {
  const invokeFn = (window.__TAURI__ && window.__TAURI__.core)
    ? window.__TAURI__.core.invoke
    : (window.__TAURI__ ? window.__TAURI__.invoke : null);

  if (!invokeFn) {
    throw new Error("Tauri core IPC namespace not detected. Is this running inside Tauri?");
  }
  return invokeFn(cmd, args);
};

class AudioBridge {
  constructor() {
    this.currentPlayerId = null;
    this.listeners = new Map();
    this.statusCheckInterval = null;
    this.lastKnownState = null;

    // Tracks whether audio has ever started playing in this session.
    // Guards against the monitoring loop seeing an empty sink BEFORE audio loads
    // and incorrectly emitting 'finished'.
    this._hasStartedPlaying = false;
  }

  // ── Event emitter ──────────────────────────────────────────────────────────

  /** Register a listener for an event. */
  on(event, callback) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, []);
    }
    this.listeners.get(event).push(callback);
  }

  /** Unregister a previously registered listener. */
  off(event, callback) {
    if (this.listeners.has(event)) {
      const callbacks = this.listeners.get(event);
      const idx = callbacks.indexOf(callback);
      if (idx > -1) callbacks.splice(idx, 1);
    }
  }

  /** Emit an event to all registered listeners. */
  emit(event, data) {
    if (this.listeners.has(event)) {
      this.listeners.get(event).forEach(cb => cb(data));
    }
  }

  // ── Playback controls ──────────────────────────────────────────────────────

  /**
   * Start playback of a stream.
   *
   * @param {string} streamUrl - HTTP/HTTPS audio stream URL
   * @param {string} trackId   - Application track identifier
   * @returns {Promise<string>} Player ID for this session
   */
  async play(streamUrl, trackId) {
    try {
      // Stop any existing session cleanly before starting a new one.
      if (this.currentPlayerId) {
        await this.stop();
      }

      const playerId = await tauriInvoke('play_stream', {
        streamUrl,
        trackId
      });

      this.currentPlayerId = playerId;
      this._hasStartedPlaying = false; // Reset — audio is still loading.
      this.lastKnownState = 'loading';
      this.startStatusMonitoring();
      this.emit('statechange', 'loading');

      return playerId;
    } catch (err) {
      this.emit('error', `Playback failed: ${err}`);
      throw err;
    }
  }

  /** Pause current playback. No-op if nothing is playing. */
  async pause() {
    if (!this.currentPlayerId) return;

    try {
      await tauriInvoke('pause_playback', { playerId: this.currentPlayerId });
      this.lastKnownState = 'paused';
      this.emit('statechange', 'paused');
    } catch (err) {
      this.emit('error', `Pause failed: ${err}`);
    }
  }

  /** Resume a paused session. No-op if nothing is paused. */
  async resume() {
    if (!this.currentPlayerId) return;

    try {
      await tauriInvoke('resume_playback', { playerId: this.currentPlayerId });
      this.lastKnownState = 'playing';
      this._hasStartedPlaying = true;
      this.emit('statechange', 'playing');
    } catch (err) {
      this.emit('error', `Resume failed: ${err}`);
    }
  }

  /**
   * Stop playback completely and remove the session.
   * Monitoring is cancelled before the stop IPC call so the interval
   * cannot fire between the Rust session being removed and JS cleanup.
   */
  async stop() {
    if (!this.currentPlayerId) return;

    // Cancel monitoring first to prevent the interval from seeing
    // the just-removed session and emitting 'finished' incorrectly.
    this.stopStatusMonitoring();

    const idToStop = this.currentPlayerId;
    this.currentPlayerId = null;
    this.lastKnownState = 'stopped';
    this._hasStartedPlaying = false;

    try {
      await tauriInvoke('stop_playback', { playerId: idToStop });
      this.emit('statechange', 'stopped');
    } catch (err) {
      // Even if the Rust side errors (e.g. already cleaned up), JS state is clear.
      this.emit('error', `Stop failed: ${err}`);
    }
  }

  // ── Volume ─────────────────────────────────────────────────────────────────

  /** Set playback volume. Value is clamped to [0.0, 1.0]. */
  async setVolume(volume) {
    if (!this.currentPlayerId) return;

    const normalized = Math.max(0, Math.min(1, Number(volume)));
    try {
      await tauriInvoke('set_volume', {
        playerId: this.currentPlayerId,
        volume: normalized
      });
      this.emit('volumechange', normalized);
    } catch (err) {
      this.emit('error', `Volume change failed: ${err}`);
    }
  }

  async getVolume() {
    if (!this.currentPlayerId) return null;

    try {
      return await tauriInvoke('get_volume', { playerId: this.currentPlayerId });
    } catch (err) {
      console.error('Get volume failed:', err);
      return null;
    }
  }

  /** Seek to a position in the track. Position is in seconds. */
  async seek(position) {
    if (!this.currentPlayerId) return;

    const pos = Math.max(0, Number(position) || 0);
    try {
      await tauriInvoke('seek_position', {
        playerId: this.currentPlayerId,
        position: pos
      });
    } catch (err) {
      console.warn('Seek not supported for this stream:', err);
    }
  }

  /** Get current playback position in seconds. */
  async getCurrentPosition() {
    if (!this.currentPlayerId) return 0;

    try {
      return await tauriInvoke('get_current_position', {
        playerId: this.currentPlayerId
      });
    } catch (err) {
      console.error('Get position failed:', err);
      return 0;
    }
  }

  // ── State queries ──────────────────────────────────────────────────────────

  /**
   * Check if playback has finished.
   *
   * Returns false while the session is in the 'loading' state to prevent the
   * monitoring loop from treating a buffering track as a completed one.
   */
  async isFinished() {
    if (!this.currentPlayerId) return true;

    // Don't report finished while still in loading state.
    if (this.lastKnownState === 'loading') return false;

    try {
      return await tauriInvoke('is_playback_finished', {
        playerId: this.currentPlayerId
      });
    } catch (err) {
      console.error('Is-finished check failed:', err);
      return false;
    }
  }

  /** Get track duration in seconds if available (populated after decode). */
  async getDuration() {
    if (!this.currentPlayerId) return null;

    try {
      return await tauriInvoke('get_track_duration', {
        playerId: this.currentPlayerId
      });
    } catch (err) {
      console.error('Get duration failed:', err);
      return null;
    }
  }

  /**
   * Get current playback state string.
   * Returns 'loading' | 'playing' | 'paused' | 'stopped'.
   */
  async getState() {
    if (!this.currentPlayerId) return 'stopped';

    try {
      return await tauriInvoke('get_playback_state', {
        playerId: this.currentPlayerId
      });
    } catch (err) {
      console.error('Get state failed:', err);
      return this.lastKnownState || 'stopped';
    }
  }

  // ── Status monitoring ──────────────────────────────────────────────────────

  /**
   * Start polling the backend for state changes and track completion.
   *
   * Polls every 750ms (reduced from 500ms — the native audio engine updates
   * state no faster than this in practice, and this saves IPC round-trips).
   *
   * The monitoring guards against emitting 'finished' before audio has actually
   * started playing by checking `_hasStartedPlaying`.
   */
  startStatusMonitoring() {
    this.stopStatusMonitoring();

    this.statusCheckInterval = setInterval(async () => {
      if (!this.currentPlayerId) {
        this.stopStatusMonitoring();
        return;
      }

      try {
        // Poll current state from Rust.
        const currentState = await this.getState();

        // Once the Rust backend reports 'playing', latch the started flag.
        if (currentState === 'playing') {
          this._hasStartedPlaying = true;
        }

        // Only emit statechange if something actually changed.
        if (currentState !== this.lastKnownState) {
          this.lastKnownState = currentState;
          this.emit('statechange', currentState);
        }

        // Only check for completion after audio has actually started playing.
        // This prevents the empty pre-load sink from triggering 'finished'.
        if (this._hasStartedPlaying) {
          const finished = await this.isFinished();
          if (finished && this.lastKnownState !== 'finished') {
            this.lastKnownState = 'finished';
            this.stopStatusMonitoring();
            this.currentPlayerId = null;
            this.emit('finished');
          }
        }
      } catch (err) {
        console.error('Status monitoring error:', err);
      }
    }, 750);
  }

  /** Stop the status polling interval. */
  stopStatusMonitoring() {
    if (this.statusCheckInterval) {
      clearInterval(this.statusCheckInterval);
      this.statusCheckInterval = null;
    }
  }

  // ── Lifecycle ──────────────────────────────────────────────────────────────

  /** Release all resources. Call when the app is tearing down. */
  destroy() {
    this.stopStatusMonitoring();
    if (this.currentPlayerId) {
      this.stop().catch(() => {});
    }
    this.listeners.clear();
  }
}

// Expose globally for use in app.js (loaded as a plain script tag).
if (typeof window !== 'undefined') {
  window.AudioBridge = AudioBridge;
}