use crate::entity::library::library_pin::LibraryPinView;

pub struct LibraryPinRepository;

impl LibraryPinRepository {

    // =========================================
    // LISTE (barre latérale)
    // =========================================
    pub async fn find_by_library_id<'e, E>(
        exec: E,
        library_id: i64,
    ) -> Result<Vec<LibraryPinView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        // Images calculées comme `get_albums` et `get_artists`, pour que le front
        // les affiche avec le même composant.
        sqlx::query_as::<_, LibraryPinView>(
            r#"
            SELECT
                CASE WHEN p.library_album_id IS NOT NULL THEN 'album' ELSE 'artist' END AS kind,
                COALESCE(la.id, ar.id)       AS id,
                COALESCE(la.title, ar.name)  AS title,
                aa.name                      AS subtitle,
                CASE WHEN p.library_album_id IS NOT NULL THEN la.cover_url
                     ELSE COALESCE(NULLIF(ar.image_url, ''), (
                        SELECT lc.thumbnail_path
                        FROM library_tracks lt
                        INNER JOIN library_cache lc ON lc.id = lt.cache_id
                        WHERE lt.artist_id = ar.id AND lt.library_id = ?1 AND lc.thumbnail_path IS NOT NULL
                        LIMIT 1
                     ))
                END AS cover
            FROM library_pins p
            LEFT JOIN library_albums la ON la.id = p.library_album_id
            LEFT JOIN artists aa ON aa.id = la.artist_id
            LEFT JOIN artists ar ON ar.id = p.artist_id
            WHERE p.library_id = ?1
              AND (la.id IS NOT NULL OR ar.id IS NOT NULL)
            ORDER BY p.position, p.created_at, p.id
            "#
        )
        .bind(library_id)
        .fetch_all(exec)
        .await
    }

    // =========================================
    // APPARTENANCE À LA BIBLIOTHÈQUE
    // =========================================
    pub async fn album_in_library<'e, E>(
        exec: E,
        library_id: i64,
        album_id: &str,
    ) -> Result<bool, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM library_albums WHERE id = ? AND library_id = ?)"
        )
        .bind(album_id)
        .bind(library_id)
        .fetch_one(exec)
        .await
    }

    pub async fn artist_in_library<'e, E>(
        exec: E,
        library_id: i64,
        artist_id: &str,
    ) -> Result<bool, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM library_artists WHERE artist_id = ? AND library_id = ?)"
        )
        .bind(artist_id)
        .bind(library_id)
        .fetch_one(exec)
        .await
    }

    // =========================================
    // ÉPINGLER (en fin de liste, idempotent)
    // =========================================
    pub async fn pin_album<'e, E>(
        exec: E,
        library_id: i64,
        album_id: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO library_pins (library_id, library_album_id, position)
            VALUES (?1, ?2, (SELECT COALESCE(MAX(position), -1) + 1 FROM library_pins WHERE library_id = ?1))
            "#
        )
        .bind(library_id)
        .bind(album_id)
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn pin_artist<'e, E>(
        exec: E,
        library_id: i64,
        artist_id: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO library_pins (library_id, artist_id, position)
            VALUES (?1, ?2, (SELECT COALESCE(MAX(position), -1) + 1 FROM library_pins WHERE library_id = ?1))
            "#
        )
        .bind(library_id)
        .bind(artist_id)
        .execute(exec)
        .await?;

        Ok(())
    }

    // =========================================
    // DÉSÉPINGLER
    // =========================================
    pub async fn unpin_album<'e, E>(
        exec: E,
        library_id: i64,
        album_id: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query("DELETE FROM library_pins WHERE library_id = ? AND library_album_id = ?")
            .bind(library_id)
            .bind(album_id)
            .execute(exec)
            .await?;

        Ok(())
    }

    pub async fn unpin_artist<'e, E>(
        exec: E,
        library_id: i64,
        artist_id: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query("DELETE FROM library_pins WHERE library_id = ? AND artist_id = ?")
            .bind(library_id)
            .bind(artist_id)
            .execute(exec)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

    async fn base() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("base mémoire");
        sqlx::migrate!("./SQL/").run(&pool).await.expect("migrations");
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')")
            .execute(&pool).await.expect("profil");
        sqlx::query("INSERT OR IGNORE INTO library (id, profil_id, name) VALUES (1, 1, 'test'), (2, 1, 'autre')")
            .execute(&pool).await.expect("bibliothèques");
        sqlx::query("INSERT INTO artists (id, name, name_normalized, sort_name, image_url) VALUES ('a1', 'Nirvana', 'nirvana', 'Nirvana', 'img.jpg')")
            .execute(&pool).await.expect("artiste");
        sqlx::query("INSERT INTO library_artists (library_id, artist_id) VALUES (1, 'a1')")
            .execute(&pool).await.expect("lien bibliothèque");
        sqlx::query("INSERT INTO library_albums (id, library_id, artist_id, title, title_normalized, cover_url) VALUES ('alb1', 1, 'a1', 'Nevermind', 'nevermind', 'nevermind.jpg')")
            .execute(&pool).await.expect("album");
        pool
    }

    #[tokio::test]
    async fn epingler_deux_fois_ne_duplique_rien_et_garde_l_ordre() {
        let pool = base().await;
        LibraryPinRepository::pin_album(&pool, 1, "alb1").await.unwrap();
        LibraryPinRepository::pin_artist(&pool, 1, "a1").await.unwrap();
        LibraryPinRepository::pin_album(&pool, 1, "alb1").await.unwrap();

        let pins = LibraryPinRepository::find_by_library_id(&pool, 1).await.unwrap();
        let vus: Vec<(&str, &str, Option<&str>, Option<&str>)> = pins.iter()
            .map(|p| (p.kind.as_str(), p.id.as_str(), p.subtitle.as_deref(), p.cover.as_deref()))
            .collect();
        assert_eq!(vus, vec![
            ("album", "alb1", Some("Nirvana"), Some("nevermind.jpg")),
            ("artist", "a1", None, Some("img.jpg")),
        ]);

        LibraryPinRepository::unpin_album(&pool, 1, "alb1").await.unwrap();
        assert_eq!(LibraryPinRepository::find_by_library_id(&pool, 1).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn l_appartenance_se_verifie_par_bibliotheque() {
        let pool = base().await;
        assert!(LibraryPinRepository::album_in_library(&pool, 1, "alb1").await.unwrap());
        assert!(!LibraryPinRepository::album_in_library(&pool, 2, "alb1").await.unwrap());
        assert!(LibraryPinRepository::artist_in_library(&pool, 1, "a1").await.unwrap());
        assert!(!LibraryPinRepository::artist_in_library(&pool, 2, "a1").await.unwrap());
    }
}
