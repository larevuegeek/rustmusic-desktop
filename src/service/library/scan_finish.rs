//! Ce qui suit chaque scan ou import d'une bibliothèque, une fois tous ses fichiers lus.

use sqlx::SqlitePool;

use crate::repository::queue::queue_track_repository::QueueTrackRepository;

/// Albums rendus à l'artiste de leurs pistes, file d'attente aux titres relus.
pub async fn finish_scan(pool: &SqlitePool, library_id: i64) {
    super::album_owner::restore_album_owners(pool, library_id).await;
    if let Err(e) = QueueTrackRepository::refresh_from_library(pool, library_id).await {
        log::warn!("📋 Titres de la file non repris des tags : {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn the_queue_takes_the_reread_titles_of_its_library_only() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.expect("base mémoire");
        sqlx::migrate!("./SQL/").run(&pool).await.expect("migrations");
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO library (id, profil_id, name) VALUES (1, 1, 'test')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library_files (id, library_id, path, filename, extension, size, status) VALUES ('f1', 1, '/giac-mo.mp3', 'f', 'mp3', 1, 'indexed')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library_cache (path, title, artist) VALUES ('/giac-mo.mp3', 'Giấc Mơ Thần Tiên', 'Lương Bích Hữu'), ('/ailleurs.mp3', 'Đúng', 'Ailleurs')")
            .execute(&pool).await.unwrap();
        for (id, path, title) in [("q1", "/giac-mo.mp3", "Gi?c M? Th?n Tiên"), ("q2", "/ailleurs.mp3", "D?ng")] {
            sqlx::query("INSERT INTO queue_tracks (queue_id, profil_id, position, path, title, artist) VALUES (?, 1, 0, ?, ?, 'ancien')")
                .bind(id).bind(path).bind(title).execute(&pool).await.unwrap();
        }

        finish_scan(&pool, 1).await;

        let titles: Vec<(String, String)> = sqlx::query_as("SELECT title, artist FROM queue_tracks ORDER BY queue_id").fetch_all(&pool).await.unwrap();
        assert_eq!(titles, vec![
            ("Giấc Mơ Thần Tiên".into(), "Lương Bích Hữu".into()),
            // Hors de la bibliothèque scannée : inchangé.
            ("D?ng".into(), "ancien".into()),
        ]);
    }
}
