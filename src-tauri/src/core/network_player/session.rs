//! Session de lecture sur un lecteur réseau : charge les pistes, suit la position
//! et parle à l'interface avec les mêmes événements que le lecteur local
//! (`playback-preparing`, `playback-ended`, `track-advanced`).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use super::control::{Controller, TransportState};
use super::didl::{self, TrackMeta};
use super::discovery::Renderer;
use super::interfaces::local_ip_toward;
use super::media;

/// Cadence d'interrogation du lecteur.
const POLL: Duration = Duration::from_millis(500);
/// Sans réponse pendant ce nombre de tours (~5 s), le lecteur est déclaré perdu.
const LOST_AFTER: u32 = 10;

#[derive(Clone)]
struct Loaded {
    path: PathBuf,
    url: String,
}

#[derive(Default)]
struct State {
    controller: Option<Controller>,
    /// Qui reçoit lecture, pause et seek : la coordinatrice d'un groupe Sonos, sinon le lecteur choisi.
    transport: Option<Controller>,
    /// Types MIME que le lecteur accepte ; `None` tant qu'on ne les a pas demandés.
    sinks: Option<Vec<String>>,
    current: Option<Loaded>,
    /// Piste préparée pour un enchaînement fait par le lecteur.
    next: Option<Loaded>,
    /// Suite annoncée par l'interface avant que la piste en cours soit chargée.
    wanted_next: Option<PathBuf>,
    position: f64,
    duration: f64,
    paused: bool,
    /// Arrêt demandé : la fin qui suit n'est pas une fin de piste.
    user_stopped: bool,
    saw_playing: bool,
    ended_sent: bool,
}

static STATE: Mutex<State> = Mutex::new(State {
    controller: None,
    transport: None,
    sinks: None,
    current: None,
    next: None,
    wanted_next: None,
    position: 0.0,
    duration: 0.0,
    paused: false,
    user_stopped: false,
    saw_playing: false,
    ended_sent: false,
});
/// Change à chaque chargement : l'ancienne boucle d'interrogation s'arrête.
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// Change à chaque suite annoncée : une préparation dépassée n'est pas envoyée.
static NEXT_SEQ: AtomicU64 = AtomicU64::new(0);
/// Tours d'interrogation à l'arrêt avant toute lecture (~10 s) : chargement raté.
const NEVER_STARTED_AFTER: u32 = 20;

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Un lecteur réseau est-il la sortie choisie ?
pub fn is_active() -> bool {
    state().controller.is_some()
}

/// Lecteur choisi : `None` revient à la sortie locale.
pub async fn select(renderer: Option<Renderer>) {
    let previous = {
        let mut s = state();
        // Celui qui jouait vraiment (coordinatrice d'un groupe Sonos, sinon le choisi).
        let previous = s.transport.take().or_else(|| s.controller.take());
        *s = State { controller: renderer.map(Controller::new), ..State::default() };
        previous
    };
    GENERATION.fetch_add(1, Ordering::Relaxed);
    if let Some(old) = previous {
        let _ = old.stop().await;
    }
}

/// Le lecteur choisi, s'il y en a un.
pub fn selected() -> Option<Renderer> {
    state().controller.as_ref().map(|c| c.renderer().clone())
}

fn controller() -> Result<Controller, String> {
    state().controller.clone().ok_or_else(|| "aucun lecteur réseau choisi".to_string())
}

/// Le lecteur qui exécute la lecture (coordinatrice d'un groupe Sonos, sinon le choisi).
fn transport() -> Result<Controller, String> {
    let s = state();
    s.transport.clone().or_else(|| s.controller.clone()).ok_or_else(|| "aucun lecteur réseau choisi".to_string())
}

/// Lance `path` sur le lecteur.
pub async fn play(app: AppHandle, path: PathBuf) -> Result<(), String> {
    let result = start(&app, path).await;
    if result.is_err() {
        let _ = app.emit("playback-preparing", false);
    }
    result
}

