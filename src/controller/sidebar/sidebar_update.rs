//! La barre latérale : chargement des données et réponse aux messages.

use crate::models::sidebar::*;
use karbon_ui::ui::drag_drop::DropPosition;
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::playlists::PlaylistMenuMsg;
use crate::commands::library::{library_command, pin_command};
use crate::commands::playlist::playlist_command;

/// Place `de` à l'endroit de `to` ; `after` vient de la
/// moitié de la cible où la source a été déposée.
pub(crate) fn moved<T: Clone + PartialEq>(list: &[T], de: usize, to: usize, after: bool) -> Vec<T> {
    let mut out = list.to_vec();
    if de >= out.len() || to >= out.len() || de == to {
        return out;
    }
    let x = out.remove(de);
    let mut target = to;
    if de < to && !after {
        target -= 1;
    } else if de > to && after {
        target += 1;
    }
    out.insert(target.min(out.len()), x);
    out
}

impl MusicApp {
    pub fn sidebar_pin_route(kind: &str, id: &str) -> Route {
        if kind == "album" {
            Route::Album(id.to_string())
        } else {
            Route::Artist(id.to_string())
        }
    }

    pub(crate) fn sidebar_load_pins(&mut self) {
        let Some(library) = self.data.library_id else {
            self.views.sidebar.pins_library = None;
            self.views.sidebar.pins.clear();
            return;
        };
        let app = self.backend.app.clone();
        self.backend.call(async move { pin_command::get_library_pins(app.state(), library).await }, move |me, r| {
            // Une réponse doublée par un changement de bibliothèque est jetée.
            if me.data.library_id != Some(library) {
                return false;
            }
            match r {
                Ok(pins) => {
                    me.views.sidebar.pins_library = Some(library);
                    me.views.sidebar.pins = pins;
                }
                Err(e) => log::error!("Épingles : {e}"),
            }
            true
        });
    }

    /// `locateOne(cheminEnLecture)` quand le morceau change ; `true` si la vue change.
    pub(crate) fn sidebar_sync(&mut self) -> bool {
        let path_part = self.data.player.path.clone();
        let known = self.views.sidebar.place.as_ref().map(|(p, _)| p.clone());
        match path_part {
            None => self.views.sidebar.place.take().is_some(),
            Some(p) if known.as_deref() == Some(p.as_str()) => false,
            Some(p) => {
                self.views.sidebar.place = Some((p.clone(), None));
                let app = self.backend.app.clone();
                let paths = vec![p.clone()];
                self.backend.call(async move { library_command::get_track_locations(app.state(), paths).await.unwrap_or_default() }, move |me, places| {
                    let Some((current_item, place)) = me.views.sidebar.place.as_mut() else { return false };
                    if *current_item != p {
                        return false;
                    }
                    *place = places.into_iter().next();
                    true
                });
                false
            }
        }
    }

    pub(crate) fn sidebar_update(&mut self, msg: SidebarMsg) -> bool {
        match msg {
            SidebarMsg::RowMenu(at, RowTarget::Playlist(id)) => return self.playlist_menu_update(PlaylistMenuMsg::Open(at, id)),
            SidebarMsg::RowMenu(at, RowTarget::Pin(kind, id)) => return self.sidebar_item_open(at, kind, id, None),
            SidebarMsg::ItemOpen(at, kind, id, title) => return self.sidebar_item_open(at, kind, id, Some(title)),
            SidebarMsg::Item(m) => return self.sidebar_item_update(m),
            SidebarMsg::PinPlaying(kind) => return self.sidebar_pin_playing(kind),
            SidebarMsg::Browse(kind) => {
                self.menu = None;
                let section = match kind {
                    PinKind::Album => crate::models::app::Section::Albums,
                    PinKind::Artist => crate::models::app::Section::Artists,
                };
                return self.handle(Msg::Go(Route::Library(section)));
            }
            SidebarMsg::HideSection(kind) => {
                self.menu = None;
                self.set_setting(
                    match kind {
                        PinKind::Album => "show_pinned_albums",
                        PinKind::Artist => "show_pinned_artists",
                    },
                    "false",
                );
            }
            SidebarMsg::DismissHint => self.set_setting("pins_hint_dismissed", "true"),
            SidebarMsg::Organize(on) => {
                self.menu = None;
                self.views.sidebar.organize = on && !self.sidebar_collapsed;
            }
            SidebarMsg::Shift(i, up) => {
                let sections = self.sidebar_sections();
                let Some(to) = (if up { i.checked_sub(1) } else { Some(i + 1) }).and_then(|j| sections.get(j)) else { return false };
                self.sidebar_move_section(sections[i], *to, !up);
            }
            SidebarMsg::DropSection(e) => {
                let sections = self.sidebar_sections();
                let (Some(de), Some(to)) = (sections.get(e.source as usize), sections.get(e.target as usize)) else { return false };
                self.sidebar_move_section(*de, *to, e.position == DropPosition::After);
            }
            SidebarMsg::DropPlaylist(e) => {
                let keys = moved(&self.sidebar_playlist_keys(), e.source as usize, e.target as usize, e.position == DropPosition::After);
                self.set_setting("sidebar_playlists_order", serde_json::to_string(&keys).unwrap_or_default());
                // Les playlists gardent aussi cet ordre en base (page Playlists, tri « comme la barre »).
                let ids: Vec<i64> = keys.iter().filter_map(|k| k.strip_prefix("playlist:")?.parse().ok()).collect();
                let pinned_items: Vec<i64> = self.data.playlists.iter().filter(|p| p.pinned && !p.is_mix).map(|p| p.id).collect();
                if ids != pinned_items {
                    if let Some(profile) = self.data.profile_id() {
                        let app = self.backend.app.clone();
                        self.backend.call(async move { playlist_command::reorder_playlists(app.state(), profile, ids).await }, |me, r| {
                            if let Err(e) = r {
                                log::warn!("Ordre des playlists : {e}");
                            }
                            me.load_playlists();
                            false
                        });
                    }
                }
            }
        }
        true
    }

    /// Les sections masquées gardent leur rang.
    pub(crate) fn sidebar_move_section(&mut self, de: Section, to: Section, after: bool) {
        let order = self.sidebar_order();
        let (Some(i), Some(j)) = (order.iter().position(|s| *s == de), order.iter().position(|s| *s == to)) else { return };
        let list: Vec<&str> = moved(&order, i, j, after).into_iter().map(Section::key).collect();
        self.set_setting("sidebar_order", serde_json::to_string(&list).unwrap_or_default());
    }
}
