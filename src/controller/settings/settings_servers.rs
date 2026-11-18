use crate::models::settings::*;
use crate::mapper::settings::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::layout::ToastKind;
use crate::commands::network::server_command;

impl MusicApp {
    pub(crate) fn reload_servers(&mut self) {
        let Some(profile) = self.data.profile.as_ref().map(|p| p.id) else { return };
        let app = self.backend.app.clone();
        self.backend.call(async move { server_command::get_servers(app.state(), profile).await }, |me, r| {
            if let Ok(s) = r {
                me.data.servers = s;
            }
            true
        });
    }

    pub(crate) fn settings_server_update(&mut self, msg: ServerMsg) -> bool {
        let app = self.backend.app.clone();
        let error = |me: &mut MusicApp, r: Result<(), String>| {
            if let Err(e) = r {
                me.toast(ToastKind::Error, t("notify.error"), Some(e));
            }
            me.reload_servers();
            true
        };
        match msg {
            ServerMsg::Enabled(id) => {
                let on = !self.data.servers.iter().any(|s| s.server.id == id && s.server.enabled);
                self.backend.call(async move { server_command::server_set_enabled(app.state(), id, on).await }, error);
            }
            ServerMsg::PinPlaylists(id) => {
                let on = !self.data.servers.iter().any(|s| s.server.id == id && s.server.pin_playlists);
                self.backend.call(async move { server_command::server_set_pin_playlists(app.state(), id, on).await }, error);
            }
            ServerMsg::Quality(id, i) => {
                let (format, bitrate) = QUALITIES.get(i as usize).copied().unwrap_or((None, None));
                self.backend.call(async move { server_command::server_set_transcode(app.state(), id, format.map(String::from), bitrate).await }, error);
            }
            ServerMsg::Sync(id) => {
                let ids: Vec<i64> = self.data.servers.iter().filter(|s| s.server.id == id).flat_map(|s| s.libraries.iter().map(|l| l.id)).collect();
                self.views.settings.syncing = Some(id);
                self.backend.call(
                    async move {
                        for l in ids {
                            crate::commands::library::library_command::rescan_library(app.clone(), app.state(), l).await?;
                        }
                        Ok(())
                    },
                    move |me, r: Result<(), String>| {
                        me.views.settings.syncing = None;
                        error(me, r)
                    },
                );
            }
            ServerMsg::AskRemove(id) => return self.typed_update(crate::models::layout::TypedMsg::RemoveServer(id)),
            ServerMsg::Remove(id) => {
                self.backend.call(async move { server_command::server_remove(app.state(), id).await }, error);
            }
        }
        true
    }
}
