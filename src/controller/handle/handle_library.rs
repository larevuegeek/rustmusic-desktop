//! Messages des sources et de la bibliothèque : serveurs, onglets, ajout de musique,
//! sorties audio, réseau, réglages, historique.

use chrono::Utc;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::app::Section;
use crate::models::app::SettingsPage;
use crate::models::home::ViewModel;
use crate::commands::audio::wasapi_command;
use crate::commands::network::network_player_command;
use crate::commands::online::{podcast_command, radio_command};
use crate::commands::playback::{recent_command, volume_command};
use crate::app::music_app::*;

impl MusicApp {
    /// Suite de `handle_player` : sources, bibliothèques, sorties, réglages.
    pub(crate) fn handle_library(&mut self, message: Msg) -> bool {
        match message {
            Msg::ChooseSource(i) => {
                self.source = i;
                self.menu = None;
                let vm = ViewModel::build(&self.data, self.route(), self.source, Utc::now());
                let source = vm.sources.get(i).and_then(|s| s.id);
                let first = self
                    .data
                    .libraries
                    .iter()
                    .find(|l| match source {
                        None => true,
                        Some(0) => l.server_id.is_none(),
                        Some(id) => l.server_id == Some(id),
                    })
                    .map(|l| l.id);
                if first.is_some() && first != self.data.library_id {
                    self.data.library_id = first;
                    self.load_library_data();
                }
                self.load_playlists();
                self.load_recent();
            }
            Msg::ChooseLibrary(id) => {
                self.menu = None;
                if self.data.library_id != Some(id) {
                    self.data.library_id = Some(id);
                    self.load_library_data();
                    self.load_page();
                }
            }
            Msg::TabPlacement(p) => {
                self.set_setting(
                    "library_tabs_position",
                    match p {
                        0 => "sidebar",
                        1 => "top",
                        _ => "both",
                    },
                );
            }
            Msg::ToggleTab(i) => {
                let mut tabs = self.data.library_tabs();
                let Some((section, ..)) = Section::ALL.get(i) else {
                    return false;
                };
                if let Some(pos) = tabs.iter().position(|s| s == section) {
                    // La dernière section affichée reste.
                    if tabs.len() > 1 {
                        tabs.remove(pos);
                    }
                } else {
                    tabs.push(*section);
                }
                let keys: Vec<&str> = Section::ALL
                    .iter()
                    .filter(|(s, ..)| tabs.contains(s))
                    .map(|(_, k, ..)| *k)
                    .collect();
                self.set_setting("library_tabs", serde_json::to_string(&keys).unwrap_or_default());
            }
            Msg::HideOpen => {
                self.set_setting("show_open_buttons", "false");
                self.menu = None;
            }
            Msg::ToggleShortcut(i) => {
                let key = if i == 0 { "show_liked_in_playlists" } else { "show_recent_in_playlists" };
                let on = self.data.flag_default_on(key);
                self.set_setting(key, if on { "false" } else { "true" });
            }
            Msg::Rescan => self.rescan(),
            Msg::AddDirectory => self.ask(crate::models::layout::Choice::PageImport { folder: true }),
            Msg::AddFiles => self.ask(crate::models::layout::Choice::PageImport { folder: false }),
            Msg::OpenFile => self.ask(crate::models::layout::Choice::PlayFile),
            Msg::OpenFolder => self.ask(crate::models::layout::Choice::PlayFolder),
            Msg::AddMusic => match self.data.library_id {
                Some(library) => self.ask(crate::models::layout::Choice::ImportFolder { library, redirect: true }),
                None => {
                    self.history.push(Route::Import);
                    self.load_page();
                }
            },

            Msg::ChooseOutput(i) => {
                self.menu = None;
                let Some(device) = self.data.devices.get(i).map(|d| d.name.clone()) else {
                    return false;
                };
                for (j, d) in self.data.devices.iter_mut().enumerate() {
                    d.is_selected = i == j;
                }
                self.backend.fire(async move { volume_command::set_device(Some(device)).await });
            }
            Msg::ToggleExclusive => {
                let on = !self.data.flag("wasapi_exclusive");
                self.set_setting("wasapi_exclusive", if on { "true" } else { "false" });
                wasapi_command::set_wasapi_exclusive_preference(on);
            }
            Msg::ToggleDop => {
                let on = !self.data.flag("dsd_dop");
                self.set_setting("dsd_dop", if on { "true" } else { "false" });
                wasapi_command::set_dop_preference(on);
            }
            Msg::ToggleNetwork(i) => {
                let key = ["network_players", "airplay", "dlna_enabled"][i.min(2)];
                let on = !self.data.flag(key);
                self.set_setting(key, if on { "true" } else { "false" });
                match i {
                    0 => self.backend.fire(async move {
                        let _ = network_player_command::set_network_players_enabled(on).await;
                    }),
                    1 => self.backend.fire(async move {
                        let _ = network_player_command::set_airplay_enabled(on).await;
                    }),
                    _ => {
                        let app = self.backend.app.clone();
                        self.backend.fire(async move {
                            let _ = if on {
                                crate::commands::network::dlna_command::dlna_start(app.state()).await
                            } else {
                                crate::commands::network::dlna_command::dlna_stop(app.state()).await
                            };
                        });
                    }
                }
            }

            Msg::PlayPage(index) => {
                let Some(page) = self.data.page.as_ref().filter(|p| p.route == *self.history.current()) else {
                    return false;
                };
                let queue = page.queue.clone();
                let Some(path) = queue.get(index).map(|t| t.path.clone()) else {
                    return false;
                };
                self.play_list(queue, &path);
            }
            Msg::PlayRadio(id) => self.play_radio(id),
            Msg::ToggleSetting(key, default_on) => {
                let on = if default_on { self.data.flag_default_on(key) } else { self.data.flag(key) };
                let next = !on;
                self.set_setting(key, if next { "true" } else { "false" });
                match key {
                    "sidebar_collapsed" => self.sidebar_collapsed = next,
                    "wasapi_exclusive" => crate::commands::audio::wasapi_command::set_wasapi_exclusive_preference(next),
                    "dsd_dop" => crate::commands::audio::wasapi_command::set_dop_preference(next),
                    "podcasts" => podcast_command::set_podcasts_enabled(next),
                    "radios" => radio_command::set_radios_enabled(next),
                    "amp_remote" => {
                        let app = self.backend.app.clone();
                        self.backend.call(
                            async move { crate::commands::network::amp_remote_command::set_amp_remote_enabled(app.state(), next).await },
                            move |me, _| {
                                if next && matches!(me.route(), Route::Settings(SettingsPage::Remote)) {
                                    me.settings_remote_load();
                                }
                                true
                            },
                        );
                    }
                    "network_players" => self.backend.fire(async move {
                        let _ = network_player_command::set_network_players_enabled(next).await;
                    }),
                    "airplay" => self.backend.fire(async move {
                        let _ = network_player_command::set_airplay_enabled(next).await;
                    }),
                    "dlna_enabled" => {
                        let app = self.backend.app.clone();
                        self.backend.fire(async move {
                            let _ = if next {
                                crate::commands::network::dlna_command::dlna_start(app.state()).await
                            } else {
                                crate::commands::network::dlna_command::dlna_stop(app.state()).await
                            };
                        });
                    }
                    _ => {}
                }
            }
            Msg::Reveal(path) => {
                reveal(&path);
                return false;
            }
            Msg::RenewMixes => {
                self.data.mixes.clear();
                self.draw_mixes();
            }
            Msg::ClearHistory => {
                self.data.recent.clear();
                if let Some(profile) = self.data.profile_id() {
                    let app = self.backend.app.clone();
                    let source = self.source_id();
                    self.backend.fire(async move {
                        let _ = recent_command::clear_recent_files(app.state(), profile, source).await;
                    });
                }
            }
            _ => return false,
        }
        true
    }
}
