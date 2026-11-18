use std::collections::HashSet;

#[derive(Default)]
pub struct SettingsState {
    pub search: String,
    pub replies: HashSet<&'static str>,
    pub audio: crate::models::settings::AudioState,
    pub dlna: crate::models::settings::DlnaState,
    /// Place prise par les épisodes téléchargés.
    pub downloads_size: Option<i64>,
    pub scrobble: crate::models::settings::ScrobbleState,
    pub remote: crate::models::settings::RemoteState,
    pub storage: crate::models::settings::StorageState,
    /// Serveur en cours de synchronisation.
    pub syncing: Option<i64>,
    /// Action dont les touches sont en saisie.
    pub shortcut_editing: Option<String>,
    pub guide: crate::models::settings::GuideState,
    pub license_open: bool,
}
