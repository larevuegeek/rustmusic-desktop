//! Suivi d'écoute (ListenBrainz, Last.fm) : chaque écoute passe par la file de chaque service relié,
//! envoyée dès que possible ; hors connexion, elle attend. Rien ne part sans compte connecté.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use sqlx::SqlitePool;

use crate::core::scrobble::lastfm::{self, ApiError, Credentials};
use crate::core::scrobble::listenbrainz::{self, Listen, SendError};
use crate::entity::scrobble::scrobble::ScrobbleAccount;
use crate::repository::library::library_cache_repository::LibraryCacheRepository;
use crate::repository::scrobble::scrobble_repository::ScrobbleRepository;

pub const LISTENBRAINZ: &str = "listenbrainz";
pub const LASTFM: &str = "lastfm";
const SERVICES: [&str; 2] = [LISTENBRAINZ, LASTFM];

/// Un seul envoi de file à la fois par service (sinon des écoutes partiraient deux fois).
static FLUSHING_LISTENBRAINZ: AtomicBool = AtomicBool::new(false);
static FLUSHING_LASTFM: AtomicBool = AtomicBool::new(false);

fn flushing(service: &str) -> &'static AtomicBool {
    if service == LASTFM { &FLUSHING_LASTFM } else { &FLUSHING_LISTENBRAINZ }
}

#[derive(Debug, Serialize)]
pub struct ScrobbleStatus {
    pub user_name: String,
    pub now_playing: bool,
    /// Écoutes en attente d'envoi.
    pub pending: i64,
    /// Last.fm : relié avec la clé d'API de l'utilisateur.
    pub own_key: bool,
}

/// Ce qu'on sait du morceau ; `None` sans artiste ou sans titre (les deux services les exigent).
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

/// Last.fm : la clé du compte si l'utilisateur a mis la sienne, sinon celle de RustMusic.
fn lastfm_credentials(account: &ScrobbleAccount) -> Option<Credentials> {
    match (&account.api_key, &account.api_secret) {
        (Some(key), Some(secret)) => Some(Credentials { key: key.clone(), secret: secret.clone() }),
        _ => lastfm::builtin(),
    }
}

// ─── ListenBrainz : jeton personnel ───

pub async fn connect(pool: &SqlitePool, profil_id: i64, token: &str) -> Result<String, String> {
    let user = listenbrainz::validate(token).await?;
    ScrobbleRepository::upsert_account(pool, profil_id, LISTENBRAINZ, token.trim(), &user, None).await.map_err(|e| e.to_string())?;
    spawn_flush(pool.clone(), profil_id);
    Ok(user)
}

// ─── Last.fm : autorisation dans le navigateur ───

/// Connexions Last.fm en cours, par profil : le jeton à faire autoriser et la clé utilisée.
static PENDING_AUTH: Mutex<Option<HashMap<i64, (String, Credentials, bool)>>> = Mutex::new(None);

fn pending_auth() -> std::sync::MutexGuard<'static, Option<HashMap<i64, (String, Credentials, bool)>>> {
    PENDING_AUTH.lock().unwrap_or_else(|e| e.into_inner())
}

/// Codes traduits par l'interface.
fn lastfm_error(e: ApiError) -> String {
    match e {
        ApiError::Network(m) => {
            log::warn!("🎧 Last.fm injoignable : {m}");
            "unreachable".into()
        }
        // Clé invalide, suspendue, signature fausse (secret erroné).
        ApiError::Service(10 | 13 | 26, _) => "bad_key".into(),
        ApiError::Service(code, m) => format!("Last.fm {code} : {m}"),
    }
}

/// Premier temps : rend la page d'autorisation à ouvrir. Sans clé personnelle, celle de RustMusic.
pub async fn lastfm_begin(profil_id: i64, own: Option<(String, String)>) -> Result<String, String> {
    let (creds, is_own) = match own {
        Some((key, secret)) if !key.trim().is_empty() && !secret.trim().is_empty() => {
            (Credentials { key: key.trim().into(), secret: secret.trim().into() }, true)
        }
        _ => (lastfm::builtin().ok_or("no_key")?, false),
    };
    let token = lastfm::get_token(&creds).await.map_err(lastfm_error)?;
    let url = lastfm::auth_url(&creds, &token);
    pending_auth().get_or_insert_with(HashMap::new).insert(profil_id, (token, creds, is_own));
    Ok(url)
}

