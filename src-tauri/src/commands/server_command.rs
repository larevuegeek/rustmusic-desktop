//! Commandes des serveurs distants (OpenSubsonic, Jellyfin, DLNA) : connexion, bibliothèques, activation.

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::core::remote::{Backend, Kind, MusicFolder, ServerInfo};
use crate::entity::library::library::LibraryCreate;
use crate::entity::server::server::ServerEntry;
use crate::service::library::library_service::IMPORT;
use crate::service::external_server::offline_service::{self, OfflineItem};
use crate::service::external_server::sync_service;
use crate::repository::library::library_repository::LibraryRepository;
use crate::repository::server::server_repository::ServerRepository;
use crate::state::AppState;

fn db(e: sqlx::Error) -> String {
    e.to_string()
}

/// La sorte demandée par l'interface (`subsonic` par défaut).
fn kind_of(kind: Option<&str>) -> Kind {
    Kind::parse(kind.unwrap_or("subsonic"))
}

/// Le serveur enregistré, prêt à être interrogé.
pub(crate) async fn saved_server(state: &AppState, server_id: i64) -> Result<Backend, String> {
    ServerRepository::server(&state.pool, server_id).await
}

/// Synchronise des bibliothèques distantes l'une après l'autre, en fond (un import à la fois).
pub(crate) fn sync_in_background(app: AppHandle, pool: sqlx::SqlitePool, library_ids: Vec<i64>) {
    tauri::async_runtime::spawn(async move {
        let Some(token) = IMPORT.start() else {
            log::info!("☁️  Synchronisation remise : un import est en cours");
            return;
        };
        for id in library_ids {
            if token.is_cancelled() {
                break;
            }
            if let Err(e) = sync_service::sync_library(app.clone(), &pool, id, &token).await {
                log::warn!("☁️  Synchronisation de la bibliothèque {id} : {e}");
            }
        }
    });
}

async fn entry(state: &AppState, server_id: i64) -> Result<ServerEntry, String> {
    let server = ServerRepository::find_one(&state.pool, server_id).await.map_err(db)?.ok_or("serveur inconnu")?;
    let libraries = ServerRepository::libraries(&state.pool, server_id).await.map_err(db)?;
    Ok(ServerEntry { server, libraries })
}

async fn profile_of(state: &AppState, server_id: i64) -> Result<Option<i64>, String> {
    sqlx::query_scalar("SELECT profil_id FROM media_servers WHERE id = ?").bind(server_id).fetch_optional(&state.pool).await.map_err(db)
}

/// Nom d'une nouvelle bibliothèque : celui du dossier, ou suffixé du serveur s'il est déjà pris.
fn library_name(folder: &str, server: &str, taken: &[String]) -> String {
    let mut name = folder.to_string();
    let mut n = 1;
    while taken.contains(&name.to_lowercase()) {
        n += 1;
        name = if n == 2 { format!("{folder} ({server})") } else { format!("{folder} ({server}) {n}") };
    }
    name
}

/// Une bibliothèque par dossier choisi (celles déjà reliées sont gardées telles quelles).
async fn create_libraries(state: &AppState, profil_id: i64, server_id: i64, server_name: &str, folders: &[MusicFolder]) -> Result<(), String> {
    let mut taken = ServerRepository::library_names(&state.pool, profil_id).await.map_err(db)?;
    let existing = ServerRepository::libraries(&state.pool, server_id).await.map_err(db)?;
    for folder in folders {
        if existing.iter().any(|l| l.remote_folder_id.as_deref() == Some(folder.id.as_str())) {
            continue;
        }
        let name = library_name(&folder.name, server_name, &taken);
        taken.push(name.to_lowercase());
        let library = LibraryRepository::insert_library(&state.pool, &LibraryCreate { profil_id, name, description: None }).await.map_err(db)?;
        ServerRepository::attach_library(&state.pool, library.id, server_id, &folder.id).await.map_err(db)?;
    }
    let _ = LibraryRepository::ensure_default(&state.pool, profil_id).await;
    Ok(())
}

