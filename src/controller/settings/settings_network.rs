use crate::app::music_app::MusicApp;
use crate::models::settings::SettingsMsg;
use crate::commands::network::dlna_command;

impl MusicApp {
    pub(crate) fn settings_network_load(&mut self) {
        let app = self.backend.app.clone();
        self.backend.call(
            async move { (dlna_command::dlna_get_settings(app.state()).await, dlna_command::dlna_status(app.state()).await) },
            |me, (settings_result, state)| {
                let d = &mut me.views.settings.dlna;
                if let Ok(r) = settings_result {
                    d.name = r.friendly_name.clone();
                    d.port = r.port;
                    d.name_draft = r.friendly_name;
                    d.port_draft = r.port.to_string();
                    d.loaded = true;
                }
                if let Ok(s) = state {
                    d.running = s.running;
                    d.url = s.url;
                }
                true
            },
        );
    }

    pub(crate) fn dlna_apply(&mut self, status: Result<dlna_command::DlnaStatus, String>) -> bool {
        let d = &mut self.views.settings.dlna;
        d.toggling = false;
        match status {
            Ok(s) => {
                d.running = s.running;
                d.url = s.url;
                d.name = s.friendly_name.clone();
                d.name_draft = s.friendly_name;
                d.port = s.port;
                d.port_draft = s.port.to_string();
            }
            Err(e) => log::warn!("DLNA : {e}"),
        }
        true
    }

    pub(crate) fn settings_network_update(&mut self, msg: SettingsMsg) -> bool {
        let app = self.backend.app.clone();
        match msg {
            SettingsMsg::DlnaToggle => {
                if self.views.settings.dlna.toggling {
                    return false;
                }
                self.views.settings.dlna.toggling = true;
                let running = self.views.settings.dlna.running;
                self.set_setting("dlna_enabled", if running { "false" } else { "true" });
                self.backend.call(
                    async move {
                        if running {
                            dlna_command::dlna_stop(app.state()).await
                        } else {
                            dlna_command::dlna_start(app.state()).await
                        }
                    },
                    |me, r| me.dlna_apply(r),
                );
            }
            SettingsMsg::DlnaName(v) => self.views.settings.dlna.name_draft = v,
            SettingsMsg::DlnaPort(v) => self.views.settings.dlna.port_draft = v.chars().filter(char::is_ascii_digit).take(5).collect(),
            SettingsMsg::DlnaSave => {
                let d = &self.views.settings.dlna;
                let name = d.name_draft.trim().to_string();
                let name = (!name.is_empty() && name != d.name).then_some(name);
                let port = d.port_draft.parse::<u16>().ok().filter(|p| *p >= 1 && *p != d.port);
                if name.is_none() && port.is_none() {
                    return false;
                }
                self.backend.call(async move { dlna_command::dlna_update_settings(app.state(), name, port).await }, |me, r| me.dlna_apply(r));
            }
            _ => return false,
        }
        true
    }
}
