#[derive(Debug, Clone, PartialEq)]
pub enum TypedKind {
    /// Réinitialisation de l'application.
    Reset,
    /// Retrait d'un serveur : nom, bibliothèques (nom, morceaux), total des morceaux.
    RemoveServer { id: i64, name: String, libraries: Vec<(String, i64)>, tracks: i64 },
}
