// ── Constants ────────────────────────────────────────────────────────────────
// Max albums/songs fetched per list request (OpenSubsonic allows up to 500).
const API_PAGE_SIZE = 500;
// How many cover art blob URLs to keep in memory before evicting oldest.
const MAX_COVER_CACHE_SIZE = 150;
// Prevent runaway search queries from the input field.
const SEARCH_INPUT_MAX_LENGTH = 500;
// Max albums returned per search query.
const SEARCH_ALBUM_LIMIT = 40;
// Max songs returned per search query.
const SEARCH_SONG_LIMIT = 100;
// How many album track fetches to run in parallel when building "Play All" queue.
// Keeps the OpenSubsonic server from being flooded with simultaneous requests.
const PLAY_ALL_CONCURRENCY = 5;
// Keyring service name used for OS credential storage.
const KEYRING_SERVICE = 'firmium-desktop';
const DEFAULT_VOLUME = 0.8;

// ── Wikipedia API ─────────────────────────────────────────────────────────────
const WikiApi = {
  /**
   * Fetch a short biography and thumbnail for an artist from Wikipedia.
   * Returns null if no results are found or if the request is aborted.
   */
  getInfo: async (artistName, signal) => {
    try {
      const searchUrl = `https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch=${encodeURIComponent(artistName + ' music')}&utf8=&format=json&origin=*`;
      const searchRes = await fetch(searchUrl, { signal });
      const searchData = await searchRes.json();
      const title = searchData.query?.search?.[0]?.title;
      if (!title) return null;
      const summaryUrl = `https://en.wikipedia.org/api/rest_v1/page/summary/${encodeURIComponent(title)}`;
      const summaryRes = await fetch(summaryUrl, { signal });
      const summaryData = await summaryRes.json();
      return {
        extract: summaryData.extract,
        image: summaryData.thumbnail?.source || null
      };
    } catch {
      return null;
    }
  }
};

// ── SafeStorage ───────────────────────────────────────────────────────────────
// Wraps localStorage with try/catch. Warns on failure so silent data loss is
// surfaced in the developer console rather than swallowed completely.
const SafeStorage = {
  getItem: (key) => {
    try { return localStorage.getItem(key); } catch (e) {
      console.warn(`SafeStorage.getItem("${key}") failed:`, e);
      return null;
    }
  },
  setItem: (key, value) => {
    try { localStorage.setItem(key, value); } catch (e) {
      console.warn(`SafeStorage.setItem("${key}") failed — storage may be full or unavailable:`, e);
    }
  },
  removeItem: (key) => {
    try { localStorage.removeItem(key); } catch (e) {
      console.warn(`SafeStorage.removeItem("${key}") failed:`, e);
    }
  }
};

// ── App Logger ────────────────────────────────────────────────────────────────
// Writes timestamped log entries to app-logs.txt via the Rust backend.
// Intercepts console.log/warn/error so all dev output is persisted to disk.
const AppLogger = (() => {
  const _write = (level, ...args) => {
    const msg = args.map(a => {
      if (a instanceof Error) return `${a.name}: ${a.message}`;
      if (typeof a === 'object' && a !== null) {
        try { return JSON.stringify(a); } catch (_) { return String(a); }
      }
      return String(a);
    }).join(' ');
    const ts = new Date().toISOString();
    const entry = `[${ts}] [${level}] ${msg}`;
    try { tauriInvoke('write_log', { entry }); } catch (_) {}
  };
  return {
    info:  (...args) => _write('INFO',  ...args),
    warn:  (...args) => _write('WARN',  ...args),
    error: (...args) => _write('ERROR', ...args),
  };
})();

// Patch console so existing log calls are also persisted.
((() => {
  const _log   = console.log.bind(console);
  const _warn  = console.warn.bind(console);
  const _error = console.error.bind(console);
  console.log   = (...a) => { _log(...a);   AppLogger.info(...a);  };
  console.warn  = (...a) => { _warn(...a);  AppLogger.warn(...a);  };
  console.error = (...a) => { _error(...a); AppLogger.error(...a); };
}))();

// ── LRC format parser ─────────────────────────────────────────────────────────
// Converts an LRC timestamp "[mm:ss.xx]" or "[mm:ss.xxx]" to milliseconds.
const parseLrcTimestamp = (mm, ss, frac) => {
  const fracMs = frac.length === 2 ? parseInt(frac, 10) * 10 : parseInt(frac, 10);
  return (parseInt(mm, 10) * 60 + parseInt(ss, 10)) * 1000 + fracMs;
};

// Parse an LRC-format string into an array of { start: ms, value: string }.
// Lines without a valid timestamp (metadata tags like [ar:], [ti:]) are skipped.
const parseLrc = (lrcText) => {
  const lines = [];
  for (const raw of lrcText.split('\n')) {
    const m = raw.match(/^\[(\d{1,2}):(\d{2})\.(\d{2,3})\]\s*(.*)/);
    if (m) lines.push({ start: parseLrcTimestamp(m[1], m[2], m[3]), value: m[4] });
  }
  return lines.sort((a, b) => a.start - b.start);
};

// ── LRCLIB external lyrics provider ───────────────────────────────────────────
// lrclib.net — free, no API key, returns synced LRC lyrics.
// Sends artist name, track title, and duration to the public API.
const LrclibApi = {
  getLyrics: async (song) => {
    const params = new URLSearchParams({
      artist_name: song.artist,
      track_name:  song.title,
      duration:    String(Math.round(song.duration ?? 0))
    });
    const res = await fetch(`https://lrclib.net/api/get?${params}`, {
      headers: { 'Lrclib-Client': 'Firmium (https://github.com/fossisawesome/firmium)' }
    });
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`LRCLIB ${res.status}`);
    const data = await res.json();
    if (data.instrumental) return { lines: [{ start: 0, value: '♪ Instrumental ♪' }], synced: false };
    if (data.syncedLyrics) {
      const lines = parseLrc(data.syncedLyrics);
      if (lines.length) return { lines, synced: true };
    }
    if (data.plainLyrics) {
      return {
        lines: data.plainLyrics.split('\n').map(v => ({ start: 0, value: v })),
        synced: false
      };
    }
    return null;
  }
};

// ── Keyring helpers ───────────────────────────────────────────────────────────
// Credentials are stored in the OS keyring via the Rust backend, NOT in localStorage.
// localStorage is readable by any JS on the page and in plaintext on disk.
// Uses tauriInvoke defined in audio-bridge.js (loaded before this script).
const Keyring = {
  save: (user, pass) =>
    tauriInvoke('save_password', { service: KEYRING_SERVICE, user, pass }),
  load: (user) =>
    tauriInvoke('get_password', { service: KEYRING_SERVICE, user }),
  remove: (user) =>
    tauriInvoke('delete_password', { service: KEYRING_SERVICE, user }),
};

