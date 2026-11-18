/// Une page des réglages (`src/routes/settings/*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsPage {
    General,
    Appearance,
    Audio,
    Network,
    Online,
    Remote,
    Shortcuts,
    Storage,
    Guide,
    About,
}
