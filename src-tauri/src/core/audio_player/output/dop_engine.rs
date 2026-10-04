//! Moteur DoP persistant (gapless DSD natif) — Windows, Linux et macOS.
//!
//! # Problème résolu
//! Un DAC DSD se **mute** pendant qu'il acquiert le lock DSD (jusqu'à ~2-3 s).
//! Si chaque piste reconstruit le stream, le DAC déverrouille puis reverrouille
//! à CHAQUE morceau → délai + clic de bascule de mode entre pistes.
//!
//! Un seul stream vivant tant que les pistes DSD sont compatibles (rate, canaux, device) :
//! silence DoP entre elles, le DAC reste verrouillé, un seul warm-up.
//! Render persistant par OS (WASAPI, ALSA `hw:`, CoreAudio) ; un décodeur par piste.

#![cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ringbuf::traits::{Producer, Split};
use ringbuf::{HeapProd, HeapRb};

use crate::core::audio_decoder::dsd::dop_encoder::DopEncoder;
use crate::core::audio_decoder::dsd::dsd_container::DsdContainerReader;

/// Silence DoP émis à l'ouverture, le temps que le DAC verrouille le DSD.
pub const PRECHAUFFAGE_S: f64 = 2.5;

/// Pilotage du render persistant, partagé avec le moteur.
#[derive(Clone)]
pub struct DopRenderCtl {
    pub is_paused: Arc<AtomicBool>,
    /// Arrête le render → teardown du moteur (DAC déverrouille). PAS un
    /// changement de piste (celui-ci utilise `drain_request`).
    pub render_stop: Arc<AtomicBool>,
    /// Frames de musique jouées (position). Remis à 0 par le moteur au début
    /// de chaque piste ; le render l'incrémente au fil de la lecture.
    pub music_frames_played: Arc<AtomicUsize>,
    pub seek_flush: Arc<AtomicBool>,
    /// Jeter la queue du ring buffer (changement de piste) sans couper le stream.
    pub drain_request: Arc<AtomicBool>,
    /// `true` quand le warm-up (lock DSD) est fini et la musique démarre.
    pub audio_started: Arc<AtomicBool>,
    /// `true` dès que le stream a démarré (init réussi). Sert au moteur pour
    /// confirmer que `new()` a bien ouvert le device.
    pub started_ok: Arc<AtomicBool>,
}

/// Le DAC à ouvrir, selon l'OS. Utilisé seulement à la création d'un moteur.
pub enum CibleDop {
    #[cfg(target_os = "windows")]
    Wasapi { device_name: String },
    /// La réservation D-Bus vit avec le moteur : PipeWire ne reprend la carte qu'à sa fermeture.
    #[cfg(target_os = "linux")]
    Alsa {
        hw_id: String,
        reservation: Option<super::device_reservation::DeviceReservation>,
    },
    #[cfg(target_os = "macos")]
    CoreAudio { device: cpal::Device, device_id: u32 },
}

/// Instance globale unique du moteur DoP (persiste entre les pistes).
static ENGINE: Mutex<Option<DopEngine>> = Mutex::new(None);

/// Un moteur vivant peut-il jouer ce DSD tel quel ? Il tient alors le DAC :
/// inutile (et, sous Linux, impossible) de le sonder à nouveau.
#[cfg_attr(target_os = "windows", allow(dead_code))] // Windows sonde sans ouvrir le DAC
pub fn moteur_compatible(carrier_rate: u32, channels: u16, device_name: &str) -> bool {
    ENGINE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .is_some_and(|e| e.is_compatible(carrier_rate, channels, device_name))
}

/// Démarre une piste sur le moteur global : réutilise le moteur vivant s'il
/// est compatible (carrier + canaux + device), sinon en crée un nouveau sur
/// `cible` (après avoir arrêté l'ancien). Retourne `(handle, warmup_needed)` où
/// `warmup_needed = true` si un NOUVEAU moteur a été créé (→ warm-up + compteur
/// figé). `false` si réutilisation → démarrage instantané.
#[allow(clippy::too_many_arguments)]
pub fn begin_track_on_engine(
    carrier_rate: u32,
    channels: u16,
    device_name: String,
    cible: CibleDop,
    is_paused: Arc<AtomicBool>,
    decoder: Box<dyn DsdContainerReader + Send>,
    lsb_first: bool,
    seek_position: Arc<AtomicU64>,
    total_duration: Arc<AtomicU64>,
) -> Result<(DopTrackHandle, bool), String> {
    let mut guard = ENGINE.lock().unwrap_or_else(|e| e.into_inner());

    let reuse = guard
        .as_ref()
        .map(|e| e.is_compatible(carrier_rate, channels, &device_name))
        .unwrap_or(false);

    if !reuse {
        // Moteur absent ou incompatible → on ferme l'ancien et on en ouvre un neuf.
        if let Some(old) = guard.take() {
            old.stop();
        }
        let engine = DopEngine::new(carrier_rate, channels, device_name, cible, is_paused)?;
        *guard = Some(engine);
    }

    let engine = guard.as_mut().unwrap();
    let handle = engine.begin_track(decoder, lsb_first, seek_position, total_duration);
    Ok((handle, !reuse))
}

