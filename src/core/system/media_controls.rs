//! Intégration System Media Transport Controls (SMTC) via `souvlaki`.
//!
//! Expose le morceau en cours à l'OS pour qu'il apparaisse :
//! - Windows : barre des tâches volume flyout + écran de verrouillage + touches média clavier
//! - macOS : Now Playing widget + Control Center + touches média
//! - Linux : MPRIS (KDE Plasma "Now Playing", GNOME, Hyprland widgets, etc.)
//!
//! L'utilisateur peut désactiver tout ça via Réglages → SMTC pour les rares cas
//! où ça pose souci (Linux sans D-Bus, Windows N sans Media Pack, etc.).
//!
//! Le module est lazy : tant que `init()` n'a pas été appelé, aucun objet
//! souvlaki n'est créé → coût zéro pour les utilisateurs qui désactivent.
//!
//! Petit serveur HTTP local pour servir les covers : Windows SMTC en contexte
//! unpackaged (dev + Tauri release non-MSIX-Store) refuse les `file://` URIs
//! (HRESULT 0x800700A1, faute de capability `internetClient`). On bind un
//! `axum` sur `127.0.0.1:0` (port random) qui sert le dossier `covers/`, et
//! on passe à souvlaki une URL `http://127.0.0.1:port/cover/...`. SMTC accepte
//! les http(s) sans capability spéciale. Le port se libère au shutdown.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use axum::Router;
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};
use crate::app::AppHandle;
use tokio::sync::oneshot;
use tower_http::services::ServeDir;

/// Slot global pour `MediaControls`. `OnceLock<Mutex<Option<_>>>` permet :
/// - une seule initialisation du Mutex (jamais détruit)
/// - mais le contenu (Option) peut osciller entre Some/None à chaque
///   enable / disable depuis les réglages
fn slot() -> &'static Mutex<Option<MediaControls>> {
    static CONTROLS: OnceLock<Mutex<Option<MediaControls>>> = OnceLock::new();
    CONTROLS.get_or_init(|| Mutex::new(None))
}

/// État du serveur HTTP local pour les covers. Set au boot par `init`,
/// take au shutdown (envoie le signal au `with_graceful_shutdown` d'axum).
struct CoverHttp {
    /// `http://127.0.0.1:PORT/cover` — base URL pour construire les URLs cover.
    base_url: String,
    /// Dossier `covers/` (AppData) — pour calculer le chemin relatif depuis
    /// un chemin absolu et le mapper sur l'URL.
    covers_dir: PathBuf,
    /// Signal qui clôt le `axum::serve.with_graceful_shutdown`.
    shutdown: oneshot::Sender<()>,
}

fn http_slot() -> &'static Mutex<Option<CoverHttp>> {
    static HTTP: OnceLock<Mutex<Option<CoverHttp>>> = OnceLock::new();
    HTTP.get_or_init(|| Mutex::new(None))
}

/// Initialise SMTC. Idempotent : ne refait rien si déjà actif.
/// Sur Windows le HWND de la fenêtre principale est requis (souvlaki utilise
/// l'API WinRT `SystemMediaTransportControls` qui s'attache à une fenêtre).
pub fn init(app: &AppHandle) -> Result<(), String> {
    let mut guard = slot().lock().map_err(|e| format!("smtc lock: {}", e))?;
    if guard.is_some() {
        return Ok(()); // déjà initialisé
    }

    // ─── HWND fenêtre principale (Windows) ───
    #[cfg(target_os = "windows")]
    let hwnd: Option<*mut std::os::raw::c_void> = {
        let raw = crate::app::platform::native_window::main()
            .ok_or_else(|| "main window introuvable".to_string())?;
        Some(raw as *mut std::os::raw::c_void)
    };
    #[cfg(not(target_os = "windows"))]
    let hwnd: Option<*mut std::os::raw::c_void> = None;

    let config = PlatformConfig {
        dbus_name: "rustmusic",
        display_name: "RustMusic",
        hwnd,
    };

    let mut controls =
        MediaControls::new(config).map_err(|e| format!("MediaControls::new: {:?}", e))?;

    // Callbacks OS → frontend. Les touches média et clics dans la barre des
    // tâches arrivent ici et sont relayés au frontend via l'event tauri
    // "smtc-command" que le playerService écoute pour appeler les bonnes
    // fonctions (handleTogglePlay, nextTrack, etc.).
    let app_for_cb = app.clone();
    controls
        .attach(move |event: MediaControlEvent| {
            let cmd: &str = match event {
                MediaControlEvent::Play => "play",
                MediaControlEvent::Pause => "pause",
                MediaControlEvent::Toggle => "toggle",
                MediaControlEvent::Next => "next",
                MediaControlEvent::Previous => "previous",
                MediaControlEvent::Stop => "stop",
                MediaControlEvent::Quit => "quit",
                MediaControlEvent::Raise => "raise",
                MediaControlEvent::SeekBy(_, _)
                | MediaControlEvent::Seek(_)
                | MediaControlEvent::SetPosition(_)
                | MediaControlEvent::SetVolume(_)
                | MediaControlEvent::OpenUri(_) => return,
                // Commandes ajoutées par les versions suivantes (lecture aléatoire…).
                _ => return,
            };
            let _ = app_for_cb.emit("smtc-command", cmd);
        })
        .map_err(|e| format!("smtc attach: {:?}", e))?;

    // ─── Serveur HTTP local pour servir les covers ───
    // Best-effort : si le bind échoue, on continue sans cover (l'OS affichera
    // l'icône de l'app à la place). Le SMTC marche quand même.
    if let Err(e) = start_cover_http_server(app) {
        log::warn!("smtc: cover HTTP server failed: {} — covers désactivées", e);
    }

    log::info!("🎛️  SMTC activé (System Media Transport Controls)");
    *guard = Some(controls);
    Ok(())
}

