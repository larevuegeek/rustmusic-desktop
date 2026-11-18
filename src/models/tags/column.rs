pub struct Column {
    pub idx: usize,
    pub label: String,
    pub heading: String,
    pub width: f32,
    pub grow: f32,
    pub numeric: bool,
    /// Trié : croissant ou non.
    pub sorted: Option<bool>,
    pub menu: bool,
    /// Menu de colonne : recopier (valeur), majuscules, effacer (nombre) et leur disponibilité.
    pub fill: (String, bool),
    pub case_ok: bool,
    pub clear: (String, bool),
}
