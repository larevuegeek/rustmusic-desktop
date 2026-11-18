use sqlx::SqlitePool;

use crate::entity::library::artist_split::{ArtistCheckRun, ArtistRelinkRow, ArtistSplitRule};

/// Règles des artistes multiples (`artist_split_rules`) et leurs vérifications (`artist_check_runs`).
pub struct ArtistSplitRepository;

pub fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

impl ArtistSplitRepository {
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<ArtistSplitRule>, sqlx::Error> {
        sqlx::query_as("SELECT name, decision, parts, source, run_id, updated_at FROM artist_split_rules")
            .fetch_all(pool)
            .await
    }

    pub async fn find(pool: &SqlitePool, name: &str) -> Result<Option<ArtistSplitRule>, sqlx::Error> {
        sqlx::query_as("SELECT name, decision, parts, source, run_id, updated_at FROM artist_split_rules WHERE name_key = ?")
            .bind(key(name))
            .fetch_optional(pool)
            .await
    }

    pub async fn upsert(pool: &SqlitePool, name: &str, decision: &str, parts: &[String], source: &str, run_id: Option<i64>) -> Result<(), sqlx::Error> {
        let parts = (!parts.is_empty()).then(|| serde_json::to_string(parts).unwrap_or_default());
        sqlx::query(
            "INSERT INTO artist_split_rules (name_key, name, decision, parts, source, run_id) VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(name_key) DO UPDATE SET name = excluded.name, decision = excluded.decision, parts = excluded.parts,
                 source = excluded.source, run_id = excluded.run_id, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(key(name))
        .bind(name.trim())
        .bind(decision)
        .bind(parts)
        .bind(source)
        .bind(run_id)
        .execute(pool)
        .await
        .map(|_| ())
    }

    pub async fn find_by_run(pool: &SqlitePool, run_id: i64) -> Result<Vec<ArtistSplitRule>, sqlx::Error> {
        sqlx::query_as("SELECT name, decision, parts, source, run_id, updated_at FROM artist_split_rules WHERE run_id = ? ORDER BY name COLLATE NOCASE")
            .bind(run_id)
            .fetch_all(pool)
            .await
    }

    /// Toutes les règles qui demandent une décision de l'utilisateur.
    pub async fn find_unknown(pool: &SqlitePool) -> Result<Vec<ArtistSplitRule>, sqlx::Error> {
        sqlx::query_as("SELECT name, decision, parts, source, run_id, updated_at FROM artist_split_rules WHERE decision = 'unknown' ORDER BY name COLLATE NOCASE")
            .fetch_all(pool)
            .await
    }

    pub async fn start_run(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("INSERT INTO artist_check_runs DEFAULT VALUES RETURNING id").fetch_one(pool).await
    }

    pub async fn finish_run(pool: &SqlitePool, run_id: i64, checked: i64, error: Option<&str>) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE artist_check_runs SET finished_at = CURRENT_TIMESTAMP, checked = ?, error = ? WHERE id = ?")
            .bind(checked)
            .bind(error)
            .bind(run_id)
            .execute(pool)
            .await
            .map(|_| ())
    }

    pub async fn find_run(pool: &SqlitePool, run_id: Option<i64>) -> Result<Option<ArtistCheckRun>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, started_at, finished_at, checked, error FROM artist_check_runs
             WHERE (?1 IS NULL OR id = ?1) ORDER BY id DESC LIMIT 1",
        )
        .bind(run_id)
        .fetch_optional(pool)
        .await
    }

    /// Champs artiste bruts des bibliothèques locales (les serveurs découpent eux-mêmes).
    pub async fn distinct_local_artists(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT DISTINCT lc.artist FROM library_tracks lt
               JOIN library_cache lc ON lc.id = lt.cache_id
              WHERE lc.artist IS NOT NULL AND TRIM(lc.artist) <> '' AND lc.path NOT LIKE '%://%'",
        )
        .fetch_all(pool)
        .await
    }

    /// Pistes locales dont le champ artiste contient ce nom (sans tenir compte de la casse).
    pub async fn tracks_mentioning(pool: &SqlitePool, name: &str) -> Result<Vec<ArtistRelinkRow>, sqlx::Error> {
        sqlx::query_as(
            "SELECT lt.id AS id, lt.library_id AS library_id, lt.artist_id AS artist_id,
                    lc.artist AS artist, lt.tags AS tags, lc.path AS path
               FROM library_tracks lt
               JOIN library_cache lc ON lc.id = lt.cache_id
              WHERE lc.path NOT LIKE '%://%' AND INSTR(LOWER(lc.artist), ?) > 0",
        )
        .bind(key(name))
        .fetch_all(pool)
        .await
    }

    /// Noms d'artistes déjà connus localement (clé normalisée), pour éviter des requêtes en ligne.
    pub async fn known_names(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT name_normalized FROM artists").fetch_all(pool).await
    }

    /// Photo connue de chaque artiste qui en a une : (nom normalisé, chemin ou adresse).
    pub async fn images(pool: &SqlitePool) -> Result<Vec<(String, String)>, sqlx::Error> {
        sqlx::query_as("SELECT name_normalized, image_url FROM artists WHERE image_url IS NOT NULL AND image_url <> ''")
            .fetch_all(pool)
            .await
    }

    pub async fn delete_track_links<'e, E>(exec: E, track_id: &str) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query("DELETE FROM library_track_artists WHERE library_track_id = ?")
            .bind(track_id)
            .execute(exec)
            .await
            .map(|_| ())
    }
}
