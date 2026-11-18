use crate::app::music_app::MusicApp;
use crate::models::settings::SettingsMsg;

impl MusicApp {
    pub(crate) fn settings_help_update(&mut self, msg: SettingsMsg) -> bool {
        let s = &mut self.views.settings;
        match msg {
            SettingsMsg::GuideMode(exclusive) => s.guide.exclusive = exclusive,
            SettingsMsg::GuideStep(i) => s.guide.step = i.min(4),
            SettingsMsg::GuideTerm(id) => {
                s.guide.term = Some(id.to_string());
                // L'éclairage s'éteint après 2 s, comme `extinction`.
                self.backend.call(async { tokio::time::sleep(std::time::Duration::from_secs(2)).await }, move |me, ()| {
                    if me.views.settings.guide.term.as_deref() == Some(id) {
                        me.views.settings.guide.term = None;
                        return true;
                    }
                    false
                });
            }
            SettingsMsg::LicenseToggle => s.license_open = !s.license_open,
            SettingsMsg::OpenUrl(url) => {
                crate::controller::settings::open_url(url);
                return false;
            }
            _ => return false,
        }
        true
    }
}
