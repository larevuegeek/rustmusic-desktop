use crate::models::library::ManageAction;

/// Le menu « Gérer » d'une bibliothèque : en-tête, groupes d'actions.
#[derive(Debug, Clone)]
pub struct ManageMenu {
    pub id: i64,
    pub name: String,
    pub remote: bool,
    pub server_name: String,
    /// Chemin du dossier unique, ou « N dossiers ».
    pub dirs_line: String,
    /// `ok`, `scanning`, `error`.
    pub state: &'static str,
    pub groups: Vec<Vec<ManageAction>>,
}
