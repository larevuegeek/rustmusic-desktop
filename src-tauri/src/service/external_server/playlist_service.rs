//! Playlists d'un serveur, importées et tenues à jour à chaque synchronisation : le serveur fait foi
//! (sa boîte d'envoi vient d'être vidée). Ce qu'on y change dans RustMusic part par `feedback_service`.

use serde_json::Value;
use sqlx::SqlitePool;

use crate::core::remote::{Backend, RemotePlaylist};
use crate::core::subsonic::catalog::parse_song;
#[cfg(test)]
use crate::core::subsonic::catalog::RemoteSong;
use crate::core::subsonic::client::{id_of, Server};
use crate::repository::playlist::playlist_repository::PlaylistRepository;
use crate::service::external_server::sync_service::song_path;

/// Bleu ciel et nuage, comme les bibliothèques d'un serveur.
const COLOR: &str = "#0ea5e9";
const ICON: &str = "material-symbols:cloud-outline-rounded";

/// Subsonic : les playlists de l'utilisateur (celles des autres, en lecture seule, sont laissées).
async fn fetch(server: &Server, username: &str) -> Result<Vec<RemotePlaylist>, String> {
    let list = server.call("getPlaylists", &[]).await?;
    let mut out = Vec::new();
    for p in list["playlists"]["playlist"].as_array().into_iter().flatten() {
        let owner = p.get("owner").and_then(Value::as_str);
        if owner.is_some_and(|o| !o.eq_ignore_ascii_case(username)) {
            continue;
        }
        let Some(id) = p.get("id").and_then(id_of) else { continue };
        let full = server.call("getPlaylist", &[("id", &id)]).await?;
        let songs = full["playlist"]["entry"].as_array().into_iter().flatten().filter_map(parse_song).collect();
        let text = |k: &str| p.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
        out.push(RemotePlaylist { name: text("name").unwrap_or_else(|| id.clone()), comment: text("comment"), id, songs });
    }
    Ok(out)
}

/// Importe les playlists du serveur et aligne les existantes ; celles disparues là-bas sont retirées.
/// Un serveur DLNA n'a pas de playlists.
pub async fn apply_server_playlists(pool: &SqlitePool, server_id: i64, server: &Backend) -> Result<usize, String> {
    let (profil_id, username): (i64, String) = sqlx::query_as("SELECT profil_id, username FROM media_servers WHERE id = ?")
        .bind(server_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let remote = match server {
        Backend::Subsonic(s) => fetch(s, &username).await?,
        Backend::Jellyfin(j) => j.playlists().await?,
        Backend::Dlna(_) => return Ok(0),
    };
    for p in &remote {
        upsert_local(pool, profil_id, server_id, p).await.map_err(|e| e.to_string())?;
    }
    let kept: Vec<&str> = remote.iter().map(|p| p.id.as_str()).collect();
    let gone: Vec<i64> = sqlx::query_as::<_, (i64, String)>("SELECT id, remote_id FROM playlists WHERE server_id = ?")
        .bind(server_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|(_, r)| !kept.contains(&r.as_str()))
        .map(|(id, _)| id)
        .collect();
    for id in gone {
        let _ = PlaylistRepository::delete_playlist(pool, id).await;
    }
    Ok(remote.len())
}

async fn upsert_local(pool: &SqlitePool, profil_id: i64, server_id: i64, p: &RemotePlaylist) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let existing: Option<i64> = sqlx::query_scalar("SELECT id FROM playlists WHERE server_id = ? AND remote_id = ?")
        .bind(server_id)
        .bind(&p.id)
        .fetch_optional(&mut *tx)
        .await?;
    let id = match existing {
        Some(id) => {
            sqlx::query("UPDATE playlists SET name = ?1, description = ?2 WHERE id = ?3 AND (name IS NOT ?1 OR description IS NOT ?2)")
                .bind(&p.name)
                .bind(&p.comment)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            id
        }
        None => {
            sqlx::query_scalar(
                "INSERT INTO playlists (profil_id, name, description, color, icon, server_id, source_id, remote_id, position, pinned)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, (SELECT COALESCE(MAX(position), -1) + 1 FROM playlists WHERE profil_id = ?1),
                         (SELECT pin_playlists FROM media_servers WHERE id = ?6))
                 RETURNING id",
            )
            .bind(profil_id)
            .bind(&p.name)
            .bind(&p.comment)
            .bind(COLOR)
            .bind(ICON)
            .bind(server_id)
            .bind(&p.id)
            .fetch_one(&mut *tx)
            .await?
        }
    };

    // Morceaux connus de RustMusic, dans l'ordre ; les autres (et les doublons) mis de côté pour l'envoi.
    let (mut tracks, mut extra): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    for song in &p.songs {
        let track: Option<String> = sqlx::query_scalar(
            "SELECT lt.id FROM library_tracks lt JOIN library_files lf ON lf.id = lt.file_id WHERE lf.path = ? LIMIT 1",
        )
        .bind(song_path(server_id, song))
        .fetch_optional(&mut *tx)
        .await?;
        match track {
            Some(t) if !tracks.contains(&t) => tracks.push(t),
            _ => extra.push(song.id.clone()),
        }
    }
    let current: Vec<String> = sqlx::query_scalar("SELECT library_track_id FROM playlist_items WHERE playlist_id = ? ORDER BY sort_index, id")
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
    if current != tracks {
        sqlx::query("DELETE FROM playlist_items WHERE playlist_id = ?").bind(id).execute(&mut *tx).await?;
        for (i, t) in tracks.iter().enumerate() {
            sqlx::query("INSERT INTO playlist_items (playlist_id, library_track_id, sort_index) VALUES (?, ?, ?)")
                .bind(id)
                .bind(t)
                .bind(i as i64)
                .execute(&mut *tx)
                .await?;
        }
    }
    let extra = (!extra.is_empty()).then(|| serde_json::to_string(&extra).unwrap_or_default());
    sqlx::query("UPDATE playlists SET remote_extra = ? WHERE id = ?").bind(extra).bind(id).execute(&mut *tx).await?;
    PlaylistRepository::recalculate_stats(&mut *tx, id).await?;
    tx.commit().await
}