// ── Application State ─────────────────────────────────────────────────────────
const Store = {
  ServerInfo: (() => {
    let _extensions = null;
    return {
      // Called once per Api.fetch response when the server includes OpenSubsonic extensions.
      setExtensions: (ext) => { _extensions = Array.isArray(ext) ? ext : null; },
      getExtensions: () => _extensions,
      isOpenSubsonic: () => _extensions !== null,
      hasExtension: (name) => _extensions?.some(e => e.name === name) ?? false,
      clear: () => { _extensions = null; }
    };
  })(),

  Auth: (() => {
    let _server = null, _username = null, _password = null;
    return {
      setAuth: (s, u, p) => {
        _server = s ? String(s).trim().replace(/\/+$/, '') : null;
        _username = u;
        _password = p;
      },
      clearAuth: () => { _server = null; _username = null; _password = null; },
      isAuthed: () => Boolean(_server && _username && _password),
      getServer: () => _server,
      getUsername: () => _username,
      getQueryParams: async () => {
        if (!_username || !_password) return {};
        return tauriInvoke('generate_auth_params', { username: _username, password: _password });
      }
    };
  })(),

  UI: (() => {
    let _view = 'albums', _navHistory = [];
    return {
      getView: () => _view,
      setView: (v) => { _view = v; },
      pushNav: (fn) => { if (typeof fn === 'function') _navHistory.push(fn); },
      popNav: () => _navHistory.pop(),
      clearNav: () => { _navHistory = []; }
    };
  })(),

  Audio: (() => {
    let _bridge = null;
    let _positionInterval = null;
    let _isSeeking = false;
    let _crossfadeStarted = false;

    const _self = {
      init: () => {
        _bridge = new AudioBridge();

        _bridge.on('statechange', (state) => {
          const isPlaying = state === 'playing';
          const playBtn = DOM.el('playBtn');
          if (playBtn) playBtn.textContent = state === 'loading' ? '⏳' : (isPlaying ? '⏸' : '▶');

          if (isPlaying) {
            _self.startPositionTracking();
          } else {
            _self.stopPositionTracking();
          }
        });

        _bridge.on('finished', () => {
          const finishedTrack = Store.Playback.getCurrentTrack();
          if (finishedTrack) {
            Api.scrobble(finishedTrack.id, true);
          }

          if (Store.Playback.getRepeatOne()) {
            playAt(Store.Playback.getQueueIdx());
          } else if (Store.Playback.getQueueIdx() < Store.Playback.getQueue().length - 1) {
            playAt(Store.Playback.getQueueIdx() + 1);
          } else if (Store.Playback.getRepeatAll()) {
            playAt(0);
          } else {
            _self.stopPositionTracking();
            document.title = 'Firmium';
            const seekBar = DOM.el('seekBar');
            if (seekBar) { seekBar.value = 0; }
            const curTime = DOM.el('curTime');
            if (curTime) curTime.textContent = '0:00';
          }
        });

        _bridge.on('volumechange', (vol) => {
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

      _cachedDuration: null,

      startPositionTracking: () => {
        _self.stopPositionTracking();
        _self._cachedDuration = null;
        _crossfadeStarted = false;
        _positionInterval = setInterval(async () => {
          if (!_bridge || !Store.Playback.getCurrentTrack()) {
            _self.stopPositionTracking();
            return;
          }

          try {
            const position = await _bridge.getCurrentPosition();
            if (!_self._cachedDuration) {
              _self._cachedDuration = await _bridge.getDuration();
            }
            if (!_isSeeking) {
              DOM.el('curTime').textContent = formatDuration(position);
              const seekBar = DOM.el('seekBar');
              if (seekBar && _self._cachedDuration) {
                seekBar.max = _self._cachedDuration;
                seekBar.value = position;
              }
            }

            // Advance the active lyric line to match the current playback position.
            Lyrics.syncToPosition(position);

            // Crossfade trigger: start fading in the next track before this one ends.
            if (
              !_crossfadeStarted &&
              _self._cachedDuration &&
              Store.Playback.getCrossfadeEnabled() &&
              !Store.Playback.getRepeatOne()
            ) {
              const fadeSec = Store.Playback.getCrossfadeDuration();
              if (position >= _self._cachedDuration - fadeSec) {
                const queue = Store.Playback.getQueue();
                let nextIdx = Store.Playback.getQueueIdx() + 1;
                if (nextIdx >= queue.length && Store.Playback.getRepeatAll()) nextIdx = 0;
                if (nextIdx < queue.length) {
                  _crossfadeStarted = true;
                  crossfadeToNext(nextIdx);
                }
              }
            }
          } catch (err) {
            console.error('Position update failed:', err);
          }
        }, 250);
      },

      stopPositionTracking: () => {
        if (_positionInterval) {
          clearInterval(_positionInterval);
          _positionInterval = null;
        }
      },


      setSeeking: (seeking) => { _isSeeking = seeking; },

      clearBridge: () => {
        _bridge = null;
        _self.stopPositionTracking();
      }
    };
    return _self;
  })(),

  Playback: (() => {
    let _queue = [], _queueIdx = -1, _playToken = 0;
    let _volume = Number(SafeStorage.getItem('firmium_volume') ?? DEFAULT_VOLUME);
    let _repeatOne = false, _repeatAll = false;
    let _crossfadeEnabled = SafeStorage.getItem('firmium_crossfade') !== 'false';
    let _crossfadeDuration = Math.max(1, Math.min(12, Number(SafeStorage.getItem('firmium_crossfade_duration') ?? 5)));
    let _abortCtrl = null, _searchCtrl = null, _observer = null;
    const _covers = new Map(), _pendingCovers = new Map();

    return {
      getQueue: () => _queue,
      getQueueIdx: () => _queueIdx,
      getCurrentTrack: () => _queue[_queueIdx] || null,
      setQueue: (items, idx = 0) => {
        _queue = Array.isArray(items) ? items : [];
        _queueIdx = _queue.length ? idx : -1;
      },
      setQueueIdx: (idx) => { if (idx >= 0 && idx < _queue.length) _queueIdx = idx; },
      getPlayToken: () => _playToken,
      bumpToken: () => ++_playToken,
      getVolume: () => _volume,
      setVolume: (v) => {
        _volume = Math.max(0, Math.min(1, Number.isFinite(Number(v)) ? Number(v) : DEFAULT_VOLUME));
        SafeStorage.setItem('firmium_volume', String(_volume));
      },
      getRepeatOne: () => _repeatOne,
      setRepeatOne: (v) => { _repeatOne = Boolean(v); if (v) _repeatAll = false; },
      getRepeatAll: () => _repeatAll,
      setRepeatAll: (v) => { _repeatAll = Boolean(v); if (v) _repeatOne = false; },
      getCrossfadeEnabled: () => _crossfadeEnabled,
      setCrossfadeEnabled: (v) => {
        _crossfadeEnabled = Boolean(v);
        SafeStorage.setItem('firmium_crossfade', _crossfadeEnabled ? 'true' : 'false');
      },
      getCrossfadeDuration: () => _crossfadeDuration,
      setCrossfadeDuration: (v) => {
        _crossfadeDuration = Math.max(1, Math.min(12, Number(v) || 5));
        SafeStorage.setItem('firmium_crossfade_duration', String(_crossfadeDuration));
      },

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
          if (oldUrl?.startsWith('blob:')) { try { URL.revokeObjectURL(oldUrl); } catch (_) {} }
          _covers.delete(oldest);
        }
      },
      getCover: (id) => {
        const url = _covers.get(id) || null;
        if (url) { _covers.delete(id); _covers.set(id, url); } // LRU touch
        return url;
      },
      getPendingCover: (id) => _pendingCovers.get(id) || null,
      setPendingCover: (id, p) => { if (id && p) _pendingCovers.set(id, p); },
      clearPendingCover: (id) => { _pendingCovers.delete(id); },
      clearAllCache: () => {
        _covers.forEach(url => {
          if (url?.startsWith('blob:')) { try { URL.revokeObjectURL(url); } catch (_) {} }
        });
        _covers.clear();
        _pendingCovers.clear();
      }
    };
  })()
};

// ── Lyrics state ─────────────────────────────────────────────────────────────
// Manages the lyrics panel: open/closed state, parsed lyric lines, and active
// line index. Rendering is done by mutating DOM directly for performance since
// the interval fires every 250ms.
const Lyrics = (() => {
  let _open = false;
  // Array of { start: number (ms), value: string }
  let _lines = [];
  let _synced = false;
  let _activeIdx = -1;
  // Track ID currently loaded — used to discard stale async results.
  let _trackId = null;

  const _updateHighlights = () => {
    const body = DOM.el('lyricsContent');
    if (!body) return;
    body.querySelectorAll('.lyric-line').forEach((el, i) => {
      el.classList.remove('active', 'past', 'upcoming');
      if (i === _activeIdx) el.classList.add('active');
      else if (i < _activeIdx) el.classList.add('past');
      else el.classList.add('upcoming');
    });
  };

  const _scrollToActive = () => {
    const body = DOM.el('lyricsContent');
    if (!body || _activeIdx < 0) return;
    const els = body.querySelectorAll('.lyric-line');
    const el = els[_activeIdx];
    if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' });
  };

  return {
    isOpen: () => _open,

    // Toggle the panel open/closed, update button state, and return new state.
    toggle: () => {
      _open = !_open;
      DOM.el('lyricsPanel')?.classList.toggle('open', _open);
      DOM.el('lyricsBtn')?.classList.toggle('active', _open);
      return _open;
    },

    close: () => {
      _open = false;
      DOM.el('lyricsPanel')?.classList.remove('open');
      DOM.el('lyricsBtn')?.classList.remove('active');
    },

    // Show a status message (loading, error, unavailable).
    setStatus: (msg) => {
      const body = DOM.el('lyricsContent');
      if (body) body.innerHTML = `<div class="lyrics-status">${DOM.safeText(msg)}</div>`;
    },

    // Render parsed lyric lines. Synced lines get position-based highlighting;
    // unsynced lines are shown as plain text at full opacity.
    setLyrics: (lines, synced) => {
      _lines = lines || [];
      _synced = synced ?? false;
      _activeIdx = -1;
      const body = DOM.el('lyricsContent');
      if (!body) return;
      if (!_lines.length) {
        body.innerHTML = '<div class="lyrics-status">No lyrics available</div>';
        return;
      }
      if (!_synced) {
        body.innerHTML = _lines.map(l =>
          `<div class="lyric-line unsynced">${l.value ? DOM.safeText(l.value) : '&nbsp;'}</div>`
        ).join('');
        return;
      }
      body.innerHTML = _lines.map((l, i) => {
        const cls = 'lyric-line' + (l.value.trim() === '' ? ' empty-line' : '');
        const text = l.value.trim() === '' ? '· · ·' : DOM.safeText(l.value);
        return `<div class="${cls}" data-idx="${i}">${text}</div>`;
      }).join('');
    },

    getTrackId: () => _trackId,
    setTrackId: (id) => { _trackId = id; },

    clear: (msg = 'No track playing') => {
      _lines = [];
      _synced = false;
      _activeIdx = -1;
      _trackId = null;
      const body = DOM.el('lyricsContent');
      if (body) body.innerHTML = `<div class="lyrics-status">${DOM.safeText(msg)}</div>`;
    },

    // Called every 250ms from position tracking. Finds the last line whose
    // start time is <= current position and highlights it.
    syncToPosition: (positionSec) => {
      if (!_open || !_synced || !_lines.length) return;
      const posMs = positionSec * 1000;
      let newIdx = -1;
      for (let i = 0; i < _lines.length; i++) {
        if (_lines[i].start <= posMs) newIdx = i;
        else break;
      }
      if (newIdx !== _activeIdx) {
        _activeIdx = newIdx;
        _updateHighlights();
        if (_activeIdx >= 0) _scrollToActive();
      }
    }
  };
})();

// ── OpenSubsonic URL builder ───────────────────────────────────────────────────
const OpenSubsonicRouter = {
  buildUrl: async (action, params = {}) => {
    const server = Store.Auth.getServer();
    if (!server) return '';
    const url = new URL(`${server}/rest/${action}`);
    const combined = { ...await Store.Auth.getQueryParams(), ...params };
    Object.entries(combined).forEach(([k, v]) => {
      if (v !== null && v !== undefined) url.searchParams.append(k, String(v));
    });
    return url.toString();
  }
};

// ── OpenSubsonic data mappers ─────────────────────────────────────────────────
const OpenSubsonicMapper = {
  mapAlbum: (a) => ({
    id: a.id,
    name: a.name ?? a.title ?? 'Unknown Album',
    // displayArtist is the OpenSubsonic multi-artist display field; fall back to artist.
    albumArtist: a.displayArtist ?? a.artist ?? 'Unknown Artist',
    coverArtId: a.coverArt,
    songCount: a.songCount,
    // releaseTypes (array) is the OpenSubsonic field; releaseType (string) is the legacy fallback.
    releaseType: a.releaseTypes?.[0] ?? a.releaseType,
    genres: a.genres,
    year: a.year,
    isCompilation: a.isCompilation ?? false
  }),
  mapArtist: (a) => ({
    id: a.id,
    name: a.name ?? 'Unknown Artist',
    albumCount: a.albumCount ?? 0
  }),
  mapSong: (s) => ({
    id: s.id,
    title: s.title ?? 'Unknown Track',
    // displayArtist is the OpenSubsonic multi-artist display field; fall back to artist.
    artist: s.displayArtist ?? s.artist ?? 'Unknown Artist',
    duration: s.duration ?? 0,
    trackNumber: s.track,
    coverArtId: s.coverArt,
    replayGain: s.replayGain,
    bpm: s.bpm,
    comment: s.comment,
    genres: s.genres
  })
};

// ── OpenSubsonic API layer ────────────────────────────────────────────────────
const Api = {
  fetch: async (action, params = {}, signal = null) => {
    const url = await OpenSubsonicRouter.buildUrl(action, params);
    const res = await fetch(url, signal ? { signal } : {});
    if (res.status === 401) { if (Store.Auth.isAuthed()) teardownApp(); throw new Error('Session Expired'); }
    if (!res.ok) throw new Error(`HTTP Error ${res.status}`);
    const json = await res.json();
    const responseObj = json['subsonic-response'];
    if (!responseObj) throw new Error('Malformed API response');
    // Detect OpenSubsonic server — the extensions array is present on every response.
    if (responseObj.openSubsonicExtensions !== undefined) {
      Store.ServerInfo.setExtensions(responseObj.openSubsonicExtensions);
    }
    if (responseObj.status === 'failed') throw new Error(responseObj.error?.message ?? 'Engine error');
    return responseObj;
  },
  getAlbums: async (sig) => {
    const d = await Api.fetch('getAlbumList2', { type: 'alphabeticalByName', size: API_PAGE_SIZE }, sig);
    return (d.albumList2?.album ?? []).map(OpenSubsonicMapper.mapAlbum);
  },
  getArtists: async (sig) => {
    const d = await Api.fetch('getArtists', {}, sig);
    const container = [];
    if (d.artists?.index) d.artists.index.forEach(i => { if (Array.isArray(i.artist)) container.push(...i.artist); });
    return container.map(OpenSubsonicMapper.mapArtist);
  },
  getAlbumTracks: async (id, sig) => {
    const d = await Api.fetch('getAlbum', { id }, sig);
    const a = d.album ?? {};
    return {
      tracks: (a.song ?? []).map(OpenSubsonicMapper.mapSong),
      albumName: a.name ?? a.title ?? 'Unknown Album',
      albumArtist: a.displayArtist ?? a.artist ?? 'Unknown Artist',
      coverArtId: a.coverArt
    };
  },
  getArtistDetails: async (id, sig) => {
    const d = await Api.fetch('getArtist', { id }, sig);
    return {
      name: d.artist?.name ?? 'Unknown Artist',
      albums: (d.artist?.album ?? []).map(OpenSubsonicMapper.mapAlbum)
    };
  },
  search: async (query, sig) => {
    const d = await Api.fetch('search3', { query, albumCount: SEARCH_ALBUM_LIMIT, songCount: SEARCH_SONG_LIMIT }, sig);
    return {
      songs: (d.searchResult3?.song ?? []).map(OpenSubsonicMapper.mapSong),
      albums: (d.searchResult3?.album ?? []).map(OpenSubsonicMapper.mapAlbum)
    };
  },
  scrobble: (id, submission, time = Date.now()) => {
    OpenSubsonicRouter.buildUrl('scrobble', { id, submission: String(submission), time: String(time) }).then(url => {
      if (!url) return;
      fetch(url)
        .then(async r => {
          const json = await r.json().catch(() => null);
          const resp = json?.['subsonic-response'];
          if (!r.ok || resp?.status === 'failed') {
            console.error(`Scrobble failed (HTTP ${r.status}):`, resp?.error ?? json);
          }
        })
        .catch(e => console.error('Scrobble network error:', e));
    });
  },

  // Fetch lyrics for a song. Priority order:
  //   1. OpenSubsonic getLyricsBySongId (synced preferred, plain as fallback)
  //   2. Legacy Subsonic getLyrics (plain text)
  //   3. LRCLIB external API (synced LRC if available, plain otherwise)
  //      — only if firmium_lrclib setting is not disabled.
  // Returns { lines: [{start: ms, value: string}], synced: bool } or null.
  getLyrics: async (song) => {
    // OpenSubsonic structured lyrics (synced preferred over unsynced).
    try {
      const d = await Api.fetch('getLyricsBySongId', { id: song.id });
      const list = d.lyricsList?.structuredLyrics ?? [];
      const best = list.find(l => l.synced) || list[0];
      if (best && best.line?.length) {
        const offset = best.offset ?? 0;
        return {
          lines: best.line.map(l => ({ start: (l.start ?? 0) + offset, value: l.value ?? '' })),
          synced: best.synced ?? false
        };
      }
    } catch (_) {}
    // Legacy getLyrics — plain text, no timestamps.
    try {
      const d = await Api.fetch('getLyrics', { artist: song.artist, title: song.title });
      const lyr = d.lyrics;
      if (lyr?.value?.trim()) {
        return {
          lines: lyr.value.split('\n').map(v => ({ start: 0, value: v })),
          synced: false
        };
      }
    } catch (_) {}
    // External fallback: LRCLIB (sends artist + title + duration to lrclib.net).
    if (SafeStorage.getItem('firmium_lrclib') !== 'false') {
      try {
        const result = await LrclibApi.getLyrics(song);
        if (result) return result;
      } catch (_) {}
    }
    return null;
  },

};

// ── Utilities ──────────────────────────────────────────────────────────────────

/** Format a duration in seconds as M:SS. */
const formatDuration = (secs) => {
  const s = Number(secs);
  if (isNaN(s) || s <= 0) return '0:00';
  const m = Math.floor(s / 60), r = Math.floor(s % 60);
  return `${m}:${r < 10 ? '0' : ''}${r}`;
};

/**
 * Run async tasks with a concurrency limit.
 * Used for "Play All" to avoid flooding the OpenSubsonic server with parallel requests.
 *
 * @param {Array}    items     - Items to process
 * @param {number}   limit     - Max simultaneous tasks
 * @param {Function} asyncFn   - Async function receiving each item
 * @returns {Promise<Array>}   - Results in original order
 */
const pooledMap = async (items, limit, asyncFn) => {
  const results = new Array(items.length);
  let nextIdx = 0;

  const worker = async () => {
    while (nextIdx < items.length) {
      const idx = nextIdx++;
      results[idx] = await asyncFn(items[idx]);
    }
  };

  // Spawn `limit` workers that each pull from the shared queue.
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
  return results;
};

// ── Cover art loading ──────────────────────────────────────────────────────────

/**
 * Load a cover image into an img element, using an in-memory blob URL cache.
 * Deduplicates in-flight requests for the same cover ID.
 */
const loadImage = async (img, coverId, signal) => {
  if (!img || !coverId) return;
  const cached = Store.Playback.getCover(coverId);
  if (cached) { img.src = cached; return; }

  // Deduplicate: if a fetch is already in flight for this cover, share the promise.
  let promise = Store.Playback.getPendingCover(coverId);
  if (!promise) {
    promise = (async () => {
      const url = await OpenSubsonicRouter.buildUrl('getCoverArt', { id: coverId });
      const res = await fetch(url, { signal });
      if (!res.ok) throw new Error('Cover art unavailable');
      const blob = await res.blob();
      const objUrl = URL.createObjectURL(blob);
      Store.Playback.addCover(coverId, objUrl);
      return objUrl;
    })();
    Store.Playback.setPendingCover(coverId, promise);
  }

  try {
    const finalUrl = await promise;
    if (finalUrl && !signal?.aborted) img.src = finalUrl;
  } catch (e) {
    if (e.name !== 'AbortError') console.error('Cover art load error:', e);
  } finally {
    Store.Playback.clearPendingCover(coverId);
  }
};

/**
 * Set up IntersectionObserver to lazy-load cover art images as they scroll
 * into view. Only observes elements with class 'lazy-art'.
 */
const observeLazyCovers = (container) => {
  Store.Playback.clearObserver();
  const ctrl = Store.Playback.getActiveCtrl();
  const observer = new IntersectionObserver((entries) => {
    entries.forEach(e => {
      if (e.isIntersecting && e.target.dataset.coverId) {
        observer.unobserve(e.target);
        loadImage(e.target, e.target.dataset.coverId, ctrl?.signal);
      }
    });
  }, { root: container, rootMargin: '100px' });
  Store.Playback.setObserver(observer);
  container.querySelectorAll('.lazy-art').forEach(img => observer.observe(img));
};

// ── DOM helpers ────────────────────────────────────────────────────────────────
const DOM = {
  el: (id) => document.getElementById(id),
  render: (id, html) => { const el = DOM.el(id); if (el) el.innerHTML = html; },
  /** Escape HTML special characters to prevent XSS when inserting into innerHTML. */
  safeText: (str) => String(str ?? '').replace(/[&<>"']/g, m => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#039;'
  }[m])),

  createAlbumCard: (album) => `
    <div class="album-row" data-action="load-album" data-id="${DOM.safeText(album.id)}">
      <div class="album-art-sm">
        ${album.coverArtId
          ? `<img class="lazy-art" data-cover-id="${DOM.safeText(album.coverArtId)}" alt="">`
          : '<div class="no-art">♪</div>'}
      </div>
      <div class="album-info">
        <div class="album-title">${DOM.safeText(album.name)}</div>
        <div class="album-artist">${DOM.safeText(album.albumArtist)}</div>
      </div>
    </div>`,

  createArtistCard: (artist) => `
    <div class="artist-row" data-action="load-artist" data-id="${DOM.safeText(artist.id)}">
      <div class="artist-info">
        <div class="artist-name">${DOM.safeText(artist.name)}</div>
        <div class="artist-album-count">${Number(artist.albumCount)} albums</div>
      </div>
    </div>`,

  createTrackCard: (track, idx) => `
    <div class="track-row" data-action="play-track" data-index="${idx}" data-id="${DOM.safeText(track.id)}">
      <div class="track-num">${DOM.safeText(track.trackNumber ?? (idx + 1))}</div>
      <div class="track-thumb">${track.coverArtId
        ? `<img class="lazy-art" data-cover-id="${DOM.safeText(track.coverArtId)}" alt="">`
        : ''}</div>
      <div class="track-info">
        <div class="track-title">${DOM.safeText(track.title)}</div>
        <div class="track-artist">${DOM.safeText(track.artist)}</div>
      </div>
      <div class="track-duration">${formatDuration(track.duration)}</div>
    </div>`
};

