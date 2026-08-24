use sqlx::Error;

use crate::entity::playlist::playlist::Playlist;

pub struct PlaylistRepository;

impl PlaylistRepository {

    pub async fn insert_playlist<'e, E>(
        exec: E,
        profil_id: i64,
        library_id: Option<i64>,
        name: String,
        description: Option<String>,
        color: String,
        icon: String,
        cover: Option<String>,
        position: i64,
    ) -> Result<Playlist, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let playlist = sqlx::query_as::<_, Playlist>(
            r#"
            INSERT INTO playlists (
                profil_id, library_id, name, description, color, icon, cover, position
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING
                id, profil_id, library_id, name, description, color, icon, cover,
                track_count, duration, position, is_smart, created_at, updated_at
            "#,
        )
        .bind(profil_id)
        .bind(library_id)
        .bind(name)
        .bind(description)
        .bind(color)
        .bind(icon)
        .bind(cover)
        .bind(position)
        .fetch_one(exec)
        .await?;

        Ok(playlist)
    }

    pub async fn update_playlist<'e, E>(
        exec: E,
        id: i64,
        name: String,
        description: Option<String>,
        color: String,
        icon: String,
        cover: Option<String>,
        library_id: Option<i64>,
        position: i64,
    ) -> Result<Option<Playlist>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let playlist = sqlx::query_as::<_, Playlist>(
            r#"
            UPDATE playlists
            SET
                name = ?, description = ?, color = ?, icon = ?,
                cover = ?, library_id = ?, position = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            RETURNING
                id, profil_id, library_id, name, description, color, icon, cover,
                track_count, duration, position, is_smart, created_at, updated_at
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(color)
        .bind(icon)
        .bind(cover)
        .bind(library_id)
        .bind(position)
        .bind(id)
        .fetch_optional(exec)
        .await?;

        Ok(playlist)
    }

    pub async fn update_playlist_stats<'e, E>(
        exec: E,
        id: i64,
        track_count: i64,
        duration: i64,
    ) -> Result<Option<Playlist>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let playlist = sqlx::query_as::<_, Playlist>(
            r#"
            UPDATE playlists
            SET track_count = ?, duration = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            RETURNING
                id, profil_id, library_id, name, description, color, icon, cover,
                track_count, duration, position, is_smart, created_at, updated_at
            "#,
        )
        .bind(track_count)
        .bind(duration)
        .bind(id)
        .fetch_optional(exec)
        .await?;

        Ok(playlist)
    }

    // Recalcule track_count et duration depuis les playlist_items
    pub async fn recalculate_stats<'e, E>(
        exec: E,
        id: i64,
    ) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query(
            r#"
            UPDATE playlists
            SET
                track_count = (SELECT COUNT(*) FROM playlist_items WHERE playlist_id = ?),
                duration = CAST(COALESCE((
                    SELECT SUM(lt.duration)
                    FROM playlist_items pi
                    JOIN library_tracks lt ON lt.id = pi.library_track_id
                    WHERE pi.playlist_id = ?
                ), 0) AS INTEGER),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(id)
        .bind(id)
        .bind(id)
        .execute(exec)
        .await?;
        Ok(())
    }

    pub async fn find_by_id<'e, E>(exec: E, id: i64) -> Result<Option<Playlist>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        // Même recalcul qu'à la liste : un compteur entretenu ne survit pas aux
        // suppressions en cascade. Voir `find_all_by_profil_id`.
        let playlist = sqlx::query_as::<_, Playlist>(
            r#"
            SELECT
                id, profil_id, library_id, name, description, color, icon, cover,
                CASE WHEN is_smart = 1 THEN track_count
                     ELSE (SELECT COUNT(*) FROM playlist_items pi WHERE pi.playlist_id = playlists.id)
                END AS track_count,
                duration, position, is_smart, created_at, updated_at
            FROM playlists
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(exec)
        .await?;

        Ok(playlist)
    }

    pub async fn find_all_by_profil_id<'e, E>(exec: E, profil_id: i64) -> Result<Vec<Playlist>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        // ─── Le compte se lit, il ne se croit pas ───
        //
        // `track_count` est un compteur entretenu à l'ajout et au retrait d'un
        // morceau. Mais rien ne l'entretient quand les entrées disparaissent
        // par cascade : retirer un dossier de la bibliothèque supprime ses
        // fichiers, donc ses pistes, donc les entrées de playlist qui les
        // désignaient — et le compteur reste sur son ancienne valeur.
        //
        // Résultat vu à l'écran : « 114 titres » dans la barre latérale, et une
        // playlist vide en l'ouvrant. Le compteur mentait, pas la page.
        //
        // On le recalcule donc à la lecture. La sous-requête porte sur une
        // poignée de playlists et `playlist_items` est indexée par playlist :
        // le coût est négligeable devant celui d'afficher un chiffre faux.
        //
        // Les playlists intelligentes ne comptent pas leurs `playlist_items` —
        // elles n'en ont pas. Leur valeur enregistrée sort telle quelle d'ici,
        // et c'est la commande `get_playlists` qui la remplace par un compte
        // frais : évaluer des règles demande de les interpréter, ce que ce
        // dépôt n'a pas à savoir faire.
        let playlists = sqlx::query_as::<_, Playlist>(
            r#"
            SELECT
                id, profil_id, library_id, name, description, color, icon, cover,
                CASE WHEN is_smart = 1 THEN track_count
                     ELSE (SELECT COUNT(*) FROM playlist_items pi WHERE pi.playlist_id = playlists.id)
                END AS track_count,
                duration, position, is_smart, created_at, updated_at
            FROM playlists
            WHERE profil_id = ?
            ORDER BY position ASC, id DESC
            "#,
        )
        .bind(profil_id)
        .fetch_all(exec)
        .await?;

        Ok(playlists)
    }

    /// Les règles d'une playlist intelligente, si c'en est une.
    ///
    /// Rendues brutes : le dépôt ne connaît pas leur grammaire, et n'a pas à la
    /// connaître. C'est l'appelant qui les interprète.
    pub async fn find_rules<'e, E>(exec: E, id: i64) -> Result<Option<String>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let rules: Option<Option<String>> =
            sqlx::query_scalar("SELECT rules FROM playlists WHERE id = ? AND is_smart = 1")
                .bind(id)
                .fetch_optional(exec)
                .await?;
        Ok(rules.flatten())
    }

    pub async fn delete_playlist<'e, E>(exec: E, id: i64) -> Result<u64, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let result = sqlx::query(
            r#"
            DELETE FROM playlists
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(exec)
        .await?;

        Ok(result.rows_affected())
    }
}
