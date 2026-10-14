//! Session AirPlay : poignée de main RTSP, horloge, envoi cadencé des paquets audio.
//!
//! Comme la sortie partagée CPAL, la session reste ouverte d'une piste à l'autre :
//! chaque piste branche son rendu, et la fin de l'une couvre le démarrage de la
//! suivante. L'appareil joue chaque paquet ~2 s après son envoi.

use std::collections::VecDeque;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use tauri::{AppHandle, Emitter};

use super::alac::{self, FRAMES_PER_PACKET};
use super::crypto::Cipher;
use super::device::{AirplayDevice, Encryption, Shows};
use super::rtsp::{self, Rtsp};
use crate::core::audio_player::output::{PlaybackAtomics, SymphoniaSharedState};

pub const SAMPLE_RATE: u32 = 44_100;
pub const CHANNELS: u16 = 2;

/// Rendu d'une piste : remplit le tampon stéréo entrelacé.
pub type RenderFn = Box<dyn FnMut(&mut [f32]) + Send>;

/// Latence annoncée dans la synchro ; l'appareil y ajoute la sienne (`Audio-Latency`).
const DECLARED_LATENCY: u64 = 77_175;
/// Latence propre à l'appareil quand il ne la donne pas.
const DEFAULT_DEVICE_LATENCY: u64 = 11_025;
/// Avance d'envoi sur l'horloge : absorbe l'imprécision des réveils (15 ms sous Windows).
const LEAD: u64 = 4_410;
/// Sans piste branchée pendant ce délai, la session est fermée.
const IDLE_GRACE: Duration = Duration::from_secs(30);
/// Silence envoyé après une fin de piste, le temps que la suivante arrive.
const MAX_SILENCE: u64 = 3 * SAMPLE_RATE as u64;
/// Paquets gardés pour les retransmissions (~4 s).
const HISTORY: usize = 512;
const KEEPALIVE: Duration = Duration::from_secs(10);
/// Attente maximale d'une réponse attendue (FLUSH, TEARDOWN).
const RTSP_WAIT: Duration = Duration::from_secs(6);

/// Une piste branchée : son rendu, et les drapeaux qui pilotent pause et seek.
pub struct Track {
    pub render: RenderFn,
    pub atomics: PlaybackAtomics,
    pub shared: SymphoniaSharedState,
}

struct Shared {
    slot: Mutex<Option<(u64, Track)>>,
    stop: AtomicBool,
    dead: AtomicBool,
    /// Arrêt demandé par l'utilisateur : l'appareil vide son tampon.
    flush: AtomicBool,
    detached_at: Mutex<Instant>,
}

impl Shared {
    fn slot(&self) -> MutexGuard<'_, Option<(u64, Track)>> {
        self.slot.lock().unwrap_or_else(|e| e.into_inner())
    }
}