// ── Theme ─────────────────────────────────────────────────────────────────────
const applyTheme = (theme) => {
  if (!theme || theme === 'firmium') {
    document.documentElement.removeAttribute('data-theme');
  } else {
    document.documentElement.setAttribute('data-theme', theme);
  }
};

// ── App lifecycle ──────────────────────────────────────────────────────────────

/**
 * Full teardown: stop audio, abort all pending requests, clear all state,
 * return to the login screen.
 */
const teardownApp = () => {
  const bridge = Store.Audio.getBridge();
  if (bridge) {
    bridge.destroy();
    Store.Audio.clearBridge(); // Null out the reference so stale callbacks can't use it.
  }

  Store.Playback.abortActive();
  Store.Playback.abortSearch();
  Store.Playback.clearObserver();
  Store.Playback.clearAllCache();
  Store.Auth.clearAuth();
  Store.ServerInfo.clear();
  Store.UI.clearNav();

  Lyrics.close();
  Lyrics.clear();

  DOM.el('setup')?.classList.remove('hidden');
  DOM.el('app')?.classList.add('hidden');
  DOM.render('setupError', '');
  document.title = 'Firmium';
};

// ── Playback ───────────────────────────────────────────────────────────────────

/**
 * Crossfade from the current track into the track at nextIdx.
 *
 * Called automatically by position tracking when the playing track is within
 * crossfadeDuration seconds of its end. Updates all UI and queue state, then
 * hands off to AudioBridge.startCrossfadeIn for the volume ramp.
 */
