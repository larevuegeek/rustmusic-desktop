/// Une piste du détail : clé (si cochable), cochée, numéro, avant, après.
#[derive(Debug, Clone)]
pub struct Detail {
    pub key: Option<String>,
    pub on: bool,
    pub number: String,
    pub from: String,
    pub to: String,
}
