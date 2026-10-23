use sqlx::SqlitePool;

use crate::core::remote::{Backend, Kind, ServerInfo, Stored};
use crate::entity::server::server::{ServerAccess, ServerLibrary, ServerView};

pub struct ServerRepository;

const COLUMNS: &str = "id, kind, name, url, username, enabled, server_type, server_version, open_subsonic, extensions, \
                       transcode_format, transcode_bitrate, pin_playlists, device_id, last_sync_at, created_at";

impl ServerRepository {
    pub async fn find_by_profile(pool: &SqlitePool, profil_id: i64) -> Result<Vec<ServerView>, sqlx::Error> {
        sqlx::query_as::<_, ServerView>(sqlx::AssertSqlSafe(format!(
            "SELECT {COLUMNS} FROM media_servers WHERE profil_id = ? ORDER BY name COLLATE NOCASE, id"
        )))
        .bind(profil_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_one(pool: &SqlitePool, id: i64) -> Result<Option<ServerView>, sqlx::Error> {
        sqlx::query_as::<_, ServerView>(sqlx::AssertSqlSafe(format!("SELECT {COLUMNS} FROM media_servers WHERE id = ?")))
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn access(pool: &SqlitePool, id: i64) -> Result<Option<ServerAccess>, sqlx::Error> {
        sqlx::query_as::<_, ServerAccess>("SELECT kind, url, username, token, salt, api_key, user_id FROM media_servers WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Le serveur enregistré, prêt à être interrogé.
    pub async fn server(pool: &SqlitePool, id: i64) -> Result<Backend, String> {
        let a = Self::access(pool, id).await.map_err(|e| e.to_string())?.ok_or("serveur inconnu")?;
        let stored = Stored { token: a.token, salt: a.salt, api_key: a.api_key, user_id: a.user_id };
        Backend::restore(Kind::parse(&a.kind), a.url, a.username, stored)
    }

    pub async fn set_synced(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE media_servers SET last_sync_at = CURRENT_TIMESTAMP WHERE id = ?").bind(id).execute(pool).await.map(|_| ())
    }

    pub async fn libraries(pool: &SqlitePool, server_id: i64) -> Result<Vec<ServerLibrary>, sqlx::Error> {
        sqlx::query_as::<_, ServerLibrary>(
            "SELECT id, name, remote_folder_id, total_tracks FROM library WHERE server_id = ? ORDER BY position, id",
        )
        .bind(server_id)
        .fetch_all(pool)
        .await
    }

    /// Enregistre le serveur ; déjà là (même adresse et utilisateur), ses identifiants et son état sont mis à jour.
    pub async fn upsert(pool: &SqlitePool, profil_id: i64, name: &str, username: &str, server: &Backend, info: &ServerInfo) -> Result<i64, sqlx::Error> {
        let s = server.stored();
        sqlx::query_scalar(
            "INSERT INTO media_servers (profil_id, name, url, username, token, salt, api_key, user_id, kind, server_type, server_version, open_subsonic, extensions, device_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(profil_id, url, username) DO UPDATE SET
               token = excluded.token, salt = excluded.salt, api_key = excluded.api_key, user_id = excluded.user_id, kind = excluded.kind,
               device_id = excluded.device_id,
               server_type = excluded.server_type, server_version = excluded.server_version,
               open_subsonic = excluded.open_subsonic, extensions = excluded.extensions, enabled = 1
             RETURNING id",
        )
        .bind(profil_id)
        .bind(name)
        .bind(server.base())
        .bind(username)
        .bind(s.token)
        .bind(s.salt)
        .bind(s.api_key)
        .bind(s.user_id)
        .bind(server.kind().as_str())
        .bind(&info.kind)
        .bind(&info.version)
        .bind(info.open_subsonic)
        .bind(info.extensions.join(","))
        .fetch_one(pool)
        .await
    }

    pub async fn set_enabled(pool: &SqlitePool, id: i64, enabled: bool) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE media_servers SET enabled = ? WHERE id = ?").bind(enabled).bind(id).execute(pool).await.map(|_| ())
    }

    /// Transcodage demandé au serveur ; `None` = fichier d'origine.
    pub async fn set_transcode(pool: &SqlitePool, id: i64, transcode: Option<(&str, i64)>) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE media_servers SET transcode_format = ?, transcode_bitrate = ? WHERE id = ?")
            .bind(transcode.map(|t| t.0))
            .bind(transcode.map(|t| t.1))
            .bind(id)
            .execute(pool)
            .await
            .map(|_| ())
    }

    /// Épingle (ou non) les playlists du serveur, celles déjà là comme les prochaines.
    pub async fn set_pin_playlists(pool: &SqlitePool, id: i64, pin: bool) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query("UPDATE media_servers SET pin_playlists = ? WHERE id = ?").bind(pin).bind(id).execute(&mut *tx).await?;
        sqlx::query("UPDATE playlists SET pinned = ? WHERE server_id = ?").bind(pin).bind(id).execute(&mut *tx).await?;
        tx.commit().await
    }

    pub async fn set_device_id(pool: &SqlitePool, id: i64, device_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE media_servers SET device_id = ? WHERE id = ?").bind(device_id).bind(id).execute(pool).await.map(|_| ())
    }

    /// Nouvelle adresse (un serveur DLNA retrouvé ailleurs sur le réseau).
    pub async fn set_url(pool: &SqlitePool, id: i64, url: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE media_servers SET url = ? WHERE id = ?").bind(url).bind(id).execute(pool).await.map(|_| ())
    }

    pub async fn rename(pool: &SqlitePool, id: i64, name: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE media_servers SET name = ? WHERE id = ?").bind(name).bind(id).execute(pool).await.map(|_| ())
    }

    /// Le serveur, ses bibliothèques et leur contenu ; aussi ce qui ne part pas en cascade
    /// (cache des tags, file d'attente, récents, titres aimés, playlists de la source).
    pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        // Une playlist rattachée à la bibliothèque est gardée, détachée.
        sqlx::query("UPDATE playlists SET library_id = NULL WHERE library_id IN (SELECT id FROM library WHERE server_id = ?)")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM library WHERE server_id = ?").bind(id).execute(&mut *tx).await?;
        // Ce qui porte la source : par index, sans lire les chemins.
        for table in ["track_liked", "recent_files", "playlists"] {
            sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table} WHERE source_id = ?"))).bind(id).execute(&mut *tx).await?;
        }
        purge_prefix(&mut tx, &format!("{}{id}/", crate::core::remote::SCHEME)).await?;
        sqlx::query("DELETE FROM media_servers WHERE id = ?").bind(id).execute(&mut *tx).await?;
        tx.commit().await
    }

