use crate::models::updates::*;
use crate::mapper::updates::update_whats_new::*;
use crate::mapper::updates::update_releases::*;
use crate::app::music_app::MusicApp;
use karbon_ui::storage;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::app::Section;
use crate::models::app::SettingsPage;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::models::layout::PopinSize;

/// Ouverture au lancement : un choix de l'utilisateur, désactivé par défaut (écart voulu
/// avec la source, qui l'ouvrait après chaque mise à jour sauf « Ne plus afficher »).
pub const AT_STARTUP: &str = "show_whats_new_at_startup";

impl MusicApp {
    pub(crate) fn whats_new(&mut self, msg: WhatsNewMsg) -> bool {
        let n = slides().len();
        match msg {
            WhatsNewMsg::Open => {
                if n == 0 {
                    return false;
                }
                if let Err(e) = storage::local().set(VIEWED_KEY, VERSION) {
                log::warn!("Préférences locales : {e}");
            }
                self.views.whats_new.viewed = true;
                self.views.whats_new.index = 0;
                // Grande fenêtre sans cadre ; `bare` implique `flush`.
                self.open_popin(Popin { title: format!("RustMusic {VERSION}"), icon: None, body: PopinBody::WhatsNew, size: PopinSize::Lg, bare: true, flush: true });
            }
            WhatsNewMsg::OpenIfDue => {
                if n == 0 || !self.data.flag(AT_STARTUP) || self.data.libraries.is_empty() || self.views.popin.is_some() {
                    return false;
                }
                return self.whats_new(WhatsNewMsg::Open);
            }
            WhatsNewMsg::Go(i) => return self.whats_new_slide(i.min(n.saturating_sub(1))),
            WhatsNewMsg::Next => {
                if self.views.whats_new.index + 1 >= n {
                    self.views.popin = None;
                } else {
                    return self.whats_new_slide(self.views.whats_new.index + 1);
                }
            }
            WhatsNewMsg::Try => {
                let Some((s, _)) = slides().get(self.views.whats_new.index).copied() else { return false };
                self.views.popin = None;
                if let Some(msg) = s.action.and_then(|a| self.slide_target(a)) {
                    self.handle(msg);
                }
            }
            WhatsNewMsg::AtStartup(on) => self.set_setting(AT_STARTUP, if on { "true" } else { "false" }),
        }
        true
    }

    /// La piste glisse vers la diapo `i` (transition CSS du gabarit).
    pub(crate) fn whats_new_slide(&mut self, i: usize) -> bool {
        let wn = &mut self.views.whats_new;
        if i == wn.index {
            return false;
        }
        wn.index = i;
        true
    }

    pub(crate) fn slide_target(&mut self, go: SlideGo) -> Option<Msg> {
        Some(match go {
            SlideGo::Settings(p) => Msg::Go(Route::Settings(p)),
            SlideGo::Enabled(key, route) => {
                if self.data.setting(key) == Some("true") {
                    Msg::Go(route())
                } else {
                    Msg::Go(Route::Settings(SettingsPage::Online))
                }
            }
            SlideGo::Folders => {
                self.data.library_id?;
                Msg::Go(Route::Library(Section::Folders))
            }
            SlideGo::Composers => {
                self.data.library_id?;
                self.set_setting("artists_tab", "composers");
                Msg::Go(Route::Library(Section::Artists))
            }
            SlideGo::SidebarOrganize => Msg::Sidebar(crate::models::sidebar::SidebarMsg::Organize(true)),
        })
    }
}