/// Second temps, appelé en boucle par l'interface : `None` tant que l'utilisateur n'a pas autorisé.
pub async fn lastfm_finish(pool: &SqlitePool, profil_id: i64) -> Result<Option<String>, String> {
    let Some((token, creds, own)) = pending_auth().as_ref().and_then(|m| m.get(&profil_id).cloned()) else {
        return Err("expired".into());
    };
    match lastfm::get_session(&creds, &token).await {
        Ok((user, session)) => {
            let api = own.then_some((creds.key.as_str(), creds.secret.as_str()));
            ScrobbleRepository::upsert_account(pool, profil_id, LASTFM, &session, &user, api).await.map_err(|e| e.to_string())?;
            lastfm_cancel(profil_id);
            spawn_flush(pool.clone(), profil_id);
            Ok(Some(user))
        }
        // Pas encore autorisé, ou réseau capricieux : on redemandera.
        Err(ApiError::Service(14, _)) | Err(ApiError::Network(_)) => Ok(None),
        // Jeton expiré (une heure) ou refusé.
        Err(ApiError::Service(4 | 15, _)) => {
            lastfm_cancel(profil_id);
            Err("expired".into())
        }
        Err(e) => {
            lastfm_cancel(profil_id);
            Err(lastfm_error(e))
        }
    }
}

pub fn lastfm_cancel(profil_id: i64) {
    if let Some(m) = pending_auth().as_mut() {
        m.remove(&profil_id);
    }
}

// ─── Communs aux deux services ───

/// Compte retiré, écoutes en attente comprises.
pub async fn disconnect(pool: &SqlitePool, profil_id: i64, service: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    ScrobbleRepository::delete_account(&mut *tx, profil_id, service).await?;
    ScrobbleRepository::delete_all_pending(&mut *tx, profil_id, service).await?;
    tx.commit().await
}

pub async fn status(pool: &SqlitePool, profil_id: i64, service: &str) -> Result<Option<ScrobbleStatus>, sqlx::Error> {
    let Some(account) = ScrobbleRepository::find_account(pool, profil_id, service).await? else {
        return Ok(None);
    };
    let pending = ScrobbleRepository::count_pending(pool, profil_id, service).await?;
    Ok(Some(ScrobbleStatus { user_name: account.user_name, now_playing: account.now_playing, pending, own_key: account.api_key.is_some() }))
}

pub async fn set_now_playing(pool: &SqlitePool, profil_id: i64, service: &str, on: bool) -> Result<(), sqlx::Error> {
    ScrobbleRepository::set_now_playing(pool, profil_id, service, on).await
}

/// Une écoute de plus : mise en file pour chaque service relié, puis envoi.
pub async fn listen(pool: &SqlitePool, profil_id: i64, path: &str, listened_at: i64) {
    let accounts = ScrobbleRepository::find_accounts(pool, profil_id).await.unwrap_or_default();
    if accounts.is_empty() {
        return;
    }
    let Some(l) = track(pool, path, listened_at).await else {
        log::info!("🎧 Écoute non envoyée (artiste ou titre manquant) : {path}");
        return;
    };
    for a in &accounts {
        let _ = ScrobbleRepository::enqueue(pool, profil_id, &a.service, &l).await;
    }
    spawn_flush(pool.clone(), profil_id);
}

/// « En cours d'écoute » : sans file, perdu s'il n'arrive pas (il ne vaut que maintenant).
pub async fn now_playing(pool: &SqlitePool, profil_id: i64, path: &str) {
    let accounts: Vec<ScrobbleAccount> = ScrobbleRepository::find_accounts(pool, profil_id).await.unwrap_or_default().into_iter().filter(|a| a.now_playing).collect();
    if accounts.is_empty() {
        return;
    }
    let Some(l) = track(pool, path, 0).await else { return };
    for a in accounts {
        let sent = match a.service.as_str() {
            LISTENBRAINZ => listenbrainz::submit(&a.token, &listenbrainz::payload("playing_now", std::slice::from_ref(&l))).await,
            LASTFM => match lastfm_credentials(&a) {
                Some(c) => lastfm::now_playing(&c, &a.token, &l).await,
                None => Err(SendError::Retry("pas de clé d'API".into())),
            },
            _ => Ok(()),
        };
        if let Err(e) = sent {
            log::debug!("🎧 « En cours » non envoyé à {} : {e:?}", a.service);
        }
    }
}

pub fn spawn_flush(pool: SqlitePool, profil_id: i64) {
    tauri::async_runtime::spawn(async move { flush(&pool, profil_id).await });
}

/// Envoie les files du profil ; chacune s'arrête à son premier échec réseau (réessai plus tard).
pub async fn flush(pool: &SqlitePool, profil_id: i64) -> usize {
    let mut sent = 0;
    for service in SERVICES {
        let guard = flushing(service);
        if guard.swap(true, Ordering::SeqCst) {
            continue;
        }
        sent += flush_service(pool, profil_id, service).await;
        guard.store(false, Ordering::SeqCst);
    }
    sent
}

