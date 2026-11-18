use sqlx::SqlitePool;

use crate::entity::server::server::OfflineItem;

/// Morceaux d'un serveur gardés hors ligne (`file` vide : en attente).
pub struct OfflineTrackRepository;

impl OfflineTrackRepository {
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<OfflineItem>, sqlx::Error> {
        sqlx::query_as("SELECT path, server_id, file IS NOT NULL AS ready, size FROM offline_tracks ORDER BY added_at")
            .fetch_all(pool)
            .await
    }

    /// Fichier téléchargé de ce morceau, s'il l'est.
    pub async fn ready_file(pool: &SqlitePool, path: &str) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT file FROM offline_tracks WHERE path = ? AND file IS NOT NULL")
            .bind(path)
            .fetch_optional(pool)
            .await
    }

    /// Chemins en attente, du plus ancien au plus récent.
    pub async fn pending_paths(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT path FROM offline_tracks WHERE file IS NULL ORDER BY added_at")
            .fetch_all(pool)
            .await
    }

    /// Rend 1 si le morceau est nouveau, 0 s'il y était déjà.
    pub async fn insert(pool: &SqlitePool, path: &str, server_id: i64) -> Result<u64, sqlx::Error> {
        sqlx::query("INSERT OR IGNORE INTO offline_tracks (path, server_id) VALUES (?, ?)")
            .bind(path)
            .bind(server_id)
            .execute(pool)
            .await
            .map(|r| r.rows_affected())
    }

    pub async fn set_file(pool: &SqlitePool, path: &str, file: &str, size: i64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE offline_tracks SET file = ?, size = ? WHERE path = ?")
            .bind(file)
            .bind(size)
            .bind(path)
            .execute(pool)
            .await
            .map(|_| ())
    }

    /// Retire le morceau ; rend son fichier téléchargé, à effacer.
    pub async fn delete(pool: &SqlitePool, path: &str) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar::<_, Option<String>>("DELETE FROM offline_tracks WHERE path = ? RETURNING file")
            .bind(path)
            .fetch_optional(pool)
            .await
            .map(Option::flatten)
    }
}
