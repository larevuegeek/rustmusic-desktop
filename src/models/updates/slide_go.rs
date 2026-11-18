use crate::models::app::Route;
use crate::models::app::SettingsPage;

/// Où mène « Essayer ».
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlideGo {
    Settings(SettingsPage),
    /// La page si la fonction est activée (`radios`, `podcasts`), sinon les réglages en ligne.
    Enabled(&'static str, fn() -> Route),
    Folders,
    Composers,
    SidebarOrganize,
}