const crossfadeToNext = async (nextIdx) => {
  const bridge = Store.Audio.getBridge();
  if (!bridge) return;

  const currentTrack = Store.Playback.getCurrentTrack();
  if (currentTrack) Api.scrobble(currentTrack.id, true);

  Store.Playback.setQueueIdx(nextIdx);
  const nextTrack = Store.Playback.getCurrentTrack();
  if (!nextTrack) return;

  updateNowPlaying(nextTrack);
  highlightCurrentTrack();

  const currentToken = Store.Playback.bumpToken();

  try {
    const streamUrl = await OpenSubsonicRouter.buildUrl('stream', { id: nextTrack.id });
    if (currentToken !== Store.Playback.getPlayToken()) return;

    const fadeDurationMs = Store.Playback.getCrossfadeDuration() * 1000;
    await bridge.startCrossfadeIn(streamUrl, nextTrack.id, Store.Playback.getVolume(), fadeDurationMs);

    Api.scrobble(nextTrack.id, false);
    document.title = `▶ ${nextTrack.title} - Firmium`;
  } catch (e) {
    console.error('Crossfade error:', e);
  }
};

/** Highlight the currently playing track row in the list panel. */
const highlightCurrentTrack = () => {
  const current = Store.Playback.getCurrentTrack();
  const currentId = current ? String(current.id) : null;
  const panel = DOM.el('listPanel');
  if (panel) {
    panel.querySelectorAll('.track-row').forEach(r => {
      r.classList.toggle('playing', currentId !== null && r.dataset.id === currentId);
    });
  }
};