/// Détruit l'instance SMTC. Idempotent.
/// Drop de l'objet → souvlaki désinscrit l'app de SMTC/MPRIS proprement.
pub fn shutdown() {
    // Arrête le serveur HTTP cover si actif.
    if let Ok(mut g) = http_slot().lock() {
        if let Some(http) = g.take() {
            let _ = http.shutdown.send(());
        }
    }

    if let Ok(mut guard) = slot().lock() {
        if guard.take().is_some() {
            log::info!("🎛️  SMTC désactivé");
        }
    }
}

/// Met à jour les métadonnées du morceau affichées par l'OS.
/// `cover_url` doit être une URL absolue (`http://127.0.0.1:port/cover/...`
/// ou `https://...`).
pub fn set_metadata(
    title: &str,
    artist: Option<&str>,
    album: Option<&str>,
    cover_url: Option<&str>,
    duration_secs: Option<f64>,
) {
    if let Ok(mut guard) = slot().lock() {
        if let Some(c) = guard.as_mut() {
            let meta = MediaMetadata {
                title: Some(title),
                album,
                artist,
                cover_url,
                duration: duration_secs
                    .filter(|d| d.is_finite() && *d > 0.0)
                    .map(Duration::from_secs_f64),
            };
            if let Err(e) = c.set_metadata(meta) {
                log::warn!("smtc set_metadata: {:?}", e);
            }
        }
    }
}

/// Met à jour l'état de lecture (joue / pause / arrêté) + position courante.
pub fn set_playback(state: PlaybackState, position_secs: Option<f64>) {
    if let Ok(mut guard) = slot().lock() {
        if let Some(c) = guard.as_mut() {
            let progress = position_secs
                .filter(|p| p.is_finite() && *p >= 0.0)
                .map(|p| MediaPosition(Duration::from_secs_f64(p)));
            let playback = match state {
                PlaybackState::Playing => MediaPlayback::Playing { progress },
                PlaybackState::Paused => MediaPlayback::Paused { progress },
                PlaybackState::Stopped => MediaPlayback::Stopped,
            };
            if let Err(e) = c.set_playback(playback) {
                log::warn!("smtc set_playback: {:?}", e);
            }
        }
    }
}

/// État de lecture côté OS — décorrélé de l'enum `PlayerStatus` du frontend
/// pour ne pas coupler les deux.
#[derive(Debug, Clone, Copy)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

/// Vrai ssi SMTC est actuellement initialisé.
pub fn is_active() -> bool {
    slot().lock().ok().map(|g| g.is_some()).unwrap_or(false)
}

/// Convertit un chemin fichier absolu en URL HTTP locale servie par notre
/// petit serveur (`http://127.0.0.1:port/cover/<relative>`). Retourne `None`
/// si :
/// - le serveur HTTP n'est pas démarré (SMTC désactivé)
/// - le chemin n'est pas sous le dossier `covers/`
/// - le chemin est vide / invalide
pub fn cover_url_for_path(file_path: &str) -> Option<String> {
    let guard = http_slot().lock().ok()?;
    let http = guard.as_ref()?;

    let path = Path::new(file_path);
    let relative = path.strip_prefix(&http.covers_dir).ok()?;
    let url_path = relative.to_string_lossy().replace('\\', "/");
    if url_path.is_empty() {
        return None;
    }
    Some(format!("{}/{}", http.base_url, url_path))
}

/// Bind un listener TCP sur `127.0.0.1:0` (port assigné par l'OS), spawn un
/// `axum::serve` en background qui sert `covers/` à `/cover/...`, et stocke
/// la base URL + le sender de shutdown dans `http_slot()`.
fn start_cover_http_server(app: &AppHandle) -> Result<(), String> {
    {
        let g = http_slot().lock().map_err(|e| format!("http lock: {}", e))?;
        if g.is_some() {
            return Ok(()); // déjà démarré (toggle SMTC off/on sans drop complet)
        }
    }

    let covers_dir = crate::app::platform::paths::covers_dir();

    // S'assure que le dossier existe — sinon ServeDir échouera silencieusement
    // sur toutes les requêtes.
    std::fs::create_dir_all(&covers_dir)
        .map_err(|e| format!("create covers dir: {}", e))?;

    // Bind sync pour récupérer le port AVANT de retourner.
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("bind: {}", e))?;
    let port = std_listener
        .local_addr()
        .map_err(|e| format!("local_addr: {}", e))?
        .port();
    std_listener
        .set_nonblocking(true)
        .map_err(|e| format!("set_nonblocking: {}", e))?;

    let base_url = format!("http://127.0.0.1:{}/cover", port);
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Persist state pour les lookups + shutdown.
    {
        let mut g = http_slot().lock().map_err(|e| format!("http lock: {}", e))?;
        *g = Some(CoverHttp {
            base_url: base_url.clone(),
            covers_dir: covers_dir.clone(),
            shutdown: shutdown_tx,
        });
    }

    // Spawn la boucle d'écoute. ServeDir gère les MIME types + range requests
    // automatiquement, parfait pour des images.
    crate::app::platform::runtime::spawn(async move {
        let listener = match tokio::net::TcpListener::from_std(std_listener) {
            Ok(l) => l,
            Err(e) => {
                log::error!("smtc cover http: from_std failed: {}", e);
                return;
            }
        };
        let app = Router::new().nest_service("/cover", ServeDir::new(&covers_dir));
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await;
        log::debug!("smtc cover http: serveur arrêté");
    });

    log::info!("🌐 SMTC cover server : http://127.0.0.1:{}/cover", port);
    Ok(())
}
