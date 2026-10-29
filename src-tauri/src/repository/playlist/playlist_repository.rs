use sqlx::Error;

use crate::core::remote::RemotePlaylist;
use crate::entity::playlist::playlist::{Playlist, PlaylistExportRow};

pub struct PlaylistRepository;

/// Nombre de morceaux, relu (une intelligente garde le sien, recalculé par `get_playlists`).
const COUNT_ITEMS: &str = "CASE WHEN is_smart = 1 THEN track_count \
     ELSE (SELECT COUNT(*) FROM playlist_items pi WHERE pi.playlist_id = playlists.id) END";

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
        source_id: i64,
    ) -> Result<Playlist, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let playlist = sqlx::query_as::<_, Playlist>(
            r#"
            INSERT INTO playlists (
                profil_id, library_id, name, description, color, icon, cover, position, source_id
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING
                id, profil_id, library_id, name, description, color, icon, cover,
                track_count, duration, position, is_smart, is_mix, pinned, server_id, source_id, created_at, updated_at
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
        .bind(source_id)
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
                track_count, duration, position, is_smart, is_mix, pinned, server_id, created_at, updated_at
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
        let playlist = sqlx::query_as::<_, Playlist>(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT
                id, profil_id, library_id, name, description, color, icon, cover,
                {COUNT_ITEMS} AS track_count,
                duration, position, is_smart, is_mix, pinned, server_id, source_id, created_at, updated_at
            FROM playlists
            WHERE id = ?
            "#
        )))
        .bind(id)
        .fetch_optional(exec)
        .await?;

        Ok(playlist)
    }

    /// `source` : celle à garder ; `None` : toutes.
    pub async fn find_all_by_profile_id<'e, E>(exec: E, profil_id: i64, source: Option<i64>) -> Result<Vec<Playlist>, Error>
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
        // Sans source : toutes les actives (celles d'un serveur désactivé restent cachées).
        let filter = if source.is_some() { "AND source_id = ?".to_string() } else { format!("AND {}", crate::core::remote::enabled_source("source_id")) };
        let mut query = sqlx::query_as::<_, Playlist>(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT
                id, profil_id, library_id, name, description, color, icon, cover,
                {COUNT_ITEMS} AS track_count,
                duration, position, is_smart, is_mix, pinned, server_id, source_id, created_at, updated_at
            FROM playlists
            WHERE profil_id = ? {filter}
            ORDER BY position ASC, id DESC
            "#
        )))
        .bind(profil_id);
        if let Some(s) = source {
            query = query.bind(s);
        }
        query.fetch_all(exec).await
    }

    /// Source d'une playlist (0 si elle n'existe pas : celle de l'ordinateur).
    pub async fn source_of<'e, E>(exec: E, id: i64) -> Result<i64, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        Ok(sqlx::query_scalar("SELECT source_id FROM playlists WHERE id = ?").bind(id).fetch_optional(exec).await?.unwrap_or(0))
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
    /// Playlists venues de ce serveur : (id, identifiant côté serveur).
    pub async fn find_remote_ids_by_server<'e, E>(exec: E, server_id: i64) -> Result<Vec<(i64, String)>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_as("SELECT id, remote_id FROM playlists WHERE server_id = ?").bind(server_id).fetch_all(exec).await
    }

    pub async fn find_id_by_remote<'e, E>(exec: E, server_id: i64, remote_id: &str) -> Result<Option<i64>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_scalar("SELECT id FROM playlists WHERE server_id = ? AND remote_id = ?")
            .bind(server_id)
            .bind(remote_id)
            .fetch_optional(exec)
            .await
    }

    /// Nom et description, réécrits seulement s'ils changent (la date de mise à jour ne bouge pas sinon).
    pub async fn update_info_if_changed<'e, E>(exec: E, id: i64, name: &str, description: Option<&str>) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query("UPDATE playlists SET name = ?1, description = ?2 WHERE id = ?3 AND (name IS NOT ?1 OR description IS NOT ?2)")
            .bind(name)
            .bind(description)
            .bind(id)
            .execute(exec)
            .await
            .map(|_| ())
    }

    /// Playlist d'un serveur, ajoutée en fin de liste ; épinglée selon le réglage du serveur. Rend son id.
    pub async fn insert_from_server<'e, E>(exec: E, profil_id: i64, server_id: i64, playlist: &RemotePlaylist, color: &str, icon: &str) -> Result<i64, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_scalar(
            "INSERT INTO playlists (profil_id, name, description, color, icon, server_id, source_id, remote_id, position, pinned)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, (SELECT COALESCE(MAX(position), -1) + 1 FROM playlists WHERE profil_id = ?1),
                     (SELECT pin_playlists FROM media_servers WHERE id = ?6))
             RETURNING id",
        )
        .bind(profil_id)
        .bind(&playlist.name)
        .bind(&playlist.comment)
        .bind(color)
        .bind(icon)
        .bind(server_id)
        .bind(&playlist.id)
        .fetch_one(exec)
        .await
    }

    /// Morceaux du serveur absents de RustMusic (JSON), gardés pour le renvoi.
    pub async fn set_remote_extra<'e, E>(exec: E, id: i64, extra: Option<&str>) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query("UPDATE playlists SET remote_extra = ? WHERE id = ?").bind(extra).bind(id).execute(exec).await.map(|_| ())
    }

    /// Rattache la playlist à celle créée sur le serveur.
    pub async fn attach_remote<'e, E>(exec: E, id: i64, server_id: i64, remote_id: &str) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query("UPDATE playlists SET server_id = ?, remote_id = ? WHERE id = ?")
            .bind(server_id)
            .bind(remote_id)
            .bind(id)
            .execute(exec)
            .await
            .map(|_| ())
    }

    /// (serveur, identifiant côté serveur), vides pour une playlist locale.
    pub async fn remote_of<'e, E>(exec: E, id: i64) -> Result<Option<(Option<i64>, Option<String>)>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_as("SELECT server_id, remote_id FROM playlists WHERE id = ?").bind(id).fetch_optional(exec).await
    }

    /// (serveur, morceaux du serveur absents de RustMusic en JSON).
    pub async fn server_and_extra<'e, E>(exec: E, id: i64) -> Result<Option<(Option<i64>, Option<String>)>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_as("SELECT server_id, remote_extra FROM playlists WHERE id = ?").bind(id).fetch_optional(exec).await
    }

    /// Playlists du profil pour un export, dans l'ordre de la barre latérale.
    pub async fn find_for_export<'e, E>(exec: E, profil_id: i64) -> Result<Vec<PlaylistExportRow>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_as(
            "SELECT id, name, description, color, icon, position,
                    CASE WHEN is_smart = 1 THEN rules END AS rules, is_mix, pinned
               FROM playlists
              WHERE profil_id = ?
           ORDER BY position, id",
        )
        .bind(profil_id)
        .fetch_all(exec)
        .await
    }

    pub async fn find_id_by_name<'e, E>(exec: E, profil_id: i64, name: &str) -> Result<Option<i64>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_scalar("SELECT id FROM playlists WHERE profil_id = ? AND name = ?")
            .bind(profil_id)
            .bind(name)
            .fetch_optional(exec)
            .await
    }

    /// Règles (playlist auto si présentes), mix et épingle.
    pub async fn set_rules_and_flags<'e, E>(exec: E, id: i64, rules: Option<&str>, is_mix: bool, pinned: bool) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query("UPDATE playlists SET is_smart = ?, rules = ?, is_mix = ?, pinned = ? WHERE id = ?")
            .bind(rules.is_some())
            .bind(rules)
            .bind(is_mix)
            .bind(pinned)
            .bind(id)
            .execute(exec)
            .await
            .map(|_| ())
    }

    /// Noms des playlists du profil, en minuscules.
    pub async fn names_lowercase<'e, E>(exec: E, profil_id: i64) -> Result<Vec<String>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_scalar("SELECT lower(name) FROM playlists WHERE profil_id = ?").bind(profil_id).fetch_all(exec).await
    }

    /// Place dans la barre latérale (seulement pour une playlist de ce profil).
    pub async fn set_position<'e, E>(exec: E, id: i64, profil_id: i64, position: i64) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query("UPDATE playlists SET position = ? WHERE id = ? AND profil_id = ?")
            .bind(position)
            .bind(id)
            .bind(profil_id)
            .execute(exec)
            .await
            .map(|_| ())
    }

    /// Épingle ou retire de la barre latérale (jamais un mix). Rend 0 si la playlist n'existe pas.
    pub async fn set_pinned<'e, E>(exec: E, id: i64, pinned: bool) -> Result<u64, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query("UPDATE playlists SET pinned = ? WHERE id = ? AND is_mix = 0")
            .bind(pinned)
            .bind(id)
            .execute(exec)
            .await
            .map(|r| r.rows_affected())
    }

    /// Règles et nature (mix) d'une playlist auto ; `None` pour une playlist classique.
    pub async fn find_rules_and_mix<'e, E>(exec: E, id: i64) -> Result<Option<(Option<String>, bool)>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_as("SELECT rules, is_mix FROM playlists WHERE id = ? AND is_smart = 1").bind(id).fetch_optional(exec).await
    }

    /// Nouvelle playlist auto (un mix ne s'épingle pas : il vit sur la page Mix). Rend son id.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_smart<'e, E>(
        exec: E,
        profil_id: i64,
        name: &str,
        color: &str,
        icon: &str,
        rules_json: &str,
        is_mix: bool,
        source_id: i64,
    ) -> Result<i64, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_scalar(
            "INSERT INTO playlists (profil_id, name, color, icon, is_smart, rules, is_mix, pinned, source_id)
             VALUES (?, ?, ?, ?, 1, ?, ?, ?, ?)
             RETURNING id",
        )
        .bind(profil_id)
        .bind(name)
        .bind(color)
        .bind(icon)
        .bind(rules_json)
        .bind(is_mix)
        .bind(!is_mix)
        .bind(source_id)
        .fetch_one(exec)
        .await
    }

    /// Règles d'une playlist auto ; nom, couleur et icône seulement s'ils sont donnés.
    pub async fn update_smart<'e, E>(
        exec: E,
        id: i64,
        name: Option<&str>,
        color: Option<&str>,
        icon: Option<&str>,
        rules_json: &str,
    ) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query(
            "UPDATE playlists SET
                 name  = COALESCE(?, name),
                 color = COALESCE(?, color),
                 icon  = COALESCE(?, icon),
                 rules = ?,
                 is_smart = 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?",
        )
        .bind(name)
        .bind(color)
        .bind(icon)
        .bind(rules_json)
        .bind(id)
        .execute(exec)
        .await
        .map(|_| ())
    }

    pub async fn set_track_count<'e, E>(exec: E, id: i64, count: i64) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query("UPDATE playlists SET track_count = ? WHERE id = ?").bind(count).bind(id).execute(exec).await.map(|_| ())
    }

}
