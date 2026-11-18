#[derive(Debug, Clone)]
pub struct CoverHit {
    pub title: String,
    pub artist: String,
    pub cover_xl: String,
    /// Vignette téléchargée.
    pub thumb: Option<String>,
}