/// Nom proposé : celui que le serveur se donne (DLNA), sinon son type et son hôte (« Navidrome · nas.local »).
fn default_name(info: &ServerInfo, url: &str) -> String {
    if let Some(name) = &info.name {
        return name.clone();
    }
    let host = reqwest::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_else(|| url.to_string());
    match info.kind.as_deref() {
        Some(kind) => {
            let mut k = kind.chars();
            let kind: String = k.next().map(|c| c.to_uppercase().chain(k).collect()).unwrap_or_default();
            format!("{kind} · {host}")
        }
        None => host,
    }
}

#[derive(Serialize)]
pub struct ServerProbe {
    /// Adresse retenue (schéma compris).
    pub url: String,
    pub name: String,
    pub info: ServerInfo,
    pub folders: Vec<MusicFolder>,
}

/// Teste la connexion et liste les bibliothèques du serveur ; rien n'est enregistré.
#[tauri::command]
pub async fn server_probe(kind: Option<String>, url: String, username: String, password: Option<String>, api_key: Option<String>) -> Result<ServerProbe, String> {
    let (server, info) = Backend::connect(kind_of(kind.as_deref()), &url, &username, password.as_deref(), api_key.as_deref()).await?;
    let folders = server.music_folders().await?;
    Ok(ServerProbe { name: default_name(&info, server.base()), url: server.base().to_string(), info, folders })
}

/// Enregistre le serveur et crée une bibliothèque par dossier choisi.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn server_add(
    app: AppHandle,
    state: State<'_, AppState>,
    profil_id: i64,
    kind: Option<String>,
    url: String,
    username: String,
    password: Option<String>,
    api_key: Option<String>,
    name: Option<String>,
    folder_ids: Vec<String>,
) -> Result<ServerEntry, String> {
    let (server, info) = Backend::connect(kind_of(kind.as_deref()), &url, &username, password.as_deref(), api_key.as_deref()).await?;
    let folders = server.folders_by_id(&folder_ids).await?;
    if folders.is_empty() {
        return Err("aucune bibliothèque choisie".into());
    }
    let name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| default_name(&info, server.base()));
    let id = ServerRepository::upsert(&state.pool, profil_id, &name, username.trim(), &server, &info).await.map_err(db)?;
    create_libraries(&state, profil_id, id, &name, &folders).await?;
    let added = entry(&state, id).await?;
    sync_in_background(app, state.pool.clone(), added.libraries.iter().map(|l| l.id).collect());
    Ok(added)
}

#[tauri::command]
pub async fn get_servers(state: State<'_, AppState>, profil_id: i64) -> Result<Vec<ServerEntry>, String> {
    let mut out = Vec::new();
    for server in ServerRepository::find_by_profile(&state.pool, profil_id).await.map_err(db)? {
        let libraries = ServerRepository::libraries(&state.pool, server.id).await.map_err(db)?;
        out.push(ServerEntry { server, libraries });
    }
    Ok(out)
}

/// Serveurs DLNA du réseau (celui de RustMusic écarté).
#[tauri::command]
pub async fn server_discover(state: State<'_, AppState>) -> Result<Vec<crate::core::dlna_client::client::Found>, String> {
    let own = crate::repository::settings::settings_repository::SettingsRepository::get(&state.pool, "dlna_uuid").await.ok().flatten().map(|u| format!("uuid:{u}"));
    Ok(crate::core::dlna_client::client::discover(std::time::Duration::from_secs(3), own.as_deref()).await)
}

/// Sous-dossiers d'un serveur DLNA (`0` : la racine), pour choisir ses bibliothèques.
#[tauri::command]
pub async fn server_browse(url: String, object_id: String) -> Result<Vec<crate::core::dlna_client::catalog::Folder>, String> {
    crate::core::dlna_client::client::Server { location: url }.folders(&object_id).await
}

/// Bibliothèques proposées par le serveur, pour en ajouter d'autres.
#[tauri::command]
pub async fn server_folders(state: State<'_, AppState>, server_id: i64) -> Result<Vec<MusicFolder>, String> {
    saved_server(&state, server_id).await?.music_folders().await
}

