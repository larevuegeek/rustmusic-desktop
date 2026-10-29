//! Suivi d'écoute (ListenBrainz) : chaque écoute passe par une file, envoyée dès que possible ;
//! hors connexion, elle attend. Rien ne part sans compte connecté.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use sqlx::SqlitePool;

use crate::core::scrobble::listenbrainz::{self, Listen, SendError};
use crate::repository::library::library_cache_repository::LibraryCacheRepository;
use crate::repository::scrobble::scrobble_repository::ScrobbleRepository;

pub const LISTENBRAINZ: &str = "listenbrainz";
/// Un seul envoi de la file à la fois (sinon des écoutes partiraient deux fois).
static FLUSHING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize)]
pub struct ScrobbleStatus {
    pub user_name: String,
    pub now_playing: bool,
    /// Écoutes en attente d'envoi.
    pub pending: i64,
}

async fn token(pool: &SqlitePool, profil_id: i64) -> Option<(String, bool)> {
    let account = ScrobbleRepository::find_account(pool, profil_id, LISTENBRAINZ).await.ok().flatten()?;
    Some((account.token, account.now_playing))
}

/// Ce qu'on sait du morceau ; `None` sans artiste ou sans titre (ListenBrainz les exige).
async fn track(pool: &SqlitePool, path: &str, listened_at: i64) -> Option<Listen> {
    let tags = LibraryCacheRepository::find_by_path(pool, path).await.ok().flatten()?;
    let clean = |s: Option<String>| s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    Some(Listen {
        listened_at,
        artist: clean(tags.artist).or(clean(tags.album_artist))?,
        title: clean(tags.title)?,
        album: clean(tags.album),
        duration_ms: tags.duration.filter(|d| *d > 0.0).map(|d| (d * 1000.0) as i64),
    })
}

pub async fn connect(pool: &SqlitePool, profil_id: i64, token: &str) -> Result<String, String> {
    let user = listenbrainz::validate(token).await?;
    ScrobbleRepository::upsert_account(pool, profil_id, LISTENBRAINZ, token.trim(), &user).await.map_err(|e| e.to_string())?;
    spawn_flush(pool.clone(), profil_id);
    Ok(user)
}

/// Compte retiré, écoutes en attente comprises.
pub async fn disconnect(pool: &SqlitePool, profil_id: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    ScrobbleRepository::delete_account(&mut *tx, profil_id, LISTENBRAINZ).await?;
    ScrobbleRepository::delete_all_pending(&mut *tx, profil_id, LISTENBRAINZ).await?;
    tx.commit().await
}

pub async fn status(pool: &SqlitePool, profil_id: i64) -> Result<Option<ScrobbleStatus>, sqlx::Error> {
    let Some(account) = ScrobbleRepository::find_account(pool, profil_id, LISTENBRAINZ).await? else {
        return Ok(None);
    };
    let pending = ScrobbleRepository::count_pending(pool, profil_id, LISTENBRAINZ).await?;
    Ok(Some(ScrobbleStatus { user_name: account.user_name, now_playing: account.now_playing, pending }))
}

pub async fn set_now_playing(pool: &SqlitePool, profil_id: i64, on: bool) -> Result<(), sqlx::Error> {
    ScrobbleRepository::set_now_playing(pool, profil_id, LISTENBRAINZ, on).await
}

/// Une écoute de plus : mise en file, puis envoi.
pub async fn listen(pool: &SqlitePool, profil_id: i64, path: &str, listened_at: i64) {
    if token(pool, profil_id).await.is_none() {
        return;
    }
    let Some(l) = track(pool, path, listened_at).await else {
        log::info!("🎧 Écoute non envoyée (artiste ou titre manquant) : {path}");
        return;
    };
    if ScrobbleRepository::enqueue(pool, profil_id, LISTENBRAINZ, &l).await.is_ok() {
        spawn_flush(pool.clone(), profil_id);
    }
}

