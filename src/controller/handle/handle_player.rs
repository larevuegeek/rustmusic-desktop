//! Messages de lecture : file, morceau, volume, aléatoire, répétition, minuteur.

use std::time::Instant;
use chrono::Utc;
use karbon_ui::ui::prelude::*;
use crate::helper::string::string::display_title;
use crate::models::app::SleepTimer;
use crate::models::app::Msg;
use crate::models::home::ViewModel;
use crate::commands::playback::queue_command;
use crate::commands::playlist::playlist_command;
use crate::entity::queue::queue_track::QueueTrack;
use crate::app::music_app::*;

impl MusicApp {
    /// Suite de `handle` : la lecture et le minuteur de veille.
    pub(crate) fn handle_player(&mut self, message: Msg) -> bool {
        match message {
            Msg::TogglePlay => self.toggle_play(),
            Msg::Stop => self.stop(),
            Msg::Next => self.next(),
            Msg::Previous => self.previous(),
            Msg::Seek(position) => self.seek(position),
            Msg::SeekBy(delta) => self.seek(self.data.player.position + delta),
            Msg::Volume(value) => self.set_volume(value.round().clamp(0., 100.) as u8),
            Msg::Mute => self.toggle_mute(),
            Msg::Like(path) => {
                let Some(profile) = self.data.profile_id() else {
                    return false;
                };
                let app = self.backend.app.clone();
                if self.data.liked.remove(&path) {
                    self.backend.fire(async move {
                        let _ = playlist_command::remove_track_liked(app.state(), path, profile).await;
                    });
                } else {
                    self.data.liked.insert(path.clone());
                    self.backend.fire(async move {
                        let _ = playlist_command::add_track_liked(app.state(), path, profile).await;
                    });
                }
            }
            Msg::Shuffle => {
                self.data.queue.is_shuffled = !self.data.queue.is_shuffled;
                if let Some(profile) = self.data.profile_id() {
                    let app = self.backend.app.clone();
                    let on = self.data.queue.is_shuffled;
                    self.backend.fire(async move {
                        let _ = queue_command::update_queue_state_shuffled(app.state(), profile, on).await;
                    });
                }
            }
            Msg::Repeat => {
                self.data.queue.repeat_mode = match self.data.queue.repeat_mode.as_str() {
                    "off" => "all",
                    "all" => "one",
                    _ => "off",
                }
                .into();
                if let Some(profile) = self.data.profile_id() {
                    let app = self.backend.app.clone();
                    let mode = self.data.queue.repeat_mode.clone();
                    self.backend.fire(async move {
                        let _ = queue_command::update_queue_state_repeat_mode(app.state(), profile, &mode).await;
                    });
                }
            }
            Msg::PlayQueue(index) => self.play_index(index),
            Msg::PlayPath(path) => {
                if let Some(index) = self.data.queue.tracks.iter().position(|t| t.path == path) {
                    self.play_index(index);
                } else {
                    let app = self.backend.app.clone();
                    let paths = vec![path.clone()];
                    self.backend.call(
                        async move { crate::commands::playlist::playlist_view_command::get_tracks_view_by_paths(app.state(), paths).await },
                        move |me, tracks| {
                            match tracks {
                                Ok(t) if !t.is_empty() => me.play_tracks_view(t, &path),
                                _ => {
                                    let profile = me.data.profile_id().unwrap_or(0);
                                    let track = QueueTrack {
                                        queue_id: String::new(),
                                        profile_id: profile,
                                        position: 0,
                                        title: display_title(None, &path, "Titre inconnu"),
                                        artist: None,
                                        duration: None,
                                        cover: None,
                                        path: path.clone(),
                                    };
                                    me.play_list(vec![track], &path);
                                }
                            }
                            true
                        },
                    );
                }
            }
            Msg::PlayRecent(path) => {
                let profile = self.data.profile_id().unwrap_or(0);
                let queue: Vec<QueueTrack> = self
                    .data
                    .recent
                    .iter()
                    .map(|r| QueueTrack {
                        queue_id: String::new(),
                        profile_id: profile,
                        position: 0,
                        path: r.path.clone(),
                        title: display_title(r.title.as_deref(), &r.path, "Titre inconnu"),
                        artist: r.artist.clone(),
                        duration: r.duration,
                        cover: r.thumbnail_path.clone(),
                    })
                    .collect();
                self.play_list(queue, &path);
            }
            Msg::PlayAlbum(id) => self.play_album(id),
            Msg::PlayAlbumOf(path, from_start) => self.play_album_of(path, from_start),
            Msg::PlayMix(i) => {
                let vm = ViewModel::build(&self.data, self.route(), self.source, Utc::now());
                let Some(mix) = vm.mixes.get(i) else {
                    return false;
                };
                let Some(draw) = self.data.mixes.get(&mix.key) else {
                    return false;
                };
                let tracks = draw.tracks.clone();
                if let Some(first) = tracks.first().map(|t| t.path.clone()) {
                    self.menu = None;
                    self.play_tracks_view(tracks, &first);
                }
            }
            Msg::PlayRandom(path) => {
                let tracks = self.data.random.clone();
                self.play_tracks_view(tracks, &path);
            }

            Msg::SleepTimer(sleep_timer) => {
                self.sleep_timer = sleep_timer;
                self.sleep_end = match sleep_timer {
                    Some(SleepTimer::Minutes(m)) => Some(Instant::now() + std::time::Duration::from_secs(u64::from(m) * 60)),
                    _ => None,
                };
                self.menu = None;
            }
            Msg::SleepInput(value) => self.sleep_input = value,
            Msg::SleepCustom => {
                let minutes = self.sleep_input.trim().parse::<u32>().unwrap_or(20).clamp(1, 600);
                return self.update(Msg::SleepTimer(Some(SleepTimer::Minutes(minutes))));
            }

            other => return self.handle_library(other),
        }
        true
    }
}
