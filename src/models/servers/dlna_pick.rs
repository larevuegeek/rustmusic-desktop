use crate::core::dlna_client::catalog::Folder;

#[derive(Debug, Clone)]
pub struct DlnaPick {
    pub url: String,
    /// Chemin ouvert (identifiant, nom), racine `0` comprise.
    pub trail: Vec<(String, String)>,
    pub folders: Vec<Folder>,
    pub loading: bool,
    pub error: Option<String>,
    /// Dossiers cochés (identifiant, nom).
    pub selected: Vec<(String, String)>,
}
