
use iced::widget::image::Handle as ImageHandle;
use iced::widget::container;
use iced::{Background, Border, ContentFit, Element, Length, Task};

use crate::icons;

use super::message::Message;
use super::App;

impl App {
    pub(crate) fn offline_home_cover_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .home_recent
            .iter()
            .chain(self.home_newest.iter())
            .chain(self.home_random.iter())
            .filter_map(|a| a.cover_art_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    pub(crate) fn load_covers(&self) -> Task<Message> {
        // Only the first screenful up front; the windowed list loads the rest
        // on scroll (avoids saturating the HTTP client with 100+ requests).
        let ids = self
            .albums
            .iter()
            .take(24)
            .filter_map(|a| a.cover_art_id.clone())
            .collect();
        self.load_cover_ids(ids)
    }

    pub(crate) fn load_cover_ids(&self, ids: Vec<String>) -> Task<Message> {
        let mut tasks = Vec::new();
        for cid in ids {
            if self.cover_cache.contains_key(&cid) {
                continue;
            }
            if cid.starts_with("local:") {
                let state = self.backend.app_state.clone();
                let arg_id = cid.clone();
                tasks.push(Task::perform(
                    firmium_backend::commands::local_library::get_local_cover_art_async(state, arg_id),
                    move |res| Message::CoverLoaded(cid.clone(), res),
                ));
            } else if let Ok(url) = firmium_backend::commands::subsonic::build_cover_url(&self.backend.app_state, &cid, 300) {
                let arg_id = cid.clone();
                tasks.push(Task::perform(
                    firmium_backend::commands::cover_cache::get_cover_art(arg_id, url),
                    move |res| Message::CoverLoaded(cid.clone(), res),
                ));
            }
        }
        Task::batch(tasks)
    }

    pub(crate) fn current_song_id(&self) -> Option<&str> {
        if self.queue_idx >= 0 {
            self.queue.get(self.queue_idx as usize).map(|s| s.id.as_str())
        } else {
            None
        }
    }

    /// Whether the given id is currently starred, checked against whichever
    /// in-memory collection holds it (album detail, queue).
    pub(crate) fn is_starred(&self, id: &str, kind: firmium_backend::commands::subsonic::StarKind) -> bool {
        use firmium_backend::commands::subsonic::StarKind;
        match kind {
            StarKind::Song => {
                self.queue.iter().find(|s| s.id == id).map(|s| s.starred)
                    .or_else(|| self.album_detail.as_ref()?.tracks.iter().find(|s| s.id == id).map(|s| s.starred))
                    .unwrap_or(false)
            }
            StarKind::Album => self.album_detail.as_ref().is_some_and(|at| at.starred),
            StarKind::Artist => false, // artist starring has no local UI state to flip (list-only in Favorites screen).
        }
    }

    /// Flips the starred flag on every in-memory copy of this id (queue, album detail).
    pub(crate) fn set_starred_locally(&mut self, id: &str, kind: firmium_backend::commands::subsonic::StarKind, starred: bool) {
        use firmium_backend::commands::subsonic::StarKind;
        match kind {
            StarKind::Song => {
                for s in self.queue.iter_mut() {
                    if s.id == id { s.starred = starred; }
                }
                if let Some(at) = self.album_detail.as_mut() {
                    for s in at.tracks.iter_mut() {
                        if s.id == id { s.starred = starred; }
                    }
                }
            }
            StarKind::Album => {
                if let Some(at) = self.album_detail.as_mut() {
                    at.starred = starred;
                }
            }
            StarKind::Artist => {}
        }
    }

    pub(crate) fn cover_image(&self, cover_id: Option<&str>, size: f32) -> Element<'_, Message> {
        let t = self.tokens;
        let radius = if size >= 80.0 { 14.0_f32 } else if size >= 40.0 { 10.0 } else { 6.0 };
        if let Some(id) = cover_id {
            if let Some(handle) = self.cover_cache.get(id) {
                return container(
                    iced::widget::image(handle.clone())
                        .width(Length::Fixed(size))
                        .height(Length::Fixed(size))
                        .content_fit(ContentFit::Cover),
                )
                .width(Length::Fixed(size))
                .height(Length::Fixed(size))
                .clip(true)
                .style(move |_| container::Style {
                    border: Border { radius: radius.into(), ..Border::default() },
                    ..container::Style::default()
                })
                .into();
            }
        }
        container(icons::icon(icons::DISC, size * 0.5, t.muted))
            .center_x(Length::Fixed(size))
            .center_y(Length::Fixed(size))
            .style(move |_| container::Style {
                background: Some(Background::Color(t.surface2)),
                border: Border { radius: radius.into(), ..Border::default() },
                ..container::Style::default()
            })
            .into()
    }

