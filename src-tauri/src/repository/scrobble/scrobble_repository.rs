use sqlx::SqlitePool;

use crate::core::scrobble::listenbrainz::Listen;
use crate::entity::scrobble::scrobble::ScrobbleAccount;

/// Comptes de suivi d'écoute (`scrobble_accounts`) et écoutes en attente d'envoi (`scrobble_queue`).
pub struct ScrobbleRepository;

impl ScrobbleRepository {
    pub async fn find_account(pool: &SqlitePool, profil_id: i64, service: &str) -> Result<Option<ScrobbleAccount>, sqlx::Error> {
        sqlx::query_as("SELECT service, token, user_name, now_playing, api_key, api_secret FROM scrobble_accounts WHERE profil_id = ? AND service = ?")
            .bind(profil_id)
            .bind(service)
            .fetch_optional(pool)
            .await
    }

    /// Tous les comptes reliés d'un profil.
    pub async fn find_accounts(pool: &SqlitePool, profil_id: i64) -> Result<Vec<ScrobbleAccount>, sqlx::Error> {
        sqlx::query_as("SELECT service, token, user_name, now_playing, api_key, api_secret FROM scrobble_accounts WHERE profil_id = ?")
            .bind(profil_id)
            .fetch_all(pool)
            .await
    }

    /// Nouveau compte, ou nouveau jeton pour le même ; `api` : la clé d'API personnelle (Last.fm).
    pub async fn upsert_account(
        pool: &SqlitePool,
        profil_id: i64,
        service: &str,
        token: &str,
        user_name: &str,
        api: Option<(&str, &str)>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO scrobble_accounts (profil_id, service, token, user_name, api_key, api_secret) VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(profil_id, service) DO UPDATE SET token = excluded.token, user_name = excluded.user_name,
                 api_key = excluded.api_key, api_secret = excluded.api_secret",
        )
        .bind(profil_id)
        .bind(service)
        .bind(token)
        .bind(user_name)
        .bind(api.map(|a| a.0))
        .bind(api.map(|a| a.1))
        .execute(pool)
        .await
        .map(|_| ())
    }

    pub async fn set_now_playing(pool: &SqlitePool, profil_id: i64, service: &str, on: bool) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE scrobble_accounts SET now_playing = ? WHERE profil_id = ? AND service = ?")
            .bind(on)
            .bind(profil_id)
            .bind(service)
            .execute(pool)
            .await
            .map(|_| ())
    }

    pub async fn delete_account<'e, E>(exec: E, profil_id: i64, service: &str) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query("DELETE FROM scrobble_accounts WHERE profil_id = ? AND service = ?")
            .bind(profil_id)
            .bind(service)
            .execute(exec)
            .await
            .map(|_| ())
    }

    pub async fn enqueue(pool: &SqlitePool, profil_id: i64, service: &str, listen: &Listen) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO scrobble_queue (profil_id, service, listened_at, artist, title, album, duration_ms) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(profil_id)
            .bind(service)
            .bind(listen.listened_at)
            .bind(&listen.artist)
            .bind(&listen.title)
            .bind(&listen.album)
            .bind(listen.duration_ms)
            .execute(pool)
            .await
            .map(|_| ())
    }

    /// Les plus anciennes écoutes en attente, au plus `limit` : (id, écoute).
    pub async fn find_pending(pool: &SqlitePool, profil_id: i64, service: &str, limit: i64) -> Result<Vec<(i64, Listen)>, sqlx::Error> {
        let rows: Vec<(i64, i64, String, String, Option<String>, Option<i64>)> = sqlx::query_as(
            "SELECT id, listened_at, artist, title, album, duration_ms FROM scrobble_queue
             WHERE profil_id = ? AND service = ? ORDER BY listened_at LIMIT ?",
        )
        .bind(profil_id)
        .bind(service)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, listened_at, artist, title, album, duration_ms)| (id, Listen { listened_at, artist, title, album, duration_ms }))
            .collect())
    }

    pub async fn count_pending(pool: &SqlitePool, profil_id: i64, service: &str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM scrobble_queue WHERE profil_id = ? AND service = ?")
            .bind(profil_id)
            .bind(service)
            .fetch_one(pool)
            .await
    }

    /// Profils qui ont des écoutes en attente.
    pub async fn profiles_with_pending(pool: &SqlitePool) -> Result<Vec<i64>, sqlx::Error> {
        sqlx::query_scalar("SELECT DISTINCT profil_id FROM scrobble_queue").fetch_all(pool).await
    }

    pub async fn delete_pending(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM scrobble_queue WHERE id = ?").bind(id).execute(pool).await.map(|_| ())
    }

    pub async fn delete_all_pending<'e, E>(exec: E, profil_id: i64, service: &str) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query("DELETE FROM scrobble_queue WHERE profil_id = ? AND service = ?")
            .bind(profil_id)
            .bind(service)
            .execute(exec)
            .await
            .map(|_| ())
    }
}