/**
 * Play the track at the given queue index.
 *
 * After starting the stream, applies the saved volume immediately so the
 * native sink doesn't blast at rodio's default 1.0 before the user touches
 * the slider.
 */
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
    const streamUrl = await OpenSubsonicRouter.buildUrl('stream', { id: track.id });
    if (currentToken !== Store.Playback.getPlayToken()) return; // Superseded by newer play request.

    await bridge.play(streamUrl, track.id);

    Api.scrobble(track.id, false);

    // Apply the stored volume to the new sink immediately.
    // Without this, the sink starts at rodio's default (1.0) until the slider moves.
    await bridge.setVolume(Store.Playback.getVolume());

    document.title = `▶ ${track.title} - Firmium`;
  } catch (e) {
    if (currentToken === Store.Playback.getPlayToken()) {
      console.error('Playback error:', e);
      DOM.render('npArtist', `Playback Error: ${DOM.safeText(e.message)}`);
    }
  }
};

/** Update the now-playing bar with track metadata and cover art. */
const updateNowPlaying = (track) => {
  DOM.render('npTitle', DOM.safeText(track?.title ?? '—'));
  DOM.render('npArtist', DOM.safeText(track?.artist ?? 'No track selected'));
  DOM.render('durTime', formatDuration(track?.duration ?? 0));

  const container = DOM.el('npArt');
  if (container) {
    if (track?.coverArtId) {
      container.innerHTML = `<img class="np-cover-img" id="npCoverImg" alt="">`;
      loadImage(DOM.el('npCoverImg'), track.coverArtId, Store.Playback.getActiveCtrl()?.signal);
    } else {
      container.innerHTML = '<div class="no-art">♪</div>';
    }
  }

  // Clear any previously loaded lyrics and fetch new ones if the panel is open.
  if (track) {
    Lyrics.clear('Loading lyrics…');
    fetchAndShowLyrics(track);
  } else {
    Lyrics.clear();
  }
};

// Fetch lyrics for the given song and populate the lyrics panel.
// No-ops immediately if the panel is closed; discards stale results if the
// track changed while the fetch was in flight.
const fetchAndShowLyrics = async (song) => {
  if (!song) return;
  Lyrics.setTrackId(song.id);
  if (!Lyrics.isOpen()) return;
  Lyrics.setStatus('Loading lyrics…');
  try {
    const result = await Api.getLyrics(song);
    // Discard if a different track started playing while we were fetching.
    if (Lyrics.getTrackId() !== song.id) return;
    if (result) {
      Lyrics.setLyrics(result.lines, result.synced);
    } else {
      Lyrics.setStatus('No lyrics available for this track');
    }
  } catch (e) {
    if (Lyrics.getTrackId() === song.id) {
      Lyrics.setStatus('Failed to load lyrics');
      console.error('Lyrics fetch error:', e);
    }
  }
};

// ── View loaders ───────────────────────────────────────────────────────────────

const loadAlbum = async (id) => {
  Store.Playback.abortActive();
  const ctrl = new AbortController();
  Store.Playback.setActiveCtrl(ctrl);
  
  DOM.render('listPanel', '<div class="loading-msg">Loading album tracks…</div>');

  try {
    const { tracks, albumName, albumArtist, coverArtId } = await Api.getAlbumTracks(id, ctrl.signal);
    if (ctrl.signal.aborted) return;

    Store.UI.pushNav(() => loadView(Store.UI.getView()));

    const html = `
      <div class="tracklist-header">
        <div class="tl-art">${coverArtId
          ? `<img class="lazy-art" data-cover-id="${DOM.safeText(coverArtId)}" alt="">`
          : '♪'}</div>
        <div class="tl-info">
          <div class="tl-title">${DOM.safeText(albumName)}</div>
          <div class="tl-subtitle">${DOM.safeText(albumArtist)}</div>
        </div>
      </div>
      <div class="track-list" id="trackListWrapper">
        ${tracks.map((t, idx) => DOM.createTrackCard(t, idx)).join('')}
      </div>`;

    DOM.render('listPanel', html);

    // Use event delegation on the wrapper div rather than adding per-render listeners.
    DOM.el('trackListWrapper')?.addEventListener('click', (e) => {
      const row = e.target.closest('[data-action="play-track"]');
      if (row) {
        Store.Playback.setQueue(tracks, Number(row.dataset.index));
        playAt(Number(row.dataset.index));
      }
    });

    const panel = DOM.el('listPanel');
    panel.scrollTop = 0;
    observeLazyCovers(panel);
    highlightCurrentTrack();
  } catch (e) {
    if (ctrl.signal.aborted) return;
    DOM.render('listPanel', `<div class="loading-msg error-msg">${DOM.safeText(e.message)}</div>`);
  } finally {
    
  }
};

const loadArtist = async (id) => {
  Store.Playback.abortActive();
  const ctrl = new AbortController();
  Store.Playback.setActiveCtrl(ctrl);
  
  DOM.render('listPanel', '<div class="loading-msg">Loading artist profile…</div>');

  try {
    const { name, albums } = await Api.getArtistDetails(id, ctrl.signal);
    if (ctrl.signal.aborted) return;

    Store.UI.pushNav(() => loadView(Store.UI.getView()));

    // Group albums by release type.
    // Relies primarily on the server-provided `releaseType` field.
    // Falls back to heuristics (title keywords, song count) only when releaseType is absent.
    // Note: heuristics are unreliable — `releaseType` from the server is always preferred.
    const groups = { Albums: [], EPs: [], Singles: [] };
    albums.forEach(a => {
      const type = String(a.releaseType || '').toLowerCase();
      const titleLower = a.name.toLowerCase();

      if (type === 'single') {
        groups.Singles.push(a);
      } else if (type === 'ep') {
        groups.EPs.push(a);
      } else if (type === 'album') {
        groups.Albums.push(a);
      } else {
        // Fallback heuristics when releaseType is not provided by the server.
        if (titleLower.includes(' - single') || titleLower.endsWith('(single)')) {
          groups.Singles.push(a);
        } else if (titleLower.includes(' - ep') || titleLower.endsWith('(ep)')) {
          groups.EPs.push(a);
        } else {
          // Without a reliable type, default everything else to Albums.
          groups.Albums.push(a);
        }
      }
    });

    let html = `
      <div class="artist-page-header">
        <img id="wikiImg" class="artist-img-circle"
          src="data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' fill='%23888' viewBox='0 0 24 24'><path d='M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z'/></svg>"
          alt="${DOM.safeText(name)}">
        <div class="artist-page-info">
          <div class="artist-page-name">${DOM.safeText(name)}</div>
          <div class="artist-page-bio" id="wikiBio">${SafeStorage.getItem('firmium_wikipedia') !== 'false' ? 'Fetching artist biography…' : 'Biography disabled.'}</div>
          <button class="play-all-btn" data-action="play-artist-all" data-id="${DOM.safeText(id)}">▶ Play All Songs</button>
        </div>
      </div>`;

    if (!albums.length) {
      html += '<div class="loading-msg">No releases discovered.</div>';
    } else {
      ['Albums', 'EPs', 'Singles'].forEach(category => {
        if (groups[category].length > 0) {
          html += `<div class="release-group-title">${category}</div>`;
          html += groups[category].map(DOM.createAlbumCard).join('');
        }
      });
    }

    DOM.render('listPanel', html);
    observeLazyCovers(DOM.el('listPanel'));

    if (SafeStorage.getItem('firmium_wikipedia') !== 'false') {
      WikiApi.getInfo(name, ctrl.signal).then(wiki => {
        if (ctrl.signal.aborted) return;
        if (wiki) {
          if (wiki.extract) DOM.render('wikiBio', DOM.safeText(wiki.extract));
          if (wiki.image) { const img = DOM.el('wikiImg'); if (img) img.src = wiki.image; }
        } else {
          DOM.render('wikiBio', 'Biography not available.');
        }
      });
    }

  } catch (e) {
    if (ctrl.signal.aborted) return;
    DOM.render('listPanel', `<div class="loading-msg error-msg">${DOM.safeText(e.message)}</div>`);
  } finally {
    
  }
};

