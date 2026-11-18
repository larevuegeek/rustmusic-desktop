//! La barre latérale : ce qu'affichent ses gabarits, bâti depuis l'état de la page.

use crate::models::sidebar::*;
use karbon_ui::ui::prelude::{Anchor, ElementId};
use crate::models::tracks::TrackMenuAt;
use crate::entity::library::library_pin::LibraryPinView;
use karbon_ui::intl;
use crate::app::music_app::MusicApp;
use crate::models::app::Menu;
use crate::models::app::Msg;

/// Groupes de glisser-déposer du mode « Organiser ».
pub const DRAG_SECTIONS: u64 = 0x5b_01;

pub const DRAG_PLAYLISTS: u64 = 0x5b_02;

/// Sous-titre d'un album épinglé : « Album · Artiste ».
pub fn pin_subtitle(pin: &LibraryPinView) -> String {
    let album = karbon_ui::intl::t("sidebar.album");
    match pin.subtitle.as_deref().filter(|s| !s.is_empty()) {
        Some(s) => format!("{album} · {s}"),
        None => album,
    }
}

pub fn new_count_label(n: i64) -> String {
    let key = if n == 1 { "podcasts.new_one" } else { "podcasts.new_n" };
    format!("{} {}", intl::number(n), karbon_ui::intl::t(key))
}

/// Aperçu hors écran (`--open …`) : menus et mode de la barre. `sb-pl:<id>` (« … » d'une
/// playlist), `sb-pl-at:<id>` (clic droit), `sb-pin:album|artist:<id>`, `sb-zone:album|artist`,
/// `lib-options`, `organize`.
pub fn export_msgs(open: &str) -> Vec<Msg> {
    let named = |t: RowTarget| Msg::Sidebar(SidebarMsg::RowMenu(TrackMenuAt::Named(t.more_id()), t));
    let kind = |k: &str| if k == "artist" { PinKind::Artist } else { PinKind::Album };
    if let Some(id) = open.strip_prefix("sb-pl:").and_then(|i| i.parse().ok()) {
        return vec![named(RowTarget::Playlist(id))];
    }
    if let Some(id) = open.strip_prefix("sb-pl-at:").and_then(|i| i.parse().ok()) {
        return vec![Msg::Sidebar(SidebarMsg::RowMenu(TrackMenuAt::Point(180., 420.), RowTarget::Playlist(id)))];
    }
    if let Some((k, id)) = open.strip_prefix("sb-pin:").and_then(|r| r.split_once(':')) {
        let t = RowTarget::Pin(kind(k), id.to_string());
        // Un artiste n'a pas de « … » : au pointeur.
        return vec![if k == "artist" { Msg::Sidebar(SidebarMsg::RowMenu(TrackMenuAt::Point(120., 520.), t)) } else { named(t) }];
    }
    if let Some(k) = open.strip_prefix("sb-zone:") {
        return vec![Msg::Menu(Menu::Popup(kind(k).zone_id()))];
    }
    match open {
        "lib-options" => vec![Msg::Menu(Menu::Popup("sidebar.library.options"))],
        "organize" => vec![Msg::Sidebar(SidebarMsg::Organize(true))],
        _ => Vec::new(),
    }
}

/// L'ordre de `sidebar_playlists_order`, les inconnus (épinglés
/// depuis) à leur place d'origine, après les rangés.
pub fn order_playlist_keys<T>(data: &crate::models::app::Data, rows: Vec<T>, key: impl Fn(&T) -> &str) -> Vec<T> {
    let lu: Vec<String> = data.setting("sidebar_playlists_order").and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();
    let mut ranks: Vec<(usize, T)> = rows
        .into_iter()
        .enumerate()
        .map(|(i, r)| (lu.iter().position(|k| k == key(&r)).unwrap_or(1000 + i), r))
        .collect();
    ranks.sort_by_key(|(r, _)| *r);
    ranks.into_iter().map(|(_, r)| r).collect()
}

impl PinKind {
    pub fn key(self) -> &'static str {
        match self {
            PinKind::Album => "album",
            PinKind::Artist => "artist",
        }
    }

    /// Identifiant du « … » de la section, et de son menu (`Menu::Popup`).
    pub fn zone_id(self) -> &'static str {
        match self {
            PinKind::Album => "sidebar.pins.album",
            PinKind::Artist => "sidebar.pins.artist",
        }
    }

    /// Ancre du menu de la section : son « … ».
    pub fn anchor(self) -> Anchor {
        Anchor::Element(ElementId::named(self.zone_id()))
    }
}

impl Section {
    pub(crate) const ALL: [Section; 3] = [Section::Playlists, Section::Online, Section::Pins];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Section::Playlists => "playlists",
            Section::Online => "online",
            Section::Pins => "pins",
        }
    }

    pub fn title(self) -> String {
        karbon_ui::intl::t(match self {
            Section::Playlists => "nav.playlists",
            Section::Online => "podcasts.kicker",
            Section::Pins => "sidebar.pinned",
        })
    }
}

impl RowTarget {
    /// Identifiant du bouton « … » de la ligne (ancre du menu).
    pub fn more_id(&self) -> String {
        match self {
            RowTarget::Playlist(id) => format!("sidebar.more.playlist.{id}"),
            RowTarget::Pin(k, id) => format!("sidebar.more.{}.{id}", k.key()),
        }
    }
}

