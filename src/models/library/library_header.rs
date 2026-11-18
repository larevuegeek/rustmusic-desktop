/// Ce que l'en-tête affiche.
#[derive(Debug, Clone)]
pub struct LibraryHeader {
    pub id: i64,
    pub name: String,
    pub kicker: String,
    pub icon: &'static str,
    /// (nombre, libellé) dans l'ordre de la section.
    pub digits: Vec<(String, String)>,
    /// Durée d'écoute (Morceaux) ou poids : (gras, suite).
    pub extra: Option<(String, String)>,
    pub scan_ago: Option<String>,
    /// Les autres bibliothèques : (id, nom, titres, courante).
    pub libraries: Vec<(i64, String, String, bool)>,
    /// Il y a d'autres bibliothèques : le menu pour en changer.
    pub many_libraries: bool,
    pub primary_icon: &'static str,
    pub primary_label: String,
    /// `ok`, `scanning`, `error`.
    pub state: &'static str,
}
