use uuid::Uuid;

use crate::entity::artist::artist::{Artist, ArtistCreate};

pub struct ArtistRepository;

impl ArtistRepository {

    pub async fn insert_artist<'e, E>(
        exec: E,
        artist: ArtistCreate
    ) -> Result<Artist, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let id: String = Uuid::new_v4().to_string();

        let result: Artist = sqlx::query_as::<_, Artist>(
            r#"
            INSERT INTO artists (
                id,
                name,
                name_normalized,
                sort_name,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)

            ON CONFLICT(name_normalized)
            DO UPDATE SET
                id = artists.id

            RETURNING
                id,
                name,
                name_normalized,
                sort_name,
                bio,
                image_url,
                musicbrainz_id,
                created_at,
                updated_at
            "#
        )
        .bind(&id)
        .bind(&artist.name)
        .bind(&artist.name_normalized)
        .bind(&artist.sort_name)
        .fetch_one(exec)
        .await?;

        Ok(result)
    }

    /// Nom d'un artiste, par son identifiant.
    pub async fn find_name_by_id<'e, E>(exec: E, id: &str) -> Result<Option<String>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_scalar::<_, String>("SELECT name FROM artists WHERE id = ?")
            .bind(id)
            .fetch_optional(exec)
            .await
    }

    pub async fn find_one_by_normalized_name<'e, E>(
        exec: E,
        name_normalized: &str
    ) -> Result<Option<Artist>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query_as::<_, Artist>(
            r#"
            SELECT * FROM artists
            WHERE name_normalized = ?
            LIMIT 1
            "#
        )
        .bind(name_normalized)
        .fetch_optional(exec)
        .await
    }

    pub async fn search<'e, E>(
        exec: E,
        search_term: &str,
        limit: i64,
    ) -> Result<Vec<crate::entity::search::search_result::SearchResult>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, crate::entity::search::search_result::SearchResult>(
            r#"
            SELECT
                a.id AS id,
                'artist' AS result_type,
                a.name AS title,
                NULL AS subtitle,
                a.image_url AS thumbnail_path,
                NULL AS path,
                la.library_id AS library_id
            FROM artists a
            LEFT JOIN library_artists la ON la.artist_id = a.id
            WHERE LOWER(a.name) LIKE ?1
            GROUP BY a.id
            LIMIT ?2
            "#
        )
        .bind(search_term)
        .bind(limit)
        .fetch_all(exec)
        .await
    }

    pub async fn update_image_url<'e, E>(
        exec: E,
        artist_id: &str,
        image_url: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query("UPDATE artists SET image_url = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(image_url)
            .bind(artist_id)
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Met à jour le chemin image_url par ancien chemin (migration)
    pub async fn update_image_url_by_path<'e, E>(
        exec: E,
        new_path: &str,
        old_path: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query("UPDATE artists SET image_url = ?, updated_at = CURRENT_TIMESTAMP WHERE image_url = ?")
            .bind(new_path)
            .bind(old_path)
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Retourne tous les artistes qui n'ont pas encore d'image (image_url IS NULL)
    pub async fn find_without_image<'e, E>(
        exec: E,
    ) -> Result<Vec<Artist>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, Artist>(
            "SELECT * FROM artists WHERE image_url IS NULL ORDER BY name"
        )
        .fetch_all(exec)
        .await
    }

    /// Reset toutes les image_url à NULL (pour forcer un re-fetch)
    pub async fn reset_all_image_urls<'e, E>(
        exec: E,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query("UPDATE artists SET image_url = NULL WHERE image_url IS NOT NULL")
            .execute(exec)
            .await?;
        Ok(())
    }

    pub async fn get_image_url<'e, E>(
        exec: E,
        artist_id: &str,
    ) -> Result<Option<String>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT image_url FROM artists WHERE id = ?"
        )
        .bind(artist_id)
        .fetch_optional(exec)
        .await?;

        Ok(row.and_then(|r| r.0))
    }
}
