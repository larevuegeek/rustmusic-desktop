//! La fiche d'une playlist : ce qu'affichent ses gabarits, bâti depuis l'état de la page,
//! et les pochettes à charger.

use crate::models::playlists::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::t;
use crate::models::app::Route;
use crate::mapper::albums::album_detail::sort_tracks;
use crate::models::albums::AlbumTrack;
use crate::models::albums::SortChoice;
use crate::models::albums::ViewMode;
use crate::query::track::track_list_row::TrackListRow;

impl MusicApp {
    /// L'ordre affiché : celui de la playlist (ou inversé), sinon une colonne.
    pub(crate) fn playlist_sorted(&self) -> Vec<TrackListRow> {
        let Some(page) = self.views.playlist.as_ref() else { return Vec::new() };
        let mut l = page.tracks.clone();
        if page.sort == "order" {
            if !page.asc {
                l.reverse();
            }
        } else {
            sort_tracks(&mut l, &page.sort, page.asc);
        }
        l
    }

    pub fn playlist_view(&self) -> Option<PlaylistView> {
        let page = self.views.playlist.as_ref()?;
        let sorted = self.playlist_sorted();
        let mut covers: Vec<String> = Vec::new();
        for c in page.tracks.iter().filter_map(|x| x.thumbnail_path.clone()) {
            if !covers.contains(&c) {
                covers.push(c);
                if covers.len() == 4 {
                    break;
                }
            }
        }
        let duration: f64 = page.tracks.iter().map(|x| x.duration.unwrap_or(0.)).sum();
        let n = page.tracks.len() as i64;
        let arrow = |k: &str| (page.sort == k).then_some(if page.asc { "material-symbols:arrow-upward-rounded" } else { "material-symbols:arrow-downward-rounded" });
        let sorts = [("order", "playlist_page.sort_order"), ("title", "artist_view.sort_title"), ("artist", "genre_view.sort_artist"), ("album", "playlist_page.sort_album")]
            .into_iter()
            .map(|(k, l)| SortChoice { key: k, label: t(l), active: page.sort == k, arrow: arrow(k) })
            .collect();
        let (mut left, mut right, mut table) = (Vec::new(), Vec::new(), None);
        if self.views.view_mode == ViewMode::Grid {
            let playing = self.data.player.path.as_deref();
            let mut rows: Vec<AlbumTrack> = sorted
                .iter()
                .enumerate()
                .map(|(i, x)| AlbumTrack {
                    i,
                    number: (i + 1).to_string(),
                    numbered: true,
                    cover: x.thumbnail_path.clone(),
                    title: x.title.clone(),
                    invite: Some([x.artist.clone(), x.album.clone()].into_iter().flatten().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")).filter(|s| !s.is_empty()),
                    duration: x.duration.map(intl::clock).unwrap_or_default(),
                    liked: self.data.liked.contains(&x.path),
                    in_progress: playing == Some(x.path.as_str()),
                    artwork: true,
                    mark: false,
                })
                .collect();
            right = rows.split_off(rows.len().div_ceil(2));
            left = rows;
        } else {
            table = Some(self.track_table(&sorted, false, (page.sort != "order").then_some(page.sort.as_str()), page.asc, page.sort == "order"));
        }
        let (playlist_id, pinned) = match page.route {
            Route::Playlist(id) => (Some(id), self.data.playlists.iter().find(|p| p.id == id).is_some_and(|p| p.pinned)),
            _ => (None, false),
        };
        let empty = match page.route {
            Route::Liked => (t("playlist_page.empty_liked"), t("playlist_page.empty_liked_desc")),
            Route::Recent => (t("playlist_page.empty_recent"), t("playlist_page.empty_recent_desc")),
            _ => (t("playlist_page.empty_playlist"), t("playlist_page.empty_playlist_desc")),
        };
        let remove = match page.route {
            Route::Liked => Some(t("playlist_page.remove_liked")),
            Route::Playlist(_) if !page.smart => Some(t("playlist_page.remove_playlist")),
            _ => None,
        };
        let tint = covers.first().and_then(|c| self.assets.as_ref()?.covers.get(c)).map(|img| img.dominant_hue()).unwrap_or((150., 0.));
        let confirm = page.confirm.then(|| {
            let key = if n == 1 { "playlist_page.delete_confirm_one" } else { "playlist_page.delete_confirm_n" };
            let text = t(key).replace("{n}", &intl::number(n));
            let (a, b) = text.split_once("{name}").map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or((text.clone(), String::new()));
            (a, b)
        });
        Some(PlaylistView {
            title: page.title.clone(),
            title_long: page.title.chars().count() > 24,
            kicker: page.kicker.clone(),
            color: page.color.clone(),
            icon: page.icon.clone(),
            tint,
            covers,
            count: (intl::number(n), t(if n == 1 { "library_head.tracks_one" } else { "library_head.tracks_n" })),
            duration: if duration > 0. { intl::duration(duration) } else { String::new() },
            sorts,
            left,
            right,
            table,
            empty,
            remove,
            playlist_id,
            pinned,
            smart: page.smart,
            confirm,
        })
    }

    pub(crate) fn playlist_covers(&self) -> Vec<String> {
        let Some(v) = self.playlist_view() else { return Vec::new() };
        let small = crate::app::resources::assets::small_key;
        let mut paths: Vec<String> = if v.covers.len() >= 4 { v.covers.iter().map(|c| small(c)).collect() } else { v.covers.iter().take(1).cloned().collect() };
        paths.extend(v.left.iter().chain(&v.right).filter_map(|x| x.cover.as_deref().map(small)));
        if let Some(tb) = &v.table {
            paths.extend(tb.rows.iter().filter_map(|r| r.cover.as_deref().map(small)));
        }
        paths
    }
}
