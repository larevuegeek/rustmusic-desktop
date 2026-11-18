#[derive(Debug, Clone, Default)]
pub struct ColumnsPopin {
    pub choice: Vec<String>,
    /// Tags recensés (clé, nombre de pistes qui le renseignent) ; `None` pendant le recensement.
    pub tags: Option<Vec<(String, i64)>>,
    pub filter: String,
}
