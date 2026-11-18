/// Un mix de « Pour vous ».
#[derive(Debug, Clone)]
pub struct Mix {
    pub key: String,
    pub kind: String,
    pub name: String,
    pub help: String,
    pub h: f32,
    pub covers_list: Vec<String>,
    pub description: String,
    pub meta: String,
    /// Le tirage est arrivé.
    pub ready: bool,
    /// Paramètres de `get_mix_tracks` : `kind` et genre.
    pub source_kind: &'static str,
    pub source_genre: Option<String>,
}
