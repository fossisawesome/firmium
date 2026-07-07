use iced::Task;
use super::message::Message;
use super::App;

mod auth;
mod library;
mod playlists;
mod search;
mod settings;
mod equalizer;
mod mix;
mod transport;
mod queue_resume;
mod recap;
mod podcasts;
mod nav;
mod visualizer;

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ServerInput(..)
            | Message::UsernameInput(..)
            | Message::PasswordInput(..)
            | Message::ToggleSavePassword(..)
            | Message::Connect
            | Message::Connected(..)
            | Message::CredentialsLoaded(..)
            | Message::ServiceCredsLoaded(..)
            | Message::Logout
            | Message::ToggleAccountSwitcher
            | Message::SwitchAccount(..)
            | Message::AddAccount => self.update_auth(message),
            Message::AlbumsLoaded(..)
            | Message::HomeAlbumsLoaded(..)
            | Message::CoverLoaded(..)
            | Message::AlbumsScrolled(..)
            | Message::ArtistsScrolled(..)
            | Message::AlbumTracksScrolled(..)
            | Message::AlbumTracksLoaded(..)
            | Message::ArtistsLoaded(..)
            | Message::ArtistDetailLoaded(..)
            | Message::ArtistInfoLoaded(..)
            | Message::SimilarArtistsLoaded(..)
            | Message::PlayAlbumAt(..)
            | Message::ShuffleAlbum
            | Message::PlaySong(..)
            | Message::SetRating(..)
            | Message::DownloadTrack(..)
            | Message::DownloadDone(..)
            | Message::GenresLoaded(..)
            | Message::GenreSongsLoaded(..)
            | Message::PlayGenreAt(..)
            | Message::DownloadAlbum => self.update_library(message),
            Message::PlaylistTracksScrolled(..)
            | Message::PlaylistsLoaded(..)
            | Message::PlaylistTracksLoaded(..)
            | Message::PlayPlaylistAt(..)
            | Message::OpenAddToPlaylist(..)
            | Message::CloseAddToPlaylist
            | Message::NewPlaylistNameInput(..)
            | Message::AddToPlaylist(..)
            | Message::CreatePlaylistAndAdd
            | Message::PlaylistSyncNoop
            | Message::OpenCreatePlaylist
            | Message::CloseCreatePlaylist
            | Message::CreatePlaylistNameInput(..)
            | Message::CreatePlaylist(..)
            | Message::PlaylistCreateSynced(..)
            | Message::SyncPlaylistNow(..)
            | Message::DeleteLocalPlaylist(..)
            | Message::RenamePlaylist(..)
            | Message::StartRenamePlaylist(..)
            | Message::CommitRenamePlaylist
            | Message::MovePlaylistTrack(..)
            | Message::RemovePlaylistTrack(..)
            | Message::MoveServerTrack(..)
            | Message::RemoveServerTrack(..) => self.update_playlists(message),
            Message::SearchInput(..)
            | Message::SubmitSearch
            | Message::SearchLoaded(..)
            | Message::SetSearchRatingFilter(..) => self.update_search(message),
            Message::SelectTheme(..)
            | Message::SelectUiTheme(..)
            | Message::SelectFont(..)
            | Message::SetCrossfadeEnabled(..)
            | Message::SetCrossfadeDuration(..)
            | Message::SetGapless(..)
            | Message::SetReplayGain(..)
            | Message::SetAutoContinue(..)
            | Message::SetBitPerfect(..)
            | Message::SetSettingsCategory(..)
            | Message::SetDownloadFormat(..)
            | Message::SetLastfmEnabled(..)
            | Message::SetLastfmKey(..)
            | Message::SetLastfmSecret(..)
            | Message::SetListenbrainzEnabled(..)
            | Message::SetListenbrainzToken(..)
            | Message::SetLrclibEnabled(..)
            | Message::SetLyricsWordFill(..)
            | Message::SetDecorations(..)
            | Message::SetScrollbarWidth(..)
            | Message::WipeCoverCache
            | Message::DeleteSettings => self.update_settings(message),
            Message::SetEqEnabled(..)
            | Message::SetEqProfile(..)
            | Message::EqBandChanged(..)
            | Message::EqNewProfileInput(..)
            | Message::SaveEqProfile
            | Message::DeleteEqProfile(..) => self.update_equalizer(message),
            Message::GenerateMix(..)
            | Message::MixFetched(..) => self.update_mix(message),
            Message::TogglePlay
            | Message::Next
            | Message::Prev
            | Message::ToggleShuffle
            | Message::CycleRepeat
            | Message::SetVolume(..)
            | Message::SeekTo(..)
            | Message::TogglePanel(..)
            | Message::SetVizMode(..)
            | Message::SetVizCoverColors(..)
            | Message::VizColorsLoaded(..)
            | Message::LyricsLoaded(..)
            | Message::SimilarLoaded(..)
            | Message::PlayQueueIndex(..)
            | Message::PlaybackDone(..) => self.update_transport(message),
            Message::SetBarsMonstercat(..)
            | Message::SetBarsWaves(..)
            | Message::SetBarsWavesSmoothing(..)
            | Message::SetBarsGradientMode(..)
            | Message::SetBarsGradientOrientation(..)
            | Message::SetBarsPeakGradientMode(..)
            | Message::SetBarsPeakMode(..)
            | Message::SetBarsPeakHoldTime(..)
            | Message::SetBarsPeakFadeTime(..)
            | Message::SetBarsPeakHeight(..)
            | Message::SetBarsBorderWidth(..)
            | Message::SetBarsLedBars(..)
            | Message::SetBarsLedSegmentHeight(..)
            | Message::SetBarsDepth3d(..)
            | Message::SetBarsFlashIntensity(..)
            | Message::SetBarsMaxBars(..)
            | Message::SetBarsTrails(..)
            | Message::SetBarsEcho(..)
            | Message::SetLinesPointCount(..)
            | Message::SetLinesLineThickness(..)
            | Message::SetLinesOutlineThickness(..)
            | Message::SetLinesOutlineOpacity(..)
            | Message::SetLinesAnimationSpeed(..)
            | Message::SetLinesGradientMode(..)
            | Message::SetLinesFillOpacity(..)
            | Message::SetLinesGlowIntensity(..)
            | Message::SetLinesMirror(..)
            | Message::SetLinesStyle(..)
            | Message::SetLinesTrails(..)
            | Message::SetLinesEcho(..)
            | Message::SetScopeRadius(..)
            | Message::SetScopeSensitivity(..)
            | Message::SetScopePointCount(..)
            | Message::SetScopeLineThickness(..)
            | Message::SetScopeFillOpacity(..)
            | Message::SetScopeGlowIntensity(..)
            | Message::SetScopeOutlineThickness(..)
            | Message::SetScopeOutlineOpacity(..)
            | Message::SetScopeGradientMode(..)
            | Message::SetScopeAnimationSpeed(..)
            | Message::SetScopeStyle(..)
            | Message::SetScopeParticles(..)
            | Message::SetScopeParticleCount(..)
            | Message::SetScopeParticleSpeed(..)
            | Message::SetScopeBeam(..)
            | Message::SetScopeTrails(..)
            | Message::SetScopeEcho(..) => self.update_visualizer(message),
            Message::PlayQueueFetched(..)
            | Message::ResumeQueue
            | Message::DismissResume => self.update_queue_resume(message),
            Message::SetRecapRange(..)
            | Message::RecapNext
            | Message::RecapPrev
            | Message::ExportStats(..)
            | Message::ExportDone(..) => self.update_recap(message),
            Message::PodcastChannelsLoaded(..)
            | Message::OpenAddPodcastModal
            | Message::CloseAddPodcastModal
            | Message::PodcastAddUrlChanged(..)
            | Message::SubmitAddPodcastChannel
            | Message::PodcastChannelAdded(..)
            | Message::PodcastEpisodesLoaded(..)
            | Message::RefreshPodcastChannel(..)
            | Message::PodcastChannelRefreshed(..)
            | Message::UnsubscribePodcastChannel(..)
            | Message::PodcastChannelUnsubscribed(..)
            | Message::PlayPodcastEpisode(..) => self.update_podcasts(message),
            Message::Navigate(..)
            | Message::NavigateBack
            | Message::Backend(..)
            | Message::VisualizerTick
            | Message::ShowToast(..)
            | Message::DismissToast(..)
            | Message::ToastTick => self.update_nav(message),
        }
    }
}
