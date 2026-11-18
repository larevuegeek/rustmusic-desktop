/// Le serveur DLNA tel que lu à l'ouverture de la page.
#[derive(Default)]
pub struct DlnaState {
    pub loaded: bool,
    pub running: bool,
    pub url: Option<String>,
    /// Nom et port enregistrés, puis tels que saisis.
    pub name: String,
    pub port: u16,
    pub name_draft: String,
    pub port_draft: String,
    pub toggling: bool,
}
