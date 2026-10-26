use sqlx::SqlitePool;

use crate::core::radio::directory::RadioResult;
use crate::entity::radio::radio::{RadioRecent, RadioView};

pub struct RadioRepository;

const COLUMNS: &str = "id, uuid, name, stream_url, homepage, favicon_url, logo_path, country_code, tags, codec, bitrate, \
                       reduced_quality, position, last_played_at, created_at";

impl RadioRepository {
    pub async fn find_by_profile(pool: &SqlitePool, profil_id: i64) -> Result<Vec<RadioView>, sqlx::Error> {
        sqlx::query_as::<_, RadioView>(sqlx::AssertSqlSafe(format!(
            "SELECT {COLUMNS} FROM radio_stations WHERE profil_id = ? ORDER BY position, created_at, id"
        )))
        .bind(profil_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_one(pool: &SqlitePool, id: i64) -> Result<Option<RadioView>, sqlx::Error> {
        sqlx::query_as::<_, RadioView>(sqlx::AssertSqlSafe(format!("SELECT {COLUMNS} FROM radio_stations WHERE id = ?")))
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Ajoute la station en dernière place ; déjà là (même flux), rend l'existante.
    pub async fn add(pool: &SqlitePool, profil_id: i64, r: &RadioResult) -> Result<i64, sqlx::Error> {
        if let Some(id) = sqlx::query_scalar::<_, i64>("SELECT id FROM radio_stations WHERE profil_id = ? AND stream_url = ?")
            .bind(profil_id)
            .bind(&r.stream_url)
            .fetch_optional(pool)
            .await?
        {
            return Ok(id);
        }
        let result = sqlx::query(
            "INSERT INTO radio_stations
               (profil_id, uuid, name, stream_url, homepage, favicon_url, country_code, tags, codec, bitrate, reduced_quality, position)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
               (SELECT COALESCE(MAX(position), -1) + 1 FROM radio_stations WHERE profil_id = ?))",
        )
        .bind(profil_id)
        .bind(&r.uuid)
        .bind(&r.name)
        .bind(&r.stream_url)
        .bind(&r.homepage)
        .bind(&r.favicon)
        .bind(&r.country_code)
        .bind(r.tags.join(","))
        .bind(&r.codec)
        .bind(r.bitrate.map(i64::from))
        .bind(r.reduced_quality)
        .bind(profil_id)
        .execute(pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM radio_stations WHERE id = ?").bind(id).execute(pool).await.map(|_| ())
    }

    pub async fn set_logo(pool: &SqlitePool, id: i64, path: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE radio_stations SET logo_path = ? WHERE id = ?").bind(path).bind(id).execute(pool).await.map(|_| ())
    }

    pub async fn rename(pool: &SqlitePool, id: i64, name: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE radio_stations SET name = ? WHERE id = ?").bind(name).bind(id).execute(pool).await.map(|_| ())
    }

    /// Nouvel ordre des favoris, `ids` dans l'ordre voulu.
    pub async fn reorder(pool: &SqlitePool, profil_id: i64, ids: &[i64]) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        for (position, id) in ids.iter().enumerate() {
            sqlx::query("UPDATE radio_stations SET position = ? WHERE id = ? AND profil_id = ?")
                .bind(position as i64)
                .bind(id)
                .bind(profil_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await
    }

    /// Retient l'écoute ; seules les `KEEP` plus récentes restent.
    /// La favorite passe à une autre version du flux de sa station.
    pub async fn set_stream(pool: &SqlitePool, id: i64, v: &crate::core::radio::directory::StreamVariant) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE radio_stations SET stream_url = ?, codec = ?, bitrate = ?, reduced_quality = ? WHERE id = ?")
            .bind(&v.stream_url)
            .bind(&v.codec)
            .bind(v.bitrate.map(i64::from))
            .bind(v.reduced_quality)
            .bind(id)
            .execute(pool)
            .await
            .map(|_| ())
    }

    /// Titre entendu ; seuls les 300 derniers du profil sont gardés.
    pub async fn record_title(pool: &SqlitePool, profil_id: i64, station: &str, stream_url: &str, title: &str) -> Result<(), sqlx::Error> {
        const KEEP: i64 = 300;
        sqlx::query("INSERT INTO radio_titles (profil_id, station, stream_url, title) VALUES (?, ?, ?, ?)")
            .bind(profil_id)
            .bind(station)
            .bind(stream_url)
            .bind(title)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM radio_titles WHERE profil_id = ?1 AND id NOT IN (SELECT id FROM radio_titles WHERE profil_id = ?1 ORDER BY id DESC LIMIT ?2)")
            .bind(profil_id)
            .bind(KEEP)
            .execute(pool)
            .await
            .map(|_| ())
    }

    pub async fn titles(pool: &SqlitePool, profil_id: i64, limit: i64) -> Result<Vec<crate::entity::radio::radio::RadioTitle>, sqlx::Error> {
        sqlx::query_as(
            "SELECT t.id, t.station, t.stream_url, t.title, t.played_at, r.logo_path
               FROM radio_titles t
               LEFT JOIN radio_recent r ON r.profil_id = t.profil_id AND r.stream_url = t.stream_url
              WHERE t.profil_id = ? ORDER BY t.id DESC LIMIT ?",
        )
        .bind(profil_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn clear_titles(pool: &SqlitePool, profil_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM radio_titles WHERE profil_id = ?").bind(profil_id).execute(pool).await.map(|_| ())
    }

    /// L'historique du profil : dernières radios écoutées et titres passés.
    pub async fn clear_history(pool: &SqlitePool, profil_id: i64) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query("DELETE FROM radio_recent WHERE profil_id = ?").bind(profil_id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM radio_titles WHERE profil_id = ?").bind(profil_id).execute(&mut *tx).await?;
        tx.commit().await
    }

    pub async fn record_recent(pool: &SqlitePool, profil_id: i64, r: &RadioResult, logo_path: Option<&str>) -> Result<(), sqlx::Error> {
        const KEEP: i64 = 30;
        sqlx::query(
            "INSERT INTO radio_recent
               (profil_id, uuid, name, stream_url, homepage, favicon_url, logo_path, country_code, tags, codec, bitrate, reduced_quality)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(profil_id, stream_url) DO UPDATE SET
               uuid = excluded.uuid, name = excluded.name, homepage = excluded.homepage, favicon_url = excluded.favicon_url,
               logo_path = COALESCE(excluded.logo_path, radio_recent.logo_path), country_code = excluded.country_code,
               tags = excluded.tags, codec = excluded.codec, bitrate = excluded.bitrate,
               reduced_quality = excluded.reduced_quality, played_at = CURRENT_TIMESTAMP",
        )
        .bind(profil_id)
        .bind(&r.uuid)
        .bind(&r.name)
        .bind(&r.stream_url)
        .bind(&r.homepage)
        .bind(&r.favicon)
        .bind(logo_path)
        .bind(&r.country_code)
        .bind(r.tags.join(","))
        .bind(&r.codec)
        .bind(r.bitrate.map(i64::from))
        .bind(r.reduced_quality)
        .execute(pool)
        .await?;
        sqlx::query(
            "DELETE FROM radio_recent WHERE profil_id = ? AND id NOT IN
               (SELECT id FROM radio_recent WHERE profil_id = ? ORDER BY played_at DESC, id DESC LIMIT ?)",
        )
        .bind(profil_id)
        .bind(profil_id)
        .bind(KEEP)
        .execute(pool)
        .await
        .map(|_| ())
    }

    pub async fn recent(pool: &SqlitePool, profil_id: i64) -> Result<Vec<RadioRecent>, sqlx::Error> {
        sqlx::query_as::<_, RadioRecent>(
            "SELECT id, uuid, name, stream_url, homepage, favicon_url, logo_path, country_code, tags, codec, bitrate,
                    reduced_quality, played_at
             FROM radio_recent WHERE profil_id = ? ORDER BY played_at DESC, id DESC",
        )
        .bind(profil_id)
        .fetch_all(pool)
        .await
    }

    pub async fn set_played(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE radio_stations SET last_played_at = CURRENT_TIMESTAMP WHERE id = ?").bind(id).execute(pool).await.map(|_| ())
    }
}