async fn flush_service(pool: &SqlitePool, profil_id: i64, service: &str) -> usize {
    let Some(account) = ScrobbleRepository::find_account(pool, profil_id, service).await.ok().flatten() else { return 0 };
    let creds = if service == LASTFM {
        match lastfm_credentials(&account) {
            Some(c) => Some(c),
            None => {
                log::warn!("🎧 Last.fm : aucune clé d'API, les écoutes attendent");
                return 0;
            }
        }
    } else {
        None
    };
    let batch = if service == LASTFM { lastfm::BATCH } else { listenbrainz::BATCH };
    let mut sent = 0;
    loop {
        let rows = ScrobbleRepository::find_pending(pool, profil_id, service, batch as i64).await.unwrap_or_default();
        if rows.is_empty() {
            return sent;
        }
        let (ids, listens): (Vec<i64>, Vec<Listen>) = rows.into_iter().unzip();
        let result = match &creds {
            Some(c) => lastfm::scrobble(c, &account.token, &listens).await,
            None => {
                let kind = if listens.len() == 1 { "single" } else { "import" };
                listenbrainz::submit(&account.token, &listenbrainz::payload(kind, &listens)).await
            }
        };
        match result {
            Ok(()) => sent += listens.len(),
            Err(SendError::Rejected(e)) => log::warn!("🎧 {service} a refusé {} écoute(s), écartées : {e}", listens.len()),
            Err(SendError::Retry(e)) => {
                log::info!("🎧 {service} : envoi remis à plus tard ({e})");
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

    async fn base() -> SqlitePool {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library_cache (path, artist, title, album, duration) VALUES ('/m/a.flac', 'Queen', 'Bohemian', 'Opera', 354.2), ('/m/b.flac', NULL, 'Sans artiste', NULL, 10)")
            .execute(&pool).await.unwrap();
        pool
    }

    async fn queued(pool: &SqlitePool, service: &str) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM scrobble_queue WHERE service = ?").bind(service).fetch_one(pool).await.unwrap()
    }

    #[tokio::test]
    async fn listens_wait_in_the_queue_only_with_an_account() {
        let pool = base().await;
        // Sans compte : rien.
        listen(&pool, 1, "/m/a.flac", 1_700_000_000).await;
        assert_eq!(queued(&pool, LISTENBRAINZ).await, 0);
        sqlx::query("INSERT INTO scrobble_accounts (profil_id, service, token, user_name) VALUES (1, 'listenbrainz', 'x', 'david')").execute(&pool).await.unwrap();
        let l = track(&pool, "/m/a.flac", 1).await.unwrap();
        assert_eq!((l.artist.as_str(), l.album.as_deref(), l.duration_ms), ("Queen", Some("Opera"), Some(354_200)));
        assert!(track(&pool, "/m/b.flac", 1).await.is_none());
        let s = status(&pool, 1, LISTENBRAINZ).await.unwrap().unwrap();
        assert_eq!((s.user_name.as_str(), s.now_playing, s.own_key), ("david", true, false));
        disconnect(&pool, 1, LISTENBRAINZ).await.unwrap();
        assert!(status(&pool, 1, LISTENBRAINZ).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn each_linked_service_gets_its_own_copy_of_a_listen() {
        let pool = base().await;
        // Les deux reliés ; Last.fm avec la clé de l'utilisateur.
        ScrobbleRepository::upsert_account(&pool, 1, LISTENBRAINZ, "jeton", "david", None).await.unwrap();
        ScrobbleRepository::upsert_account(&pool, 1, LASTFM, "session", "david_fm", Some(("cle", "secret"))).await.unwrap();
        // Pas de réseau dans les tests : on remplit la file sans la vider.
        let l = track(&pool, "/m/a.flac", 1_700_000_000).await.unwrap();
        for a in ScrobbleRepository::find_accounts(&pool, 1).await.unwrap() {
            ScrobbleRepository::enqueue(&pool, 1, &a.service, &l).await.unwrap();
        }
        assert_eq!((queued(&pool, LISTENBRAINZ).await, queued(&pool, LASTFM).await), (1, 1));

        let fm = status(&pool, 1, LASTFM).await.unwrap().unwrap();
        assert_eq!((fm.user_name.as_str(), fm.own_key, fm.pending), ("david_fm", true, 1));
        let account = ScrobbleRepository::find_account(&pool, 1, LASTFM).await.unwrap().unwrap();
        assert_eq!(lastfm_credentials(&account), Some(Credentials { key: "cle".into(), secret: "secret".into() }), "la clé de l'utilisateur passe avant celle de RustMusic");

        // Se déconnecter de Last.fm ne touche pas à ListenBrainz.
        disconnect(&pool, 1, LASTFM).await.unwrap();
        assert_eq!((queued(&pool, LISTENBRAINZ).await, queued(&pool, LASTFM).await), (1, 0));
        assert!(status(&pool, 1, LISTENBRAINZ).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn finishing_without_a_pending_authorisation_says_expired() {
        let pool = base().await;
        assert_eq!(lastfm_finish(&pool, 1).await, Err("expired".to_string()));
    }

    #[tokio::test]
    async fn without_any_key_the_connection_cannot_start() {
        // Les tests sont compilés sans clé intégrée.
        if lastfm::builtin().is_none() {
            assert_eq!(lastfm_begin(1, None).await, Err("no_key".to_string()));
            assert_eq!(lastfm_begin(1, Some((" ".into(), "".into()))).await, Err("no_key".to_string()), "une clé vide ne compte pas");
        }
    }
}
