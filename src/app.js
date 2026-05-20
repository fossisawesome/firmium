const API_PAGE_SIZE = 500;
const MAX_COVER_CACHE_SIZE = 150;
const SEARCH_INPUT_MAX_LENGTH = 500;
const TRACK_RESTART_THRESHOLD_SECS = 3;
const SEARCH_ALBUM_LIMIT = 40;
const SEARCH_SONG_LIMIT = 100;

const md5 = (string) => {
  const encoder = new TextEncoder();
  const bytes = encoder.encode(string);
  const k = [], s = [7, 12, 17, 22, 5, 9, 14, 20, 4, 11, 16, 23, 6, 10, 15, 21];
  for (let i = 0; i < 64; i++) k[i] = Math.floor(Math.abs(Math.sin(i + 1)) * 4294967296);
  let h0 = 0x67452301, h1 = 0xefcdab89, h2 = 0x98badcfe, h3 = 0x10325476;
  const words = [];
  for (let i = 0; i < bytes.length; i++) words[i >> 2] |= (bytes[i] & 0xff) << ((i % 4) * 8);
  words[bytes.length >> 2] |= 0x80 << ((bytes.length % 4) * 8);
  words[(((bytes.length + 8) >> 6) + 1) * 16 - 2] = bytes.length * 8;
  for (let i = 0; i < words.length; i += 16) {
    let a = h0, b = h1, c = h2, d = h3;
    for (let j = 0; j < 64; j++) {
      let f, g;
      if (j < 16) { f = (b & c) | (~b & d); g = j; }
      else if (j < 32) { f = (d & b) | (~d & c); g = (5 * j + 1) % 16; }
      else if (j < 48) { f = b ^ c ^ d; g = (3 * j + 5) % 16; }
      else { f = c ^ (b | ~d); g = (7 * j) % 16; }
      let temp = d; d = c; c = b;
      b = b + ((q, r) => (q << r) | (q >>> (32 - r)))((a + f + k[j] + (words[i + g] || 0)), s[(Math.floor(j / 16) * 4) + (j % 4)]);
      a = temp;
    }
    h0 = (h0 + a) | 0; h1 = (h1 + b) | 0; h2 = (h2 + c) | 0; h3 = (h3 + d) | 0;
  }
  return [h0, h1, h2, h3].map(v => ('00000000' + (v >>> 0).toString(16)).slice(-8).match(/../g).reverse().join('')).join('');
};

const generateSecureSalt = () => {
  const arr = new Uint8Array(8);
  crypto.getRandomValues(arr);
  return Array.from(arr, b => b.toString(16).padStart(2, '0')).join('');
};

const WikiApi = {
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
    } catch (e) {
      return null;
    }
  }
};

const SafeStorage = {
  getItem: (key) => {
    try { return localStorage.getItem(key); } catch (e) { return null; }
  },
  setItem: (key, value) => {
    try { localStorage.setItem(key, value); } catch (e) { }
  },
  removeItem: (key) => {
    try { localStorage.removeItem(key); } catch (e) { }
  }
};