async fn start(app: &AppHandle, path: PathBuf) -> Result<(), String> {
    let chosen = controller()?;
    // Le groupe Sonos a pu changer depuis le dernier morceau : on redemande.
    let ctl = match super::sonos::coordinator(chosen.renderer()).await {
        Some(coordinator) => Controller::new(coordinator),
        None => chosen,
    };
    state().transport = Some(ctl.clone());
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let _ = app.emit("playback-preparing", true);

    let (loaded, meta, didl) = prepare(&ctl, &path).await?;
    // Une lecture plus récente a été demandée pendant la préparation.
    if GENERATION.load(Ordering::Relaxed) != generation {
        return Ok(());
    }
    {
        let mut s = state();
        s.current = Some(loaded.clone());
        s.next = None;
        s.position = 0.0;
        s.duration = meta.duration.unwrap_or(0.0);
        s.paused = false;
        s.user_stopped = false;
        s.saw_playing = false;
        s.ended_sent = false;
    }

    if let Err(e) = ctl.set_uri(&loaded.url, &didl).await {
        // Certains lecteurs refusent une nouvelle adresse en pleine lecture.
        log::debug!("📡 Nouvelle adresse refusée, arrêt puis nouvel essai : {e}");
        let _ = ctl.stop().await;
        ctl.set_uri(&loaded.url, &didl).await?;
    }
    if GENERATION.load(Ordering::Relaxed) != generation {
        return Ok(());
    }
    ctl.play().await?;
    log::info!("📡 Lecture sur {} : {}", ctl.renderer().name, path.display());

    // Suite annoncée pendant le chargement : préparée maintenant, si l'enchaînement est voulu.
    let wanted = state().wanted_next.clone();
    if let Some(next) = wanted.filter(|_| crate::core::audio_player::preload::gapless_enabled()) {
        let seq = NEXT_SEQ.load(Ordering::Relaxed);
        let _ = prepare_next(&ctl, &next, seq).await;
    }

    tokio::spawn(poll(app.clone(), ctl, generation));
    Ok(())
}

/// Piste suivante annoncée par l'interface (`None` : plus rien après).
pub async fn set_next(path: Option<PathBuf>) {
    let seq = NEXT_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    state().wanted_next = path.clone();
    let Ok(ctl) = transport() else { return };
    if state().current.is_none() {
        return;
    }
    match path {
        Some(p) if crate::core::audio_player::preload::gapless_enabled() => {
            if let Err(e) = prepare_next(&ctl, &p, seq).await {
                log::debug!("📡 Suite non préparée sur le lecteur : {e}");
            }
        }
        _ => {
            // Suite retirée (fin de file, minuterie) : le lecteur l'oublie aussi.
            if state().next.take().is_some() {
                if let Err(e) = ctl.set_next_uri("", "").await {
                    log::debug!("📡 Suite non effacée sur le lecteur : {e}");
                }
            }
        }
    }
}

/// `seq` : la suite demandée ; si une autre l'a remplacée entre-temps, rien n'est envoyé.
async fn prepare_next(ctl: &Controller, path: &Path, seq: u64) -> Result<(), String> {
    if state().next.as_ref().is_some_and(|n| n.path == path) {
        return Ok(());
    }
    let (loaded, _, didl) = prepare(ctl, path).await?;
    if NEXT_SEQ.load(Ordering::Relaxed) != seq {
        return Ok(());
    }
    ctl.set_next_uri(&loaded.url, &didl).await?;
    state().next = Some(loaded);
    Ok(())
}

/// Publie `path` et prépare ses métadonnées.
async fn prepare(ctl: &Controller, path: &Path) -> Result<(Loaded, TrackMeta, String), String> {
    let host = local_ip_toward(ctl.renderer().ip).ok_or("adresse locale introuvable vers le lecteur")?;
    let owned = path.to_path_buf();
    let (mut meta, cover) = tokio::task::spawn_blocking(move || read_meta(&owned))
        .await
        .map_err(|e| e.to_string())?;
    check_format(ctl, path).await?;
    let published = media::publish(path, cover, host).await?;
    meta.cover_url = published.cover_url.clone();
    let didl = didl::item(&meta, &published.url, &published.mime);
    Ok((Loaded { path: path.to_path_buf(), url: published.url }, meta, didl))
}