#[tauri::command]
pub async fn server_add_libraries(app: AppHandle, state: State<'_, AppState>, server_id: i64, folder_ids: Vec<String>) -> Result<ServerEntry, String> {
    let current = entry(&state, server_id).await?;
    let before: Vec<i64> = current.libraries.iter().map(|l| l.id).collect();
    let profil_id = profile_of(&state, server_id).await?.ok_or("serveur inconnu")?;
    let folders = saved_server(&state, server_id).await?.folders_by_id(&folder_ids).await?;
    create_libraries(&state, profil_id, server_id, &current.server.name, &folders).await?;
    let after = entry(&state, server_id).await?;
    sync_in_background(app, state.pool.clone(), after.libraries.iter().map(|l| l.id).filter(|id| !before.contains(id)).collect());
    Ok(after)
}

/// Synchronise une bibliothèque distante (progression comme un import).
#[tauri::command]
pub async fn server_sync(app: AppHandle, state: State<'_, AppState>, library_id: i64) -> Result<(), String> {
    let Some(token) = IMPORT.start() else { return Err("deja_en_cours".into()) };
    sync_service::sync_library(app, &state.pool, library_id, &token).await.map(|_| ())
}

/// Serveurs activés du profil qui répondent : `(id, joignable)`, vérifiés ensemble, 6 s au plus chacun.
#[tauri::command]
pub async fn servers_check(state: State<'_, AppState>, profil_id: i64) -> Result<Vec<(i64, bool)>, String> {
    let views: Vec<_> = ServerRepository::find_by_profile(&state.pool, profil_id)
        .await
        .map_err(db)?
        .into_iter()
        .filter(|s| s.enabled)
        .collect();
    let mut checks = tokio::task::JoinSet::new();
    for id in views.iter().map(|s| s.id) {
        let server = saved_server(&state, id).await;
        checks.spawn(async move {
            match server {
                Ok(s) => match tokio::time::timeout(std::time::Duration::from_secs(6), s.ping()).await {
                    Ok(Ok(info)) => (id, true, info.device_id),
                    _ => (id, false, None),
                },
                Err(_) => (id, false, None),
            }
        });
    }
    let pings = checks.join_all().await;
    // Serveur DLNA ajouté avant qu'on garde son UDN : noté au premier contrôle réussi.
    for (id, _, udn) in &pings {
        if let Some(udn) = udn.as_deref().filter(|_| views.iter().any(|s| s.id == *id && s.device_id.is_none())) {
            ServerRepository::set_device_id(&state.pool, *id, udn).await.map_err(db)?;
        }
    }
    let mut results: Vec<(i64, bool)> = pings.into_iter().map(|(id, ok, _)| (id, ok)).collect();
    // DLNA muet : il a peut-être changé d'adresse (DHCP) ; on le cherche par son UDN.
    let lost: Vec<(i64, String)> = views
        .iter()
        .filter(|s| s.kind == "dlna" && results.iter().any(|(id, ok)| *id == s.id && !ok))
        .filter_map(|s| Some((s.id, s.device_id.clone()?)))
        .collect();
    if !lost.is_empty() {
        let found = crate::core::dlna_client::client::discover(std::time::Duration::from_secs(3), None).await;
        for (id, location) in crate::core::dlna_client::client::relocations(&lost, &found) {
            ServerRepository::set_url(&state.pool, id, &location).await.map_err(db)?;
            log::info!("📡 Serveur DLNA {id} retrouvé à {location}");
            if let Some(r) = results.iter_mut().find(|(x, _)| *x == id) {
                r.1 = true;
            }
        }
    }
    Ok(results)
}

/// Désactivé : ses bibliothèques disparaissent du sélecteur, sans rien perdre.
#[tauri::command]
pub async fn server_set_enabled(state: State<'_, AppState>, server_id: i64, enabled: bool) -> Result<(), String> {
    ServerRepository::set_enabled(&state.pool, server_id, enabled).await.map_err(db)
}

