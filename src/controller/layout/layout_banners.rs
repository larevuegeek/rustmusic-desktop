use crate::models::layout::*;
use crate::app::music_app::MusicApp;
use karbon_ui::storage;
use crate::mapper::updates::update_whats_new::VERSION;

pub(crate) const VERSION_KEY: &str = "rm-support-version";

pub(crate) const NEVER_KEY: &str = "rm-support-never";

pub(crate) fn support_due() -> bool {
    if storage::local().get(NEVER_KEY).as_deref() == Some("1") {
        return false;
    }
    match storage::local().get(VERSION_KEY) {
        None => {
            if let Err(e) = storage::local().set(VERSION_KEY, VERSION) {
                log::warn!("Préférences locales : {e}");
            }
            false
        }
        Some(v) => v != VERSION,
    }
}

pub fn support_url() -> String {
    let lang = karbon_ui::intl::locale().tag();
    let prefix = if ["en", "es", "it", "de"].contains(&lang) { format!("/{lang}") } else { String::new() };
    format!("https://rustmusic.dev{prefix}/#support")
}

impl MusicApp {
    pub(crate) fn banners_update(&mut self, msg: BannerMsg) -> bool {
        match msg {
            BannerMsg::Start => {
                if support_due() {
                    self.banner_wait(8000);
                }
                self.updater_start();
                return false;
            }
            BannerMsg::UpdateCheck => return self.update_check(false, 0),
            BannerMsg::UpdateInstall => return self.update_install(),
            BannerMsg::SupportDue => {
                if self.views.banners.support_handled {
                    return false;
                }
                // Pas pendant les nouveautés : la carte attend 30 s après leur fermeture.
                if self.views.popin.as_ref().is_some_and(|p| p.is_whats_new()) {
                    self.banner_wait(30000);
                    return false;
                }
                self.views.banners.support = true;
            }
            BannerMsg::SupportLater | BannerMsg::SupportOpen => {
                if msg == BannerMsg::SupportOpen {
                    crate::controller::settings::open_url(&support_url());
                }
                self.views.banners.support = false;
                self.views.banners.support_handled = true;
                if let Err(e) = storage::local().set(VERSION_KEY, VERSION) {
                log::warn!("Préférences locales : {e}");
            }
            }
            BannerMsg::SupportNever => {
                self.views.banners.support = false;
                self.views.banners.support_handled = true;
                if let Err(e) = storage::local().set(NEVER_KEY, "1") {
                log::warn!("Préférences locales : {e}");
            }
                if let Err(e) = storage::local().set(VERSION_KEY, VERSION) {
                log::warn!("Préférences locales : {e}");
            }
            }
            BannerMsg::UpdateLater => {
                self.views.banners.update = Update::Idle;
                self.views.banners.pending = None;
            }
        }
        true
    }

    pub(crate) fn banner_wait(&mut self, ms: u64) {
        self.backend.call(async move { tokio::time::sleep(std::time::Duration::from_millis(ms)).await }, |me, ()| me.banners_update(BannerMsg::SupportDue));
    }
}