    /// Insert a decoded cover handle, evicting the oldest entries once the
    /// in-memory budget is exceeded.
    pub(crate) fn cache_cover(&mut self, id: String, handle: ImageHandle) {
        if self.cover_cache.insert(id.clone(), handle).is_none() {
            self.cover_cache_order.push_back(id);
            while self.cover_cache_order.len() > MAX_COVER_HANDLES {
                if let Some(old) = self.cover_cache_order.pop_front() {
                    self.cover_cache.remove(&old);
                }
            }
        }
    }

    /// Rebuild the deduplicated recent-artists list. Called when
    /// `home_recent_plays` changes, not every frame.
    pub(crate) fn recompute_home_recent_artists(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.home_recent_artists_cache = self
            .home_recent_plays
            .iter()
            .filter_map(|p| {
                let id = p.artist_id.as_ref()?;
                let name = p.artist_name.as_ref()?;
                if seen.insert(id.clone()) {
                    Some((id.clone(), name.clone(), p.cover_art_id.clone()))
                } else {
                    None
                }
            })
            .collect();
    }
}

/// Approximate row heights (cover/avatar + padding) and assumed viewport
/// height, used by the windowed lists.
pub(crate) const ALBUM_ROW_H: f32 = 60.0;
pub(crate) const ARTIST_ROW_H: f32 = 60.0;
pub(crate) const TRACK_ROW_H: f32 = 52.0;
pub(crate) const VIEWPORT_H: f32 = 640.0;

/// Max number of decoded cover-art image handles kept in memory at once.
pub(crate) const MAX_COVER_HANDLES: usize = 512;

/// Visible window for a virtualized list of `total` rows of height `row_h`,
/// given the current `scroll` offset. Returns the first/last visible indices
/// and the spacer heights that stand in for the off-screen rows so the
/// scrollbar stays correct for large lists.
pub(crate) fn list_window(total: usize, scroll: f32, row_h: f32) -> (usize, usize, f32, f32) {
    let first = ((scroll / row_h).floor().max(0.0) as usize).min(total);
    let count = (VIEWPORT_H / row_h).ceil() as usize + 4;
    let end = (first + count).min(total);
    let top = first as f32 * row_h;
    let bottom = total.saturating_sub(end) as f32 * row_h;
    (first, end, top, bottom)
}

pub(crate) fn rgb_to_color(c: firmium_backend::commands::cover_colors::Rgb) -> iced::Color {
    iced::Color::from_rgb8(c.r, c.g, c.b)
}

/// Build the 8-stop gradient LUT the visualizer shaders expect, smoothly
/// cycling `c0 -> c1 -> c2 -> c0` (the same 3-stop palette cycling the Android
/// visualizer uses).
pub(crate) fn ramp8(c0: iced::Color, c1: iced::Color, c2: iced::Color) -> Vec<iced::Color> {
    let lerp = |a: iced::Color, b: iced::Color, t: f32| iced::Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    };
    (0..8)
        .map(|i| {
            let t = i as f32 / 8.0;
            if t < 1.0 / 3.0 {
                lerp(c0, c1, t * 3.0)
            } else if t < 2.0 / 3.0 {
                lerp(c1, c2, (t - 1.0 / 3.0) * 3.0)
            } else {
                lerp(c2, c0, (t - 2.0 / 3.0) * 3.0)
            }
        })
        .collect()
}

/// Decode a cover art file and bake rounded corners into the RGBA pixels.
/// The corner radius is scaled proportionally so that images displayed at
/// 130 px get ~28 px visual radius regardless of the source file's native size.
pub(crate) fn load_rounded_cover(path: &str) -> ImageHandle {
    let img = match image::open(path) {
        Ok(i) => i.into_rgba8(),
        Err(_) => return ImageHandle::from_path(path),
    };
    let w = img.width();
    let h = img.height();
    let r = (14.0 * (w.min(h) as f32 / 130.0))
        .min(w as f32 / 2.0)
        .min(h as f32 / 2.0);
    let mut pixels = img.into_raw();
    for y in 0..h {
        for x in 0..w {
            let xf = x as f32 + 0.5;
            let yf = y as f32 + 0.5;
            let wf = w as f32;
            let hf = h as f32;
            let outside = if xf < r && yf < r {
                let (dx, dy) = (r - xf, r - yf);
                dx * dx + dy * dy > r * r
            } else if xf > wf - r && yf < r {
                let (dx, dy) = (xf - (wf - r), r - yf);
                dx * dx + dy * dy > r * r
            } else if xf < r && yf > hf - r {
                let (dx, dy) = (r - xf, yf - (hf - r));
                dx * dx + dy * dy > r * r
            } else if xf > wf - r && yf > hf - r {
                let (dx, dy) = (xf - (wf - r), yf - (hf - r));
                dx * dx + dy * dy > r * r
            } else {
                false
            };
            if outside {
                pixels[((y * w + x) * 4 + 3) as usize] = 0;
            }
        }
    }
    ImageHandle::from_rgba(w, h, pixels)
}