/// Crée la playlist sur le serveur ; rend son identifiant là-bas.
pub async fn create_on_server(pool: &SqlitePool, server_id: i64, name: &str) -> Result<String, String> {
    match crate::repository::server::server_repository::ServerRepository::server(pool, server_id).await? {
        Backend::Subsonic(server) => {
            let created = server.call("createPlaylist", &[("name", name)]).await?;
            created["playlist"].get("id").and_then(id_of).ok_or_else(|| "le serveur n'a pas rendu la playlist créée".to_string())
        }
        Backend::Jellyfin(j) => j.create_playlist(name).await,
        Backend::Dlna(_) => Err("un serveur DLNA n'a pas de playlists".into()),
    }
}

/// Rattache la playlist locale à celle créée sur le serveur.
pub async fn attach(pool: &SqlitePool, playlist_id: i64, server_id: i64, remote_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE playlists SET server_id = ?, remote_id = ? WHERE id = ?")
        .bind(server_id)
        .bind(remote_id)
        .bind(playlist_id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Playlist d'un serveur : (serveur, identifiant côté serveur).
pub async fn remote_of(pool: &SqlitePool, playlist_id: i64) -> Option<(i64, String)> {
    sqlx::query_as::<_, (Option<i64>, Option<String>)>("SELECT server_id, remote_id FROM playlists WHERE id = ?")
        .bind(playlist_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|(s, r)| Some((s?, r?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn server_playlists_are_imported_and_follow_the_server() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO media_servers (id, profil_id, name, url, username) VALUES (1, 1, 'NAS', 'http://nas', 'd')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library (id, profil_id, name, server_id) VALUES (5, 1, 'L', 1)").execute(&pool).await.unwrap();
        for i in 0..3 {
            sqlx::query("INSERT INTO library_files (id, library_id, path, filename) VALUES (?, 5, ?, 'x')")
                .bind(format!("f{i}")).bind(format!("remote://1/al/s{i}")).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, title, title_normalized) VALUES (?, 5, ?, 't', 't')")
                .bind(format!("t{i}")).bind(format!("f{i}")).execute(&pool).await.unwrap();
        }
        let song = |id: &str| RemoteSong { id: id.into(), album_id: Some("al".into()), ..Default::default() };
        // s9 : morceau d'un dossier non ajouté ; s1 en double.
        let mut p = RemotePlaylist { id: "p1".into(), name: "Soirée".into(), comment: None, songs: vec![song("s2"), song("s9"), song("s1"), song("s1")] };
        upsert_local(&pool, 1, 1, &p).await.unwrap();
        let (local, name, extra, count): (i64, String, Option<String>, i64) =
            sqlx::query_as("SELECT id, name, remote_extra, track_count FROM playlists WHERE remote_id = 'p1'").fetch_one(&pool).await.unwrap();
        assert_eq!((name.as_str(), extra.as_deref(), count), ("Soirée", Some(r#"["s9","s1"]"#), 2));
        let order: Vec<String> = sqlx::query_scalar("SELECT library_track_id FROM playlist_items WHERE playlist_id = ? ORDER BY sort_index").bind(local).fetch_all(&pool).await.unwrap();
        assert_eq!(order, vec!["t2", "t1"]);
        // Ce qu'on renverrait au serveur : l'ordre local, puis les morceaux mis de côté.
        let sent = super::super::feedback_service::playlist_song_ids(&pool, local).await.unwrap().unwrap();
        assert_eq!(sent, vec!["s2", "s1", "s9", "s1"]);

        // Renommée et réordonnée sur le serveur : suivie ici.
        p.name = "Soirée d'été".into();
        p.songs = vec![song("s0"), song("s2")];
        upsert_local(&pool, 1, 1, &p).await.unwrap();
        let order: Vec<String> = sqlx::query_scalar("SELECT library_track_id FROM playlist_items WHERE playlist_id = ? ORDER BY sort_index").bind(local).fetch_all(&pool).await.unwrap();
        assert_eq!(order, vec!["t0", "t2"]);
        let name: String = sqlx::query_scalar("SELECT name FROM playlists WHERE id = ?").bind(local).fetch_one(&pool).await.unwrap();
        assert_eq!(name, "Soirée d'été");
        assert_eq!(remote_of(&pool, local).await, Some((1, "p1".to_string())));
    }

    // Essai réel sur la démo : playlist créée là-bas, importée, modifiée ici, renvoyée, supprimée.
    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_demo_playlist_round_trip() {
        use crate::core::subsonic::client::Auth;
        use crate::repository::server::server_repository::ServerRepository;
        use crate::service::external_server::feedback_service::{send_to, Feedback};
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        let (server, info) = Server::connect("demo.navidrome.org", Auth::from_password("demo", "demo")).await.unwrap();
        let sid = ServerRepository::upsert(&pool, 1, "Démo", "demo", &crate::core::remote::Backend::Subsonic(server.clone()), &info).await.unwrap();
        sqlx::query("INSERT INTO library (id, profil_id, name, server_id) VALUES (5, 1, 'L', ?)").bind(sid).execute(&pool).await.unwrap();

        // Deux morceaux connus de RustMusic, dans une playlist créée sur le serveur.
        let random = server.call("getRandomSongs", &[("size", "2")]).await.unwrap();
        let songs: Vec<RemoteSong> = random["randomSongs"]["song"].as_array().unwrap().iter().filter_map(parse_song).collect();
        for (i, s) in songs.iter().enumerate() {
            sqlx::query("INSERT INTO library_files (id, library_id, path, filename) VALUES (?, 5, ?, 'x')").bind(format!("f{i}")).bind(song_path(sid, s)).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, title, title_normalized) VALUES (?, 5, ?, 't', 't')").bind(format!("t{i}")).bind(format!("f{i}")).execute(&pool).await.unwrap();
        }
        let name = format!("RustMusic essai {}", chrono::Utc::now().timestamp());
        let created = server.call("createPlaylist", &[("name", &name), ("songId", &songs[0].id), ("songId", &songs[1].id)]).await.unwrap();
        let remote_id = id_of(&created["playlist"]["id"]).unwrap();

        apply_server_playlists(&pool, sid, &crate::core::remote::Backend::Subsonic(server.clone())).await.unwrap();
        let (local, count): (i64, i64) = sqlx::query_as("SELECT id, track_count FROM playlists WHERE remote_id = ?").bind(&remote_id).fetch_one(&pool).await.unwrap();
        println!("« {name} » importée : {count} titres");
        assert_eq!(count, 2);

        // Retirée ici, puis renvoyée : le serveur n'en a plus qu'un.
        sqlx::query("DELETE FROM playlist_items WHERE playlist_id = ? AND library_track_id = 't0'").bind(local).execute(&pool).await.unwrap();
        send_to(&pool, sid, &remote_id, Feedback::PlaylistSongs(local)).await;
        let back = server.call("getPlaylist", &[("id", &remote_id)]).await.unwrap();
        let left: Vec<String> = back["playlist"]["entry"].as_array().unwrap().iter().filter_map(|e| id_of(&e["id"])).collect();
        assert_eq!(left, vec![songs[1].id.clone()]);

        send_to(&pool, sid, &remote_id, Feedback::PlaylistDelete).await;
        assert!(server.call("getPlaylist", &[("id", &remote_id)]).await.is_err());
        let outbox: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_outbox").fetch_one(&pool).await.unwrap();
        assert_eq!(outbox, 0);
    }

    // Essai réel Jellyfin (démo) : créée depuis RustMusic, importée, vidée d'un morceau, renommée, supprimée.
    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_jellyfin_playlist_round_trip() {
        use crate::core::remote::Kind;
        use crate::repository::server::server_repository::ServerRepository;
        use crate::service::external_server::feedback_service::{send_to, Feedback};
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        let (server, info) = Backend::connect(Kind::Jellyfin, "demo.jellyfin.org/stable", "demo", Some(""), None).await.unwrap();
        let Backend::Jellyfin(j) = &server else { unreachable!() };
        let sid = ServerRepository::upsert(&pool, 1, "Démo", "demo", &server, &info).await.unwrap();
        sqlx::query("INSERT INTO library (id, profil_id, name, server_id) VALUES (5, 1, 'L', ?)").bind(sid).execute(&pool).await.unwrap();
        let folder = server.music_folders().await.unwrap().remove(0);
        let songs = server.all_songs(&folder.id, |_| {}).await.unwrap();
        for (i, s) in songs.iter().take(2).enumerate() {
            sqlx::query("INSERT INTO library_files (id, library_id, path, filename) VALUES (?, 5, ?, 'x')").bind(format!("f{i}")).bind(song_path(sid, s)).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, title, title_normalized) VALUES (?, 5, ?, 't', 't')").bind(format!("t{i}")).bind(format!("f{i}")).execute(&pool).await.unwrap();
        }

        let name = format!("RustMusic essai {}", chrono::Utc::now().timestamp());
        let remote_id = match create_on_server(&pool, sid, &name).await {
            Ok(id) => id,
            Err(e) => {
                println!("création refusée par la démo ({e}) : lecture seule");
                println!("{} playlist(s) lue(s)", j.playlists().await.unwrap().len());
                return;
            }
        };
        let ids: Vec<String> = songs.iter().take(2).map(|s| s.id.clone()).collect();
        j.update_playlist(&remote_id, None, Some(&ids)).await.unwrap();
        apply_server_playlists(&pool, sid, &server).await.unwrap();
        let (local, count): (i64, i64) = sqlx::query_as("SELECT id, track_count FROM playlists WHERE remote_id = ?").bind(&remote_id).fetch_one(&pool).await.unwrap();
        println!("« {name} » importée : {count} titres");
        assert_eq!(count, 2);

        sqlx::query("DELETE FROM playlist_items WHERE playlist_id = ? AND library_track_id = 't0'").bind(local).execute(&pool).await.unwrap();
        send_to(&pool, sid, &remote_id, Feedback::PlaylistSongs(local)).await;
        sqlx::query("UPDATE playlists SET name = 'Renommée par RustMusic' WHERE id = ?").bind(local).execute(&pool).await.unwrap();
        send_to(&pool, sid, &remote_id, Feedback::PlaylistInfo(local)).await;
        let back = j.playlists().await.unwrap().into_iter().find(|p| p.id == remote_id).unwrap();
        assert_eq!((back.name.as_str(), back.songs.len()), ("Renommée par RustMusic", 1));

        send_to(&pool, sid, &remote_id, Feedback::PlaylistDelete).await;
        assert!(j.playlists().await.unwrap().iter().all(|p| p.id != remote_id));
        let outbox: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_outbox").fetch_one(&pool).await.unwrap();
        assert_eq!(outbox, 0);
    }

    // Essai réel : playlist créée sur la démo depuis RustMusic, puis supprimée.
    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_demo_create_on_server() {
        use crate::core::subsonic::client::Auth;
        use crate::repository::server::server_repository::ServerRepository;
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        let (server, info) = Server::connect("demo.navidrome.org", Auth::from_password("demo", "demo")).await.unwrap();
        let sid = ServerRepository::upsert(&pool, 1, "Démo", "demo", &crate::core::remote::Backend::Subsonic(server.clone()), &info).await.unwrap();
        let id = create_on_server(&pool, sid, "RustMusic création").await.unwrap();
        let got = server.call("getPlaylist", &[("id", &id)]).await.unwrap();
        assert_eq!(got["playlist"]["name"].as_str(), Some("RustMusic création"));
        server.call("deletePlaylist", &[("id", &id)]).await.unwrap();
    }
}
