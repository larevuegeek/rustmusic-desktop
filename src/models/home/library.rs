/// Une bibliothèque de la liste.
#[derive(Debug, Clone)]
pub struct Library {
    pub id: i64,
    pub name: String,
    pub tracks_count: String,
    pub albums: String,
    /// « 1 234 titres · 56 albums ».
    pub summary: String,
    pub default_value: bool,
}