impl MusicApp {
    /// `order` : les sections dans l'ordre choisi, puis les autres.
    pub(crate) fn sidebar_order(&self) -> Vec<Section> {
        let lu: Vec<String> = self.data.setting("sidebar_order").and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();
        let mut out: Vec<Section> = Vec::new();
        for k in &lu {
            if let Some(s) = Section::ALL.iter().find(|s| s.key() == k) {
                if !out.contains(s) {
                    out.push(*s);
                }
            }
        }
        for s in Section::ALL {
            if !out.contains(&s) {
                out.push(s);
            }
        }
        out
    }

    /// Les sections affichées, avec leur rang et les flèches possibles.
    pub fn sidebar_section_rows(&self) -> Vec<crate::models::sidebar::SectionRow> {
        let sections = self.sidebar_sections();
        let count = sections.len();
        sections
            .into_iter()
            .enumerate()
            .map(|(index, section)| crate::models::sidebar::SectionRow {
                index,
                section,
                title: section.title(),
                can_move_up: index > 0,
                can_move_down: index + 1 < count,
            })
            .collect()
    }

    /// `sections` : les sections affichées, dans l'ordre.
    pub fn sidebar_sections(&self) -> Vec<Section> {
        self.sidebar_order()
            .into_iter()
            .filter(|s| match s {
                Section::Playlists => self.data.flag_default_on("show_sidebar_playlists"),
                Section::Online => self.data.flag("podcasts") || self.data.flag("radios"),
                Section::Pins => self.data.library_id.is_some(),
            })
            .collect()
    }

    /// Les podcasts épinglés, par titre.
    pub fn sidebar_podcast_favs(&self) -> Vec<PodcastFav> {
        let mut favs: Vec<PodcastFav> = self
            .data
            .podcasts
            .iter()
            .filter(|p| p.pinned)
            .map(|p| PodcastFav {
                id: p.id,
                title: p.title.clone(),
                subtitle: if p.new_count > 0 {
                    new_count_label(p.new_count)
                } else {
                    p.author.clone().unwrap_or_else(|| karbon_ui::intl::t("podcasts.kicker_podcast"))
                },
                cover: p.cover_path.clone(),
            })
            .collect();
        favs.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        favs
    }

    /// Pastille du « … » des playlists.
    pub fn sidebar_hidden_shortcuts(&self) -> usize {
        let favorites = self.data.flag_default_on("show_favorites");
        usize::from(favorites && self.data.setting("show_liked_in_playlists") == Some("false"))
            + usize::from(self.data.setting("show_recent_in_playlists") == Some("false"))
    }

    /// Les épingles d'un genre, si sa section est affichée.
    pub fn sidebar_pins(&self, kind: PinKind) -> Vec<&LibraryPinView> {
        let show = match kind {
            PinKind::Album => self.data.flag_default_on("show_pinned_albums"),
            PinKind::Artist => self.data.flag_default_on("show_pinned_artists"),
        };
        if !show || self.views.sidebar.pins_library != self.data.library_id {
            return Vec::new();
        }
        self.views.sidebar.pins.iter().filter(|p| p.kind == kind.key()).collect()
    }

    /// `accueil` : rien d'épinglé, carte d'aide pas encore fermée.
    pub fn sidebar_pins_hint(&self) -> bool {
        let albums = self.data.flag_default_on("show_pinned_albums");
        let artists = self.data.flag_default_on("show_pinned_artists");
        !self.sidebar_collapsed
            && !self.data.flag("pins_hint_dismissed")
            && (albums || artists)
            && self.sidebar_pins(PinKind::Album).is_empty()
            && self.sidebar_pins(PinKind::Artist).is_empty()
    }

    /// L'id du morceau en cours dans cette bibliothèque.
    pub fn sidebar_playing(&self, kind: PinKind) -> Option<&str> {
        let place = self.views.sidebar.place.as_ref()?.1.as_ref()?;
        if Some(place.library_id) != self.data.library_id {
            return None;
        }
        match kind {
            PinKind::Album => place.library_album_id.as_deref(),
            PinKind::Artist => place.artist_id.as_deref(),
        }
    }

    /// Pochettes des épingles (vignettes de 160 px, `CoverImg size="1x"`).
    pub(crate) fn sidebar_covers(&self) -> Vec<String> {
        [PinKind::Album, PinKind::Artist]
            .into_iter()
            .flat_map(|k| self.sidebar_pins(k))
            .filter_map(|p| p.cover.as_deref().filter(|c| !c.is_empty()).map(crate::app::resources::assets::small_key))
            .collect()
    }

    /// Clés des lignes de la section Playlists, dans l'ordre affiché.
    pub(crate) fn sidebar_playlist_keys(&self) -> Vec<String> {
        crate::mapper::home::sidebar_playlists(&self.data, self.route()).into_iter().map(|p| p.key).collect()
    }

    /// Ouvre le menu d'une section d'épinglés.
    pub fn sidebar_zone_open(&self, kind: PinKind) -> bool {
        self.open(Menu::Popup(kind.zone_id()))
    }
}