/// « En cours d'écoute » : sans file, perdu s'il n'arrive pas (il ne vaut que maintenant).
pub async fn now_playing(pool: &SqlitePool, profil_id: i64, path: &str) {
    let Some((token, true)) = token(pool, profil_id).await else { return };
    let Some(l) = track(pool, path, 0).await else { return };
    if let Err(e) = listenbrainz::submit(&token, &listenbrainz::payload("playing_now", &[l])).await {
        log::debug!("🎧 « En cours » non envoyé : {e:?}");
    }
}

pub fn spawn_flush(pool: SqlitePool, profil_id: i64) {
    tauri::async_runtime::spawn(async move { flush(&pool, profil_id).await });
}

/// Envoie la file, par paquets ; s'arrête au premier échec réseau (réessai à la prochaine écoute ou au lancement).
pub async fn flush(pool: &SqlitePool, profil_id: i64) -> usize {
    if FLUSHING.swap(true, Ordering::SeqCst) {
        return 0;
    }
    let sent = flush_inner(pool, profil_id).await;
    FLUSHING.store(false, Ordering::SeqCst);
    sent
}

async fn flush_inner(pool: &SqlitePool, profil_id: i64) -> usize {
    let Some((token, _)) = token(pool, profil_id).await else { return 0 };
    let mut sent = 0;
    loop {
        let rows = ScrobbleRepository::find_pending(pool, profil_id, LISTENBRAINZ, listenbrainz::BATCH as i64).await.unwrap_or_default();
        if rows.is_empty() {
            return sent;
        }
        let (ids, listens): (Vec<i64>, Vec<Listen>) = rows.into_iter().unzip();
        let kind = if listens.len() == 1 { "single" } else { "import" };
        match listenbrainz::submit(&token, &listenbrainz::payload(kind, &listens)).await {
            Ok(()) => sent += listens.len(),
            Err(SendError::Rejected(e)) => log::warn!("🎧 ListenBrainz a refusé {} écoute(s), écartées : {e}", listens.len()),
            Err(SendError::Retry(e)) => {
                log::info!("🎧 ListenBrainz : envoi remis à plus tard ({e})");
                return sent;
            }
        }
        for id in ids {
            let _ = ScrobbleRepository::delete_pending(pool, id).await;
        }
    }
}

/// Au lancement : les écoutes restées en attente partent.
pub async fn flush_all(pool: &SqlitePool) {
    let profiles = ScrobbleRepository::profiles_with_pending(pool).await.unwrap_or_default();
    for p in profiles {
        flush(pool, p).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn listens_wait_in_the_queue_only_with_an_account() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library_cache (path, artist, title, album, duration) VALUES ('/m/a.flac', 'Queen', 'Bohemian', 'Opera', 354.2), ('/m/b.flac', NULL, 'Sans artiste', NULL, 10)")
            .execute(&pool).await.unwrap();
        let queued = || async { sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM scrobble_queue").fetch_one(&pool).await.unwrap() };
        // Sans compte : rien.
        listen(&pool, 1, "/m/a.flac", 1_700_000_000).await;
        assert_eq!(queued().await, 0);
        // Avec un compte (faux jeton : l'envoi échoue, l'écoute reste en file).
        sqlx::query("INSERT INTO scrobble_accounts (profil_id, service, token, user_name) VALUES (1, 'listenbrainz', 'x', 'david')").execute(&pool).await.unwrap();
        let l = track(&pool, "/m/a.flac", 1).await.unwrap();
        assert_eq!((l.artist.as_str(), l.album.as_deref(), l.duration_ms), ("Queen", Some("Opera"), Some(354_200)));
        assert!(track(&pool, "/m/b.flac", 1).await.is_none());
        let s = status(&pool, 1).await.unwrap().unwrap();
        assert_eq!((s.user_name.as_str(), s.now_playing), ("david", true));
        disconnect(&pool, 1).await.unwrap();
        assert!(status(&pool, 1).await.unwrap().is_none());
    }
}