/// Fin d'une piste (naturelle OU via stop_play du changement de piste) : arrête
/// le décodeur courant mais GARDE le moteur vivant (le DAC reste locké, prêt
/// pour la piste suivante — c'est ce qui rend l'enchaînement gapless).
///
/// Programme un **watchdog** : si aucune nouvelle piste ne démarre dans les 5 s
/// (= vrai arrêt utilisateur, pas un changement de piste), on ferme le moteur
/// pour libérer le DAC (sinon il resterait locké en exclusif indéfiniment).
pub fn end_current_track_on_engine(drain_now: bool) {
    let gen = {
        let mut guard = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_mut() {
            Some(e) => {
                e.end_current_track(drain_now);
                Some(e.generation.load(Ordering::Relaxed))
            }
            None => None,
        }
    };

    if let Some(gen_at_end) = gen {
        std::thread::Builder::new()
            .name("rustmusic-dop-watchdog".into())
            .spawn(move || {
                std::thread::sleep(Duration::from_secs(5));
                let mut guard = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
                let idle = guard
                    .as_ref()
                    .map(|e| e.generation.load(Ordering::Relaxed) == gen_at_end)
                    .unwrap_or(false);
                if idle {
                    if let Some(e) = guard.take() {
                        log::info!("💤 Moteur DoP inactif 5 s → fermeture (DAC libéré)");
                        e.stop();
                    }
                }
            })
            .ok();
    }
}

/// Teardown complet du moteur (coupe le stream → le DAC déverrouille).
/// À appeler sur STOP utilisateur ou avant une lecture non-DSD/incompatible.
pub fn teardown_engine() {
    if let Some(e) = ENGINE.lock().unwrap_or_else(|e| e.into_inner()).take() {
        e.stop();
    }
}

/// Handle d'une piste en cours dans le moteur : ce que le thread appelant
/// observe pour piloter la position et détecter la fin.
pub struct DopTrackHandle {
    /// Frames de musique jouées (position). `secondes = frames / carrier_rate`.
    pub music_frames_played: Arc<AtomicUsize>,
    /// Passe à `true` quand le décodeur de CETTE piste a fini (EOF).
    pub decoder_done: Arc<AtomicBool>,
    /// Passe à `true` quand le warm-up DAC est fini (1re piste seulement ;
    /// déjà `true` pour les pistes suivantes → démarrage instantané).
    pub audio_started: Arc<AtomicBool>,
}

/// Moteur DoP persistant (instance unique dans `ENGINE`).
pub struct DopEngine {
    carrier_rate: u32,
    channels: u16,
    device_name: String,

    // Producer partagé : le décodeur courant y pousse.
    producer: Arc<Mutex<HeapProd<i32>>>,

    // Contrôle du render persistant.
    ctl: DopRenderCtl,
    render_handle: Option<JoinHandle<Result<(), String>>>,

    // Décodeur courant (remplacé à chaque piste).
    decoder_stop: Arc<AtomicBool>,
    decoder_handle: Option<JoinHandle<()>>,
    decoder_done: Arc<AtomicBool>,

    // Génération : incrémentée à chaque `begin_track`. Le watchdog d'inactivité
    // l'utilise pour savoir si une nouvelle piste a démarré depuis sa mise en
    // place (→ annuler le teardown).
    generation: Arc<AtomicUsize>,
}

