//! Tauri commands exposing DLNA server lifecycle to the frontend.
//!
//! Settings are persisted in the existing key/value `settings` table under
//! the keys `dlna_enabled`, `dlna_friendly_name`, `dlna_port`, `dlna_uuid`.

use std::sync::Arc;

use serde::Serialize;
use sqlx::SqlitePool;
use tauri::State;

use crate::core::dlna_server::config::DlnaConfig;
use crate::core::dlna_server::library::provider::LibraryProvider;
use crate::core::dlna_server::library::sqlite_provider::SqliteLibraryProvider;
use crate::core::dlna_server::net::detect_local_ipv4;
use crate::core::dlna_server::server::DlnaServer;
use crate::repository::settings::settings_repository::SettingsRepository;
use crate::state::AppState;

const KEY_ENABLED: &str = "dlna_enabled";
const KEY_NAME: &str = "dlna_friendly_name";
const KEY_PORT: &str = "dlna_port";
const KEY_UUID: &str = "dlna_uuid";

const DEFAULT_NAME: &str = "RustMusic Media Server";
const DEFAULT_PORT: u16 = 8200;

// ─── Public type returned to the frontend ──────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct DlnaStatus {
    pub running: bool,
    pub friendly_name: String,
    pub port: u16,
    /// Best-guess local URL the user can copy (e.g. `http://192.168.1.10:8200`)
    /// — only filled when `running` is true.
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DlnaSettings {
    pub enabled: bool,
    pub friendly_name: String,
    pub port: u16,
    pub uuid: String,
}

// ─── Helpers ───────────────────────────────────────────────────────────

/// Read all DLNA settings from the DB, generating sensible defaults for
/// missing keys (and persisting the UUID once on first call).
async fn load_settings(pool: &SqlitePool) -> Result<DlnaSettings, String> {
    let enabled = SettingsRepository::get(pool, KEY_ENABLED)
        .await
        .map_err(|e| format!("read dlna_enabled: {e}"))?
        .map(|v| v == "true")
        .unwrap_or(false);

    let friendly_name = SettingsRepository::get(pool, KEY_NAME)
        .await
        .map_err(|e| format!("read dlna_friendly_name: {e}"))?
        .unwrap_or_else(|| DEFAULT_NAME.to_string());

    let port = SettingsRepository::get(pool, KEY_PORT)
        .await
        .map_err(|e| format!("read dlna_port: {e}"))?
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);

    let uuid = match SettingsRepository::get(pool, KEY_UUID)
        .await
        .map_err(|e| format!("read dlna_uuid: {e}"))?
    {
        Some(u) => u,
        None => {
            // Generate + persist a stable UUID for this server instance
            let new_uuid = uuid::Uuid::new_v4().to_string();
            SettingsRepository::set(pool, KEY_UUID, &new_uuid)
                .await
                .map_err(|e| format!("write dlna_uuid: {e}"))?;
            new_uuid
        }
    };

    Ok(DlnaSettings {
        enabled,
        friendly_name,
        port,
        uuid,
    })
}

/// Build a DlnaConfig from persisted settings.
fn build_config(settings: &DlnaSettings) -> DlnaConfig {
    DlnaConfig {
        port: settings.port,
        friendly_name: settings.friendly_name.clone(),
        uuid: settings.uuid.clone(),
        bind_address: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
    }
}

fn build_status(settings: &DlnaSettings, running: bool) -> DlnaStatus {
    let url = if running {
        Some(format!("http://{}:{}", detect_local_ipv4(), settings.port))
    } else {
        None
    };
    DlnaStatus {
        running,
        friendly_name: settings.friendly_name.clone(),
        port: settings.port,
        url,
    }
}

// ─── Tauri commands ────────────────────────────────────────────────────

#[tauri::command]
pub async fn dlna_get_settings(state: State<'_, AppState>) -> Result<DlnaSettings, String> {
    load_settings(&state.pool).await
}

#[tauri::command]
pub async fn dlna_status(state: State<'_, AppState>) -> Result<DlnaStatus, String> {
    let settings = load_settings(&state.pool).await?;
    let running = state.dlna_server.lock().await.is_some();
    Ok(build_status(&settings, running))
}

#[tauri::command]
pub async fn dlna_start(state: State<'_, AppState>) -> Result<DlnaStatus, String> {
    let mut guard = state.dlna_server.lock().await;
    if guard.is_some() {
        // Already running → just return current status
        let settings = load_settings(&state.pool).await?;
        return Ok(build_status(&settings, true));
    }

    let settings = load_settings(&state.pool).await?;
    let config = build_config(&settings);
    let provider: Arc<dyn LibraryProvider> = Arc::new(SqliteLibraryProvider::new(
        Arc::new(state.pool.clone()),
    ));

    let mut server = DlnaServer::new(config, provider);
    server
        .start()
        .await
        .map_err(|e| format!("DLNA start failed: {e}"))?;
    *guard = Some(server);

    // Persist enabled flag so the server auto-starts on next app launch
    SettingsRepository::set(&state.pool, KEY_ENABLED, "true")
        .await
        .map_err(|e| format!("save dlna_enabled: {e}"))?;

    Ok(build_status(&settings, true))
}