const executeSearch = async () => {
  const input = DOM.el('searchInput');
  const query = input?.value?.trim() ?? '';
  if (!query) return;

  Store.Playback.abortSearch();
  const ctrl = new AbortController();
  Store.Playback.setSearchCtrl(ctrl);

  document.querySelectorAll('.search-results-container, .loading-msg').forEach(e => e.remove());
  const status = document.createElement('div');
  status.className = 'loading-msg';
  status.textContent = 'Searching…';
  DOM.el('listPanel').appendChild(status);

  try {
    const results = await Api.search(query, ctrl.signal);
    if (ctrl.signal.aborted) return;
    status.remove();

    const songs = results.songs ?? [];
    const albums = results.albums ?? [];

    if (!songs.length && !albums.length) {
      DOM.el('listPanel').insertAdjacentHTML('beforeend', '<div class="loading-msg">No results found.</div>');
      return;
    }

    let innerHTML = `<div class="search-results-container">`;
    if (songs.length) {
      // Using a data attribute to tag this wrapper so the delegated listener below
      // can identify which song list to queue when a track is clicked.
      innerHTML += `<div class="section-header">Songs</div>
                    <div class="track-list" id="searchTrackListWrapper" data-context="search">
                      ${songs.map((t, i) => DOM.createTrackCard(t, i)).join('')}
                    </div>`;
    }
    if (albums.length) {
      innerHTML += `<div class="section-header">Albums</div>${albums.map(DOM.createAlbumCard).join('')}`;
    }
    innerHTML += `</div>`;

    DOM.el('listPanel').insertAdjacentHTML('beforeend', innerHTML);

    // Attach click listener to the wrapper div, not to each track row.
    // The songs variable is captured in this closure — it is fresh for each search.
    const wrapper = DOM.el('searchTrackListWrapper');
    if (wrapper) {
      wrapper.addEventListener('click', (e) => {
        const row = e.target.closest('[data-action="play-track"]');
        if (row) {
          Store.Playback.setQueue(songs, Number(row.dataset.index));
          playAt(Number(row.dataset.index));
        }
      });
    }

    observeLazyCovers(DOM.el('listPanel'));
    highlightCurrentTrack();
  } catch (e) {
    if (ctrl.signal.aborted) return;
    status.remove();
    DOM.el('listPanel').insertAdjacentHTML('beforeend',
      `<div class="loading-msg error-msg">${DOM.safeText(e.message)}</div>`);
  }
};

