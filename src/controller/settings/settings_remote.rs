use crate::app::music_app::MusicApp;
use crate::models::settings::SettingsMsg;
use crate::commands::network::amp_remote_command;

impl MusicApp {
    /// L'ampli piloté ; première visite, télécommande active : on cherche les amplis.
    pub(crate) fn settings_remote_load(&mut self) {
        self.views.settings.remote.pilot = amp_remote_command::amp_remote_status().map(|v| (v.amp, v.status.state.connected));
        let r = &self.views.settings.remote;
        if self.data.flag("amp_remote") && !r.searched && !r.searching {
            self.settings_remote_update(SettingsMsg::AmpSearch);
        }
    }

    pub(crate) fn settings_remote_update(&mut self, msg: SettingsMsg) -> bool {
        match msg {
            SettingsMsg::AmpSearch => {
                self.views.settings.remote.searching = true;
                self.backend.call(async move { amp_remote_command::amp_remote_discover().await }, |me, found| {
                    let r = &mut me.views.settings.remote;
                    r.found = found;
                    r.searched = true;
                    r.searching = false;
                    true
                });
            }
            SettingsMsg::AmpSelect(amp) => {
                let app = self.backend.app.clone();
                self.backend.call(async move { amp_remote_command::amp_remote_select(app.state(), amp).await }, |me, r| match r {
                    Ok(v) => {
                        me.views.settings.remote.pilot = v.map(|v| (v.amp, v.status.state.connected));
                        true
                    }
                    Err(e) => {
                        log::warn!("Télécommande : {e}");
                        false
                    }
                });
            }
            _ => return false,
        }
        true
    }
}
