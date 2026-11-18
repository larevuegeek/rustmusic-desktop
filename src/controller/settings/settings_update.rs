//! Les pages de réglages : ce que chacune charge à l'ouverture et la réponse aux messages.

use crate::models::settings::*;
use crate::mapper::settings::*;
use crate::models::app::Msg;
use crate::models::settings::SettingsMsg;
use crate::app::music_app::MusicApp;
use crate::models::app::Route;
use crate::models::app::SettingsPage;
use crate::commands::app::settings_command;

impl MusicApp {
    /// À l'ouverture d'une page : ce qu'elle lit du moteur.
    pub(crate) fn settings_open(&mut self, page: SettingsPage) {
        match page {
            SettingsPage::Audio => self.settings_audio_load(),
            SettingsPage::Network => self.settings_network_load(),
            SettingsPage::Online => self.settings_online_load(),
            SettingsPage::Remote => self.settings_remote_load(),
            _ => {}
        }
    }

    pub(crate) fn settings_update(&mut self, msg: SettingsMsg) -> bool {
        match msg {
            SettingsMsg::Search(v) => self.views.settings.search = v,
            SettingsMsg::Fold(k) => {
                if !self.views.settings.replies.remove(k) {
                    self.views.settings.replies.insert(k);
                }
            }
            SettingsMsg::Restore => {
                let Route::Settings(page) = self.route().clone() else { return false };
                let Some(section) = SECTIONS.iter().find(|s| s.0 == page) else { return false };
                for k in section.6 {
                    let def = DEFAULTS.iter().find(|(d, _)| d == k).map(|(_, v)| *v).unwrap_or("");
                    if self.data.setting(k).is_some_and(|v| v != def) {
                        self.set_setting(k, def);
                    }
                }
            }
            SettingsMsg::Toggle(key) => {
                let on = self.setting_or_default(key) == "true";
                // Les effets propres à chaque clé (sortie exclusive, podcasts…) : `ToggleSetting`.
                return self.handle(Msg::ToggleSetting(key, on));
            }
            SettingsMsg::Choose(key, i) => {
                let Route::Settings(page) = self.route().clone() else { return false };
                let value = self.settings_view(page).groups.into_iter().flat_map(|g| g.rows).find_map(|r| match r.control {
                    Control::Select(k, _, opts) if k == key => opts.get(i as usize).map(|o| o.0.clone()),
                    _ => None,
                });
                let Some(value) = value else { return false };
                return self.settings_update(SettingsMsg::Set(key, value));
            }
            SettingsMsg::Tab(key) => self.toggle_library_tab(key),
            SettingsMsg::ClearHistory(kind, ok) => return self.settings_clear_history(kind, ok),
            SettingsMsg::Scrobble(m) => return self.settings_scrobble_update(m),
            m @ (SettingsMsg::ShortcutEdit(_) | SettingsMsg::ShortcutReset(_) | SettingsMsg::ShortcutResetAll) => {
                return self.settings_shortcut_update(m);
            }
            m @ (SettingsMsg::GuideMode(_) | SettingsMsg::GuideStep(_) | SettingsMsg::GuideTerm(_) | SettingsMsg::LicenseToggle | SettingsMsg::OpenUrl(_)) => {
                return self.settings_help_update(m);
            }
            SettingsMsg::Storage(task) => return self.settings_storage_update(task),
            SettingsMsg::Server(m) => return self.settings_server_update(m),
            m @ (SettingsMsg::AmpSearch | SettingsMsg::AmpSelect(_)) => return self.settings_remote_update(m),
            m @ (SettingsMsg::DlnaToggle | SettingsMsg::DlnaName(_) | SettingsMsg::DlnaPort(_) | SettingsMsg::DlnaSave) => {
                return self.settings_network_update(m);
            }
            m @ (SettingsMsg::RefreshDevices | SettingsMsg::Exclusive(_) | SettingsMsg::Quality(_) | SettingsMsg::ReplayGain(_) | SettingsMsg::DacTest) => {
                return self.settings_audio_update(m);
            }
            SettingsMsg::Set(key, value) => {
                self.set_setting(key, value.clone());
                if key == "language" {
                    crate::app::resources::translations::set_language(&value);
                    self.tray_refresh();
                }
            }
        }
        true
    }
}

impl MusicApp {
    pub(crate) fn set_setting(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();
        self.data.settings.insert(key.to_string(), value.clone());
        let app = self.backend.app.clone();
        let key = key.to_string();
        self.backend.fire(async move {
            if let Err(e) = settings_command::set_setting(app.state(), key.clone(), value).await {
                log::warn!("Réglage {key} : {e}");
            }
        });
    }
}
