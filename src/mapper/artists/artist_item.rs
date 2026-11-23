use crate::models::artists::*;
use karbon_ui::intl;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::query::artist::artist_list_row::ArtistListRow;
use crate::service::library::composer::ComposerView;

pub(crate) fn nb(n: i64, one: &str, many: &str) -> String {
    format!("{n} {}", t(if n == 1 { one } else { many }))
}

pub(crate) fn label(n: i64, one: &str, many: &str) -> String {
    t(if n == 1 { one } else { many })
}

/// `ArtistAvatar` (`two`) : initiales du premier et du dernier mot, teinte du nom.
pub fn avatar(name: &str) -> (String, f32) {
    let words: Vec<&str> = name.split_whitespace().filter(|m| m.chars().any(char::is_alphabetic)).collect();
    let chosen_items: Vec<&str> = match words.as_slice() {
        [] => vec![name.trim()],
        [only] => vec![*only],
        [first, .., last] => vec![*first, *last],
    };
    let initials: String = chosen_items
        .iter()
        .filter_map(|m| m.chars().find(|c| c.is_alphabetic()))
        .flat_map(char::to_uppercase)
        .collect();
    let hue = name.trim().to_lowercase().encode_utf16().fold(7u32, |h, c| (h * 31 + u32::from(c)) % 360);
    (initials, hue as f32)
}

impl ArtistItem {
    /// Ancre du « … » de la carte ou de la ligne.
    pub fn more_id(&self) -> String {
        format!("artist.more.{}", self.id)
    }

    /// Menu de l'artiste, ou du compositeur (ses morceaux).
    pub fn menu(&self) -> Msg {
        let at = crate::models::tracks::TrackMenuAt::Named(self.more_id());
        match &self.route {
            Route::Composer(name) => Msg::CollOpen(at, name.clone(), crate::models::library::CollectionSource::Composer(name.clone())),
            _ => Msg::Sidebar(crate::models::sidebar::SidebarMsg::ItemOpen(at, crate::models::sidebar::PinKind::Artist, self.id.clone(), self.name.clone())),
        }
    }

    pub fn from_view(a: &ArtistListRow) -> Self {
        Self {
            id: a.id.clone(),
            name: a.name.clone(),
            portrait: a.thumbnail_path.clone(),
            avatar: None,
            counts: format!(
                "{} · {}",
                nb(a.total_albums, "library_head.albums_one", "library_head.albums_n"),
                nb(a.total_tracks, "library_head.tracks_one", "library_head.tracks_n")
            ),
            albums_n: a.total_albums.to_string(),
            albums_label: label(a.total_albums, "library_head.albums_one", "library_head.albums_n"),
            tracks_n: a.total_tracks.to_string(),
            tracks_label: label(a.total_tracks, "library_head.tracks_one", "library_head.tracks_n"),
            duration: if a.total_duration > 0. { intl::duration(a.total_duration) } else { String::new() },
            route: Route::Artist(a.id.clone()),
            play: Msg::PlayArtist(a.id.clone()),
            enqueue: Msg::EnqueueArtist(a.id.clone()),
        }
    }

    pub fn from_composer(c: &ComposerView) -> Self {
        let (albums, tracks) = (c.total_albums as i64, c.total_tracks as i64);
        Self {
            id: c.name.clone(),
            name: c.name.clone(),
            portrait: c.thumbnail_path.clone(),
            avatar: c.thumbnail_path.is_none().then(|| avatar(&c.name)),
            counts: format!(
                "{} · {}",
                nb(albums, "library_head.albums_one", "library_head.albums_n"),
                nb(tracks, "library_head.tracks_one", "library_head.tracks_n")
            ),
            albums_n: albums.to_string(),
            albums_label: label(albums, "library_head.albums_one", "library_head.albums_n"),
            tracks_n: tracks.to_string(),
            tracks_label: label(tracks, "library_head.tracks_one", "library_head.tracks_n"),
            duration: if c.total_duration > 0. { intl::duration(c.total_duration) } else { String::new() },
            route: Route::Composer(c.name.clone()),
            play: Msg::PlayComposer(c.name.clone()),
            enqueue: Msg::EnqueueComposer(c.name.clone()),
        }
    }
}