#[tauri::command]
pub async fn dlna_stop(state: State<'_, AppState>) -> Result<DlnaStatus, String> {
    let mut guard = state.dlna_server.lock().await;
    if let Some(mut server) = guard.take() {
        server
            .stop()
            .await
            .map_err(|e| format!("DLNA stop failed: {e}"))?;
    }

    SettingsRepository::set(&state.pool, KEY_ENABLED, "false")
        .await
        .map_err(|e| format!("save dlna_enabled: {e}"))?;

    let settings = load_settings(&state.pool).await?;
    Ok(build_status(&settings, false))
}

/// Called once at app launch. If `dlna_enabled` is `true` in settings,
/// build and start the server in the background.
/// Errors are logged but don't block app startup.
pub async fn auto_start_if_enabled(state: &AppState) {
    let settings = match load_settings(&state.pool).await {
        Ok(s) => s,
        Err(e) => {
            log::warn!("DLNA auto-start: cannot load settings ({e}), skipping");
            return;
        }
    };
    if !settings.enabled {
        return;
    }

    log::debug!("DLNA auto-start: launching server (enabled in settings)");
    let config = build_config(&settings);
    let port = config.port;
    let provider: Arc<dyn LibraryProvider> = Arc::new(SqliteLibraryProvider::new(
        Arc::new(state.pool.clone()),
    ));
    let mut server = DlnaServer::new(config, provider);
    if let Err(e) = server.start().await {
        log_dlna_start_failure(&e, port);
        return;
    }
    *state.dlna_server.lock().await = Some(server);
    log::debug!("DLNA server running");
}

/// Log d'erreur enrichi : identifie la cause probable et propose des solutions.
/// L'erreur la plus fréquente est `EADDRINUSE` (port occupé), qui peut être due
/// à : (1) une instance précédente de l'app mal fermée, (2) un autre serveur
/// DLNA déjà actif (Plex, Jellyfin, MiniDLNA, Windows Media Player Network
/// Sharing Service), (3) un processus quelconque qui squatte le port.
fn log_dlna_start_failure(err: &crate::core::dlna_server::error::DlnaError, port: u16) {
    use crate::core::dlna_server::error::DlnaError;
    let msg = format!("{err}");

    // Détection portable de EADDRINUSE (Windows = 10048, Unix = 98).
    // On regarde aussi le texte pour les wrappers qui n'exposent pas le code.
    let is_addr_in_use = matches!(err, DlnaError::Io(e) if e.kind() == std::io::ErrorKind::AddrInUse)
        || msg.contains("10048")
        || msg.contains("Address in use")
        || msg.contains("address already in use");

    if is_addr_in_use {
        log::error!(
            "DLNA auto-start: port {port} déjà utilisé.\n\
             Causes possibles :\n\
             • Une instance précédente de RustMusic est encore en cours d'exécution.\n\
             • Un autre serveur DLNA tourne sur ta machine (Plex, Jellyfin, MiniDLNA,\n\
               Windows Media Player Network Sharing).\n\
             • Un autre processus écoute sur ce port.\n\
             Solutions :\n\
             • Vérifier les apps en cours (Gestionnaire de tâches Windows / ps -ef).\n\
             • Changer le port dans Réglages → Réseau → Port HTTP.\n\
             • Désactiver l'auto-start DLNA dans les réglages."
        );
    } else {
        log::error!("DLNA auto-start failed: {msg}");
    }
}

/// Update friendly name and/or port. If the server is currently running,
/// it is stopped and restarted to pick up the new config.
#[tauri::command]
pub async fn dlna_update_settings(
    state: State<'_, AppState>,
    friendly_name: Option<String>,
    port: Option<u16>,
) -> Result<DlnaStatus, String> {
    if let Some(name) = &friendly_name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("friendly_name cannot be empty".into());
        }
        SettingsRepository::set(&state.pool, KEY_NAME, trimmed)
            .await
            .map_err(|e| format!("save dlna_friendly_name: {e}"))?;
    }
    if let Some(p) = port {
        if p == 0 {
            return Err("port must be > 0".into());
        }
        SettingsRepository::set(&state.pool, KEY_PORT, &p.to_string())
            .await
            .map_err(|e| format!("save dlna_port: {e}"))?;
    }

    // If running, restart with new config
    let was_running = state.dlna_server.lock().await.is_some();
    if was_running {
        let mut guard = state.dlna_server.lock().await;
        if let Some(mut server) = guard.take() {
            server
                .stop()
                .await
                .map_err(|e| format!("DLNA restart (stop) failed: {e}"))?;
        }

        let settings = load_settings(&state.pool).await?;
        let config = build_config(&settings);
        let provider: Arc<dyn LibraryProvider> = Arc::new(SqliteLibraryProvider::new(
            Arc::new(state.pool.clone()),
        ));
        let mut server = DlnaServer::new(config, provider);
        server
            .start()
            .await
            .map_err(|e| format!("DLNA restart (start) failed: {e}"))?;
        *guard = Some(server);
    }

    let settings = load_settings(&state.pool).await?;
    Ok(build_status(&settings, was_running))
}