const loadView = async (view) => {
  Store.UI.clearNav();
  Store.UI.setView(view);
  Store.Playback.abortActive();
  Store.Playback.clearObserver();

  document.querySelectorAll('.nav-btn').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.view === view);
  });

  if (view === 'search') {
    DOM.render('listPanel', `
      <div class="search-row">
        <input type="text" id="searchInput" placeholder="Search albums, songs…" maxLength="${SEARCH_INPUT_MAX_LENGTH}">
        <button id="searchSubmitBtn" data-action="search-submit">Search</button>
      </div>`);
    DOM.el('searchInput').focus();
    return;
  }

  if (view === 'settings') {
    const isDecorated = SafeStorage.getItem('firmium_decorations') !== 'false';
    const isWikiEnabled = SafeStorage.getItem('firmium_wikipedia') !== 'false';
    const isAutoLoginEnabled = SafeStorage.getItem('firmium_auto_login') !== 'false';
    const isLrclibEnabled = SafeStorage.getItem('firmium_lrclib') !== 'false';
    const isCrossfadeEnabled = Store.Playback.getCrossfadeEnabled();
    const crossfadeDuration = Store.Playback.getCrossfadeDuration();
    const currentTheme = SafeStorage.getItem('firmium_theme') || 'firmium';
    const themes = [
      ['firmium',             'Firmium'],
      ['gruvbox',             'Gruvbox'],
      ['tokyo-night',         'Tokyo Night'],
      ['dracula',             'Dracula'],
      ['catppuccin-mocha',    'Catppuccin Mocha'],
      ['catppuccin-macchiato','Catppuccin Macchiato'],
      ['catppuccin-frappe',   'Catppuccin Frappé'],
      ['catppuccin-latte',    'Catppuccin Latte'],
      ['nord',                'Nord'],
    ];
    const themeItems = themes
      .map(([val, label]) => `<div class="theme-option${currentTheme === val ? ' selected' : ''}" data-value="${val}">${label}</div>`)
      .join('');
    const currentThemeLabel = (themes.find(([v]) => v === currentTheme) || themes[0])[1];
    const isOpenSubsonic = Store.ServerInfo.isOpenSubsonic();
    const extensions = Store.ServerInfo.getExtensions() ?? [];
    const html = `
      <div class="section-header">Settings</div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Window Decorations</div>
          <div class="settings-desc">Show native title bar and borders</div>
        </div>
        <label class="toggle-switch">
          <input type="checkbox" id="toggleDecorations" ${isDecorated ? 'checked' : ''}>
          <span class="toggle-slider"></span>
        </label>
      </div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Theme</div>
          <div class="settings-desc">Color scheme for the interface</div>
        </div>
        <div class="theme-selector" id="themeSelector">
          <div class="theme-selector-value" id="themeSelectorValue">${currentThemeLabel}</div>
          <span class="theme-selector-arrow">▾</span>
          <div class="theme-selector-dropdown" id="themeSelectorDropdown">${themeItems}</div>
        </div>
      </div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Auto-Login</div>
          <div class="settings-desc">Automatically connect on startup when credentials are saved</div>
        </div>
        <label class="toggle-switch">
          <input type="checkbox" id="toggleAutoLogin" ${isAutoLoginEnabled ? 'checked' : ''}>
          <span class="toggle-slider"></span>
        </label>
      </div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Wikipedia Integration</div>
          <div class="settings-desc">Show artist biography and photo from Wikipedia</div>
        </div>
        <label class="toggle-switch">
          <input type="checkbox" id="toggleWikipedia" ${isWikiEnabled ? 'checked' : ''}>
          <span class="toggle-slider"></span>
        </label>
      </div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">External Lyrics (LRCLIB)</div>
          <div class="settings-desc">Fetch synced lyrics from lrclib.net when your server has none. Sends song title and artist name.</div>
        </div>
        <label class="toggle-switch">
          <input type="checkbox" id="toggleLrclib" ${isLrclibEnabled ? 'checked' : ''}>
          <span class="toggle-slider"></span>
        </label>
      </div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Crossfade</div>
          <div class="settings-desc">Smoothly blend between tracks</div>
        </div>
        <label class="toggle-switch">
          <input type="checkbox" id="toggleCrossfade" ${isCrossfadeEnabled ? 'checked' : ''}>
          <span class="toggle-slider"></span>
        </label>
      </div>
      <div class="settings-row" id="crossfadeDurationRow" style="${isCrossfadeEnabled ? '' : 'display:none'}">
        <div class="settings-info">
          <div class="settings-title">Crossfade Duration</div>
          <div class="settings-desc">Length of the blend in seconds</div>
        </div>
        <div class="crossfade-duration-control">
          <input type="range" id="crossfadeDurationSlider" min="1" max="12" step="1" value="${crossfadeDuration}">
          <span id="crossfadeDurationLabel">${crossfadeDuration}s</span>
        </div>
      </div>
      <div class="section-header">Debug</div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">App Version</div>
          <div class="settings-desc" id="debugAppVersion">Loading…</div>
        </div>
      </div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">System</div>
          <div class="settings-desc" id="debugSystemInfo">Loading…</div>
        </div>
      </div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Log File</div>
          <div class="settings-desc debug-path" id="debugLogPath">Loading…</div>
        </div>
      </div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Wipe Cache</div>
          <div class="settings-desc">Clear in-memory cover art cache</div>
        </div>
        <button class="debug-btn" id="debugWipeCache">Wipe</button>
      </div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Delete Logs</div>
          <div class="settings-desc">Remove the app-logs.txt file from disk</div>
        </div>
        <button class="debug-btn debug-btn--danger" id="debugDeleteLogs">Delete</button>
      </div>
      <div class="settings-row">
        <div class="settings-info">
          <div class="settings-title">Delete User Settings</div>
          <div class="settings-desc">Reset all preferences to defaults</div>
        </div>
        <button class="debug-btn debug-btn--danger" id="debugDeleteSettings">Delete</button>
      </div>`;
    DOM.render('listPanel', html);

    const selector = DOM.el('themeSelector');
    const dropdown = DOM.el('themeSelectorDropdown');
    const valueEl = DOM.el('themeSelectorValue');
    selector?.addEventListener('click', (e) => {
      e.stopPropagation();
      selector.classList.toggle('open');
    });
    dropdown?.addEventListener('click', (e) => {
      const opt = e.target.closest('.theme-option');
      if (!opt) return;
      const theme = opt.dataset.value;
      dropdown.querySelectorAll('.theme-option').forEach(el => el.classList.remove('selected'));
      opt.classList.add('selected');
      valueEl.textContent = opt.textContent;
      selector.classList.remove('open');
      applyTheme(theme);
      SafeStorage.setItem('firmium_theme', theme);
    });
    document.addEventListener('click', () => selector?.classList.remove('open'), { capture: true });

    DOM.el('toggleAutoLogin')?.addEventListener('change', (e) => {
      SafeStorage.setItem('firmium_auto_login', e.target.checked ? 'true' : 'false');
    });

    DOM.el('toggleWikipedia')?.addEventListener('change', (e) => {
      SafeStorage.setItem('firmium_wikipedia', e.target.checked ? 'true' : 'false');
    });

    DOM.el('toggleLrclib')?.addEventListener('change', (e) => {
      SafeStorage.setItem('firmium_lrclib', e.target.checked ? 'true' : 'false');
    });

    DOM.el('toggleCrossfade')?.addEventListener('change', (e) => {
      Store.Playback.setCrossfadeEnabled(e.target.checked);
      const durationRow = DOM.el('crossfadeDurationRow');
      if (durationRow) durationRow.style.display = e.target.checked ? '' : 'none';
    });

    DOM.el('crossfadeDurationSlider')?.addEventListener('input', (e) => {
      const val = Number(e.target.value);
      Store.Playback.setCrossfadeDuration(val);
      const label = DOM.el('crossfadeDurationLabel');
      if (label) label.textContent = `${val}s`;
    });

    // ── Debug section: async info loaders ──────────────────────────────────────
    tauriInvoke('get_app_version').then(v => {
      const el = DOM.el('debugAppVersion');
      if (el) el.textContent = `v${v}`;
    }).catch(() => {
      const el = DOM.el('debugAppVersion');
      if (el) el.textContent = 'unavailable';
    });

    tauriInvoke('get_machine_info').then(info => {
      const el = DOM.el('debugSystemInfo');
      if (el) el.textContent = `${info.distro} ${info.version}`;
    }).catch(() => {
      const el = DOM.el('debugSystemInfo');
      if (el) el.textContent = 'unavailable';
    });

    tauriInvoke('get_log_path').then(p => {
      const el = DOM.el('debugLogPath');
      if (el) el.textContent = p;
    }).catch(() => {
      const el = DOM.el('debugLogPath');
      if (el) el.textContent = 'unavailable';
    });

    DOM.el('debugWipeCache')?.addEventListener('click', (e) => {
      Store.Playback.clearAllCache();
      e.target.textContent = 'Wiped!';
      setTimeout(() => { e.target.textContent = 'Wipe'; }, 1500);
      AppLogger.info('Cover art cache wiped by user');
    });

    DOM.el('debugDeleteLogs')?.addEventListener('click', async (e) => {
      e.target.disabled = true;
      try {
        await tauriInvoke('delete_logs');
        e.target.textContent = 'Deleted!';
        AppLogger.info('Log file deleted by user');
      } catch (err) {
        e.target.textContent = 'Failed';
        console.error('Failed to delete logs:', err);
      }
      setTimeout(() => { e.target.textContent = 'Delete'; e.target.disabled = false; }, 1500);
    });

    DOM.el('debugDeleteSettings')?.addEventListener('click', (e) => {
      const SETTINGS_KEYS = [
        'firmium_server', 'firmium_user', 'firmium_save_pass',
        'firmium_auto_login', 'firmium_wikipedia', 'firmium_lrclib', 'firmium_theme',
        'firmium_decorations', 'firmium_crossfade', 'firmium_crossfade_duration',
        'firmium_volume',
      ];
      SETTINGS_KEYS.forEach(k => SafeStorage.removeItem(k));
      AppLogger.info('User settings deleted by user');
      e.target.textContent = 'Deleted!';
      setTimeout(() => { e.target.textContent = 'Delete'; }, 1500);
    });

    DOM.el('toggleDecorations')?.addEventListener('change', async (e) => {
      const show = e.target.checked;
      SafeStorage.setItem('firmium_decorations', show ? 'true' : 'false');
      try {
        if (window.__TAURI__) {
          const tauriWindow = window.__TAURI__.window || (window.__TAURI__.core ? window.__TAURI__ : null);
          if (tauriWindow && typeof tauriWindow.getCurrentWindow === 'function') {
            await tauriWindow.getCurrentWindow().setDecorations(show);
            return;
          }
          if (tauriWindow && typeof tauriWindow.getCurrent === 'function') {
            await tauriWindow.getCurrent().setDecorations(show);
            return;
          }
        }
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().setDecorations(show);
      } catch (err) {
        console.error('Failed to set window decorations:', err);
      }
    });
    return;
  }

  
  const ctrl = new AbortController();
  Store.Playback.setActiveCtrl(ctrl);
  DOM.render('listPanel', `<div class="loading-msg">Loading ${view}…</div>`);

  try {
    if (view === 'albums') {
      const albums = await Api.getAlbums(ctrl.signal);
      if (ctrl.signal.aborted) return;
      if (!albums.length) { DOM.render('listPanel', '<div class="loading-msg">No albums found.</div>'); return; }
      DOM.render('listPanel', `<div class="section-header">Albums</div>${albums.map(DOM.createAlbumCard).join('')}`);
    } else if (view === 'artists') {
      const artists = await Api.getArtists(ctrl.signal);
      if (ctrl.signal.aborted) return;
      if (!artists.length) { DOM.render('listPanel', '<div class="loading-msg">No artists found.</div>'); return; }
      DOM.render('listPanel', `<div class="section-header">Artists</div><div class="artist-list">${artists.map(DOM.createArtistCard).join('')}</div>`);
    }
    observeLazyCovers(DOM.el('listPanel'));
  } catch (e) {
    if (ctrl.signal.aborted) return;
    DOM.render('listPanel', `<div class="loading-msg error-msg">${DOM.safeText(e.message)}</div>`);
  } finally {
    
  }
};

// ── App startup ────────────────────────────────────────────────────────────────

// Attempt to connect with the given credentials. Returns true on success.
const doConnect = async (sUrl, uName, pWord) => {
  let parsed;
  try { parsed = new URL(sUrl); } catch (_) { throw new Error('Invalid URL format'); }
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
    throw new Error('Protocol must be HTTP or HTTPS');
  }
  Store.Auth.setAuth(sUrl, uName, pWord);
  await Api.fetch('getAlbumList2', { type: 'alphabeticalByName', size: 1 });
  if (!Store.ServerInfo.isOpenSubsonic()) {
    console.warn('Connected to a legacy Subsonic server — OpenSubsonic features will not be available.');
  }
  return true;
};

const showApp = () => {
  DOM.el('setup')?.classList.add('hidden');
  DOM.el('app')?.classList.remove('hidden');
  try {
    DOM.render('serverLabel', new URL(Store.Auth.getServer()).hostname);
  } catch (_) {
    DOM.render('serverLabel', 'online');
  }
  const savedVol = Number(SafeStorage.getItem('firmium_volume') ?? DEFAULT_VOLUME);
  DOM.el('volSlider').value = savedVol;
  loadView('albums');
};

