//! Lecture : file, morceau en cours, pause, suivant, précédent, horloge.

use std::time::Instant;
use crate::models::app::Status;
use crate::models::app::SleepTimer;
use crate::commands::library::library_command;
use crate::commands::playback::{player_command, queue_command, recent_command, volume_command};
use crate::entity::queue::queue_track::QueueTrack;
use karbon_ui::ui::prelude::*;
use crate::app::music_app::*;
use crate::models::app::Msg;
use crate::query::track::track_list_row::TrackListRow;

impl MusicApp {
    // ─── Lecture ────────────────────────────────────────────────────────────

    pub(crate) fn set_status(&mut self, status: Status) {
        self.data.player.status = status;
        self.backend.set_playing(status == Status::Playing);
        player_command::set_sleep_blocked(status == Status::Playing && self.data.flag_default_on("prevent_sleep"));
    }

    /// Lance le morceau `index` de la file.
    pub(crate) fn play_index(&mut self, index: usize) {
        let Some(track) = self.data.queue.tracks.get(index).cloned() else {
            return;
        };
        self.data.queue.current_index = index as i32;
        if let Some(profile) = self.data.profile_id() {
            let app = self.backend.app.clone();
            self.backend.fire(async move {
                let _ = queue_command::update_queue_state_index(app.state(), profile, index as i32).await;
            });
        }
        self.play_now(track.path, track.duration.unwrap_or(0.));
    }

    /// Remplace la file par `tracks` et lance celui dont le chemin est `path`.
    pub(crate) fn play_list(&mut self, tracks: Vec<QueueTrack>, path: &str) {
        let Some(profile) = self.data.profile_id() else {
            return;
        };
        let index = tracks.iter().position(|t| t.path == path).unwrap_or(0);
        self.views.queue_window.reset();
        let mut tracks = tracks;
        for (i, t) in tracks.iter_mut().enumerate() {
            t.profile_id = profile;
            t.position = i as i32;
            if t.queue_id.is_empty() {
                t.queue_id = uuid::Uuid::new_v4().to_string();
            }
        }
        self.data.queue.tracks = tracks.clone();
        let app = self.backend.app.clone();
        self.backend.fire(async move {
            let _ = queue_command::replace_queue_tracks(app.state(), profile, tracks).await;
        });
        self.play_index(index);
    }

    /// Sans lecture en cours, la file devient `tracks` et le morceau
    /// est chargé dans le lecteur, sans démarrer.
    pub(crate) fn select_track(&mut self, tracks: Vec<QueueTrack>, path: &str) {
        if matches!(self.data.player.status, Status::Playing | Status::Paused) {
            return;
        }
        let Some(profile) = self.data.profile_id() else { return };
        let index = tracks.iter().position(|t| t.path == path).unwrap_or(0);
        let mut tracks = tracks;
        for (i, t) in tracks.iter_mut().enumerate() {
            t.profile_id = profile;
            t.position = i as i32;
            if t.queue_id.is_empty() {
                t.queue_id = uuid::Uuid::new_v4().to_string();
            }
        }
        self.data.queue.tracks = tracks.clone();
        self.data.queue.current_index = index as i32;
        let app = self.backend.app.clone();
        self.backend.fire(async move {
            let _ = queue_command::replace_queue_tracks(app.state(), profile, tracks).await;
            let _ = queue_command::update_queue_state_index(app.state(), profile, index as i32).await;
        });
        self.data.player.path = None;
        self.data.player.audio = None;
        self.data.player.position = 0.;
        self.restore_current_track();
    }

