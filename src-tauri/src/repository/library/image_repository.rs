use sqlx::SqlitePool;

/// Colonnes qui citent une image du cache (pochettes, vignettes, portraits).
const IMAGE_COLUMNS: [&str; 5] = [
    "SELECT cover_url FROM library_albums",
    "SELECT thumbnail_path FROM library_cache",
    "SELECT image_url FROM artists",
    "SELECT cover FROM queue_tracks",
    "SELECT cover FROM playlists",
];

pub struct ImageRepository;

impl ImageRepository {
    /// Tous les chemins d'images cités par la base, doublons compris.
    pub async fn referenced_paths(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
        let mut paths = Vec::new();
        for sql in IMAGE_COLUMNS {
            let rows: Vec<Option<String>> = sqlx::query_scalar(sql).fetch_all(pool).await?;
            paths.extend(rows.into_iter().flatten());
        }
        Ok(paths)
    }
}
