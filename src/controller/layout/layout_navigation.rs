//! Chargement de la page de la route courante.

use crate::models::tracks::*;
use crate::mapper::tracks::track_page::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::t_count;
use crate::models::app::Route;
use crate::models::app::Section;
use crate::commands::library::library_command;
use crate::entity::queue::queue_track::QueueTrack;
use std::rc::Rc;

impl MusicApp {
    /// Charge la page de la route courante (après une navigation ou un changement de
    /// bibliothèque).
    pub(crate) fn load_page(&mut self) {
        let route = self.route().clone();
        match &route {
            Route::Library(Section::Tracks) => return self.tracks_open(),
            Route::Library(Section::Albums) => return self.albums_open(),
            Route::Album(id) => return self.album_open(id.clone()),
            Route::Library(Section::Artists) => return self.artists_open(),
            Route::Artist(id) => return self.artist_open(id.clone()),
            Route::Library(Section::Genres) => return self.genres_open(),
            Route::Genre(name) => return self.genre_open(name.clone()),
            Route::Composer(name) => return self.composer_open(name.clone()),
            Route::Library(Section::Years) => return self.years_open(),
            Route::Decade(d) => return self.period_open(*d),
            Route::Library(Section::Folders) => return self.folders_open(),
            Route::Library(Section::Mixes) => return self.mixes_open(),
            Route::Playlists => return,
            Route::Playlist(id) => return self.playlist_open(*id),
            Route::Liked => return self.liked_open(),
            Route::Recent => return self.recent_open(),
            Route::Search(q) => return self.search_open(q.clone()),
            Route::Radios => return self.radios_open(),
            Route::Podcasts | Route::PodcastSubscriptions => return self.podcasts_open(),
            Route::Podcast(id) => return self.podcast_open(*id),
            Route::PodcastDownloads => return self.downloads_open(),
            Route::Stats => return,
            Route::Settings(p) => return self.settings_open(*p),
            Route::Import | Route::Feedback => return,
            Route::Track(id) => return self.track_open(id.clone()),
            Route::Tags(album) => return self.tags_open(album.clone()),
            _ => {}
        }
        // Toutes les autres routes ont leur page : restent l'accueil (rien à charger) et un
        // dossier ouvert, dont les titres sont lus ici.
        let Route::Folder(id, name) = route.clone() else {
            let mut page = Page::new(route, PageKind::Loading);
            page.kind = PageKind::Placeholder;
            self.data.page = Some(page);
            return;
        };
        self.data.page = Some(Page::new(route.clone(), PageKind::Loading));
        let profile = self.data.profile_id().unwrap_or(0);
        let lib = self.data.library_id.unwrap_or(0);
        let app = self.backend.app.clone();
        self.backend.call(
            async move { library_command::get_tracks_by_dir(app.state(), lib, id).await.unwrap_or_default() },
            move |me, tracks| {
                if *me.route() != route {
                    return true;
                }
                let (queue, albums) = from_views(profile, tracks);
                let mut page = Page::new(route, PageKind::Tracks);
                page.kicker = "Dossier".into();
                page.title = name;
                me.data.page = Some(page.with_tracks(queue, &albums));
                true
            },
        );
    }
}

impl Page {
    pub(crate) fn with_tracks(mut self, tracks: Vec<QueueTrack>, albums: &[String]) -> Self {
        self.rows = Rc::new(
            tracks
                .iter()
                .zip(albums.iter().chain(std::iter::repeat(&String::new())))
                .enumerate()
                .map(|(n, (t, album))| TrackRow {
                    n,
                    title: t.title.clone(),
                    artist: t.artist.clone().unwrap_or_default(),
                    album: album.clone(),
                    duration: t.duration.map(intl::clock).unwrap_or_default(),
                })
                .collect(),
        );
        if self.subtitle.is_empty() {
            let total: f64 = tracks.iter().filter_map(|t| t.duration).sum();
            self.subtitle = format!("{} · {}", t_count(tracks.len() as i64, "home.track_one", "home.tracks_n"), intl::duration(total));
        }
        self.queue = tracks;
        self.kind = PageKind::Tracks;
        self
    }
}
