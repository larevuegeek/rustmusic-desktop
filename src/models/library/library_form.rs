#[derive(Debug, Clone)]
pub struct LibraryForm {
    /// Renommer : id, nom actuel, nombre de morceaux.
    pub edit: Option<(i64, String, i64)>,
    pub name: String,
    pub desc: String,
    /// Clés de traduction.
    pub error: Option<&'static str>,
    pub global_error: Option<&'static str>,
    pub busy: bool,
}