    /// Lance ce chemin dans le moteur et charge sa fiche.
    pub(crate) fn play_now(&mut self, path: String, duration: f64) {
        self.data.player.path = Some(path.clone());
        self.data.player.audio = None;
        self.data.player.position = 0.;
        self.data.player.duration = duration;
        self.data.player.preparing = true;
        self.data.player.seek_when_ready = None;
        self.data.player.stale_output = false;
        self.set_status(Status::Playing);
        let app = self.backend.app.clone();
        let details_path = path.clone();
        self.backend.call(
            async move {
                // Un morceau en cours : le moteur basculerait lecture/pause au lieu de changer
                // de fichier (`playback::play`).
                player_command::stop_play().await;
                let audio = player_command::open_file(app.clone(), details_path.clone()).await;
                player_command::play_file(app, details_path).await;
                audio
            },
            move |me, audio| {
                if me.data.player.path.as_deref() == Some(path.as_str()) {
                    if me.data.player.duration <= 0. {
                        me.data.player.duration = f64::from(audio.duration);
                    }
                    me.data.player.audio = Some(audio);
                }
                true
            },
        );
        if let Some(profile) = self.data.profile_id() {
            let app = self.backend.app.clone();
            let recent_path = self.data.player.path.clone().unwrap_or_default();
            self.backend.call(
                async move {
                    let _ = recent_command::insert_recent_file(app.state(), profile, recent_path.clone()).await;
                    let _ = library_command::mark_track_played(app.state(), recent_path).await;
                },
                |me, ()| {
                    me.load_recent();
                    false
                },
            );
        }
    }

    pub(crate) fn toggle_play(&mut self) {
        if self.data.player.status == Status::Paused && self.data.player.stale_output {
            self.restart_current(self.data.player.position);
            return;
        }
        if self.data.player.path.is_some() {
            let next = match self.data.player.status {
                Status::Playing => Status::Paused,
                Status::Paused => Status::Playing,
                Status::Ready | Status::Ended => {
                    // Fin de morceau ou démarrage : relancer le courant.
                    if let Some(index) = usize::try_from(self.data.queue.current_index).ok() {
                        self.play_index(index);
                    } else if let Some(path) = self.data.player.path.clone() {
                        self.play_now(path, self.data.player.duration);
                    }
                    return;
                }
            };
            self.set_status(next);
            self.backend.fire(async { player_command::pause_play().await });
        } else if !self.data.queue.tracks.is_empty() {
            let index = usize::try_from(self.data.queue.current_index).unwrap_or(0);
            self.play_index(index.min(self.data.queue.tracks.len() - 1));
        }
    }

    pub(crate) fn stop(&mut self) {
        self.set_status(Status::Ready);
        self.data.player.position = 0.;
        self.backend.fire(async { player_command::stop_play().await });
    }

    pub(crate) fn next(&mut self) {
        let n = self.data.queue.tracks.len();
        if n == 0 {
            return;
        }
        let current = usize::try_from(self.data.queue.current_index).unwrap_or(0);
        if current + 1 < n {
            self.play_index(current + 1);
        } else if self.data.queue.repeat_mode == "all" {
            self.play_index(0);
        } else {
            self.stop();
            self.set_status(Status::Ended);
        }
    }

    pub(crate) fn previous(&mut self) {
        if self.data.player.position > 3. {
            self.seek(0.);
            return;
        }
        let current = usize::try_from(self.data.queue.current_index).unwrap_or(0);
        if current > 0 {
            self.play_index(current - 1);
        } else {
            self.seek(0.);
        }
    }

    pub(crate) fn seek(&mut self, position: f64) {
        let position = position.clamp(0., self.data.player.duration.max(0.));
        self.data.player.position = position;
        self.backend.fire(async move { player_command::seek_to(position).await });
    }

    /// `playback-ended` : le morceau suivant, selon la répétition.
    pub(crate) fn on_track_ended(&mut self, path: &str) {
        if self.data.player.path.as_deref() != Some(path) {
            return;
        }
        if self.sleep_timer == Some(SleepTimer::TrackEnd) {
            self.sleep_timer = None;
            self.stop();
            return;
        }
        if self.data.queue.repeat_mode == "one" {
            if let Ok(index) = usize::try_from(self.data.queue.current_index) {
                self.play_index(index);
                return;
            }
        }
        self.next();
    }

    pub(crate) fn set_volume(&mut self, volume: u8) {
        self.data.volume = volume;
        self.backend.fire(async move { volume_command::set_volume(volume).await });
    }