impl DopEngine {
    /// Crée le moteur et démarre le render persistant (ouvre le stream).
    /// `is_paused` est partagé avec l'`AudioPlayer` (pause globale).
    /// Bloque brièvement (3 s max) le temps de confirmer l'ouverture du stream.
    pub fn new(
        carrier_rate: u32,
        channels: u16,
        device_name: String,
        cible: CibleDop,
        is_paused: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        // Ring ~4 s de musique (tient le warm-up + marge). Capacité paire en
        // trames entières : pushes et pops restent alignés sur les canaux.
        let capacity = (carrier_rate as usize * channels as usize * 4).max(1 << 16);
        let ring = HeapRb::<i32>::new(capacity);
        let (producer, consumer) = ring.split();
        let producer = Arc::new(Mutex::new(producer));

        let ctl = DopRenderCtl {
            is_paused,
            render_stop: Arc::new(AtomicBool::new(false)),
            music_frames_played: Arc::new(AtomicUsize::new(0)),
            seek_flush: Arc::new(AtomicBool::new(false)),
            drain_request: Arc::new(AtomicBool::new(false)),
            audio_started: Arc::new(AtomicBool::new(false)),
            started_ok: Arc::new(AtomicBool::new(false)),
        };

        let render_ctl = ctl.clone();
        let render_handle = std::thread::Builder::new()
            .name("rustmusic-dop-render".into())
            .spawn(move || match cible {
                #[cfg(target_os = "windows")]
                CibleDop::Wasapi { device_name } => {
                    crate::core::audio_player::audio_output_wasapi::run_wasapi_dop_playback(
                        carrier_rate,
                        channels,
                        consumer,
                        Some(device_name),
                        render_ctl,
                    )
                    .map_err(|e| e.to_string())
                }
                #[cfg(target_os = "linux")]
                CibleDop::Alsa { hw_id, reservation } => {
                    // Gardée jusqu'au retour du render (stream fermé), puis relâchée.
                    let _reservation = reservation;
                    super::dop_alsa::run_alsa_dop_render(&hw_id, carrier_rate, channels, consumer, render_ctl)
                }
                #[cfg(target_os = "macos")]
                CibleDop::CoreAudio { device, device_id } => {
                    super::dop_coreaudio::run_coreaudio_dop_render(
                        &device,
                        device_id,
                        carrier_rate,
                        channels,
                        consumer,
                        render_ctl,
                    )
                }
            })
            .map_err(|e| format!("spawn render DoP: {e}"))?;

        // Attendre l'ouverture du stream (started_ok) ou l'échec (thread fini).
        let start = Instant::now();
        loop {
            if ctl.started_ok.load(Ordering::Relaxed) {
                break;
            }
            if render_handle.is_finished() {
                // Le render est sorti avant de démarrer → erreur d'init.
                let msg = match render_handle.join() {
                    Ok(Err(e)) => e,
                    _ => "render DoP terminé avant start".to_string(),
                };
                return Err(msg);
            }
            if start.elapsed() > Duration::from_secs(3) {
                ctl.render_stop.store(true, Ordering::Relaxed);
                return Err("timeout ouverture stream DoP".to_string());
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        Ok(Self {
            carrier_rate,
            channels,
            device_name,
            producer,
            ctl,
            render_handle: Some(render_handle),
            decoder_stop: Arc::new(AtomicBool::new(false)),
            decoder_handle: None,
            decoder_done: Arc::new(AtomicBool::new(true)),
            generation: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Le moteur peut-il jouer cette piste sans reconstruire le stream ?
    pub fn is_compatible(&self, carrier_rate: u32, channels: u16, device_name: &str) -> bool {
        self.carrier_rate == carrier_rate
            && self.channels == channels
            && self.device_name == device_name
    }

    /// Démarre une nouvelle piste sur le moteur vivant (le DAC reste locké).
    /// Arrête proprement le décodeur précédent, draine la queue, remet la
    /// position à 0, puis lance le décodeur de la nouvelle piste.
    pub fn begin_track(
        &mut self,
        decoder: Box<dyn DsdContainerReader + Send>,
        lsb_first: bool,
        seek_position: Arc<AtomicU64>,
        total_duration: Arc<AtomicU64>,
    ) -> DopTrackHandle {
        // 1. Stopper le décodeur précédent.
        self.decoder_stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.decoder_handle.take() {
            let _ = h.join();
        }

        // 2. Drainer la queue de la piste précédente (le render jette le buffer,
        //    warm-up compris : sinon le début de la nouvelle piste serait jeté).
        self.ctl.drain_request.store(true, Ordering::Release);
        // Laisser le render traiter le drain (~2 cycles).
        std::thread::sleep(Duration::from_millis(45));

        // 3. Reset position + flags pour la nouvelle piste.
        self.ctl.music_frames_played.store(0, Ordering::Relaxed);
        self.ctl.seek_flush.store(false, Ordering::Relaxed);
        self.decoder_stop = Arc::new(AtomicBool::new(false));
        self.decoder_done = Arc::new(AtomicBool::new(false));
        self.generation.fetch_add(1, Ordering::Relaxed);
        seek_position.store(u64::MAX, Ordering::Relaxed);

        // 4. Spawn du décodeur de la nouvelle piste (pousse dans le producer partagé).
        let carrier = self.carrier_rate;
        let producer = self.producer.clone();
        let decoder_stop = self.decoder_stop.clone();
        let decoder_done = self.decoder_done.clone();
        let music_frames = self.ctl.music_frames_played.clone();
        let seek_flush = self.ctl.seek_flush.clone();
        let handle = std::thread::Builder::new()
            .name("rustmusic-dop-decode".into())
            .spawn(move || {
                dop_decode_loop(
                    decoder,
                    lsb_first,
                    carrier,
                    producer,
                    decoder_stop,
                    seek_position,
                    music_frames,
                    seek_flush,
                    total_duration,
                    decoder_done,
                );
            })
            .expect("spawn décodeur DoP");
        self.decoder_handle = Some(handle);

        DopTrackHandle {
            music_frames_played: self.ctl.music_frames_played.clone(),
            decoder_done: self.decoder_done.clone(),
            audio_started: self.ctl.audio_started.clone(),
        }
    }

    /// Arrête proprement le décodeur courant (fin de piste) SANS couper le
    /// stream : le render se met à écrire du silence (DAC reste locké).
    ///
    /// `drain_now = true` (stop utilisateur) : on JETTE immédiatement la musique
    /// déjà bufferisée (jusqu'à ~4 s) → l'audio se tait tout de suite. `false`
    /// (fin naturelle) : on laisse la petite queue se jouer (transition douce).
    pub fn end_current_track(&mut self, drain_now: bool) {
        if drain_now {
            self.ctl.drain_request.store(true, Ordering::Release);
        }
        self.decoder_stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.decoder_handle.take() {
            let _ = h.join();
        }
    }

    /// Teardown complet : coupe le stream (le DAC déverrouille le DSD).
    pub fn stop(mut self) {
        self.decoder_stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.decoder_handle.take() {
            let _ = h.join();
        }
        self.ctl.render_stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.render_handle.take() {
            let _ = h.join();
        }
        log::info!("⏹  Moteur DoP arrêté (stream fermé)");
    }
}

/// Boucle de décodage DSD → DoP poussant dans le producer partagé du moteur.
#[allow(clippy::too_many_arguments)]
fn dop_decode_loop(
    mut decoder: Box<dyn DsdContainerReader + Send>,
    lsb_first: bool,
    carrier_rate: u32,
    producer: Arc<Mutex<HeapProd<i32>>>,
    decoder_stop: Arc<AtomicBool>,
    seek_position: Arc<AtomicU64>,
    music_frames_played: Arc<AtomicUsize>,
    seek_flush: Arc<AtomicBool>,
    total_duration: Arc<AtomicU64>,
    decoder_done: Arc<AtomicBool>,
) {
    let channels = decoder.channel_count();
    let mut encoder = DopEncoder::new(channels, lsb_first);

    loop {
        if decoder_stop.load(Ordering::Relaxed) {
            break;
        }

        // Seek.
        let seek_bits = seek_position.load(Ordering::Relaxed);
        if seek_bits != u64::MAX {
            seek_position.store(u64::MAX, Ordering::Relaxed);
            let total = f64::from_bits(total_duration.load(Ordering::Relaxed));
            let secs = f64::from_bits(seek_bits).max(0.0).min(total.max(0.0));
            if let Ok(actual) = decoder.seek_to_seconds(secs) {
                encoder.reset();
                music_frames_played.store((actual * carrier_rate as f64) as usize, Ordering::Relaxed);
                seek_flush.store(true, Ordering::Relaxed);
                let t = Instant::now();
                while seek_flush.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(1));
                    if t.elapsed().as_millis() > 80 {
                        seek_flush.store(false, Ordering::Relaxed);
                        break;
                    }
                }
            }
            continue;
        }

        // Lire + encoder + pousser.
        let blocks = match decoder.read_next_blocks() {
            Ok(Some(b)) => b,
            Ok(None) => break, // EOF
            Err(e) => {
                log::error!("DoP décodage: {:?}", e);
                break;
            }
        };
        let samples = encoder.encode_blocks(&blocks);
        if samples.is_empty() {
            continue;
        }

        let mut off = 0usize;
        while off < samples.len() {
            if decoder_stop.load(Ordering::Relaxed) {
                break;
            }
            if seek_position.load(Ordering::Relaxed) != u64::MAX {
                break;
            }
            let written = {
                let mut p = producer.lock().unwrap_or_else(|e| e.into_inner());
                p.push_slice(&samples[off..])
            };
            off += written;
            if written == 0 {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    decoder_done.store(true, Ordering::Relaxed);
    log::debug!("🧵 [DoP] Décodeur de piste terminé");
}