struct Engine {
    device_id: String,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Engine {
    fn shutdown(mut self) {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

static ENGINE: Mutex<Option<Engine>> = Mutex::new(None);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
/// Trames rendues mais pas encore entendues : la position affichée les retranche.
static LAG_FRAMES: AtomicUsize = AtomicUsize::new(0);
static SELECTED: Mutex<Option<AirplayDevice>> = Mutex::new(None);
static APP: OnceLock<AppHandle> = OnceLock::new();
/// Infos du morceau qui démarre, prises par la session au tour suivant.
static NOW_PLAYING: Mutex<Option<NowPlaying>> = Mutex::new(None);
static NOW_PLAYING_GEN: AtomicU64 = AtomicU64::new(0);

/// Ce que l'appareil affiche : titre/artiste/album (DMAP), pochette, durée.
struct NowPlaying {
    text: Vec<u8>,
    artwork: Option<Vec<u8>>,
    duration: Option<f64>,
}

fn engine() -> MutexGuard<'static, Option<Engine>> {
    ENGINE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Pour prévenir l'interface d'une session perdue.
pub fn set_app(app: AppHandle) {
    let _ = APP.set(app);
}

/// Choisit l'appareil AirPlay de sortie (`None` : aucun) ; la session en cours est fermée.
pub fn select(device: Option<AirplayDevice>) {
    *SELECTED.lock().unwrap_or_else(|e| e.into_inner()) = device;
    close();
}

pub fn selected() -> Option<AirplayDevice> {
    SELECTED.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

pub fn is_active() -> bool {
    SELECTED.lock().unwrap_or_else(|e| e.into_inner()).is_some()
}

/// Un morceau démarre (ou s'enchaîne) : ses infos sont lues à part, puis affichées
/// sur l'appareil quand son début s'entend.
pub fn now_playing(path: std::path::PathBuf) {
    if !is_active() {
        return;
    }
    let gen = NOW_PLAYING_GEN.fetch_add(1, Ordering::AcqRel) + 1;
    std::thread::spawn(move || {
        let (meta, cover) = super::super::session::read_meta(&path);
        // Pochette réduite : l'appareil n'affiche pas plus, et l'envoi reste court.
        let artwork = cover.and_then(|(bytes, _)| crate::core::audio_metadata::injector::image_prep::thumbnail(&bytes, 600).ok());
        let info = NowPlaying {
            text: dmap_item(&meta.title, meta.artist.as_deref(), meta.album.as_deref()),
            artwork,
            duration: meta.duration,
        };
        // Un morceau plus récent a pu être lancé entre-temps.
        if NOW_PLAYING_GEN.load(Ordering::Acquire) == gen {
            *NOW_PLAYING.lock().unwrap_or_else(|e| e.into_inner()) = Some(info);
        }
    });
}

#[cfg(test)]
pub(crate) fn test_now_playing(title: &str, duration: f64) {
    *NOW_PLAYING.lock().unwrap() = Some(NowPlaying { text: dmap_item(title, None, None), artwork: None, duration: Some(duration) });
}

/// Retard entre le son rendu et le son entendu, en secondes.
pub fn lag_secs() -> f64 {
    LAG_FRAMES.load(Ordering::Relaxed) as f64 / SAMPLE_RATE as f64
}

/// Branche une piste ; ouvre la session si besoin. Rend l'identifiant à passer à [`detach`].
pub fn attach(device: &AirplayDevice, track: Track) -> Result<u64, String> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let mut guard = engine();
    if let Some(e) = guard.as_ref() {
        if e.device_id == device.id && !e.shared.dead.load(Ordering::Acquire) {
            let old = e.shared.slot().replace((id, track));
            drop(old);
            return Ok(id);
        }
    }
    if let Some(old) = guard.take() {
        old.shutdown();
    }

    let conn = Connection::open(device)?;
    log::info!("📡 AirPlay : session ouverte avec {} ({})", device.name, device.ip);
    let shared = Arc::new(Shared {
        slot: Mutex::new(Some((id, track))),
        stop: AtomicBool::new(false),
        dead: AtomicBool::new(false),
        flush: AtomicBool::new(false),
        detached_at: Mutex::new(Instant::now()),
    });
    let streamer = Streamer::new(conn, shared.clone(), device.name.clone(), device.shows);
    let thread = std::thread::Builder::new()
        .name("rustmusic-airplay".into())
        .spawn(move || streamer.run())
        .map_err(|e| e.to_string())?;
    *guard = Some(Engine { device_id: device.id.clone(), shared, thread: Some(thread) });
    Ok(id)
}

/// Débranche la piste si c'est encore elle. Arrêtée par l'utilisateur, l'appareil se tait
/// aussitôt ; finie d'elle-même, sa fin joue encore et la suivante prend le relais.
pub fn detach(id: u64) {
    let old = {
        let guard = engine();
        let Some(e) = guard.as_ref() else { return };
        let mut slot = e.shared.slot();
        if slot.as_ref().map(|(i, _)| *i) != Some(id) {
            return;
        }
        let old = slot.take();
        if old.as_ref().is_some_and(|(_, t)| t.atomics.is_stopped.load(Ordering::Relaxed)) {
            e.shared.flush.store(true, Ordering::Release);
        }
        *e.shared.detached_at.lock().unwrap_or_else(|p| p.into_inner()) = Instant::now();
        old
    };
    drop(old);
}

/// Ferme la session tout de suite (changement de sortie).
pub fn close() {
    let old = engine().take();
    if let Some(e) = old {
        e.shutdown();
        LAG_FRAMES.store(0, Ordering::Relaxed);
    }
}

// ─── Connexion ───

struct Connection {
    /// Requêtes au fil RTSP : le son n'attend que FLUSH et TEARDOWN.
    rtsp: rtsp::Worker,
    audio: UdpSocket,
    control: Arc<UdpSocket>,
    control_addr: SocketAddr,
    cipher: Option<Cipher>,
    ssrc: u32,
    seq: u16,
    /// Horodatage RTP de la prochaine trame à envoyer.
    head: u64,
    /// Délai entre l'envoi d'une trame et son écoute.
    latency: u64,
    history: Arc<Mutex<VecDeque<(u16, Vec<u8>)>>>,
    _helpers: Helpers,
}

/// Fils d'horloge et de contrôle, arrêtés avec la connexion (ou son échec).
struct Helpers {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl Drop for Helpers {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

impl Connection {
    fn open(device: &AirplayDevice) -> Result<Self, String> {
        let addr = SocketAddr::new(device.ip.into(), device.port);
        let mut rtsp = Rtsp::connect(addr)?;
        let local_ip = rtsp.local_ip().ok_or("adresse locale inconnue")?;
        let cipher = match device.encryption {
            Encryption::None => None,
            Encryption::Rsa => Some(Cipher::random()?),
        };
        rtsp.request("OPTIONS", Some("*"), &[], None)?;
        let sdp = announce_sdp(rtsp::random_u64() as u32, &local_ip.to_string(), &device.ip.to_string(), cipher.as_ref())?;
        rtsp.request("ANNOUNCE", None, &[], Some(("application/sdp", sdp.as_bytes())))?;

        // L'appareil interroge notre horloge dès la configuration : les ports écoutent avant.
        let bind = |what: &str| -> Result<UdpSocket, String> {
            let s = UdpSocket::bind(("0.0.0.0", 0)).map_err(|e| format!("port {what} : {e}"))?;
            s.set_read_timeout(Some(Duration::from_millis(200))).ok();
            Ok(s)
        };
        let control = Arc::new(bind("de contrôle")?);
        let timing = bind("d'horloge")?;
        let port = |s: &UdpSocket| s.local_addr().map(|a| a.port()).unwrap_or(0);
        let (control_port, timing_port) = (port(&control), port(&timing));
        let history = Arc::new(Mutex::new(VecDeque::with_capacity(HISTORY)));
        let stop = Arc::new(AtomicBool::new(false));
        let helpers = Helpers {
            threads: vec![
                spawn_timing(timing, stop.clone()),
                spawn_control(control.clone(), history.clone(), stop.clone()),
            ],
            stop,
        };

        let transport = format!(
            "RTP/AVP/UDP;unicast;interleaved=0-1;mode=record;control_port={control_port};timing_port={timing_port}"
        );
        let setup = rtsp.request("SETUP", None, &[("Transport", transport)], None)?;
        let session = setup.header("Session").map(|s| s.split(';').next().unwrap_or(s).trim().to_string());
        let reply = setup.header("Transport").unwrap_or_default();
        let (Some(session), Some(server_port)) = (session, rtsp::transport_param(reply, "server_port")) else {
            return Err("SETUP : réponse incomplète".into());
        };
        let remote_control = rtsp::transport_param(reply, "control_port").unwrap_or(server_port + 1);
        rtsp.set_session(Some(session));

        let audio = UdpSocket::bind(("0.0.0.0", 0)).map_err(|e| e.to_string())?;
        audio.connect((device.ip, server_port)).map_err(|e| e.to_string())?;
        let random = rtsp::random_u64();
        let (seq, head) = (random as u16, (random >> 16) as u32 as u64);
        let rtp_info = format!("seq={seq};rtptime={}", head as u32);
        let record = rtsp.request("RECORD", None, &[("Range", "npt=0-".into()), ("RTP-Info", rtp_info)], None)?;
        let device_latency = record.header("Audio-Latency").and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_DEVICE_LATENCY);

        Ok(Self {
            rtsp: rtsp::Worker::spawn(rtsp)?,
            audio,
            control,
            control_addr: SocketAddr::new(device.ip.into(), remote_control),
            cipher,
            ssrc: (random >> 32) as u32,
            seq,
            head,
            latency: DECLARED_LATENCY + device_latency,
            history,
            _helpers: helpers,
        })
    }

    /// L'appareil jette ce qu'il a en tampon ; la suite reprend à `seq`/`head`.
    fn flush(&mut self) -> Result<(), String> {
        let rtp_info = format!("seq={};rtptime={}", self.seq, self.head as u32);
        self.rtsp.call("FLUSH", vec![("RTP-Info", rtp_info)], RTSP_WAIT)
    }

    /// Infos d'affichage, sans attendre l'appareil ; un refus n'interrompt pas la lecture.
    fn set_parameter(&mut self, content_type: &str, body: &[u8], rtptime: u32) {
        let info = format!("rtptime={rtptime}");
        self.rtsp.send("SET_PARAMETER", content_type, vec![("RTP-Info", info)], Some((content_type, body)), false);
    }

    /// Maintien de la connexion ; un échec remonte par [`rtsp::Worker::error`].
    fn keepalive(&mut self) {
        self.rtsp.send("OPTIONS", "keepalive", Vec::new(), None, true);
    }

    fn teardown(&mut self) {
        let _ = self.rtsp.call("TEARDOWN", Vec::new(), RTSP_WAIT);
    }

    /// Paquet de synchro : la trame `now_ts` correspond à notre horloge actuelle.
    fn sync(&self, now_ts: u64, first: bool) {
        let p = sync_packet(now_ts, ntp_now(), first);
        let _ = self.control.send_to(&p, self.control_addr);
    }

    fn send(&mut self, payload: &[u8], marker: bool) {
        let packet = audio_packet(self.seq, self.head as u32, self.ssrc, marker, payload);
        if let Err(e) = self.audio.send(&packet) {
            log::debug!("📡 AirPlay : paquet non envoyé ({e})");
        }
        let mut history = self.history.lock().unwrap_or_else(|p| p.into_inner());
        if history.len() >= HISTORY {
            history.pop_front();
        }
        history.push_back((self.seq, packet));
        self.seq = self.seq.wrapping_add(1);
        self.head += FRAMES_PER_PACKET as u64;
    }
}

// ─── Envoi ───

/// Positions des paquets en vol : la position entendue est celle du dernier paquet joué.
#[derive(Default)]
struct Heard {
    entries: VecDeque<(u64, usize)>,
    base: usize,
}

impl Heard {
    fn reset(&mut self, position: usize) {
        self.entries.clear();
        self.base = position;
    }

    fn push(&mut self, end_ts: u64, position: usize) {
        self.entries.push_back((end_ts, position));
    }

    fn at(&mut self, playing_ts: u64) -> usize {
        while let Some(&(end, position)) = self.entries.front() {
            if end > playing_ts {
                break;
            }
            self.base = position;
            self.entries.pop_front();
        }
        self.base
    }
}

struct Streamer {
    conn: Connection,
    shared: Arc<Shared>,
    name: String,
    shows: Shows,
    info: Option<NowPlaying>,
    /// Horodatage du début du morceau en cours.
    track_start: u64,
    /// Infos à afficher quand cet horodatage s'entend ; `true` : titre et pochette, sinon progression seule.
    announce: Option<(u64, bool)>,
    /// (horodatage, instant) d'ancrage de l'horloge RTP.
    anchor: (u64, Instant),
    /// L'appareil n'a plus rien en attente : la ligne de temps repart au prochain paquet.
    flushed: bool,
    marker: bool,
    track_id: u64,
    heard: Heard,
    silence: u64,
    last_sync: Instant,
    last_keepalive: Instant,
    samples: Vec<f32>,
    pcm: Vec<i16>,
    payload: Vec<u8>,
}

impl Streamer {
    fn new(conn: Connection, shared: Arc<Shared>, name: String, shows: Shows) -> Self {
        let now = Instant::now();
        Self {
            anchor: (conn.head, now),
            track_start: conn.head,
            conn,
            shared,
            name,
            shows,
            info: None,
            announce: None,
            flushed: true,
            marker: true,
            track_id: 0,
            heard: Heard::default(),
            silence: 0,
            last_sync: now,
            last_keepalive: now,
            samples: vec![0.0; FRAMES_PER_PACKET * CHANNELS as usize],
            pcm: vec![0; FRAMES_PER_PACKET * CHANNELS as usize],
            payload: Vec::with_capacity(1500),
        }
    }

    fn run(mut self) {
        loop {
            if self.shared.stop.load(Ordering::Acquire) {
                self.conn.teardown();
                log::info!("📡 AirPlay : session fermée");
                break;
            }
            match self.step() {
                Ok(true) => {}
                Ok(false) => {
                    self.conn.teardown();
                    log::info!("📡 AirPlay : session rendue après inactivité");
                    break;
                }
                Err(e) => {
                    self.fail(&e);
                    break;
                }
            }
        }
        self.shared.dead.store(true, Ordering::Release);
        LAG_FRAMES.store(0, Ordering::Relaxed);
    }

    /// Session perdue : la lecture s'arrête et l'interface le dit.
    fn fail(&self, error: &str) {
        log::warn!("📡 AirPlay : session perdue avec {} ({error})", self.name);
        if let Some((_, track)) = self.shared.slot().as_ref() {
            track.atomics.is_stopped.store(true, Ordering::SeqCst);
            if let Some(app) = APP.get() {
                let _ = app.emit("network-player-error", format!("AirPlay — {} : {error}", self.name));
            }
        }
    }

    fn now_ts(&self, now: Instant) -> u64 {
        let elapsed = now.saturating_duration_since(self.anchor.1);
        self.anchor.0 + (elapsed.as_nanos() * SAMPLE_RATE as u128 / 1_000_000_000) as u64
    }

    /// La ligne de temps repart : le prochain paquet sera joué après la latence.
    fn restart(&mut self, now: Instant) {
        self.anchor = (self.conn.head, now);
        self.flushed = false;
        self.marker = true;
        self.silence = 0;
        self.conn.sync(self.conn.head, true);
        self.last_sync = now;
    }

    fn periodic(&mut self, now: Instant) -> Result<(), String> {
        if !self.flushed && now.duration_since(self.last_sync) >= Duration::from_secs(1) {
            self.conn.sync(self.now_ts(now), false);
            self.last_sync = now;
        }
        if now.duration_since(self.last_keepalive) >= KEEPALIVE {
            self.conn.keepalive();
            self.last_keepalive = now;
        }
        // Connexion de contrôle perdue : la session s'arrête.
        self.conn.rtsp.error().map_or(Ok(()), Err)
    }

    /// Un tour de boucle ; `Ok(false)` : session inactive à fermer.
    fn step(&mut self) -> Result<bool, String> {
        let now = Instant::now();
        if self.shared.flush.swap(false, Ordering::AcqRel) && !self.flushed {
            self.conn.flush()?;
            self.flushed = true;
        }

        let shared = self.shared.clone();
        let mut slot = shared.slot();
        let Some((id, track)) = slot.as_mut() else {
            drop(slot);
            return self.idle(now);
        };
        if *id != self.track_id {
            self.track_id = *id;
            self.heard.reset(0);
            self.track_start = self.conn.head;
        }
        // Pris après le changement de piste : `track_start` est celui du nouveau morceau.
        if let Some(info) = NOW_PLAYING.lock().unwrap_or_else(|e| e.into_inner()).take() {
            self.info = Some(info);
            self.announce = Some((self.track_start, true));
        }

        let paused = track.atomics.is_paused.load(Ordering::Relaxed);
        let stopped = track.atomics.is_stopped.load(Ordering::Relaxed);
        if paused || stopped {
            if !self.flushed {
                let heard = self.heard.at(self.now_ts(now).saturating_sub(self.conn.latency));
                self.conn.flush()?;
                self.flushed = true;
                // Ce qui était en vol n'a pas été entendu : la reprise repart d'où l'oreille s'est arrêtée.
                if paused && !stopped {
                    rewind(track, heard);
                }
                self.heard.reset(track.atomics.current_position_frames.load(Ordering::Relaxed));
                LAG_FRAMES.store(0, Ordering::Relaxed);
            }
            drop(slot);
            self.periodic(now)?;
            std::thread::sleep(Duration::from_millis(10));
            return Ok(true);
        }

        let seeking = track.shared.seek_flush.load(Ordering::Acquire);
        if seeking && !self.flushed {
            self.conn.flush()?;
            self.flushed = true;
        }
        // Trop de retard (rendu bloqué) : les paquets arriveraient après leur heure.
        if self.flushed || self.now_ts(now) > self.conn.head + SAMPLE_RATE as u64 {
            self.restart(now);
        }
        if self.conn.head >= self.now_ts(now) + LEAD {
            drop(slot);
            self.periodic(now)?;
            std::thread::sleep(Duration::from_millis(4));
            return Ok(true);
        }

        (track.render)(&mut self.samples);
        let position = track.atomics.current_position_frames.load(Ordering::Relaxed);
        drop(slot);
        let resumed = self.marker;
        if seeking {
            self.heard.reset(position);
        }
        self.send_samples();
        self.heard.push(self.conn.head, position);
        // `position` trames du morceau sont sorties jusqu'à `head`.
        self.track_start = self.conn.head.saturating_sub(position as u64);
        if seeking || resumed {
            // Après un seek ou une reprise, la progression repart de là.
            let text = self.announce.is_some_and(|(_, t)| t);
            self.announce = Some((self.conn.head - FRAMES_PER_PACKET as u64, text));
        }
        let playing = self.now_ts(now).saturating_sub(self.conn.latency);
        let heard = self.heard.at(playing);
        LAG_FRAMES.store(position.saturating_sub(heard), Ordering::Relaxed);
        if let Some((ts, text)) = self.announce {
            if playing >= ts {
                self.announce = None;
                self.show(text, playing);
            }
        }
        self.periodic(now)?;
        Ok(true)
    }

    /// Aucune piste branchée : du silence le temps que la suivante arrive, puis on se tait.
    fn idle(&mut self, now: Instant) -> Result<bool, String> {
        LAG_FRAMES.store(0, Ordering::Relaxed);
        let detached_at = *self.shared.detached_at.lock().unwrap_or_else(|p| p.into_inner());
        if now.duration_since(detached_at) > IDLE_GRACE {
            return Ok(false);
        }
        if !self.flushed && self.silence >= MAX_SILENCE {
            self.conn.flush()?;
            self.flushed = true;
        }
        if !self.flushed && self.conn.head < self.now_ts(now) + LEAD {
            self.samples.fill(0.0);
            self.send_samples();
            self.silence += FRAMES_PER_PACKET as u64;
        } else {
            std::thread::sleep(Duration::from_millis(if self.flushed { 20 } else { 4 }));
        }
        self.periodic(now)?;
        Ok(true)
    }

    /// Envoie à l'appareil ce qu'il sait afficher.
    fn show(&mut self, text: bool, playing: u64) {
        let Some(info) = self.info.take() else { return };
        let start = self.track_start;
        if text && self.shows.text {
            self.conn.set_parameter("application/x-dmap-tagged", &info.text, start as u32);
        }
        if text && self.shows.artwork {
            if let Some(art) = &info.artwork {
                self.conn.set_parameter("image/jpeg", art, start as u32);
            }
        }
        if self.shows.progress {
            if let Some(d) = info.duration {
                let end = start + (d * SAMPLE_RATE as f64) as u64;
                let body = format!("progress: {}/{}/{}\r\n", start as u32, playing.max(start) as u32, end as u32);
                self.conn.set_parameter("text/parameters", body.as_bytes(), start as u32);
            }
        }
        self.info = Some(info);
    }

    fn send_samples(&mut self) {
        for (out, s) in self.pcm.iter_mut().zip(&self.samples) {
            *out = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        }
        alac::encode_stereo(&self.pcm, &mut self.payload);
        if let Some(c) = &self.conn.cipher {
            c.encrypt(&mut self.payload);
        }
        let marker = std::mem::take(&mut self.marker);
        let payload = std::mem::take(&mut self.payload);
        self.conn.send(&payload, marker);
        self.payload = payload;
    }
}

/// Recule la lecture à `heard` (lecture depuis la RAM seulement) : rien n'est perdu à la reprise.
fn rewind(track: &Track, heard: usize) {
    if track.shared.current_source.load(Ordering::Acquire) != 1 {
        return;
    }
    track.shared.pending_seek_frames.store(heard, Ordering::Release);
    track.shared.full_buffer_cursor.store(heard * CHANNELS as usize, Ordering::Release);
    track.atomics.current_position_frames.store(heard, Ordering::Release);
    track.shared.seek_flush.store(true, Ordering::Release);
}

// ─── Paquets ───

fn audio_packet(seq: u16, timestamp: u32, ssrc: u32, marker: bool, payload: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(12 + payload.len());
    p.push(0x80);
    p.push(if marker { 0xE0 } else { 0x60 });
    p.extend_from_slice(&seq.to_be_bytes());
    p.extend_from_slice(&timestamp.to_be_bytes());
    p.extend_from_slice(&ssrc.to_be_bytes());
    p.extend_from_slice(payload);
    p
}

fn sync_packet(now_ts: u64, ntp: [u8; 8], first: bool) -> [u8; 20] {
    let mut p = [0u8; 20];
    p[0] = if first { 0x90 } else { 0x80 };
    p[1] = 0xD4;
    p[3] = 0x07;
    p[4..8].copy_from_slice(&(now_ts.wrapping_sub(DECLARED_LATENCY) as u32).to_be_bytes());
    p[8..16].copy_from_slice(&ntp);
    p[16..20].copy_from_slice(&(now_ts as u32).to_be_bytes());
    p
}

/// Fiche DAAP du morceau (`mlit` : titre, artiste, album).
fn dmap_item(title: &str, artist: Option<&str>, album: Option<&str>) -> Vec<u8> {
    let mut inner = Vec::new();
    for (code, value) in [(b"minm", Some(title)), (b"asar", artist), (b"asal", album)] {
        if let Some(v) = value.filter(|v| !v.is_empty()) {
            dmap_tag(&mut inner, code, v.as_bytes());
        }
    }
    let mut out = Vec::with_capacity(inner.len() + 8);
    dmap_tag(&mut out, b"mlit", &inner);
    out
}

fn dmap_tag(out: &mut Vec<u8>, code: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(code);
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(data);
}

/// Annonce SDP : ALAC 16 bits / 44,1 kHz stéréo, trames de 352.
fn announce_sdp(session: u32, local_ip: &str, remote_ip: &str, cipher: Option<&Cipher>) -> Result<String, String> {
    let mut sdp = format!(
        "v=0\r\no=iTunes {session} 0 IN IP4 {local_ip}\r\ns=iTunes\r\nc=IN IP4 {remote_ip}\r\nt=0 0\r\n\
         m=audio 0 RTP/AVP 96\r\na=rtpmap:96 AppleLossless\r\n\
         a=fmtp:96 {FRAMES_PER_PACKET} 0 16 40 10 14 2 255 0 0 {SAMPLE_RATE}\r\n"
    );
    if let Some(c) = cipher {
        let b64 = base64::engine::general_purpose::STANDARD_NO_PAD;
        sdp.push_str(&format!("a=rsaaeskey:{}\r\na=aesiv:{}\r\n", b64.encode(c.wrapped_key()?), b64.encode(c.iv)));
    }
    Ok(sdp)
}

/// Horloge NTP (secondes depuis 1900, 32.32), monotone pendant la session.
fn ntp_now() -> [u8; 8] {
    static BASE: OnceLock<(Instant, Duration)> = OnceLock::new();
    let (i0, unix0) = BASE.get_or_init(|| (Instant::now(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()));
    let t = *unix0 + i0.elapsed();
    let secs = (t.as_secs() + 2_208_988_800) as u32 as u64;
    let frac = ((t.subsec_nanos() as u64) << 32) / 1_000_000_000;
    ((secs << 32) | frac).to_be_bytes()
}

/// Répond aux questions d'horloge de l'appareil.
fn spawn_timing(socket: UdpSocket, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buf = [0u8; 128];
        while !stop.load(Ordering::Acquire) {
            let Ok((n, from)) = socket.recv_from(&mut buf) else { continue };
            if n < 32 || buf[1] & 0x7F != 0x52 {
                continue;
            }
            let mut reply = [0u8; 32];
            reply[0] = 0x80;
            reply[1] = 0xD3;
            reply[3] = 0x07;
            reply[8..16].copy_from_slice(&buf[24..32]);
            reply[16..24].copy_from_slice(&ntp_now());
            reply[24..32].copy_from_slice(&ntp_now());
            let _ = socket.send_to(&reply, from);
        }
    })
}

/// Renvoie les paquets perdus que l'appareil réclame.
fn spawn_control(socket: Arc<UdpSocket>, history: Arc<Mutex<VecDeque<(u16, Vec<u8>)>>>, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buf = [0u8; 128];
        while !stop.load(Ordering::Acquire) {
            let Ok((n, from)) = socket.recv_from(&mut buf) else { continue };
            if n < 8 || buf[1] & 0x7F != 0x55 {
                continue;
            }
            let first = u16::from_be_bytes([buf[4], buf[5]]);
            let count = u16::from_be_bytes([buf[6], buf[7]]).min(128);
            let history = history.lock().unwrap_or_else(|p| p.into_inner());
            for i in 0..count {
                let seq = first.wrapping_add(i);
                if let Some((_, packet)) = history.iter().find(|(s, _)| *s == seq) {
                    let mut resend = Vec::with_capacity(4 + packet.len());
                    resend.extend_from_slice(&[0x80, 0xD6, 0x00, 0x01]);
                    resend.extend_from_slice(packet);
                    let _ = socket.send_to(&resend, from);
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heard_position_trails_what_was_sent() {
        let mut h = Heard::default();
        h.reset(1000);
        h.push(352, 1352);
        h.push(704, 1704);
        assert_eq!(h.at(100), 1000);
        assert_eq!(h.at(400), 1352);
        assert_eq!(h.at(10_000), 1704);
        h.reset(0);
        assert_eq!(h.at(10_000), 0);
    }

    #[test]
    fn builds_packets() {
        let s = sync_packet(100_000, [1, 2, 3, 4, 5, 6, 7, 8], true);
        assert_eq!(&s[..4], &[0x90, 0xD4, 0x00, 0x07]);
        assert_eq!(u32::from_be_bytes(s[4..8].try_into().unwrap()), (100_000 - DECLARED_LATENCY) as u32);
        assert_eq!(u32::from_be_bytes(s[16..20].try_into().unwrap()), 100_000);
        let a = audio_packet(7, 0xDEADBEEF, 1, true, &[9, 9]);
        assert_eq!(a, vec![0x80, 0xE0, 0, 7, 0xDE, 0xAD, 0xBE, 0xEF, 0, 0, 0, 1, 9, 9]);
        let sdp = announce_sdp(42, "10.0.0.2", "10.0.0.30", None).unwrap();
        assert!(sdp.contains("a=fmtp:96 352 0 16 40 10 14 2 255 0 0 44100\r\n"));
        assert!(!sdp.contains("rsaaeskey"));
        let item = dmap_item("Été", Some("AC/DC"), None);
        assert_eq!(&item[..8], &[b'm', b'l', b'i', b't', 0, 0, 0, 26]);
        assert_eq!(&item[8..16], &[b'm', b'i', b'n', b'm', 0, 0, 0, 5]);
        assert_eq!(&item[16..21], "Été".as_bytes());
        assert_eq!(&item[21..29], &[b'a', b's', b'a', b'r', 0, 0, 0, 5]);
        let c = Cipher::random().unwrap();
        assert!(announce_sdp(42, "10.0.0.2", "10.0.0.30", Some(&c)).unwrap().contains("a=aesiv:"));
    }
}
