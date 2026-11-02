//! Compositeurs : lus dans les tags des morceaux (`tags.composer`), sans table à eux.

use sqlx::SqlitePool;

/// Un morceau qui a un compositeur : (id, album, durée, compositeur brut).
pub type ComposerRow = (String, Option<String>, Option<f64>, String);

pub struct ComposerRepository;

impl ComposerRepository {
    /// Les compositeurs bruts de toutes les bibliothèques, sans doublon.
    pub async fn all_raw(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT DISTINCT TRIM(json_extract(tags, '$.composer')) FROM library_tracks
             WHERE TRIM(COALESCE(json_extract(tags, '$.composer'), '')) <> ''",
        )
        .fetch_all(pool)
        .await
    }

    /// Photos cherchées : (nom normalisé, chemin ou "" si introuvable).
    pub async fn images(pool: &SqlitePool) -> Result<Vec<(String, String)>, sqlx::Error> {
        sqlx::query_as("SELECT name_key, image_url FROM composer_images").fetch_all(pool).await
    }

    pub async fn save_image(pool: &SqlitePool, name_key: &str, image_url: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO composer_images (name_key, image_url) VALUES (?, ?)
             ON CONFLICT(name_key) DO UPDATE SET image_url = excluded.image_url, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(name_key)
        .bind(image_url)
        .execute(pool)
        .await
        .map(|_| ())
    }

    /// Les introuvables redeviennent à chercher (« relancer » des portraits).
    pub async fn reset_not_found(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM composer_images WHERE image_url = ''").execute(pool).await.map(|_| ())
    }

    pub async fn rows(pool: &SqlitePool, library_id: i64) -> Result<Vec<ComposerRow>, sqlx::Error> {
        sqlx::query_as(
            "SELECT lt.id, lt.library_album_id, lt.duration, json_extract(lt.tags, '$.composer') AS c
             FROM library_tracks lt
             WHERE lt.library_id = ? AND TRIM(COALESCE(json_extract(lt.tags, '$.composer'), '')) <> ''",
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
    }

    /// Les artistes de la bibliothèque : (nom normalisé, id), pour relier un compositeur à sa page d'artiste.
    pub async fn library_artists(pool: &SqlitePool, library_id: i64) -> Result<Vec<(String, String)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT a.name_normalized, a.id FROM library_artists lat INNER JOIN artists a ON a.id = lat.artist_id WHERE lat.library_id = ?",
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
    }
}