document.addEventListener('DOMContentLoaded', async () => {
  // Initialize audio bridge — must happen before any playback calls.
  Store.Audio.init();

  // Apply saved theme before anything renders.
  applyTheme(SafeStorage.getItem('firmium_theme'));

  // Restore non-sensitive settings from localStorage.
  const savedServer = SafeStorage.getItem('firmium_server');
  const savedUser = SafeStorage.getItem('firmium_user');
  const savePasswordEnabled = SafeStorage.getItem('firmium_save_pass') === 'true';

  if (savedServer) DOM.el('serverUrl').value = savedServer;
  if (savedUser) DOM.el('username').value = savedUser;

  // Attempt to load the saved password from the OS keyring (NOT localStorage).
  let resolvedPass = null;
  if (savePasswordEnabled && savedUser) {
    const saveCb = DOM.el('savePassword');
    if (saveCb) saveCb.checked = true;
    try {
      const savedPass = await Keyring.load(savedUser);
      if (savedPass) {
        DOM.el('password').value = savedPass;
        resolvedPass = savedPass;
      }
    } catch {
      // Keyring entry may not exist yet (first run after migrating from localStorage).
      // Silently ignore — the user will just need to re-enter their password.
    }
  }

  // Auto-login: if all credentials are available and the setting is on (default), skip the login screen.
  const autoLoginEnabled = SafeStorage.getItem('firmium_auto_login') !== 'false';
  if (autoLoginEnabled && savedServer && savedUser && resolvedPass) {
    const statusEl = DOM.el('setupError');
    if (statusEl) { statusEl.textContent = 'Connecting…'; statusEl.style.color = ''; }
    try {
      await doConnect(savedServer, savedUser, resolvedPass);
      showApp();
    } catch (err) {
      Store.Auth.clearAuth();
      if (statusEl) { statusEl.textContent = err.message ?? 'Auto-login failed'; statusEl.style.color = 'var(--error, red)'; }
    }
  }

  const isDecorated = SafeStorage.getItem('firmium_decorations') !== 'false';

  try {
    if (window.__TAURI__) {
      const tauriWindow = window.__TAURI__.window || window.__TAURI__;
      if (tauriWindow && typeof tauriWindow.getCurrentWindow === 'function') {
        tauriWindow.getCurrentWindow().setDecorations(isDecorated);
      } else if (tauriWindow && typeof tauriWindow.getCurrent === 'function') {
        tauriWindow.getCurrent().setDecorations(isDecorated);
      }
    } else {
      import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
        getCurrentWindow().setDecorations(isDecorated);
      }).catch(() => {});
    }
  } catch (_) {}

  document.addEventListener('contextmenu', (e) => e.preventDefault());

  // ── Main click dispatcher ────────────────────────────────────────────────────
  document.body.addEventListener('click', async (e) => {
    const target = e.target.closest('[data-action]');
    if (!target) return;
    e.stopPropagation();

    const action = target.dataset.action;
    switch (action) {

      case 'nav-view':
        loadView(target.dataset.view);
        break;

      case 'load-album':
        loadAlbum(target.dataset.id);
        break;

      case 'load-artist':
        loadArtist(target.dataset.id);
        break;

      case 'play-artist-all': {
        const artistId = target.dataset.id;
        const ogText = target.textContent;
        target.textContent = 'Loading Queue…';
        target.style.opacity = '0.5';
        target.style.pointerEvents = 'none';
        try {
          const { albums } = await Api.getArtistDetails(artistId);
          // Use a concurrency pool to avoid firing all fetches simultaneously.
          // PLAY_ALL_CONCURRENCY = 5 keeps the server load manageable.
          const completedAlbums = await pooledMap(albums, PLAY_ALL_CONCURRENCY, (a) =>
            Api.getAlbumTracks(a.id)
          );
          const allTracks = completedAlbums.flatMap(res => res.tracks);
          if (allTracks.length > 0) {
            Store.Playback.setQueue(allTracks, 0);
            playAt(0);
          } else {
            alert('No playable tracks found for this artist.');
          }
        } catch (err) {
          console.error('Play artist all failed:', err);
          alert('Failed to load artist queue.');
        } finally {
          target.textContent = ogText;
          target.style.opacity = '1';
          target.style.pointerEvents = 'auto';
        }
        break;
      }

      case 'play-toggle': {
        const bridge = Store.Audio.getBridge();
        if (!Store.Playback.getCurrentTrack() || !bridge) return;
        try {
          const state = await bridge.getState();
          if (state === 'paused') {
            await bridge.resume();
          } else if (state === 'playing') {
            await bridge.pause();
          } else if (state === 'stopped') {
            // Song finished — restart from the beginning.
            playAt(Store.Playback.getQueueIdx());
          }
          // Ignore clicks during 'loading' state — audio isn't ready yet.
        } catch (err) {
          console.error('Play toggle failed:', err);
        }
        break;
      }

      case 'prev-track':
        if (Store.Playback.getQueueIdx() > 0) {
          playAt(Store.Playback.getQueueIdx() - 1);
        }
        break;

      case 'next-track':
        if (Store.Playback.getQueueIdx() < Store.Playback.getQueue().length - 1) {
          playAt(Store.Playback.getQueueIdx() + 1);
        } else if (Store.Playback.getRepeatAll()) {
          playAt(0);
        }
        break;

      case 'toggle-repeat-one': {
        const nextR1 = !Store.Playback.getRepeatOne();
        Store.Playback.setRepeatOne(nextR1);
        target.classList.toggle('active', nextR1);
        DOM.el('rAllBtn')?.classList.remove('active');
        break;
      }

      case 'toggle-repeat-all': {
        const nextRA = !Store.Playback.getRepeatAll();
        Store.Playback.setRepeatAll(nextRA);
        target.classList.toggle('active', nextRA);
        DOM.el('rOneBtn')?.classList.remove('active');
        break;
      }

      case 'toggle-lyrics': {
        const nowOpen = Lyrics.toggle();
        // If just opened, fetch lyrics for the current track if not already loaded.
        if (nowOpen) {
          const track = Store.Playback.getCurrentTrack();
          if (track && Lyrics.getTrackId() !== track.id) {
            fetchAndShowLyrics(track);
          } else if (!track) {
            Lyrics.setStatus('No track playing');
          }
        }
        break;
      }

      // Fixed: was 'logout-action' in HTML but 'logout' in switch → button did nothing.
      // HTML data-action is now 'logout' to match this case.
      case 'logout':
        teardownApp();
        break;

      case 'search-submit':
        executeSearch();
        break;

      case 'connect': {
        const sUrl = DOM.el('serverUrl')?.value ?? '';
        const uName = DOM.el('username')?.value ?? '';
        const pWord = DOM.el('password')?.value ?? '';
        if (!sUrl || !uName || !pWord) { alert('Please fill out all fields'); return; }

        target.textContent = 'Connecting…';
        DOM.render('setupError', '');

        try {
          await doConnect(sUrl, uName, pWord);

          // Save non-sensitive values to localStorage.
          SafeStorage.setItem('firmium_server', sUrl);
          SafeStorage.setItem('firmium_user', uName);

          // Save the password to the OS keyring, NOT localStorage.
          if (DOM.el('savePassword')?.checked) {
            SafeStorage.setItem('firmium_save_pass', 'true');
            try {
              await Keyring.save(uName, pWord);
            } catch (kErr) {
              console.warn('Keyring save failed — password will not be remembered:', kErr);
            }
          } else {
            SafeStorage.setItem('firmium_save_pass', 'false');
            // Remove any previously saved keyring entry.
            Keyring.remove(uName).catch(() => {});
          }

          showApp();
        } catch (err) {
          Store.Auth.clearAuth();
          DOM.render('setupError', DOM.safeText(err.message ?? 'Authentication rejected'));
        } finally {
          target.textContent = 'Connect';
        }
        break;
      }
    }
  });

  // Search on Enter key in the search input.
  document.body.addEventListener('keydown', (e) => {
    if (e.target.id === 'searchInput' && e.key === 'Enter') {
      executeSearch();
    }
  });

  // Volume slider — update the bridge and persist the value.
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

  const seekBar = DOM.el('seekBar');
  if (seekBar) {
    const startSeek = () => Store.Audio.setSeeking(true);
    const endSeek = async () => {
      Store.Audio.setSeeking(false);
      const bridge = Store.Audio.getBridge();
      if (bridge) {
        try {
          await bridge.seek(Number(seekBar.value));
        } catch (err) {
          console.error('Seek failed:', err);
        }
      }
    };
    seekBar.addEventListener('mousedown', startSeek);
    seekBar.addEventListener('mouseup', endSeek);
    seekBar.addEventListener('touchstart', startSeek);
    seekBar.addEventListener('touchend', endSeek);
  }
});