    /// Sourdine, ou retour au volume d'avant (80 % s'il était déjà nul).
    pub(crate) fn toggle_mute(&mut self) {
        let volume = self.data.volume;
        if volume == 0 {
            let before = self.data.player.volume_before_mute;
            self.set_volume(if before > 0 { before } else { 80 });
        } else {
            self.data.player.volume_before_mute = volume;
            self.set_volume(0);
        }
    }

    /// Annonce au moteur le morceau qui suivra, dès qu'il change : il le précharge pour la
    /// lecture sans blanc, ou le prépare sur le lecteur réseau. Rien après une minuterie
    /// « fin du morceau ».
    pub(crate) fn sync_next_track(&mut self) {
        let next = match self.sleep_timer {
            Some(SleepTimer::TrackEnd) => None,
            _ if self.data.player.path.is_none() => None,
            _ => self.data.following().map(|(_, t)| t.path.clone()),
        };
        if self.data.player.announced_next.as_ref() == Some(&next) {
            return;
        }
        self.data.player.announced_next = Some(next.clone());
        let app = self.backend.app.clone();
        self.backend.fire(async move {
            if let Err(e) = crate::commands::audio::audio_command::set_next_track(app.state(), next).await {
                log::warn!("⏭  Morceau suivant non annoncé : {e}");
            }
        });
    }

    pub(crate) fn tick(&mut self) -> bool {
        self.sync_next_track();
        if let Some(end) = self.sleep_end {
            if Instant::now() >= end {
                self.sleep_timer = None;
                self.sleep_end = None;
                if self.playing() {
                    self.toggle_play();
                }
            }
        }
        let lyrics = self.lyrics_sync() | self.sidebar_sync();
        if !self.playing() || self.data.player.preparing {
            return self.sleep_end.is_some() || lyrics;
        }
        let (position, duration) = crate::commands::playback::playback::progress();
        let changed = (position - self.data.player.position).abs() >= 0.05
            || (duration > 0. && (duration - self.data.player.duration).abs() > 0.5);
        self.data.player.position = position;
        if duration > 0. {
            self.data.player.duration = duration;
        }
        changed || self.sleep_end.is_some()
    }
}

impl MusicApp {
    pub(crate) fn play_tracks_view(&mut self, tracks: Vec<TrackListRow>, path: &str) {
        let profile = self.data.profile_id().unwrap_or(0);
        let queue: Vec<QueueTrack> = tracks
            .into_iter()
            .map(|t| QueueTrack {
                queue_id: uuid::Uuid::new_v4().to_string(),
                profile_id: profile,
                position: 0,
                path: t.path,
                title: t.title,
                artist: t.artist,
                duration: t.duration,
                cover: t.thumbnail_path,
            })
            .collect();
        self.play_list(queue, path);
    }

    /// L'album de ce morceau en bibliothèque, lancé depuis le début ou
    /// depuis lui ; hors bibliothèque, le morceau seul.
    pub(crate) fn play_album_of(&mut self, path: String, from_start: bool) {
        let app = self.backend.app.clone();
        let paths = vec![path.clone()];
        self.backend.call(
            async move {
                let places = library_command::get_track_locations(app.state(), paths).await.unwrap_or_default();
                let Some(place) = places.into_iter().next() else {
                    return Vec::new();
                };
                let Some(album) = place.library_album_id else {
                    return Vec::new();
                };
                library_command::get_tracks_by_album(app.state(), place.library_id, album).await.unwrap_or_default()
            },
            move |me, tracks| {
                if tracks.is_empty() {
                    return me.update(Msg::PlayPath(path));
                }
                let start = if from_start { tracks[0].path.clone() } else { path };
                me.play_tracks_view(tracks, &start);
                true
            },
        );
    }

    /// Lance un album de la bibliothèque depuis son premier morceau.
    pub(crate) fn play_album(&mut self, album_id: String) {
        let Some(library) = self.data.library_id else {
            return;
        };
        let app = self.backend.app.clone();
        self.backend.call(
            async move { library_command::get_tracks_by_album(app.state(), library, album_id).await },
            |me, tracks| {
                if let Ok(tracks) = tracks {
                    if let Some(first) = tracks.first().map(|t| t.path.clone()) {
                        me.play_tracks_view(tracks, &first);
                    }
                }
                true
            },
        );
    }
}
