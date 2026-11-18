/// Carte du morceau en cours dans le volet de la file.
pub struct QueueNowPlayingView {
    pub title: String,
    /// L'artiste, ou « Artiste inconnu ».
    pub artist: String,
    pub cover: Option<String>,
    /// « 03:42 ».
    pub duration: String,
}
