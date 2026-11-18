/// Un fichier que le lot n'a pas pu traiter.
pub struct BatchFailure {
    pub path: String,
    /// Le nom du fichier seul.
    pub file: String,
    pub message: String,
}
