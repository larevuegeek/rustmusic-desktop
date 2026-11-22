//! Démarrage : profil, bibliothèques, playlists, récents, sorties audio, mix.

use std::time::Instant;
use chrono::Utc;
use crate::models::app::MixDraw;
use crate::models::updates::WhatsNewMsg;
use crate::models::layout::BannerMsg;
use crate::models::home::ViewModel;
use crate::commands::app::{profile_command, settings_command, system_command};
use crate::commands::audio::{device_command, wasapi_command};
use crate::commands::library::{library_command, sidebar_command};
use crate::commands::network::server_command;
use crate::commands::online::{podcast_command, radio_command};
use crate::commands::playback::{player_command, queue_command, recent_command, volume_command};
use crate::commands::playlist::playlist_command;
use super::*;

impl MusicApp {
    // ─── Démarrage ──────────────────────────────────────────────────────────

    /// Au démarrage : réglages, profil, bibliothèques, file, puis le reste.
    pub(crate) fn start(&mut self) {
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let settings = settings_command::get_all_settings(app.state()).await;
                let profiles = profile_command::get_all_profiles(app.state()).await;
                (settings, profiles)
            },
            |me, (settings, profiles)| {
                match settings {
                    Ok(s) => me.data.settings = s,
                    Err(e) => log::error!("Réglages : {e}"),
                }
                me.sidebar_collapsed = me.data.flag("sidebar_collapsed");
                crate::app::resources::translations::set_language(me.data.setting("language").unwrap_or("fr"));
                // Menu de l'icône dans la langue choisie.
                me.tray_refresh();
                match profiles {
                    Ok(p) => {
                        me.data.profile = p.iter().find(|x| x.id == 1).or(p.first()).cloned();
                        me.data.profiles = p;
                    }
                    Err(e) => log::error!("Profils : {e}"),
                }
                wasapi_command::set_wasapi_exclusive_preference(me.data.flag("wasapi_exclusive"));
                wasapi_command::set_dop_preference(me.data.flag("dsd_dop"));
                podcast_command::set_podcasts_enabled(me.data.flag("podcasts"));
                radio_command::set_radios_enabled(me.data.flag("radios"));
                me.load_profile_data();
                me.load_devices();
                me.amp_remote_refresh();
                me.amp_remote_cover();
                system_command::notify_ui_ready(me.backend.app.state());
                true
            },
        );
    }

    /// Ce qui dépend du profil : bibliothèques, file, playlists, likés, récents, serveurs.
    pub(crate) fn load_profile_data(&mut self) {
        let Some(profile) = self.data.profile_id() else {
            self.booting = false;
            return;
        };
        let app = self.backend.app.clone();
        self.backend.call(
            async move { library_command::get_libraries(app.state(), profile).await },
            |me, libraries| {
                match libraries {
                    Ok(l) => {
                        me.data.libraries = l;
                        me.data.library_id = me.data.pick_default_library();
                        me.load_library_data();
                        // 1,2 s après le chargement, sans fenêtre ouverte.
                        if !me.export {
                            me.banners_update(BannerMsg::Start);
                            me.backend.call(async { tokio::time::sleep(std::time::Duration::from_millis(1200)).await }, |me, ()| me.whats_new(WhatsNewMsg::OpenIfDue));
                        }
                        if me.data.flag("scan_on_startup") {
                            me.rescan();
                        }
                    }
                    Err(e) => log::error!("Bibliothèques : {e}"),
                }
                me.booting = false;
                true
            },
        );
        let app = self.backend.app.clone();
        self.backend.call(
            async move { queue_command::get_queue(app.state(), profile).await },
            |me, queue| {
                match queue {
                    Ok(q) => {
                        me.data.queue = q;
                        me.restore_current_track();
                    }
                    Err(e) => log::error!("File : {e}"),
                }
                true
            },
        );
        self.load_playlists();
        self.load_recent();
        let app = self.backend.app.clone();
        self.backend.call(
            async move { playlist_command::get_tracks_liked(app.state(), profile, None).await },
            |me, liked| {
                if let Ok(rows) = liked {
                    me.data.liked = rows.into_iter().map(|r| r.path).collect();
                }
                true
            },
        );
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let servers = server_command::get_servers(app.state(), profile).await;
                let online = server_command::servers_check(app.state(), profile).await;
                (servers, online)
            },
            |me, (servers, online)| {
                if let Ok(s) = servers {
                    me.data.servers = s;
                }
                if let Ok(o) = online {
                    me.data.server_online = o.into_iter().collect();
                }
                true
            },
        );
        if self.data.flag("podcasts") {
            let app = self.backend.app.clone();
            self.backend.call(
                async move { podcast_command::get_podcasts(app.state(), profile).await },
                |me, podcasts| {
                    if let Ok(p) = podcasts {
                        me.data.podcasts = p;
                    }
                    true
                },
            );
        }
        if self.data.flag("radios") {
            let app = self.backend.app.clone();
            self.backend.call(
                async move { radio_command::get_radios(app.state(), profile).await },
                |me, radios| {
                    if let Ok(r) = radios {
                        me.data.radios = r;
                    }
                    true
                },
            );
        }
    }

    /// Le morceau courant de la file, remis dans le lecteur sans le lancer.
    pub(crate) fn restore_current_track(&mut self) {
        let Some(track) = self.data.current_track().cloned() else {
            return;
        };
        if self.data.player.path.is_some() {
            return;
        }
        self.data.player.path = Some(track.path.clone());
        self.data.player.duration = track.duration.unwrap_or(0.);
        let app = self.backend.app.clone();
        let path = track.path.clone();
        self.backend.call(
            async move { player_command::open_file(app, path).await },
            move |me, audio| {
                if me.data.player.path.as_deref() == Some(track.path.as_str()) {
                    if me.data.player.duration <= 0. {
                        me.data.player.duration = f64::from(audio.duration);
                    }
                    me.data.player.audio = Some(audio);
                }
                true
            },
        );
    }

    pub(crate) fn load_playlists(&mut self) {
        let Some(profile) = self.data.profile_id() else {
            return;
        };
        let source = self.source_id();
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let playlists = playlist_command::get_playlists(app.state(), profile, source).await;
                let covers = sidebar_command::get_sidebar_covers(app.state(), profile, source).await;
                (playlists, covers)
            },
            |me, (playlists, covers)| {
                match playlists {
                    Ok(p) => me.data.playlists = p,
                    Err(e) => log::error!("Playlists : {e}"),
                }
                if let Ok(c) = covers {
                    me.data.covers = c.into_iter().map(|c| (c.key, c.covers)).collect();
                }
                true
            },
        );
    }

    pub(crate) fn load_recent(&mut self) {
        let Some(profile) = self.data.profile_id() else {
            return;
        };
        let source = self.source_id();
        let app = self.backend.app.clone();
        self.backend.call(
            async move { recent_command::get_recent_files(app.state(), profile, source).await },
            |me, recent| {
                match recent {
                    Ok(r) => me.data.recent = r,
                    Err(e) => log::error!("Récents : {e}"),
                }
                true
            },
        );
    }

    /// Statistiques, genres, mix et albums de la bibliothèque choisie.
    pub(crate) fn load_library_data(&mut self) {
        self.load_library_dirs();
        self.sidebar_load_pins();
        self.data.stats = None;
        self.data.genres.clear();
        self.data.mix_genres.clear();
        self.data.albums.clear();
        self.data.artists.clear();
        self.data.mixes.clear();
        self.data.random.clear();
        self.data.last_scan = None;
        self.data.library_loading = false;
        let Some(id) = self.data.library_id else {
            return;
        };
        self.data.library_loading = true;
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let stats = library_command::get_library_stats(app.state(), id).await;
                let genres = library_command::get_genres(app.state(), id).await;
                let mix_genres = library_command::get_mix_genres(app.state(), id).await;
                let albums = library_command::get_albums(app.state(), id, None).await;
                let artists = library_command::get_artists(app.state(), id).await;
                let last_scan = library_command::get_library_last_scan(app.state(), id).await;
                let random = library_command::get_mix_tracks(app.state(), id, "hasard".into(), None, None, None, 4).await;
                (stats, genres, mix_genres, albums, artists, last_scan, random)
            },
            move |me, (stats, genres, mix_genres, albums, artists, last_scan, random)| {
                if me.data.library_id != Some(id) {
                    return false;
                }
                me.data.stats = stats.ok();
                me.data.genres = genres.unwrap_or_default();
                me.data.mix_genres = mix_genres.unwrap_or_default();
                me.data.albums = albums.unwrap_or_default();
                me.data.artists = artists.unwrap_or_default();
                me.data.last_scan = last_scan.ok().flatten();
                me.data.random = random.unwrap_or_default();
                me.data.library_loading = false;
                me.draw_mixes();
                true
            },
        );
    }

    /// `draw` — le tirage de chaque mix « Pour vous ».
    pub(crate) fn draw_mixes(&mut self) {
        let Some(id) = self.data.library_id else {
            return;
        };
        let vm = ViewModel::build(&self.data, self.route(), self.source, Utc::now());
        for mix in vm.mixes {
            if self.data.mixes.contains_key(&mix.key) {
                continue;
            }
            self.data.mixes.insert(mix.key.clone(), MixDraw::default());
            let app = self.backend.app.clone();
            let key = mix.key.clone();
            let kind = mix.source_kind.to_string();
            let genre = mix.source_genre.clone();
            self.backend.call(
                async move {
                    library_command::get_mix_tracks(app.state(), id, kind, genre, None, None, MIX_SIZE).await
                },
                move |me, tracks| {
                    if me.data.library_id != Some(id) {
                        return false;
                    }
                    me.data.mixes.insert(
                        key,
                        MixDraw {
                            tracks: tracks.unwrap_or_default(),
                            loaded: true,
                        },
                    );
                    true
                },
            );
        }
    }

    pub(crate) fn load_devices(&mut self) {
        self.backend.call(
            async move {
                let devices = device_command::get_output_devices().await;
                let volume = volume_command::get_volume().await;
                (devices, volume)
            },
            |me, (devices, volume)| {
                if let Ok(d) = devices {
                    me.data.devices = d;
                }
                me.data.volume = volume;
                true
            },
        );
    }

    /// La bibliothèque où importer, s'il y en a une.
    pub fn import_library_name(&self) -> Option<String> {
        self.data.libraries.iter().find(|l| Some(l.id) == self.data.library_id).map(|l| l.name.clone())
    }

    /// `None` toutes, `Some(0)` cet ordinateur, sinon le serveur.
    pub(crate) fn source_id(&self) -> Option<i64> {
        let vm_sources = crate::models::home::ViewModel::build(&self.data, self.route(), self.source, Utc::now()).sources;
        vm_sources.get(self.source).and_then(|s| s.id).or(Some(0))
    }

    /// Aperçu hors écran : applique les résultats des commandes jusqu'à un calme d'une
    /// seconde et demie (chargements en chaîne compris), `limit` au plus.
    pub(crate) fn settle(&mut self, limit: std::time::Duration) {
        let inbox = self.inbox.clone();
        let start = Instant::now();
        while start.elapsed() < limit && inbox.wait(std::time::Duration::from_millis(1500)) {
            inbox.deliver(self);
        }
    }
}