#[tauri::command]
pub async fn server_rename(state: State<'_, AppState>, server_id: i64, name: String) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("nom vide".into());
    }
    ServerRepository::rename(&state.pool, server_id, name).await.map_err(db)
}

/// Morceaux gardés hors ligne (prêts ou en attente) ; relance au passage les téléchargements en attente.
#[tauri::command]
pub async fn offline_list(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<Vec<OfflineItem>, String> {
    let items = offline_service::list(&state.pool).await.map_err(db)?;
    if items.iter().any(|i| !i.ready) {
        offline_service::kick(app, state.pool.clone());
    }
    Ok(items)
}

/// Garde ces morceaux hors ligne (les locaux sont ignorés) ; le téléchargement part en arrière-plan.
#[tauri::command]
pub async fn offline_add(app: tauri::AppHandle, state: State<'_, AppState>, paths: Vec<String>) -> Result<u64, String> {
    let added = offline_service::add(&state.pool, &paths).await.map_err(db)?;
    let _ = app.emit("offline-changed", ());
    offline_service::kick(app, state.pool.clone());
    Ok(added)
}

/// Ne garde plus ces morceaux hors ligne (fichiers effacés).
#[tauri::command]
pub async fn offline_remove(app: tauri::AppHandle, state: State<'_, AppState>, paths: Vec<String>) -> Result<(), String> {
    offline_service::remove(&state.pool, &paths).await.map_err(db)?;
    let _ = app.emit("offline-changed", ());
    Ok(())
}

/// Qualité de lecture : fichier d'origine (`format` absent) ou transcodé par le serveur.
#[tauri::command]
pub async fn server_set_transcode(state: State<'_, AppState>, server_id: i64, format: Option<String>, bitrate: Option<i64>) -> Result<(), String> {
    let transcode = match format.as_deref() {
        None => None,
        Some(f @ ("mp3" | "opus")) => Some((f, bitrate.unwrap_or(192).clamp(32, 320))),
        Some(other) => return Err(format!("format inconnu : {other}")),
    };
    ServerRepository::set_transcode(&state.pool, server_id, transcode).await.map_err(db)
}

/// Épingle ou non les playlists du serveur dans la barre latérale.
#[tauri::command]
pub async fn server_set_pin_playlists(state: State<'_, AppState>, server_id: i64, pin: bool) -> Result<(), String> {
    ServerRepository::set_pin_playlists(&state.pool, server_id, pin).await.map_err(db)
}

/// Retire le serveur et ses bibliothèques de RustMusic (rien n'est touché sur le serveur).
#[tauri::command]
pub async fn server_remove(state: State<'_, AppState>, server_id: i64) -> Result<(), String> {
    let profil_id = profile_of(&state, server_id).await?;
    ServerRepository::delete(&state.pool, server_id).await.map_err(db)?;
    let _ = tokio::task::spawn_blocking(move || crate::core::subsonic::stream::forget_server(server_id)).await;
    if let Some(p) = profil_id {
        let _ = LibraryRepository::ensure_default(&state.pool, p).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_server_after_its_kind_and_host() {
        let info = ServerInfo { kind: Some("navidrome".into()), ..Default::default() };
        assert_eq!(default_name(&info, "https://demo.navidrome.org"), "Navidrome · demo.navidrome.org");
        assert_eq!(default_name(&ServerInfo::default(), "http://nas:4533"), "nas");
    }

    #[test]
    fn library_names_never_collide() {
        let taken = vec!["music library".to_string(), "music library (nas)".to_string()];
        assert_eq!(library_name("Jazz", "NAS", &taken), "Jazz");
        assert_eq!(library_name("Music Library", "NAS", &taken), "Music Library (NAS) 3");
    }

    #[test]
    fn subsonic_unless_said_otherwise() {
        assert_eq!(kind_of(None), Kind::Subsonic);
        assert_eq!(kind_of(Some("jellyfin")), Kind::Jellyfin);
    }
}
