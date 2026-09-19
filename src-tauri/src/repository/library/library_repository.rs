use crate::entity::library::library::{Library, LibraryCreate};

pub struct LibraryRepository;

impl LibraryRepository {

    pub async fn insert_library<'e, E>(
        exec: E,
        library: &LibraryCreate,
    ) -> Result<Library, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let inserted: Library = sqlx::query_as::<_, Library>(
            r#"
            INSERT INTO library (
                profil_id,
                name,
                description,
                position,
                created_at,
                updated_at
            )
            VALUES (
                ?1, ?2, ?3, ?4,
                CURRENT_TIMESTAMP,
                CURRENT_TIMESTAMP
            )
            RETURNING
                id,
                profil_id,
                name,
                description,
                cover,
                position,
                total_tracks,
                total_albums,
                total_artists,
                is_default,
                created_at,
                updated_at
            "#
        )
        .bind(library.profil_id)
        .bind(&library.name)
        .bind(&library.description)
        .bind(0)
        .fetch_one(exec)
        .await?;

        Ok(inserted)
    }

    pub async fn find_library_by_id<'e, E>(
        exec: E,
        library_id: i64,
    ) -> Result<Library, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let library: Library = sqlx::query_as::<_, Library>(
            r#"
            SELECT
                id,
                profil_id,
                name,
                description,
                cover,
                position,
                total_tracks,
                total_albums,
                -- Compte vivant : `total_artists`, tenue par des declencheurs,
                -- ne redescend jamais quand une cascade efface pistes et
                -- albums. Par ensembles et non par artiste — la version
                -- correlee prenait 8,5 s contre 16 ms.
                (SELECT COUNT(*) FROM (
                    SELECT la.artist_id FROM library_artists la
                    WHERE la.library_id = library.id
                    INTERSECT
                    SELECT artist_id FROM (
                        SELECT lt.artist_id FROM library_tracks lt
                        WHERE lt.library_id = library.id AND lt.artist_id IS NOT NULL
                        UNION
                        SELECT lta.artist_id FROM library_track_artists lta
                        WHERE lta.library_id = library.id
                        UNION
                        SELECT lb.artist_id FROM library_albums lb
                        WHERE lb.library_id = library.id AND lb.artist_id IS NOT NULL
                    )
                )) AS total_artists,
                is_default,
                created_at,
                updated_at
            FROM library
            WHERE id = ?
            "#
        )
        .bind(library_id)
        .fetch_one(exec)
        .await?;

        Ok(library)
    }

    /// All libraries across all profiles, ordered by position then id.
    /// Used by features that span profiles (e.g. DLNA server exposing every lib).
    pub async fn find_all<'e, E>(exec: E) -> Result<Vec<Library>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, Library>(
            r#"
            SELECT
                id,
                profil_id,
                name,
                description,
                cover,
                position,
                total_tracks,
                total_albums,
                -- Compte vivant : `total_artists`, tenue par des declencheurs,
                -- ne redescend jamais quand une cascade efface pistes et
                -- albums. Par ensembles et non par artiste — la version
                -- correlee prenait 8,5 s contre 16 ms.
                (SELECT COUNT(*) FROM (
                    SELECT la.artist_id FROM library_artists la
                    WHERE la.library_id = library.id
                    INTERSECT
                    SELECT artist_id FROM (
                        SELECT lt.artist_id FROM library_tracks lt
                        WHERE lt.library_id = library.id AND lt.artist_id IS NOT NULL
                        UNION
                        SELECT lta.artist_id FROM library_track_artists lta
                        WHERE lta.library_id = library.id
                        UNION
                        SELECT lb.artist_id FROM library_albums lb
                        WHERE lb.library_id = library.id AND lb.artist_id IS NOT NULL
                    )
                )) AS total_artists,
                is_default,
                created_at,
                updated_at
            FROM library
            ORDER BY position ASC, id ASC
            "#
        )
        .fetch_all(exec)
        .await
    }

    pub async fn find_libraries_by_profil_id<'e, E>(
        exec: E,
        profil_id: i64,
    ) -> Result<Vec<Library>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let libraries: Vec<Library> = sqlx::query_as::<_, Library>(
            r#"
            SELECT
                id,
                profil_id,
                name,
                description,
                cover,
                position,
                total_tracks,
                total_albums,
                -- Compte vivant : `total_artists`, tenue par des declencheurs,
                -- ne redescend jamais quand une cascade efface pistes et
                -- albums. Par ensembles et non par artiste — la version
                -- correlee prenait 8,5 s contre 16 ms.
                (SELECT COUNT(*) FROM (
                    SELECT la.artist_id FROM library_artists la
                    WHERE la.library_id = library.id
                    INTERSECT
                    SELECT artist_id FROM (
                        SELECT lt.artist_id FROM library_tracks lt
                        WHERE lt.library_id = library.id AND lt.artist_id IS NOT NULL
                        UNION
                        SELECT lta.artist_id FROM library_track_artists lta
                        WHERE lta.library_id = library.id
                        UNION
                        SELECT lb.artist_id FROM library_albums lb
                        WHERE lb.library_id = library.id AND lb.artist_id IS NOT NULL
                    )
                )) AS total_artists,
                is_default,
                created_at,
                updated_at
            FROM library
            WHERE profil_id = ?
            ORDER BY position ASC
            "#
        )
        .bind(profil_id)
        .fetch_all(exec)
        .await?;

        Ok(libraries)
    }

    /// Désigne `library_id` comme bibliothèque par défaut de son profil.
    ///
    /// Les deux ordres sont indissociables : l'index partiel n'admet qu'un seul
    /// défaut par profil, donc poser le nouveau avant d'avoir retiré l'ancien
    /// échouerait. D'où la transaction — et la signature qui prend le pool
    /// plutôt qu'un exécuteur quelconque, puisqu'il faut pouvoir l'ouvrir.
    pub async fn set_default(
        pool: &sqlx::SqlitePool,
        library_id: i64,
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE library
            SET is_default = 0
            WHERE profil_id = (SELECT profil_id FROM library WHERE id = ?1)
            "#
        )
        .bind(library_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE library
            SET is_default = 1
            WHERE id = ?1
            "#
        )
        .bind(library_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await
    }

    /// S'assure que le profil a bien une bibliothèque par défaut.
    ///
    /// Deux moments l'exigent : la création de la toute première bibliothèque
    /// d'un profil, et la suppression de celle qui était par défaut. Sans ça le
    /// profil se retrouverait sans point d'entrée et le démarrage retomberait
    /// silencieusement sur la première venue — le comportement qu'on remplace.
    ///
    /// La clause `NOT EXISTS` rend l'appel sans effet quand un défaut existe
    /// déjà : on peut l'appeler sans avoir à vérifier d'abord.
    pub async fn ensure_default<'e, E>(exec: E, profil_id: i64) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query(
            r#"
            UPDATE library
            SET is_default = 1
            WHERE id = (
                SELECT id FROM library
                WHERE profil_id = ?1
                ORDER BY position ASC, id ASC
                LIMIT 1
            )
            AND NOT EXISTS (
                SELECT 1 FROM library WHERE profil_id = ?1 AND is_default = 1
            )
            "#
        )
        .bind(profil_id)
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn remove_library<'e, E>(
        exec: E,
        library_id: i64,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query(
            r#"
            DELETE FROM library
            WHERE id = ?
            "#
        )
        .bind(library_id)
        .execute(exec)
        .await?;

        Ok(())
    }
}
