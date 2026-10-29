use sqlx::SqlitePool;

/// Actions à renvoyer à un serveur injoignable (favori, note, écoute, playlist).
pub struct ServerOutboxRepository;

impl ServerOutboxRepository {
    pub async fn insert(pool: &SqlitePool, server_id: i64, action: &str, song_id: &str, value: Option<i64>) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO server_outbox (server_id, action, song_id, value) VALUES (?, ?, ?, ?)")
            .bind(server_id)
            .bind(action)
            .bind(song_id)
            .bind(value)
            .execute(pool)
            .await
            .map(|_| ())
    }

    /// En attente pour ce serveur, dans l'ordre : (id, action, song_id, value).
    pub async fn find_by_server(pool: &SqlitePool, server_id: i64) -> Result<Vec<(i64, String, String, Option<i64>)>, sqlx::Error> {
        sqlx::query_as("SELECT id, action, song_id, value FROM server_outbox WHERE server_id = ? ORDER BY id")
            .bind(server_id)
            .fetch_all(pool)
            .await
    }

    pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM server_outbox WHERE id = ?").bind(id).execute(pool).await.map(|_| ())
    }

    /// Retire l'action en attente sur ce morceau (ou cette playlist).
    pub async fn delete_action(pool: &SqlitePool, server_id: i64, song_id: &str, action: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM server_outbox WHERE server_id = ? AND song_id = ? AND action = ?")
            .bind(server_id)
            .bind(song_id)
            .bind(action)
            .execute(pool)
            .await
            .map(|_| ())
    }
}
