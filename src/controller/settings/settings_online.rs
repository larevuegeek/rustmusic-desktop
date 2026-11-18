use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::layout::Popin;
use crate::models::settings::SettingsMsg;
use crate::commands::online::{podcast_command, radio_command};

impl MusicApp {
    /// La place prise par les épisodes téléchargés du profil.
    pub(crate) fn settings_online_load(&mut self) {
        let Some(profile) = self.data.profile.as_ref().map(|p| p.id) else { return };
        let app = self.backend.app.clone();
        if self.data.flag("podcasts") {
            self.backend.call(async move { podcast_command::get_podcast_downloads(app.state(), profile).await }, |me, r| {
                me.views.settings.downloads_size = r.ok().map(|l| l.iter().map(|d| d.size as i64).sum::<i64>());
                true
            });
        }
        self.settings_scrobble_load();
    }

    /// La confirmation, puis l'effacement.
    pub(crate) fn settings_clear_history(&mut self, kind: &'static str, confirmed: bool) -> bool {
        if !confirmed {
            self.open_popin(Popin::confirm(
                t(&format!("{kind}.clear_history")),
                t(&format!("{kind}.clear_history_confirm")),
                t("common.delete"),
                t("common.cancel"),
                true,
                Msg::Settings(SettingsMsg::ClearHistory(kind, true)),
            ));
            return true;
        }
        let Some(profile) = self.data.profile.as_ref().map(|p| p.id) else { return false };
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                if kind == "podcasts" {
                    podcast_command::podcast_clear_history(app.state(), profile).await
                } else {
                    radio_command::clear_radio_history(app.state(), profile).await
                }
            },
            move |me, r| {
                if let Err(e) = r {
                    log::warn!("Historique {kind} : {e}");
                }
                // Le toast « Historique effacé » attend les toasts globaux.
                let app = me.backend.app.clone();
                if kind == "podcasts" {
                    me.backend.call(async move { podcast_command::get_podcasts(app.state(), profile).await }, |me, r| {
                        if let Ok(p) = r {
                            me.data.podcasts = p;
                        }
                        true
                    });
                } else {
                    me.backend.call(async move { radio_command::get_radios(app.state(), profile).await }, |me, r| {
                        if let Ok(l) = r {
                            me.data.radios = l;
                        }
                        true
                    });
                }
                true
            },
        );
        true
    }
}
