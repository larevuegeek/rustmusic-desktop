use crate::models::servers::DlnaPick;

#[derive(Debug, Clone)]
pub struct ServerFolders {
    pub server_id: i64,
    /// Dossiers déjà ajoutés (identifiants distants).
    pub existing: Vec<String>,
    pub folders: Vec<(String, String)>,
    pub loading: bool,
    pub chosen: Vec<String>,
    pub dlna: Option<DlnaPick>,
    pub busy: bool,
    pub error: Option<String>,
}
