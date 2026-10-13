// Typed one-line wrappers over the backend commands (src-tauri/src/commands.rs).
// No logic here: argument shaping only.
import { call } from './ipc'
import type * as T from './types'

export const Api = {
  // session
  appVersion: () => call<string>('get_app_version'),
  themes: () => call<T.Theme[]>('list_themes'),
  authState: () => call<T.AuthState>('get_auth_state'),
  autoLogin: () => call<T.AuthState | null>('auto_login'),
  connect: (server: string, username: string, password: string, savePassword: boolean) =>
    call<T.AuthState>('connect', { server, username, password, savePassword }),
  switchAccount: (server: string, username: string) => call<T.AuthState>('switch_account', { server, username }),
  logout: () => call<T.AuthState>('logout'),
  removeAccount: (server: string, username: string) => call<T.AuthState>('remove_account', { server, username }),
  serverExtensions: () => call<string[]>('get_server_extensions'),

  // settings
  settings: () => call<T.Settings>('get_settings'),
  setSetting: <K extends T.SettingKey>(key: K, value: T.Settings[K]) => call<T.Settings>('set_setting', { key, value }),
  pickTlsCert: (kind: T.TlsCertKind) => call<T.Settings | null>('pick_tls_cert', { kind }),
  clearTlsCert: (kind: T.TlsCertKind) => call<T.Settings>('clear_tls_cert', { kind }),
  resetSettings: () => call<T.Settings>('reset_settings'),
  setWindowDecorations: (enabled: boolean) => call('set_window_decorations', { enabled }),

  // library (routed server/offline in Rust)
  albums: () => call<T.Album[]>('get_albums'),
  artists: () => call<T.Artist[]>('get_artists'),
  albumTracks: (id: string) => call<T.AlbumTracks>('get_album_tracks', { id }),
  artistDetails: (id: string) => call<T.ArtistDetails>('get_artist_details', { id }),
  artistInfo: (id: string, artistName: string) => call<T.ArtistInfo | null>('get_artist_info', { id, artistName }),
  artistImage: (id: string) => call<string | null>('get_artist_image', { id }),
  playArtist: (id: string, shuffle: boolean) => call<number>('play_artist', { id, shuffle }),
  artistRecommendations: (id: string, name: string) => call<T.Artist[]>('get_artist_recommendations', { id, name }),
  similarArtists: (id: string, count?: number) => call<string[]>('get_similar_artists', { id, count }),
  search: (query: string) => call<T.SearchResult>('search', { query }),
  albumList: (kind: 'recent' | 'random' | 'newest', size: number) => call<T.Album[]>('get_album_list', { kind, size }),
  genres: () => call<T.Genre[]>('get_genres'),
  songsByGenre: (genre: string, count?: number) => call<T.Song[]>('get_songs_by_genre', { genre, count }),
  lyrics: (song: T.Song) => call<T.Lyrics | null>('get_lyrics', { song }),
  similarTracks: (song: T.Song, count?: number) => call<T.SimilarMatch[]>('get_similar_tracks', { song, count }),
  cover: (coverId: string) => call<string>('get_cover', { coverId }),
  coverColors: (coverId: string) => call<T.CoverColors>('get_cover_colors', { coverId }),
  clearCoverCache: () => call('clear_cover_cache'),

  // favorites & ratings
  setStarred: (id: string, kind: T.StarKind, starred: boolean) => call('set_starred', { id, kind, starred }),
  starred: () => call<T.Starred>('get_starred'),
  setRating: (id: string, rating: number) => call('set_rating', { id, rating }),

  // queue & transport
  setQueue: (songs: T.Song[], startIdx: number) => call('set_queue', { songs, startIdx }),
  setQueueSeamless: (songs: T.Song[], startIdx: number) => call('set_queue_seamless', { songs, startIdx }),
  appendAndPlay: (songs: T.Song[]) => call('append_and_play', { songs }),
  shuffleAndPlay: (songs: T.Song[]) => call('shuffle_and_play', { songs }),
  playIndex: (idx: number) => call('play_queue_index', { idx }),
  next: () => call('queue_next'),
  prev: () => call('queue_prev'),
  togglePlay: () => call('toggle_play'),
  seek: (position: number) => call('seek_queue', { position }),
  toggleShuffle: () => call('toggle_shuffle'),
  setRepeat: (repeatOne: boolean, repeatAll: boolean) => call('set_repeat_mode', { repeatOne, repeatAll }),
  queueState: () => call<T.QueueState>('get_queue_state'),
  shuffleAll: () => call<number>('shuffle_all'),
  generateMix: (energy: T.Energy, genre?: string) => call<number>('generate_mix', { energy, genre }),
  startRadio: (seed: T.Song) => call('start_radio', { seed }),
  remotePlayQueue: () => call<T.RemotePlayQueue | null>('get_remote_play_queue'),
  resumeRemoteQueue: () => call('resume_remote_queue'),
  audioDevices: () => call<T.AudioDevice[]>('list_audio_devices'),

  // equalizer
  eqState: () => call<T.EqState>('get_eq_state'),
  saveEqProfile: (name: string, kind: T.EqMode, bands: T.EqBandSpec[]) => call('save_eq_profile', { name, kind, bands }),
  deleteEqProfile: (name: string) => call('delete_eq_profile', { name }),
  setEqActiveProfile: (device: string, profile: string) => call('set_eq_active_profile', { device, profile }),
  setEqBands: (profile: string, bands: T.EqBandSpec[]) => call('set_eq_bands', { profile, bands }),
  setEqEnabled: (enabled: boolean) => call('set_eq_enabled', { enabled }),

  // playlists (every call returns the updated local list)
  localPlaylists: () => call<T.Playlist[]>('get_local_playlists'),
  syncServerPlaylists: () => call<T.PlaylistLists>('get_server_playlists'),
  addToServerPlaylist: (serverId: string, songIds: string[]) => call('add_to_server_playlist', { serverId, songIds }),
  reorderServerPlaylist: (serverId: string, orderedIds: string[]) => call('reorder_server_playlist', { serverId, orderedIds }),
  removeFromServerPlaylist: (serverId: string, index: number) => call('remove_from_server_playlist', { serverId, index }),
  serverPlaylistTracks: (id: string) => call<T.PlaylistTracks>('get_server_playlist_tracks', { id }),
  createPlaylist: (name: string, firstTrack?: T.Song) => call<T.Playlist[]>('create_playlist', { name, firstTrack }),
  syncPlaylistNow: (localId: string) => call<T.Playlist[]>('sync_playlist_now', { localId }),
  deletePlaylist: (localId: string) => call<T.Playlist[]>('delete_playlist', { localId }),
  renamePlaylist: (localId: string, name: string) => call<T.Playlist[]>('rename_playlist', { localId, name }),
  setPlaylistCover: (localId: string, clear = false) => call<T.Playlist[] | null>('set_playlist_cover', { localId, clear }),
  addToPlaylist: (localId: string, song: T.Song) => call<T.Playlist[]>('add_to_playlist', { localId, song }),
  movePlaylistTrack: (localId: string, from: number, to: number) => call<T.Playlist[]>('move_playlist_track', { localId, from, to }),
  removePlaylistTrack: (localId: string, trackId: string) => call<T.Playlist[]>('remove_playlist_track', { localId, trackId }),

  // downloads & local library
  downloadTrack: (song: T.Song, albumArtist?: string) => call('download_track', { song, albumArtist }),
  downloadAlbum: (albumId: string) => call('download_album', { albumId }),
  localAlbumTrackKeys: (albumArtist: string, album: string) =>
    call<{ trackNumber: number | null; title: string }[]>('get_local_album_track_keys', { albumArtist, album }),
  importLocalFiles: (paths: string[]) => call<number>('import_local_files', { paths }),
  pickAndImport: () => call<number | null>('pick_and_import'),

  // stats
  historySummary: () => call<T.PlayHistorySummary>('get_history_summary'),
  recentPlays: (limit: number) => call<T.RecentPlay[]>('get_recent_plays', { limit }),
  exportHistory: (format: 'csv' | 'json') => call<boolean>('export_history', { format }),

  // podcasts
  podcastChannels: () => call<T.PodcastChannel[]>('get_podcast_channels'),
  podcastEpisodes: (channelId: string) => call<T.PodcastEpisode[]>('get_podcast_episodes', { channelId }),
  addPodcast: (feedUrl: string) => call<T.PodcastChannel>('add_podcast', { feedUrl }),
  refreshPodcast: (channelId: string, feedUrl: string) => call<number>('refresh_podcast', { channelId, feedUrl }),
  unsubscribePodcast: (channelId: string) => call('unsubscribe_podcast', { channelId }),
  playPodcastEpisode: (episode: T.PodcastEpisode, channelTitle: string, coverUrl?: string) =>
    call('play_podcast_episode', { episode, channelTitle, coverUrl }),
}
