//! Les playlists de l'accueil et de la barre latérale.

use crate::models::home::*;
use crate::models::app::Data;
use karbon_ui::intl::{t, t_count};
use crate::models::app::Route;

pub(crate) fn playlist_subtitle(p: &crate::entity::playlist::playlist::Playlist) -> String {
    let genre = t(if p.is_smart { "sidebar.playlist_auto" } else { "home.playlist" });
    if p.track_count == 0 {
        if p.is_smart { genre } else { format!("{genre} · {}", t("sidebar.empty")) }
    } else {
        format!("{genre} · {}", t_count(p.track_count, "home.track_one", "home.tracks_n"))
    }
}

/// Les épinglées d'abord, puis les plus récentes, 12 au plus.
pub(crate) fn home_playlists(data: &Data) -> Vec<PlaylistRow> {
    let mut playlists: Vec<_> = data.playlists.iter().filter(|p| !p.is_mix).collect();
    playlists.sort_by(|x, y| y.pinned.cmp(&x.pinned).then(y.created_at.cmp(&x.created_at)));
    playlists
        .into_iter()
        .take(12)
        .map(|p| PlaylistRow {
            key: format!("playlist:{}", p.id),
            name: p.name.clone(),
            subtitle: playlist_subtitle(p),
            color: p.color.clone(),
            icon: p.icon.clone(),
            auto: p.is_smart,
            covers: data.covers.get(&format!("playlist:{}", p.id)).cloned().unwrap_or_default(),
            route: Route::Playlist(p.id),
            active: false,
        })
        .collect()
}

/// Les épinglées, puis Titres likés et Récemment
/// joués, dans l'ordre de `sidebar_playlists_order`.
pub fn sidebar_playlists(data: &Data, route: &Route) -> Vec<PlaylistRow> {
    let mut rows: Vec<PlaylistRow> = data
        .playlists
        .iter()
        .filter(|p| p.pinned && !p.is_mix)
        .map(|p| PlaylistRow {
            key: format!("playlist:{}", p.id),
            name: p.name.clone(),
            subtitle: playlist_subtitle(p),
            color: p.color.clone(),
            icon: p.icon.clone(),
            auto: p.is_smart,
            covers: data.covers.get(&format!("playlist:{}", p.id)).cloned().unwrap_or_default(),
            route: Route::Playlist(p.id),
            active: *route == Route::Playlist(p.id),
        })
        .collect();
    if data.flag_default_on("show_favorites") && data.flag_default_on("show_liked_in_playlists") {
        rows.push(PlaylistRow {
            key: "liked".into(),
            name: t("home.liked_tracks_plural"),
            subtitle: format!("{} · {}", t("home.playlist"), t_count(data.liked.len() as i64, "home.track_one", "home.tracks_n")),
            color: "#f43f5e".into(),
            icon: "material-symbols:favorite-rounded".into(),
            auto: false,
            covers: data.covers.get("liked").cloned().unwrap_or_default(),
            route: Route::Liked,
            active: *route == Route::Liked,
        });
    }
    if data.flag_default_on("show_recent_in_playlists") {
        rows.push(PlaylistRow {
            key: "recent".into(),
            name: t("home.recently_played"),
            subtitle: format!("{} · {}", t("sidebar.history"), t_count(data.recent.len() as i64, "home.track_one", "home.tracks_n")),
            color: "#0ea5e9".into(),
            icon: "material-symbols:history-rounded".into(),
            auto: false,
            covers: data.covers.get("recent").cloned().unwrap_or_default(),
            route: Route::Recent,
            active: *route == Route::Recent,
        });
    }
    crate::mapper::sidebar::order_playlist_keys(data, rows, |r| r.key.as_str())
}
