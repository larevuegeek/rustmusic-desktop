//! Les mix : chargement des données et réponse aux messages.

use crate::models::mixes::*;
use chrono::Utc;
use crate::app::music_app::MusicApp;
use crate::models::app::MixDraw;
use karbon_ui::intl::t_with;
use crate::models::app::Menu;
use crate::commands::library::library_command;
use crate::commands::playlist::{playlist_command, playlist_view_command};

/// Taille d'un tirage.
pub(crate) const SIZE: i64 = 50;

impl MusicApp {
    /// Tire les mix qui n'ont pas encore leur liste du jour.
    pub(crate) fn mixes_open(&mut self) {
        let Some(library) = self.data.library_id else { return };
        let (a, b, c, d) = self.mix_specs();
        for spec in a.into_iter().chain(b).chain(c).chain(d) {
            if self.data.mixes.contains_key(&spec.key) {
                continue;
            }
            self.data.mixes.insert(spec.key.clone(), MixDraw::default());
            let app = self.backend.app.clone();
            let key = spec.key.clone();
            let source = spec.source.clone();
            self.backend.call(
                async move {
                    match source {
                        MixSource::Playlist(id) => playlist_view_command::get_playlist_tracks_view(app.state(), id).await,
                        MixSource::Kind { kind, genre, decade } => {
                            library_command::get_mix_tracks(app.state(), library, kind.into(), genre, decade, None, SIZE).await
                        }
                    }
                },
                move |me, tracks| {
                    if me.data.library_id != Some(library) {
                        return false;
                    }
                    me.data.mixes.insert(key, MixDraw { tracks: tracks.unwrap_or_default(), loaded: true });
                    true
                },
            );
        }
    }

    pub(crate) fn mixes_update(&mut self, msg: MixesMsg) -> bool {
        match msg {
            MixesMsg::Play(key) => {
                let tracks = self.data.mixes.get(&key).map(|d| d.tracks.clone()).unwrap_or_default();
                if let Some(first) = tracks.first().map(|t| t.path.clone()) {
                    self.play_tracks_view(tracks, &first);
                }
            }
            MixesMsg::Menu(key) => {
                let open = self.menu == Some(Menu::Popup("mix")) && self.views.mix_menu.as_deref() == Some(key.as_str());
                self.menu = if open { None } else { Some(Menu::Popup("mix")) };
                self.views.mix_menu = Some(key);
            }
            MixesMsg::Redraw(key) => {
                self.menu = None;
                match key {
                    Some(c) => {
                        self.data.mixes.remove(&c);
                    }
                    None => self.data.mixes.clear(),
                }
                self.mixes_open();
                self.draw_mixes();
            }
            MixesMsg::Keep(key) => {
                self.menu = None;
                let Some(profile) = self.data.profile_id() else { return false };
                let tracks = self.data.mixes.get(&key).map(|d| d.tracks.clone()).unwrap_or_default();
                if tracks.is_empty() {
                    return false;
                }
                let (a, b, c, d) = self.mix_specs();
                let name = a.into_iter().chain(b).chain(c).chain(d).find(|m| m.key == key).map(|m| m.name).unwrap_or_default();
                const MONTHS: [&str; 12] = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];
                let now = Utc::now().with_timezone(&chrono::Local);
                let date = format!("{} {}", chrono::Datelike::day(&now), MONTHS[chrono::Datelike::month0(&now) as usize]);
                let name = t_with("mix_view.keep_name", &[("name", &name), ("date", &date)]);
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        let payload = playlist_command::CreatePlaylistPayload {
                            profile_id: profile,
                            name,
                            description: None,
                            color: "#8b5cf6".into(),
                            icon: String::new(),
                            server_id: None,
                            source_id: None,
                        };
                        if let Ok(p) = playlist_command::create_playlist(app.state(), payload).await {
                            for x in tracks {
                                let _ = playlist_command::add_track_to_playlist(app.state(), p.id, Some(x.id), None).await;
                            }
                        }
                    },
                    |me, ()| {
                        me.load_playlists();
                        true
                    },
                );
            }
            MixesMsg::Delete(id) => {
                self.menu = None;
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        let _ = playlist_command::delete_playlist(app.state(), id).await;
                    },
                    |me, ()| {
                        me.load_playlists();
                        true
                    },
                );
            }
        }
        true
    }
}