const Store = {
  Auth: (() => {
    let _server = null, _username = null, _password = null;
    return {
      setAuth: (s, u, p) => { _server = s ? String(s).trim().replace(/\/+$/, '') : null; _username = u; _password = p; },
      clearAuth: () => { _server = null; _username = null; _password = null; },
      isAuthed: () => Boolean(_server && _username && _password),
      getServer: () => _server,
      getQueryParams: () => {
        if (!_username || !_password) return {};
        const salt = generateSecureSalt();
        return { u: _username, t: md5(_password + salt), s: salt, v: '1.16.1', c: 'firmium', f: 'json' };
      }
    };
  })(),

  UI: (() => {
    let _view = 'albums', _loading = false, _navHistory = [];
    return {
      getView: () => _view,
      setView: (v) => { _view = v; },
      setLoading: (l) => { _loading = Boolean(l); },
      pushNav: (fn) => { if (typeof fn === 'function') _navHistory.push(fn); },
      popNav: () => _navHistory.pop(),
      clearNav: () => { _navHistory = []; }
    };
  })(),

  Playback: (() => {
    let _audio = null, _queue = [], _queueIdx = -1, _playToken = 0;
    let _seeking = false, _volume = Number(SafeStorage.getItem('firmium_volume') ?? 0.8);
    let _repeatOne = false, _repeatAll = false, _lastSec = -1;
    let _abortCtrl = null, _searchCtrl = null, _observer = null;
    const _covers = new Map(), _pendingCovers = new Map();

    return {
      initAudio: (el) => { _audio = el; _audio.volume = _volume; },
      getAudio: () => _audio,
      getQueue: () => _queue,
      getQueueIdx: () => _queueIdx,
      getCurrentTrack: () => _queue[_queueIdx] || null,
      setQueue: (items, idx = 0) => { _queue = Array.isArray(items) ? items : []; _queueIdx = _queue.length ? idx : -1; },
      setQueueIdx: (idx) => { if (idx >= 0 && idx < _queue.length) _queueIdx = idx; },
      getPlayToken: () => _playToken,
      bumpToken: () => ++_playToken,
      isSeeking: () => _seeking,
      setSeeking: (s) => { _seeking = Boolean(s); },
      getVolume: () => _volume,
      setVolume: (v) => { _volume = Math.max(0, Math.min(1, Number.isFinite(Number(v)) ? Number(v) : 0.8)); SafeStorage.setItem('firmium_volume', String(_volume)); if (_audio) _audio.volume = _volume; },
      getRepeatOne: () => _repeatOne,
      setRepeatOne: (v) => { _repeatOne = Boolean(v); if (v) _repeatAll = false; },
      getRepeatAll: () => _repeatAll,
      setRepeatAll: (v) => { _repeatAll = Boolean(v); if (v) _repeatOne = false; },
      getLastSec: () => _lastSec,
      setLastSec: (s) => { _lastSec = s; },

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
  })()
};

const SubsonicRouter = {
  buildUrl: (action, params = {}) => {
    const server = Store.Auth.getServer();
    if (!server) return '';
    const url = new URL(`${server}/rest/${action}.view`);
    const combined = { ...Store.Auth.getQueryParams(), ...params };
    Object.entries(combined).forEach(([k, v]) => {
      if (v !== null && v !== undefined) url.searchParams.append(k, String(v));
    });
    return url.toString();
  }
};

const SubsonicMapper = {
  mapAlbum: (a) => ({ id: a.id, name: a.name ?? a.title ?? 'Unknown Album', albumArtist: a.artist ?? 'Unknown Artist', coverArtId: a.coverArt, songCount: a.songCount, releaseType: a.releaseType }),
  mapArtist: (a) => ({ id: a.id, name: a.name ?? 'Unknown Artist', albumCount: a.albumCount ?? 0 }),
  mapSong: (s) => ({ id: s.id, title: s.title ?? 'Unknown Track', artist: s.artist ?? 'Unknown Artist', duration: s.duration ?? 0, trackNumber: s.track, coverArtId: s.coverArt })
};

const Api = {
  fetch: async (action, params = {}, signal = null) => {
    const url = SubsonicRouter.buildUrl(action, params);
    const res = await fetch(url, signal ? { signal } : {});
    if (res.status === 401) { if (Store.Auth.isAuthed()) teardownApp(); throw new Error('Session Expired'); }
    if (!res.ok) throw new Error(`HTTP Error ${res.status}`);
    const json = await res.json();
    const responseObj = json['subsonic-response'];
    if (!responseObj) throw new Error('Malformed API response');
    if (responseObj.status === 'failed') throw new Error(responseObj.error?.message ?? 'Engine error');
    return responseObj;
  },
  getAlbums: async (sig) => {
    const d = await Api.fetch('getAlbumList2', { type: 'alphabeticalByName', size: API_PAGE_SIZE }, sig);
    return (d.albumList2?.album ?? d.albumList?.album ?? []).map(SubsonicMapper.mapAlbum);
  },
  getArtists: async (sig) => {
    const d = await Api.fetch('getArtists', {}, sig);
    const container = [];
    if (d.artists?.index) d.artists.index.forEach(i => { if (Array.isArray(i.artist)) container.push(...i.artist); });
    return container.map(SubsonicMapper.mapArtist);
  },
  getAlbumTracks: async (id, sig) => {
    const d = await Api.fetch('getAlbum', { id }, sig);
    const a = d.album ?? {};
    return { tracks: (a.song ?? []).map(SubsonicMapper.mapSong), albumName: a.name ?? a.title ?? 'Unknown Album', albumArtist: a.artist ?? 'Unknown Artist', coverArtId: a.coverArt };
  },
  getArtistDetails: async (id, sig) => {
    const d = await Api.fetch('getArtist', { id }, sig);
    return {
      name: d.artist?.name ?? 'Unknown Artist',
      albums: (d.artist?.album ?? []).map(SubsonicMapper.mapAlbum)
    };
  },
  search: async (query, sig) => {
    const d = await Api.fetch('search3', { query, albumCount: SEARCH_ALBUM_LIMIT, songCount: SEARCH_SONG_LIMIT }, sig);
    return { songs: (d.searchResult3?.song ?? []).map(SubsonicMapper.mapSong), albums: (d.searchResult3?.album ?? []).map(SubsonicMapper.mapAlbum) };
  }
};

const formatDuration = (secs) => {
  const s = Number(secs);
  if (isNaN(s) || s <= 0) return '0:00';
  const m = Math.floor(s / 60), r = Math.floor(s % 60);
  return `${m}:${r < 10 ? '0' : ''}${r}`;
};

const loadImage = async (img, coverId, signal) => {
  if (!img || !coverId) return;
  const cached = Store.Playback.getCover(coverId);
  if (cached) { img.src = cached; return; }

  let promise = Store.Playback.getPendingCover(coverId);
  if (!promise) {
    promise = (async () => {
      const url = SubsonicRouter.buildUrl('getCoverArt', { id: coverId });
      const res = await fetch(url, { signal });
      if (!res.ok) throw new Error('Cover art asset down');
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
    if (e.name !== 'AbortError') console.error(e);
  } finally {
    Store.Playback.clearPendingCover(coverId);
  }
};

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

const DOM = {
  el: (id) => document.getElementById(id),
  render: (id, html) => { const el = DOM.el(id); if (el) el.innerHTML = html; },
  safeText: (str) => String(str ?? '').replace(/[&<>"']/g, m => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#039;' }[m])),
  
  createAlbumCard: (album) => `
    <div class="album-row" data-action="load-album" data-id="${DOM.safeText(album.id)}">
      <div class="album-art-sm">
        ${album.coverArtId ? `<img class="lazy-art" data-cover-id="${DOM.safeText(album.coverArtId)}" alt="">` : '<div class="no-art">♪</div>'}
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
      <div class="track-info">
        <div class="track-title">${DOM.safeText(track.title)}</div>
        <div class="track-artist">${DOM.safeText(track.artist)}</div>
      </div>
      <div class="track-duration">${formatDuration(track.duration)}</div>
    </div>`
};

const teardownApp = () => {
  Store.Playback.abortActive();
  Store.Playback.abortSearch();
  Store.Playback.clearObserver();

  const audio = Store.Playback.getAudio();
  if (audio) {
    try { audio.pause(); } catch(e){}
    if (audio.src?.startsWith('blob:')) { try { URL.revokeObjectURL(audio.src); } catch(e){} }
    audio.src = '';
  }

  Store.Playback.clearAllCache();
  Store.Auth.clearAuth();
  Store.UI.clearNav();

  DOM.el('setup')?.classList.remove('hidden');
  DOM.el('app')?.classList.add('hidden');
  DOM.render('setupError', '');
  document.title = 'Firmium';
};

const BlacklistFilter = () => {
  const current = Store.Playback.getCurrentTrack();
  const currentId = current ? String(current.id) : null;
  const panel = DOM.el('listPanel');
  if (panel) {
    panel.querySelectorAll('.track-row').forEach(r => {
      r.classList.toggle('playing', currentId !== null && r.dataset.id === currentId);
    });
  }
};

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

    if (audio.src?.startsWith('blob:')) { try { URL.revokeObjectURL(audio.src); } catch(e){} }
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
};

const loadAlbum = async (id) => {
  Store.Playback.abortActive();
  const ctrl = new AbortController();
  Store.Playback.setActiveCtrl(ctrl);
  Store.UI.setLoading(true);

  DOM.render('listPanel', '<div class="loading-msg">Loading album tracks…</div>');

  try {
    const { tracks, albumName, albumArtist, coverArtId } = await Api.getAlbumTracks(id, ctrl.signal);
    if (ctrl.signal.aborted) return;

    Store.UI.pushNav(() => loadView(Store.UI.getView()));
    
    let html = `
      <div class="tracklist-header">
        <div class="tl-art">${coverArtId ? `<img class="lazy-art" data-cover-id="${DOM.safeText(coverArtId)}" alt="">` : '♪'}</div>
        <div class="tl-info">
          <div class="tl-title">${DOM.safeText(albumName)}</div>
          <div class="tl-subtitle">${DOM.safeText(albumArtist)}</div>
        </div>
      </div>
      <div class="track-list" id="trackListWrapper">
        ${tracks.map((t, idx) => DOM.createTrackCard(t, idx)).join('')}
      </div>`;
      
    DOM.render('listPanel', html);

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
    if (!ctrl.signal.aborted) Store.UI.setLoading(false);
  }
};

const loadArtist = async (id) => {
  Store.Playback.abortActive();
  const ctrl = new AbortController();
  Store.Playback.setActiveCtrl(ctrl);
  Store.UI.setLoading(true);
  DOM.render('listPanel', '<div class="loading-msg">Loading artist profile…</div>');

  try {
    const { name, albums } = await Api.getArtistDetails(id, ctrl.signal);
    if (ctrl.signal.aborted) return;

    Store.UI.pushNav(() => loadView(Store.UI.getView()));
    
    const groups = { Albums: [], EPs: [], Singles: [] };
    albums.forEach(a => {
      const type = String(a.releaseType || '').toLowerCase();
      const titleLower = a.name.toLowerCase();
      if (type === 'single' || titleLower.includes('single') || a.songCount === 1) {
        groups.Singles.push(a);
      } else if (type === 'ep' || titleLower.includes('ep') || (a.songCount > 1 && a.songCount <= 4)) {
        groups.EPs.push(a);
      } else {
        groups.Albums.push(a);
      }
    });

    let html = `
      <div class="artist-page-header">
        <img id="wikiImg" class="artist-img-circle" src="data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' fill='%23888' viewBox='0 0 24 24'><path d='M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z'/></svg>" alt="${DOM.safeText(name)}">
        <div class="artist-page-info">
          <div class="artist-page-name">${DOM.safeText(name)}</div>
          <div class="artist-page-bio" id="wikiBio">Fetching artist biography...</div>
          <button class="play-all-btn" data-action="play-artist-all" data-id="${DOM.safeText(id)}">▶ Play All Songs</button>
        </div>
      </div>
    `;

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

    WikiApi.getInfo(name, ctrl.signal).then(wiki => {
      if (wiki && !ctrl.signal.aborted) {
        if (wiki.extract) DOM.render('wikiBio', DOM.safeText(wiki.extract));
        if (wiki.image) DOM.el('wikiImg').src = wiki.image;
      } else if (!wiki && !ctrl.signal.aborted) {
        DOM.render('wikiBio', 'Biography not available.');
      }
    });

  } catch (e) {
    if (ctrl.signal.aborted) return;
    DOM.render('listPanel', `<div class="loading-msg error-msg">${DOM.safeText(e.message)}</div>`);
  } finally {
    if (!ctrl.signal.aborted) Store.UI.setLoading(false);
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
      innerHTML += `<div class="section-header">Songs</div>
                    <div class="track-list" id="searchTrackListWrapper">
                      ${songs.map((t, i) => DOM.createTrackCard(t, i)).join('')}
                    </div>`;
    }
    if (albums.length) {
      innerHTML += `<div class="section-header">Albums</div>${albums.map(DOM.createAlbumCard).join('')}`;
    }
    innerHTML += `</div>`;

    DOM.el('listPanel').insertAdjacentHTML('beforeend', innerHTML);
    
    DOM.el('searchTrackListWrapper')?.addEventListener('click', (e) => {
      const row = e.target.closest('[data-action="play-track"]');
      if (row) {
        Store.Playback.setQueue(songs, Number(row.dataset.index));
        playAt(Number(row.dataset.index));
      }
    });

    observeLazyCovers(DOM.el('listPanel'));
    highlightCurrentTrack();
  } catch (e) {
    if (ctrl.signal.aborted) return;
    status.remove();
    DOM.el('listPanel').insertAdjacentHTML('beforeend', `<div class="loading-msg error-msg">${DOM.safeText(e.message)}</div>`);
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
        <button id="searchSubmitBtn">Search</button>
      </div>`);
    DOM.el('searchInput').focus();
    return;
  }

  if (view === 'settings') {
    const isDecorated = SafeStorage.getItem('firmium_decorations') !== 'false';
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
    `;
    DOM.render('listPanel', html);

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
        console.error("Failed to alter window decorations status:", err);
      }
    });
    return;
  }

  Store.UI.setLoading(true);
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
    if (!ctrl.signal.aborted) Store.UI.setLoading(false);
  }
};

