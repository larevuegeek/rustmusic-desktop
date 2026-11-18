//! La liste des playlists : ce qu'affichent ses gabarits, bâti depuis l'état de la page.

use crate::models::playlists::*;
use crate::models::app::Choice;
use crate::models::app::Route;
use karbon_ui::intl::t_count;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Msg;
use crate::mapper::albums::natural_cmp;
use crate::mapper::albums::sort_key;
use crate::models::albums::AlbumsMsg;
use crate::models::albums::ViewMode;
use crate::entity::playlist::playlist::Playlist;

impl Default for PlaylistsPage {
    fn default() -> Self {
        Self { search: String::new(), kind: "toutes", sort: "recent" }
    }
}

pub const SORTS: [(&str, &str, &str); 4] = [
    ("recent", "playlists_view.sort_recent", "material-symbols:schedule-outline-rounded"),
    ("name", "playlists_view.sort_name", "material-symbols:sort-by-alpha-rounded"),
    ("size", "playlists_view.sort_size", "material-symbols:format-list-numbered-rounded"),
    ("sidebar", "playlists_view.sort_sidebar", "material-symbols:side-navigation-rounded"),
];

/// Les playlists par nom, taille, ordre de la barre latérale ou date de création (par défaut).
pub(crate) fn sort_playlists(list: &mut [&Playlist], sort: &str) {
    let by_name = |x: &&Playlist, y: &&Playlist| natural_cmp(&sort_key(&x.name), &sort_key(&y.name));
    match sort {
        "name" => list.sort_by(by_name),
        "size" => list.sort_by(|x, y| y.track_count.cmp(&x.track_count).then(by_name(x, y))),
        "sidebar" => list.sort_by(|x, y| x.position.cmp(&y.position).then(y.id.cmp(&x.id))),
        _ => list.sort_by(|x, y| y.created_at.cmp(&x.created_at).then(by_name(x, y))),
    }
}

impl MusicApp {
    pub(crate) fn playlist_tile(&self, p: &Playlist) -> PlaylistTile {
        let genre = t(if p.is_smart { "sidebar.playlist_auto" } else { "sidebar.playlist" });
        let subtitle = if p.track_count == 0 {
            if p.is_smart { genre } else { format!("{genre} · {}", t("sidebar.empty")) }
        } else {
            format!("{genre} · {}", t_count(p.track_count, "home.track_one", "home.tracks_n"))
        };
        PlaylistTile {
            key: format!("playlist:{}", p.id),
            name: p.name.clone(),
            subtitle,
            color: p.color.clone(),
            icon: p.icon.clone(),
            auto: p.is_smart,
            covers: self.data.covers.get(&format!("playlist:{}", p.id)).cloned().unwrap_or_default(),
            route: Route::Playlist(p.id),
            pinned: Some((p.id, p.pinned)),
        }
    }

