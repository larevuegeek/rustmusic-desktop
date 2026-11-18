//! La fiche d'un artiste : chargement des données et réponse aux messages.

use crate::models::artists::*;
use std::collections::HashSet;
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::commands::library::library_command;

/// « Various Artists » n'est pas un artiste voisin.
pub(crate) fn compilation(name: &str) -> bool {
    matches!(
        name.trim().to_lowercase().as_str(),
        "various artists" | "various artist" | "va" | "artistes divers" | "artiste divers" | "compilation" | "compilations"
    )
}

impl MusicApp {
    pub(crate) fn artist_open(&mut self, id: String) {
        let Some(library) = self.data.library_id else { return };
        if self.views.artist.as_ref().is_none_or(|p| p.id != id) {
            self.views.artist = Some(ArtistPage {
                id: id.clone(),
                artist: None,
                portrait: None,
                tracks: Vec::new(),
                albums: Vec::new(),
                similar: Vec::new(),
                loading_tracks: false,
                loading_albums: false,
                error: false,
                sort: "year".into(),
                asc: false,
                open_items: HashSet::new(),
            });
        }
        let app = self.backend.app.clone();
        let aid = id.clone();
        let auto = self.data.setting("auto_download_artist_images") != Some("false");
        self.backend.call(
            async move {
                let artist = library_command::get_artist(app.state(), aid.clone()).await.ok();
                let Some(a) = artist.clone() else { return (None, Vec::new(), Vec::new(), Vec::new(), None) };
                let tracks = library_command::get_tracks_by_artist(app.state(), library, a.id.clone()).await.unwrap_or_default();
                let albums = library_command::get_albums_by_artist(app.state(), library, a.id.clone()).await.unwrap_or_default();
                let similar = library_command::get_similar_artists(app.state(), library, a.id.clone(), Some(12)).await.unwrap_or_default();
                // Portrait depuis Deezer : seul appel réseau automatique, mis en cache en base.
                let portrait = if auto {
                    library_command::fetch_artist_image(app.state(), a.id.clone(), a.name.clone()).await.ok().flatten()
                } else {
                    None
                };
                (artist, tracks, albums, similar, portrait)
            },
            move |me, (artist, tracks, albums, similar, portrait)| {
                let Some(page) = me.views.artist.as_mut().filter(|p| p.id == id) else { return false };
                page.error = artist.is_none();
                page.portrait = portrait.or_else(|| artist.as_ref().and_then(|a| a.thumbnail_path.clone()));
                page.artist = artist;
                page.tracks = tracks;
                page.albums = albums;
                page.similar = similar.into_iter().filter(|x| !compilation(&x.name)).take(10).collect();
                page.loading_tracks = true;
                page.loading_albums = true;
                me.artist_open_first();
                true
            },
        );
    }

    pub(crate) fn artist_open_first(&mut self) {
        let first = self.artist_groups().first().map(|g| g.key.clone());
        if let Some(page) = self.views.artist.as_mut() {
            page.open_items = first.into_iter().collect();
        }
    }

    /// La même clé inverse le sens ; l'année part de la plus récente.
    pub(crate) fn artist_sort(&mut self, key: String, from_table: bool) -> bool {
        let Some(page) = self.views.artist.as_mut() else { return false };
        if page.sort == key {
            page.asc = !page.asc;
        } else {
            page.asc = from_table || key != "year";
            page.sort = key;
        }
        self.artist_open_first();
        true
    }

    pub(crate) fn artist_update(&mut self, msg: ArtistMsg) -> bool {
        match msg {
            ArtistMsg::Sort(key) => return self.artist_sort(key, false),
            ArtistMsg::Toggle(key) => {
                let Some(page) = self.views.artist.as_mut() else { return false };
                if !page.open_items.remove(&key) {
                    page.open_items.insert(key);
                }
            }
            ArtistMsg::ToggleAll => {
                let keys: Vec<String> = self.artist_groups().into_iter().map(|g| g.key).collect();
                let Some(page) = self.views.artist.as_mut() else { return false };
                if keys.iter().all(|c| page.open_items.contains(c)) {
                    page.open_items.clear();
                } else {
                    page.open_items = keys.into_iter().collect();
                }
            }
            ArtistMsg::GoTo(id) => {
                if !self.artist_group() {
                    if let Some(page) = self.views.artist.as_mut() {
                        page.sort = "year".into();
                        page.asc = false;
                    }
                }
                if !self.artist_groups().iter().any(|g| g.key == id) {
                    return self.handle(Msg::Go(Route::Album(id)));
                }
                if let Some(page) = self.views.artist.as_mut() {
                    page.open_items.insert(id);
                }
            }
            ArtistMsg::Covers => self.views.artist_artwork = !self.views.artist_artwork,
            ArtistMsg::PlayAll | ArtistMsg::Shuffle => {
                let tracks = if msg == ArtistMsg::Shuffle {
                    let t = self.views.artist.as_ref().map(|p| p.tracks.clone()).unwrap_or_default();
                    crate::helper::random::random_sample(&t, t.len())
                } else {
                    self.artist_order()
                };
                if let Some(first) = tracks.first().map(|t| t.path.clone()) {
                    self.play_tracks_view(tracks, &first);
                }
            }
            ArtistMsg::PlayGroup(key) => {
                let Some(g) = self.artist_groups().into_iter().find(|g| g.key == key) else { return false };
                if let Some(first) = g.tracks_list.first().map(|t| t.path.clone()) {
                    self.play_tracks_view(g.tracks_list, &first);
                }
            }
            ArtistMsg::LikeAll => {
                let paths: Vec<String> = self.views.artist.as_ref().map(|p| p.tracks.iter().map(|x| x.path.clone()).collect()).unwrap_or_default();
                self.like_all(paths);
            }
        }
        true
    }
}