/// Refuse d'emblée un format que le lecteur annonce ne pas lire (DSD…).
async fn check_format(ctl: &Controller, path: &Path) -> Result<(), String> {
    let known = state().sinks.clone();
    let sinks = match known {
        Some(s) => s,
        None => {
            let s = ctl.sink_protocols().await;
            state().sinks = Some(s.clone());
            s
        }
    };
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let mime = crate::core::dlna_server::library::sqlite_provider::guess_mime_from_extension(&ext);
    if supports(&sinks, &mime) {
        Ok(())
    } else {
        Err(format!("{} ne lit pas les fichiers .{ext}", ctl.renderer().name))
    }
}

/// Liste vide : le lecteur ne dit rien, on essaie. `audio/x-flac` vaut `audio/flac`.
fn supports(sinks: &[String], mime: &str) -> bool {
    let norm = |m: &str| m.trim().to_ascii_lowercase().replace("/x-", "/");
    let wanted = norm(mime);
    sinks.is_empty()
        || sinks.iter().any(|p| p.split(':').nth(2).is_some_and(|m| m == "*" || norm(m) == wanted))
}

/// Titre, artiste, album, format et pochette intégrée, lus dans le fichier.
pub(super) fn read_meta(path: &Path) -> (TrackMeta, Option<(Vec<u8>, String)>) {
    let fallback = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let Ok(file) = crate::core::audio_analyser::audio_analyser::AudioAnalyser::analyse_audio_file(&path.to_path_buf()) else {
        return (TrackMeta { title: fallback, ..Default::default() }, None);
    };
    let tags = file.tags;
    let cover = tags
        .attached_images
        .into_iter()
        .find(|i| !i.image_data.is_empty())
        .map(|i| (i.image_data, i.mime_type));
    let meta = TrackMeta {
        title: tags.title.filter(|t| !t.is_empty()).unwrap_or(fallback),
        artist: tags.artist,
        album: tags.album,
        album_artist: tags.album_artist,
        cover_url: None,
        duration: (file.duration > 0.0).then_some(file.duration as f64),
        size: Some(file.file_size),
        sample_rate: (file.sample_rate > 0).then_some(file.sample_rate),
        bits: (file.bits_per_sample > 0).then_some(file.bits_per_sample),
        channels: (file.channels > 0).then_some(file.channels as u32),
    };
    (meta, cover)
}

/// Lecture / pause.
pub async fn toggle_pause() -> Result<(), String> {
    let ctl = transport()?;
    let paused = state().paused;
    if paused { ctl.play().await? } else { ctl.pause().await? }
    state().paused = !paused;
    Ok(())
}

/// Arrêt demandé : aucune fin de piste ne sera annoncée.
pub async fn stop() -> Result<(), String> {
    {
        let mut s = state();
        s.user_stopped = true;
        s.current = None;
        s.next = None;
    }
    GENERATION.fetch_add(1, Ordering::Relaxed);
    transport()?.stop().await
}

pub async fn seek(seconds: f64) -> Result<(), String> {
    transport()?.seek(seconds).await?;
    state().position = seconds;
    Ok(())
}

/// (position, durée) en secondes, pour la barre de progression.
pub fn progress() -> (f64, f64) {
    let s = state();
    (s.position, s.duration)
}

pub async fn volume() -> Option<u8> {
    controller().ok()?.volume().await.ok().flatten()
}

pub async fn set_volume(volume: u8) -> Result<(), String> {
    controller()?.set_volume(volume).await
}

