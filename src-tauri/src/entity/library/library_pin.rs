use serde::Serialize;

/// Un album ou un artiste épinglé dans la barre latérale.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LibraryPinView {
    /// `album` ou `artist`.
    pub kind: String,
    /// `library_albums.id` ou `artists.id`, selon `kind`.
    pub id: String,
    pub title: String,
    /// L'artiste de l'album ; toujours vide pour un artiste.
    pub subtitle: Option<String>,
    /// Même valeur que `cover_url` des albums et `thumbnail_path` des artistes.
    pub cover: Option<String>,
}