const showApp = () => {
  DOM.el('setup')?.classList.add('hidden');
  DOM.el('app')?.classList.remove('hidden');
  try {
    DOM.render('serverLabel', new URL(Store.Auth.getServer()).hostname);
  } catch(e) {
    DOM.render('serverLabel', 'online');
  }
  DOM.el('volSlider').value = Store.Playback.getVolume();
  loadView('albums');
};

document.addEventListener('DOMContentLoaded', () => {
  let audio = DOM.el('audioEl');
  if (!audio) {
    audio = document.createElement('audio');
    audio.id = 'audioEl';
    audio.preload = 'auto';
    document.body.appendChild(audio);
  }
  Store.Playback.initAudio(audio);

  const savedServer = SafeStorage.getItem('firmium_server');
  const savedUser = SafeStorage.getItem('firmium_user');
  const savedPass = SafeStorage.getItem('firmium_pass');
  
  if (savedServer) DOM.el('serverUrl').value = savedServer;
  if (savedUser) DOM.el('username').value = savedUser;
  if (savedPass) {
    DOM.el('password').value = savedPass;
    const saveCb = DOM.el('savePassword');
    if (saveCb) saveCb.checked = true;
  }

  const isDecorated = SafeStorage.getItem('firmium_decorations') !== 'false';
  const decoCheckbox = DOM.el('toggleDecorations');
  if (decoCheckbox) {
    decoCheckbox.checked = isDecorated;
  }

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
  } catch(e) {}

  audio.addEventListener('play', () => { DOM.render('playBtn', '⏸'); });
  audio.addEventListener('pause', () => { DOM.render('playBtn', '▶'); });
  audio.addEventListener('durationchange', () => { DOM.render('durTime', formatDuration(audio.duration || 0)); });
  
  audio.addEventListener('error', () => {
    const err = audio.error;
    if (err && err.code === 4) {
      const currentTimeSave = audio.currentTime;
      audio.load(); 
      audio.currentTime = currentTimeSave;
      audio.play().catch(() => {});
    }
  });

  audio.addEventListener('stalled', () => {
    if (!audio.paused) {
       audio.play().catch(() => {});
    }
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

  document.addEventListener('contextmenu', (e) => {
  e.preventDefault();
  });

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
        target.textContent = 'Loading Queue...';
        target.style.opacity = '0.5';
        target.style.pointerEvents = 'none';
        try {
          const { albums } = await Api.getArtistDetails(artistId);
          const trackPromises = albums.map(a => Api.getAlbumTracks(a.id));
          const completedAlbums = await Promise.all(trackPromises);
          const allTracks = completedAlbums.flatMap(res => res.tracks);
          if (allTracks.length > 0) {
            Store.Playback.setQueue(allTracks, 0);
            playAt(0);
          } else {
            alert('No playable tracks found for this artist.');
          }
        } catch (err) {
          alert('Failed to load artist queue.');
        } finally {
          target.textContent = ogText;
          target.style.opacity = '1';
          target.style.pointerEvents = 'auto';
        }
        break;
      }
      case 'play-toggle':
        if (!Store.Playback.getCurrentTrack()) return;
        if (audio.paused) audio.play().catch(() => {}); else audio.pause();
        break;
      case 'prev-track':
        if (audio.currentTime > TRACK_RESTART_THRESHOLD_SECS) {
          audio.currentTime = 0;
        } else if (Store.Playback.getQueueIdx() > 0) {
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
      case 'toggle-repeat-one':
        const nextR1 = !Store.Playback.getRepeatOne();
        Store.Playback.setRepeatOne(nextR1);
        target.classList.toggle('active', nextR1);
        DOM.el('rAllBtn')?.classList.remove('active');
        break;
      case 'toggle-repeat-all':
        const nextRA = !Store.Playback.getRepeatAll();
        Store.Playback.setRepeatAll(nextRA);
        target.classList.toggle('active', nextRA);
        DOM.el('rOneBtn')?.classList.remove('active');
        break;
      case 'logout':
        teardownApp();
        break;
      case 'search-submit':
        executeSearch();
        break;
      case 'connect':
        const sUrl = DOM.el('serverUrl')?.value ?? '';
        const uName = DOM.el('username')?.value ?? '';
        const pWord = DOM.el('password')?.value ?? '';
        if (!sUrl || !uName || !pWord) { alert('Please fill out all fields'); return; }

        target.textContent = 'Connecting…';
        DOM.render('setupError', '');

        try {
          let parsed;
          try { parsed = new URL(sUrl); } catch(err) { throw new Error('Invalid URL format'); }
          if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') throw new Error('Protocol must be HTTP or HTTPS');

          Store.Auth.setAuth(sUrl, uName, pWord);
          await Api.fetch('getAlbumList2', { type: 'alphabeticalByName', size: 1 });
          
          SafeStorage.setItem('firmium_server', sUrl);
          SafeStorage.setItem('firmium_user', uName);
          if (DOM.el('savePassword')?.checked) {
            SafeStorage.setItem('firmium_pass', pWord);
          } else {
            SafeStorage.removeItem('firmium_pass');
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
  });

  document.body.addEventListener('keydown', (e) => {
    if (e.target.id === 'searchInput' && e.key === 'Enter') {
      executeSearch();
    }
  });

  DOM.el('seekBar')?.addEventListener('input', () => Store.Playback.setSeeking(true));
  DOM.el('seekBar')?.addEventListener('change', (e) => {
    audio.currentTime = (Number(e.target.value) / 100) * (audio.duration || 0);
    Store.Playback.setSeeking(false);
  });

  DOM.el('volSlider')?.addEventListener('input', (e) => {
    Store.Playback.setVolume(e.target.value);
  });
});