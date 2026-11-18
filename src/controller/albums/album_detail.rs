//! La fiche d'un album : chargement des données et réponse aux messages.

use crate::models::albums::*;
use crate::mapper::albums::album_detail::*;
use crate::app::music_app::MusicApp;
use crate::helper::random::random_sample;
use crate::commands::library::library_command;
use crate::query::album::album_list_row::AlbumListRow;

impl MusicApp {
    pub(crate) fn album_open(&mut self, id: String) {
        let Some(library) = self.data.library_id else { return };
        let other = self.views.album.as_ref().is_none_or(|p| p.id != id);
        if other {
            self.views.album = Some(AlbumPage {
                id: id.clone(),
                album: None,
                tracks: Vec::new(),
                loading_state: true,
                error: false,
                sort: None,
                asc: true,
                others_by_year: self.views.album.as_ref().is_none_or(|p| p.others_by_year),
                similar: Vec::new(),
                similar_artists: Vec::new(),
            });
        }
        let app = self.backend.app.clone();
        let aid = id.clone();
        self.backend.call(
            async move {
                let album = library_command::get_album(app.state(), aid.clone()).await.ok();
                let tracks = library_command::get_tracks_by_album(app.state(), library, aid).await.unwrap_or_default();
                (album, tracks)
            },
            move |me, (album, tracks)| {
                let Some(page) = me.views.album.as_mut().filter(|p| p.id == id) else { return false };
                page.loading_state = false;
                page.error = album.is_none();
                page.album = album;
                page.tracks = tracks;
                me.album_draw_neighbours();
                true
            },
        );
    }

    /// Dix albums et dix artistes du même genre au hasard, tirés une fois par album.
    pub(crate) fn album_draw_neighbours(&mut self) {
        let Some(page) = self.views.album.as_ref() else { return };
        let Some(album) = page.album.as_ref() else { return };
        let Some(genre) = album.genre.as_ref().map(|g| g.to_lowercase()) else {
            return;
        };
        let same_genre: Vec<&AlbumListRow> = self
            .data
            .albums
            .iter()
            .filter(|x| x.genre.as_ref().is_some_and(|g| g.to_lowercase() == genre) && x.artist_id.as_deref() != Some(album.artist_id.as_str()))
            .collect();
        let similar = random_sample(&same_genre.iter().map(|a| a.id.clone()).collect::<Vec<_>>(), 10);
        let ids: std::collections::HashSet<&str> = same_genre.iter().filter_map(|x| x.artist_id.as_deref()).collect();
        let artists: Vec<String> = self.data.artists.iter().filter(|a| ids.contains(a.id.as_str())).map(|a| a.id.clone()).collect();
        let artists = random_sample(&artists, 10);
        if let Some(page) = self.views.album.as_mut() {
            page.similar = similar;
            page.similar_artists = artists;
        }
    }

    /// `liked.toggleAll` : tous aimés, sinon aucun.
    pub(crate) fn like_all(&mut self, paths: Vec<String>) {
        let like = !paths.iter().all(|p| self.data.liked.contains(p));
        let Some(profile) = self.data.profile_id() else { return };
        for path in paths {
            let already = self.data.liked.contains(&path);
            let app = self.backend.app.clone();
            if like && !already {
                self.data.liked.insert(path.clone());
                self.backend.fire(async move {
                    let _ = crate::commands::playlist::playlist_command::add_track_liked(app.state(), path, profile).await;
                });
            } else if !like {
                self.data.liked.remove(&path);
                self.backend.fire(async move {
                    let _ = crate::commands::playlist::playlist_command::remove_track_liked(app.state(), path, profile).await;
                });
            }
        }
    }

    /// « n » revient à l'ordre du disque ; la même clé inverse le sens.
    pub(crate) fn album_sort(&mut self, key: String) -> bool {
        let Some(page) = self.views.album.as_mut() else { return false };
        if key == "n" {
            page.sort = None;
            page.asc = true;
        } else if page.sort.as_deref() == Some(key.as_str()) {
            page.asc = !page.asc;
        } else {
            page.sort = Some(key);
            page.asc = true;
        }
        true
    }

    pub(crate) fn album_update(&mut self, msg: AlbumMsg) -> bool {
        match msg {
            AlbumMsg::Sort(key) => return self.album_sort(key),
            AlbumMsg::Covers => self.views.without_artwork = !self.views.without_artwork,
            AlbumMsg::OthersByYear(year) => {
                let Some(page) = self.views.album.as_mut() else { return false };
                page.others_by_year = year;
                self.carousel_rewind("autres");
            }
            AlbumMsg::PlayAll | AlbumMsg::Shuffle => {
                let mut tracks = self.album_sorted();
                if self.views.album.as_ref().is_some_and(|p| p.sort.is_some()) {
                    // La lecture suit l'ordre du disque, pas le tri affiché.
                    let page = self.views.album.as_ref().unwrap();
                    tracks = page.tracks.clone();
                    if !tracks.iter().all(|x| x.track_number.is_some()) {
                        tracks.sort_by(disc_order);
                    }
                }
                if msg == AlbumMsg::Shuffle {
                    tracks = random_sample(&tracks, tracks.len());
                }
                if let Some(first) = tracks.first().map(|t| t.path.clone()) {
                    self.play_tracks_view(tracks, &first);
                }
            }
            AlbumMsg::LikeAll => {
                let paths: Vec<String> = self.views.album.as_ref().map(|p| p.tracks.iter().map(|x| x.path.clone()).collect()).unwrap_or_default();
                self.like_all(paths);
            }
        }
        true
    }
}
