//! Morceaux d'un serveur gardés hors ligne : téléchargés un par un, en entier, dans un dossier que
//! la limite du cache ne vide jamais. Jouables serveur éteint.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};

use crate::core::subsonic::client::is_unreachable;
use crate::core::subsonic::stream;
use crate::entity::server::server::OfflineItem;
use crate::repository::server::offline_track_repository::OfflineTrackRepository;
use crate::service::external_server::track_service::{parse, start_download, stream_request};

/// Un téléchargement à la fois : le réseau reste libre pour l'écoute.
static RUNNING: AtomicBool = AtomicBool::new(false);

/// Le fichier hors ligne d'un morceau, s'il est prêt.
pub async fn ready_file(pool: &SqlitePool, path: &str) -> Option<PathBuf> {
    let file = OfflineTrackRepository::ready_file(pool, path).await.ok()??;
    let file = PathBuf::from(file);
    file.exists().then_some(file)
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<OfflineItem>, sqlx::Error> {
    OfflineTrackRepository::find_all(pool).await
}

/// Ajoute les morceaux distants (les autres sont ignorés) ; rend le nombre de nouveaux.
pub async fn add(pool: &SqlitePool, paths: &[String]) -> Result<u64, sqlx::Error> {
    let mut added = 0;
    for path in paths {
        let Some((server_id, _)) = parse(path) else { continue };
        added += OfflineTrackRepository::insert(pool, path, server_id).await?;
    }
    Ok(added)
}

/// Retire les morceaux du hors-ligne, fichiers compris.
pub async fn remove(pool: &SqlitePool, paths: &[String]) -> Result<(), sqlx::Error> {
    for path in paths {
        if let Some(file) = OfflineTrackRepository::delete(pool, path).await? {
            let _ = std::fs::remove_file(file);
        }
    }
    Ok(())
}

/// Lance le téléchargement des morceaux en attente, s'il ne tourne pas déjà.
pub fn kick(app: AppHandle, pool: SqlitePool) {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let done = run(&app, &pool).await;
        RUNNING.store(false, Ordering::SeqCst);
        if done > 0 {
            log::info!("☁️  {done} morceau(x) gardé(s) hors ligne");
        }
    });
}

async fn run(app: &AppHandle, pool: &SqlitePool) -> usize {
    let mut failed: HashSet<String> = HashSet::new();
    let mut done = 0;
    loop {
        let next = OfflineTrackRepository::pending_paths(pool).await.unwrap_or_default().into_iter().find(|p| !failed.contains(p));
        let Some(path) = next else { break };
        match fetch(pool, &path).await {
            Ok((file, size)) => {
                let _ = OfflineTrackRepository::set_file(pool, &path, &file.to_string_lossy(), size as i64).await;
                done += 1;
            }
            // Serveur injoignable : on reprendra plus tard (synchronisation, relance).
            Err(e) if is_unreachable(&e) => {
                log::info!("☁️  Hors-ligne en pause : {e}");
                break;
            }
            Err(e) => {
                log::warn!("☁️  Hors-ligne impossible pour {path} : {e}");
                failed.insert(path);
            }
        }
        let _ = app.emit("offline-changed", ());
    }
    done
}

/// Télécharge le morceau dans le dossier hors ligne ; déjà dans le cache, il y est simplement déplacé.
async fn fetch(pool: &SqlitePool, path: &str) -> Result<(PathBuf, u64), String> {
    let req = stream_request(pool, path).await?;
    let file = stream::offline_dir().join(req.server_id.to_string()).join(&req.name);
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let cached = stream::cache_dir().join(req.server_id.to_string()).join(&req.name);
    if !file.exists() && cached.exists() && std::fs::rename(&cached, &file).is_err() {
        std::fs::copy(&cached, &file).map_err(|e| e.to_string())?;
    }
    if !file.exists() {
        let dl = start_download(pool, &req, &file, |_| {}).await?;
        tokio::task::spawn_blocking(move || dl.wait_for(u64::MAX, Duration::from_secs(3600)))
            .await
            .map_err(|e| e.to_string())??;
    }
    let size = std::fs::metadata(&file).map(|m| m.len()).map_err(|e| e.to_string())?;
    Ok((file, size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn offline_list_keeps_remote_tracks_only() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO media_servers (id, profil_id, name, url, username) VALUES (1, 1, 'NAS', 'http://nas', 'd')").execute(&pool).await.unwrap();
        let paths = vec!["remote://1/al/s1".to_string(), r"C:\\Musique\\a.flac".to_string(), "remote://1/al/s1".to_string()];
        assert_eq!(add(&pool, &paths).await.unwrap(), 1);
        let items = list(&pool).await.unwrap();
        assert_eq!(items.len(), 1);
        assert!(!items[0].ready);
        assert!(ready_file(&pool, "remote://1/al/s1").await.is_none());

        // Prêt : son fichier est rendu tant qu'il existe.
        let file = std::env::temp_dir().join("rustmusic-offline-test.mp3");
        std::fs::write(&file, b"x").unwrap();
        sqlx::query("UPDATE offline_tracks SET file = ?, size = 1").bind(file.to_string_lossy()).execute(&pool).await.unwrap();
        assert_eq!(ready_file(&pool, "remote://1/al/s1").await, Some(file.clone()));
        remove(&pool, &["remote://1/al/s1".to_string()]).await.unwrap();
        assert!(!file.exists());
        assert!(list(&pool).await.unwrap().is_empty());
    }

    // Essai réel : un morceau de la démo gardé hors ligne, puis préparé sans passer par le cache.
    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_demo_offline() {
        use crate::core::subsonic::catalog::parse_song;
        use crate::core::subsonic::client::{Auth, Server};
        use crate::repository::server::server_repository::ServerRepository;
        use crate::service::external_server::sync_service::song_path;
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        let (server, info) = Server::connect("demo.navidrome.org", Auth::from_password("demo", "demo")).await.unwrap();
        let sid = ServerRepository::upsert(&pool, 1, "Démo", "demo", &crate::core::remote::Backend::Subsonic(server.clone()), &info).await.unwrap() + 9000;
        sqlx::query("UPDATE media_servers SET id = ?").bind(sid).execute(&pool).await.unwrap();
        let random = server.call("getRandomSongs", &[("size", "1")]).await.unwrap();
        let song = parse_song(&random["randomSongs"]["song"][0]).unwrap();
        let path = song_path(sid, &song);
        sqlx::query("INSERT INTO library (id, profil_id, name, server_id) VALUES (5, 1, 'L', ?)").bind(sid).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library_files (library_id, path, filename, extension) VALUES (5, ?, 'x', ?)").bind(&path).bind(&song.suffix).execute(&pool).await.unwrap();

        add(&pool, std::slice::from_ref(&path)).await.unwrap();
        let (file, size) = fetch(&pool, &path).await.unwrap();
        println!("{} → {} ({} Ko)", song.title, file.display(), size / 1024);
        sqlx::query("UPDATE offline_tracks SET file = ?, size = ?").bind(file.to_string_lossy()).bind(size as i64).execute(&pool).await.unwrap();
        crate::service::external_server::track_service::prepare(&pool, std::path::Path::new(&path), crate::service::external_server::track_service::Wait::Start).await.unwrap();
        assert_eq!(stream::cached(std::path::Path::new(&path)), Some(file.clone()));
        stream::forget_server(sid);
        assert!(!file.exists());
    }
}