    pub fn playlists_view(&self) -> PlaylistsView {
        let page = &self.views.playlists;
        let all_items: Vec<&Playlist> = self.data.playlists.iter().filter(|p| !p.is_mix).collect();
        let q = page.search.trim().to_lowercase();
        let mut visible: Vec<&Playlist> = all_items
            .iter()
            .copied()
            .filter(|p| {
                (q.is_empty() || format!("{} {}", p.name, p.description.clone().unwrap_or_default()).to_lowercase().contains(&q))
                    && (page.kind == "toutes" || (page.kind == "auto") == p.is_smart)
            })
            .collect();
        sort_playlists(&mut visible, page.sort);
        let filtres = !q.is_empty() || page.kind != "toutes";
        let app: Vec<PlaylistTile> = visible.iter().filter(|p| p.server_id.is_none()).map(|p| self.playlist_tile(p)).collect();
        let server: Vec<PlaylistTile> = visible.iter().filter(|p| p.server_id.is_some()).map(|p| self.playlist_tile(p)).collect();
        let groups = if app.is_empty() || server.is_empty() {
            vec![(None, app.into_iter().chain(server).collect())]
        } else {
            vec![
                (Some((format!("{} · {}", t("servers.playlists_app"), intl::number(app.len() as i64)), "material-symbols:hard-drive-outline")), app),
                (Some((format!("{} · {}", t("servers.playlists_server"), intl::number(server.len() as i64)), "material-symbols:cloud-outline-rounded")), server),
            ]
        };
        let mut shortcuts = Vec::new();
        if !filtres {
            if self.data.flag_default_on("show_favorites") {
                shortcuts.push(PlaylistTile {
                    key: "liked".into(),
                    name: t("playlist_page.liked_title"),
                    subtitle: format!("{} · {}", t("sidebar.playlist"), t_count(self.data.liked.len() as i64, "home.track_one", "home.tracks_n")),
                    color: "#f43f5e".into(),
                    icon: "material-symbols:favorite-rounded".into(),
                    auto: false,
                    covers: self.data.covers.get("liked").cloned().unwrap_or_default(),
                    route: Route::Liked,
                    pinned: None,
                });
            }
            shortcuts.push(PlaylistTile {
                key: "recent".into(),
                name: t("playlist_page.recent_title"),
                subtitle: format!("{} · {}", t("sidebar.history"), t_count(self.data.recent.len() as i64, "home.track_one", "home.tracks_n")),
                color: "#0ea5e9".into(),
                icon: "material-symbols:history-rounded".into(),
                auto: false,
                covers: self.data.covers.get("recent").cloned().unwrap_or_default(),
                route: Route::Recent,
                pinned: None,
            });
        }
        let pl = |n: usize, one: &str, many: &str| (intl::number(n as i64), t(if n == 1 { one } else { many }));
        let pinned_items = all_items.iter().filter(|p| p.pinned).count();
        let kinds = [("toutes", "playlists_view.kind_all"), ("classiques", "playlists_view.kind_classic"), ("auto", "playlists_view.kind_smart")]
            .into_iter()
            .map(|(k, l)| Choice { label: t(l), icon: String::new(), active: page.kind == k, msg: Msg::Playlists(PlaylistsMsg::Kind(k)), icon_only: false })
            .collect();
        let (sort_label, sort_icon) = SORTS.iter().find(|x| x.0 == page.sort).map(|x| (t(x.1), x.2)).unwrap_or((t(SORTS[0].1), SORTS[0].2));
        let sorts = SORTS
            .iter()
            .map(|x| Choice { label: t(x.1), icon: x.2.into(), active: x.0 == page.sort, msg: Msg::Playlists(PlaylistsMsg::Sort(x.0)), icon_only: false })
            .collect();
        let views = [(ViewMode::Grid, "albums_view.grid", "material-symbols:grid-view-outline-rounded"), (ViewMode::List, "albums_view.list", "material-symbols:view-list-outline-rounded")]
            .into_iter()
            .map(|(mode, l, i)| Choice { label: t(l), icon: i.into(), active: self.views.view_mode == mode, msg: Msg::Albums(AlbumsMsg::View(mode)), icon_only: true })
            .collect();
        PlaylistsView {
            count: pl(all_items.len(), "playlists_view.count_one", "playlists_view.count_n"),
            pinned: pl(pinned_items, "playlists_view.pinned_one", "playlists_view.pinned_n"),
            placeholder: t_with("playlists_view.filter", &[("n", &intl::number(all_items.len() as i64))]),
            kinds,
            sort_label,
            sort_icon,
            sorts,
            views,
            filtres,
            no_match: visible.is_empty() && filtres,
            empty: all_items.is_empty(),
            groups,
            shortcuts,
            remote: self.source_id().is_some_and(|s| s > 0),
        }
    }

    pub(crate) fn playlists_covers(&self) -> Vec<String> {
        let v = self.playlists_view();
        let small = crate::app::resources::assets::small_key;
        let list = self.views.view_mode == ViewMode::List;
        v.shortcuts
            .iter()
            .chain(v.groups.iter().flat_map(|(_, l)| l.iter()))
            .flat_map(|p| {
                if p.covers.len() >= 4 || list {
                    p.covers.iter().take(4).map(|c| small(c)).collect::<Vec<_>>()
                } else {
                    p.covers.iter().take(1).cloned().collect()
                }
            })
            .collect()
    }
}
