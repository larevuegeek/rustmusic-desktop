use crate::models::layout::*;

/// Retirer un serveur : son nom, ses bibliothèques et ce que le retrait emporte.
pub struct RemoveServerView {
    pub intro: BoldLine,
    /// (nom, « · 1 234 » titres).
    pub libraries: Vec<(String, String)>,
    pub items: Vec<String>,
}
