#[derive(Debug, Clone, PartialEq)]
pub enum SettingsMsg {
    Search(String),
    Fold(&'static str),
    Restore,
    Toggle(&'static str),
    Set(&'static str, String),
    /// Choix d'une liste (`<select>` : l'indice de l'option).
    Choose(&'static str, u64),
    /// Section de bibliothèque cochée ou décochée.
    Tab(&'static str),
    /// Audio : relister les sorties, mode exclusif, qualité, Replay Gain, test du DAC.
    RefreshDevices,
    Exclusive(bool),
    Quality(&'static str),
    ReplayGain(&'static str),
    DacTest,
    /// Réseau : serveur DLNA (marche, nom et port saisis, enregistrement).
    DlnaToggle,
    DlnaName(String),
    DlnaPort(String),
    DlnaSave,
    /// Écoute en ligne : historique des podcasts ou des radios (demande, puis confirmé).
    ClearHistory(&'static str, bool),
    Scrobble(crate::models::settings::ScrobbleMsg),
    /// Télécommande : chercher les amplis, en piloter un (ou aucun).
    AmpSearch,
    AmpSelect(Option<crate::core::amp_remote::model::Amp>),
    /// Stockage : une tâche (rescan, portraits…), une action sur un serveur.
    Storage(crate::models::settings::StorageTask),
    Server(crate::models::settings::ServerMsg),
    /// Raccourcis : saisie d'une action, retour aux touches d'origine (une, toutes).
    ShortcutEdit(&'static str),
    ShortcutReset(&'static str),
    ShortcutResetAll,
    /// Guide : mode du schéma, étape, terme du glossaire. À propos : licence, liens.
    GuideMode(bool),
    GuideStep(usize),
    GuideTerm(&'static str),
    LicenseToggle,
    OpenUrl(&'static str),
}
