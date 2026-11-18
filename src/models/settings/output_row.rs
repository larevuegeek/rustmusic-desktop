/// Une ligne de la liste des sorties audio.
pub struct OutputRow {
    pub index: usize,
    pub title: String,
    pub name: String,
    pub detail: String,
    pub icon: &'static str,
    pub active: bool,
    pub default_value: bool,
    /// Libellé et ton du palier de qualité.
    pub tier: Option<(String, &'static str)>,
    pub specs: Vec<String>,
}