/// Suit le lecteur : position, enchaînement fait par lui, fin de piste.
async fn poll(app: AppHandle, ctl: Controller, generation: u64) {
    let mut failures = 0u32;
    let mut idle = 0u32;
    loop {
        tokio::time::sleep(POLL).await;
        if GENERATION.load(Ordering::Relaxed) != generation {
            return;
        }
        let (transport, position) = tokio::join!(ctl.transport_state(), ctl.position());
        let (Ok(transport), Ok(position)) = (transport, position) else {
            failures += 1;
            if failures == LOST_AFTER {
                log::warn!("📡 {} ne répond plus", ctl.renderer().name);
                let _ = app.emit("network-player-lost", ctl.renderer().name.clone());
            }
            continue;
        };
        failures = 0;
        if GENERATION.load(Ordering::Relaxed) != generation {
            return;
        }

        let mut events: Vec<(&str, String)> = Vec::new();
        {
            let mut s = state();
            // Le lecteur est passé à la piste préparée : c'est un enchaînement.
            let advanced = s.next.as_ref().is_some_and(|n| uri_matches(&position.track_uri, &n.url));
            if advanced {
                let next = s.next.take().unwrap();
                events.push(("track-advanced", next.path.to_string_lossy().to_string()));
                s.current = Some(next);
                s.wanted_next = None;
                s.ended_sent = false;
                s.duration = 0.0;
            }
            if let Some(e) = position.elapsed {
                s.position = e;
            }
            if let Some(d) = position.duration.filter(|d| *d > 0.0) {
                s.duration = d;
            }
            match transport {
                TransportState::Playing => {
                    if !s.saw_playing {
                        s.saw_playing = true;
                        events.push(("playback-preparing", "false".into()));
                    }
                    s.paused = false;
                }
                TransportState::Paused => s.paused = true,
                TransportState::Stopped | TransportState::NoMedia
                    if s.saw_playing && !s.user_stopped && !s.ended_sent =>
                {
                    s.ended_sent = true;
                    let path = s.current.as_ref().map(|c| c.path.to_string_lossy().to_string()).unwrap_or_default();
                    events.push(("playback-ended", path));
                }
                // Jamais parti : le lecteur a refusé la piste sans le dire.
                TransportState::Stopped | TransportState::NoMedia if !s.saw_playing && !s.user_stopped => {
                    idle += 1;
                    if idle == NEVER_STARTED_AFTER {
                        events.push(("playback-preparing", "false".into()));
                        events.push(("network-player-error", format!("{} n'a pas démarré la lecture", ctl.renderer().name)));
                    }
                }
                _ => {}
            }
        }
        for (name, payload) in events {
            log::debug!("📡 {name} {payload}");
            let _ = match name {
                "playback-preparing" => app.emit(name, false),
                _ => app.emit(name, payload),
            };
        }
    }
}

/// Le lecteur peut réécrire l'adresse (encodage, port) : on compare le jeton.
fn uri_matches(reported: &str, ours: &str) -> bool {
    let token = |u: &str| u.rsplit('/').next().unwrap_or("").split('.').next().unwrap_or("").to_string();
    !reported.is_empty() && token(reported) == token(ours)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_formats_against_the_renderer_list() {
        let sinks = vec!["http-get:*:audio/x-flac:*".to_string(), "http-get:*:audio/mpeg:DLNA.ORG_PN=MP3".to_string()];
        assert!(supports(&sinks, "audio/flac"));
        assert!(supports(&sinks, "audio/mpeg"));
        assert!(!supports(&sinks, "audio/x-dsd"));
        assert!(supports(&[], "audio/x-dsd"));
    }

    #[test]
    fn recognises_our_track_in_a_rewritten_uri() {
        let ours = "http://10.0.0.121:8201/stream/0a1b2c.flac";
        assert!(uri_matches("http://10.0.0.121:8201/stream/0a1b2c.flac", ours));
        assert!(uri_matches("http://10.0.0.121:8201/stream/0a1b2c", ours));
        assert!(!uri_matches("http://10.0.0.121:8201/stream/ffffff.flac", ours));
        assert!(!uri_matches("", ours));
    }
}
