/// Progression d'un import ou d'un scan, telle que l'affichent le chargeur de la
/// bibliothèque et la pastille de la barre d'état.
#[derive(Debug, Clone, Default)]
pub struct ImportProgressView {
    pub import: bool,
    pub listing: bool,
    pub stopping: bool,
    /// 0 à 100.
    pub percent: f32,
    pub percent_label: String,
    /// « Importation en cours », « Recherche des fichiers… »…
    pub title: String,
    /// Libellé court de la pastille : « Import en cours », « Rescan : Musique ».
    pub task: String,
    /// Icône de la pastille.
    pub icon: &'static str,
    /// Détail de la pastille : fichiers trouvés, puis fichier en cours.
    pub detail: String,
    /// « 120 / 9 440 » ou « 312 fichiers trouvés ».
    pub counter: String,
    pub elapsed: String,
    pub eta: String,
    pub directory: String,
    pub file_name: String,
    /// Le chargeur remplace la page de la bibliothèque (premier import, bibliothèque vide).
    pub loader: bool,
    pub library_id: Option<i64>,
}