    /// Relie une bibliothèque fraîchement créée à son dossier musical distant.
    pub async fn attach_library(pool: &SqlitePool, library_id: i64, server_id: i64, folder_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE library SET server_id = ?, remote_folder_id = ? WHERE id = ?")
            .bind(server_id)
            .bind(folder_id)
            .bind(library_id)
            .execute(pool)
            .await
            .map(|_| ())
    }

    /// Noms de bibliothèques déjà pris par le profil (sans la casse).
    pub async fn library_names(pool: &SqlitePool, profil_id: i64) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT lower(name) FROM library WHERE profil_id = ?").bind(profil_id).fetch_all(pool).await
    }
}

/// Traces d'un serveur liées par chemin seulement : cache des tags, file, hors ligne.
async fn purge_prefix(conn: &mut sqlx::SqliteConnection, prefix: &str) -> Result<(), sqlx::Error> {
    let like = format!("{}%", prefix.replace('%', "").replace('_', "\\_"));
    for table in ["queue_tracks", "offline_tracks", "library_cache"] {
        sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table} WHERE path LIKE ? ESCAPE '\\'")))
            .bind(&like)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

/// Les mêmes traces pour des morceaux précis (disparus du serveur, bibliothèque retirée).
pub async fn purge_paths(conn: &mut sqlx::SqliteConnection, paths: &[String]) -> Result<(), sqlx::Error> {
    for path in paths {
        for table in ["queue_tracks", "track_liked", "offline_tracks", "library_cache"] {
            sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table} WHERE path = ?"))).bind(path).execute(&mut *conn).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::library::library::LibraryCreate;
    use crate::repository::library::library_repository::LibraryRepository;
    use crate::repository::playlist::playlist_repository::PlaylistRepository;
    use crate::repository::playlist::track_liked_repository::TrackLikedRepository;
    use crate::repository::recent::recent_file_repository::RecentFileRepository;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn sources_are_kept_apart() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO media_servers (id, profil_id, name, url, username) VALUES (7, 1, 'NAS', 'http://nas', '')").execute(&pool).await.unwrap();
        let p = &pool;
        for (n, path) in [(1, r"C:\Musique.flac"), (2, "remote://7/al/s1"), (3, "remote://7/al/s2")] {
            sqlx::query("INSERT INTO library_cache (id, path) VALUES (?, ?)").bind(n).bind(path).execute(&pool).await.unwrap();
            TrackLikedRepository::insert_track_liked(&pool, path, 1, None).await.unwrap();
            RecentFileRepository::insert_recent_file(&pool, 1, path.to_string()).await.unwrap();
        }
        let liked = |s| async move { TrackLikedRepository::find_all_by_profile(p, 1, s).await.unwrap().len() };
        assert_eq!((liked(None).await, liked(Some(0)).await, liked(Some(7)).await, liked(Some(8)).await), (3, 1, 2, 0));
        let recent = |s| async move { RecentFileRepository::get_recent_files(p, 1, s).await.unwrap().len() };
        assert_eq!((recent(None).await, recent(Some(0)).await, recent(Some(7)).await), (3, 1, 2));
        assert_eq!(RecentFileRepository::count_recent_files(&pool, 1, Some(7)).await.unwrap(), 2);
        sqlx::query("INSERT INTO profil (id, name) VALUES (2, 'Autre')").execute(&pool).await.unwrap();
        RecentFileRepository::insert_recent_file(&pool, 2, r"C:\Musique\a.flac".to_string()).await.unwrap();
        assert_eq!(RecentFileRepository::get_recent_files(&pool, 2, None).await.unwrap().len(), 1);
        assert_eq!(RecentFileRepository::count_recent_files(&pool, 1, None).await.unwrap(), 3);
        RecentFileRepository::clear_recent_files(&pool, 1, Some(7)).await.unwrap();
        assert_eq!(RecentFileRepository::count_recent_files(&pool, 1, None).await.unwrap(), 1);

        let make = |name: &'static str, source: i64| PlaylistRepository::insert_playlist(&pool, 1, None, name.into(), None, "#fff".into(), "x".into(), None, 0, source);
        make("Locale", 0).await.unwrap();
        let distante = make("Du NAS", 7).await.unwrap();
        assert_eq!(distante.source_id, 7);
        let names = |s| async move { PlaylistRepository::find_all_by_profile_id(p, 1, s).await.unwrap().into_iter().map(|p| p.name).collect::<Vec<_>>() };
        assert_eq!(names(Some(0)).await, vec!["Locale"]);
        assert_eq!(names(Some(7)).await, vec!["Du NAS"]);
        assert_eq!(names(None).await.len(), 2);
        // Serveur désactivé : « toutes les sources » ne montre plus rien de lui.
        sqlx::query("UPDATE media_servers SET enabled = 0 WHERE id = 7").execute(&pool).await.unwrap();
        assert_eq!((names(None).await, liked(None).await), (vec!["Locale".to_string()], 1));
        assert_eq!(PlaylistRepository::source_of(&pool, distante.id).await.unwrap(), 7);
    }

    #[tokio::test]
    async fn disabled_server_hides_its_libraries_and_removal_deletes_them() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        let local = LibraryRepository::insert_library(&pool, &LibraryCreate { profil_id: 1, name: "Locale".into(), description: None }).await.unwrap();
        let info = ServerInfo { kind: Some("navidrome".into()), ..Default::default() };
        use crate::core::subsonic::client::{Auth, Server};
        let nas = |auth| Backend::Subsonic(Server { base: "https://nas".into(), auth });
        let sid = ServerRepository::upsert(&pool, 1, "NAS", "demo", &nas(Auth::from_password("demo", "demo")), &info).await.unwrap();
        // Même adresse et utilisateur : le même serveur, identifiants rafraîchis.
        assert_eq!(ServerRepository::upsert(&pool, 1, "NAS", "demo", &nas(Auth::from_password("demo", "demo")), &info).await.unwrap(), sid);
        assert!(matches!(ServerRepository::server(&pool, sid).await.unwrap(), Backend::Subsonic(_)));
        let remote = LibraryRepository::insert_library(&pool, &LibraryCreate { profil_id: 1, name: "Jazz".into(), description: None }).await.unwrap();
        ServerRepository::attach_library(&pool, remote.id, sid, "7").await.unwrap();

        let names = |l: Vec<crate::entity::library::library::Library>| l.into_iter().map(|x| (x.name, x.server_name)).collect::<Vec<_>>();
        let all = names(LibraryRepository::find_libraries_by_profile_id(&pool, 1).await.unwrap());
        assert_eq!(all, vec![("Locale".to_string(), None), ("Jazz".to_string(), Some("NAS".to_string()))]);

        // Une playlist de la source du serveur : cachée d'un bloc quand il est désactivé (« toutes les sources »).
        let pl = PlaylistRepository::insert_playlist(&pool, 1, None, "Du NAS".into(), None, "#fff".into(), "x".into(), None, 0, sid).await.unwrap();
        let shown = || async { PlaylistRepository::find_all_by_profile_id(&pool, 1, None).await.unwrap().len() };
        assert_eq!(shown().await, 1);
        ServerRepository::set_enabled(&pool, sid, false).await.unwrap();
        assert_eq!(shown().await, 0);
        ServerRepository::set_enabled(&pool, sid, true).await.unwrap();
        assert_eq!(shown().await, 1);
        sqlx::query("DELETE FROM playlists WHERE id = ?").bind(pl.id).execute(&pool).await.unwrap();

        ServerRepository::set_enabled(&pool, sid, false).await.unwrap();
        assert_eq!(names(LibraryRepository::find_libraries_by_profile_id(&pool, 1).await.unwrap()), vec![("Locale".to_string(), None)]);
        // Les bibliothèques distantes ne sont jamais servies en DLNA.
        assert!(LibraryRepository::find_all(&pool).await.unwrap().iter().all(|l| l.id == local.id));

        // Traces liées par chemin, côté serveur et côté local (qui doit survivre).
        for (n, path) in [(1, "remote://1/al/s1"), (2, r"C:\Musique.flac")] {
            sqlx::query("INSERT INTO library_cache (id, path) VALUES (?, ?)").bind(n).bind(path).execute(&pool).await.unwrap();
            RecentFileRepository::insert_recent_file(&pool, 1, path.to_string()).await.unwrap();
            TrackLikedRepository::insert_track_liked(&pool, path, 1, Some(n)).await.unwrap();
            sqlx::query("INSERT INTO queue_tracks (queue_id, profil_id, position, path, title) VALUES (?, 1, ?, ?, 't')").bind(path).bind(n).bind(path).execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO playlists (profil_id, library_id, name) VALUES (1, ?, 'Jazz du NAS')").bind(remote.id).execute(&pool).await.unwrap();

        ServerRepository::delete(&pool, sid).await.unwrap();
        let count = |sql: &'static str| sqlx::query_scalar::<_, i64>(sql).fetch_one(&pool);
        assert_eq!(count("SELECT COUNT(*) FROM library").await.unwrap(), 1);
        for table in ["library_cache", "recent_files", "track_liked", "queue_tracks"] {
            let paths: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT path FROM {table}"))).fetch_all(&pool).await.unwrap();
            assert_eq!(paths, vec![r"C:\Musique.flac".to_string()], "{table}");
        }
        // La playlist reste, détachée.
        assert_eq!(count("SELECT COUNT(*) FROM playlists WHERE library_id IS NULL").await.unwrap(), 1);

        // Une bibliothèque locale avec une playlist rattachée se supprime aussi.
        sqlx::query("INSERT INTO playlists (profil_id, library_id, name) VALUES (1, ?, 'Locale')").bind(local.id).execute(&pool).await.unwrap();
        LibraryRepository::remove_library(&pool, local.id).await.unwrap();
        assert_eq!(count("SELECT COUNT(*) FROM library").await.unwrap(), 0);
        assert_eq!(count("SELECT COUNT(*) FROM playlists WHERE library_id IS NULL").await.unwrap(), 2);
    }
}
