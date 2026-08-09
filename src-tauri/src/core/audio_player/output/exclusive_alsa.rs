//! Sortie PCM bit-perfect via ALSA `hw:` — Linux uniquement.
//!
//! # Pourquoi un backend dédié plutôt que CPAL
//! Sur Linux, CPAL passe par le device ALSA `default`, c'est-à-dire PipeWire ou
//! PulseAudio : le flux est mixé avec les autres applications et souvent
//! rééchantillonné vers le rate du serveur son. Un FLAC 24/192 peut donc
//! arriver au DAC en 48 kHz sans que rien ne le signale.
//!
//! Ce backend ouvre le périphérique **matériel** `hw:X,0` en direct, avec
//! `set_rate_resample(false)` et le rate source exact : c'est l'équivalent
//! Linux du WASAPI exclusive de Windows. Il réutilise les mêmes briques que le
//! DoP ALSA (réservation D-Bus, ouverture avec réessai sur EBUSY).
//!
//! # Négociation de format
//! On tente `S32_LE` puis `S16_LE`. Le 24 bits packé (`S24_3LE`) est
//! volontairement écarté : quasiment tous les DAC USB acceptent S32_LE, dans
//! lequel un mot 24 bits se loge sans perte, et le packing 3 octets est une
//! source d'erreurs sans bénéfice audible.
//!
//! Le rate, lui, n'est **jamais** négocié à la baisse : si le DAC ne sait pas
//! jouer le rate source, on refuse le chemin exclusif et l'appelant retombe sur
//! CPAL partagé. Mieux vaut un rééchantillonnage assumé par le serveur son
//! qu'un « bit-perfect » qui n'en serait pas un.
//!
//! ⚠️ Comme sur les autres chemins PCM, le volume logiciel et le Replay Gain
//! modifient les échantillons : bit-perfect strict = volume à 100 % et Replay
//! Gain désactivé.

#![cfg(target_os = "linux")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use alsa::pcm::{Access, Format, HwParams, PCM};
use alsa::{Direction, ValueOr};
use ringbuf::traits::{Consumer, Observer};

use super::device_reservation::{card_index_from_hw_id, DeviceReservation};
use super::traits::{AudioOutput, AudioOutputError};
use super::types::{AudioBackend, PlaybackAtomics, SymphoniaSharedState};

/// Formats tentés, dans l'ordre de préférence.
const CANDIDATE_FORMATS: [Format; 2] = [Format::S32LE, Format::S16LE];

/// Ouvre le PCM `hw:` en réessayant sur EBUSY : après une réservation D-Bus,
/// PipeWire ferme le device de façon asynchrone (jusqu'à quelques centaines de
/// ms). Même stratégie que le DoP ALSA.
fn open_hw_pcm(hw_id: &str) -> Result<PCM, String> {
    let mut last = String::new();
    for _ in 0..25 {
        match PCM::new(hw_id, Direction::Playback, false) {
            Ok(pcm) => return Ok(pcm),
            Err(e) => {
                let busy = e.errno() == 16 || e.errno() == -16; // EBUSY
                last = e.to_string();
                if busy {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                return Err(last);
            }
        }
    }
    Err(format!("device toujours occupé après attente : {last}"))
}

/// Applique les `HwParams` PCM et renvoie la taille de période obtenue.
///
/// Échoue si le rate obtenu diffère du rate demandé : c'est le garde-fou qui
/// distingue une vraie sortie bit-perfect d'un rééchantillonnage silencieux.
fn configure(pcm: &PCM, format: Format, rate: u32, channels: u16) -> Result<usize, String> {
    let hwp = HwParams::any(pcm).map_err(|e| format!("hw_params any: {e}"))?;
    hwp.set_access(Access::RWInterleaved)
        .map_err(|e| format!("set_access: {e}"))?;
    hwp.set_format(format)
        .map_err(|e| format!("set_format {format:?}: {e}"))?;
    hwp.set_channels(channels as u32)
        .map_err(|e| format!("set_channels {channels}: {e}"))?;
    // CRUCIAL : interdit le rééchantillonnage par la couche ALSA.
    hwp.set_rate_resample(false)
        .map_err(|e| format!("set_rate_resample(false): {e}"))?;
    hwp.set_rate(rate, ValueOr::Nearest)
        .map_err(|e| format!("set_rate {rate}: {e}"))?;
    // ~120 ms de buffer, périodes ~24 ms : un peu plus large que le DoP car le
    // décodeur PCM peut hoqueter sur un seek ou un fichier réseau.
    let _ = hwp.set_buffer_time_near(120_000, ValueOr::Nearest);
    let _ = hwp.set_period_time_near(24_000, ValueOr::Nearest);
    pcm.hw_params(&hwp)
        .map_err(|e| format!("apply hw_params: {e}"))?;

    let current = pcm
        .hw_params_current()
        .map_err(|e| format!("hw_params_current: {e}"))?;
    let got_rate = current.get_rate().map_err(|e| format!("get_rate: {e}"))?;
    if got_rate != rate {
        return Err(format!("rate obtenu {got_rate} ≠ demandé {rate}"));
    }
    let period = current
        .get_period_size()
        .map_err(|e| format!("get_period_size: {e}"))? as usize;

    Ok(period.max(64))
}

/// Résultat d'une négociation réussie.
#[derive(Debug, Clone, Copy)]
pub struct AlsaNegotiation {
    pub format: Format,
    pub period_frames: usize,
}

/// Le DAC accepte-t-il ce rate en accès exclusif, et dans quel format ?
///
/// Ouvre puis referme le `hw:` — appelé **avant** que le `Consumer` du ring
/// buffer ne soit consommé, pour que le repli sur CPAL reste possible.
pub fn probe(hw_id: &str, rate: u32, channels: u16) -> Option<AlsaNegotiation> {
    // La réservation ne vit que le temps du probe ; le thread de rendu la
    // reprendra. Fenêtre de re-capture par PipeWire couverte par le réessai
    // EBUSY de `open_hw_pcm`.
    let reservation = card_index_from_hw_id(hw_id).and_then(|idx| {
        DeviceReservation::acquire(idx)
            .map_err(|e| log::debug!("🔒 Réservation carte {idx} impossible : {e}"))
            .ok()
    });

    let pcm = match open_hw_pcm(hw_id) {
        Ok(pcm) => pcm,
        Err(e) => {
            log::info!("🎚️  ALSA exclusive : ouverture de {hw_id} impossible ({e}) → CPAL partagé");
            drop(reservation);
            return None;
        }
    };

    for format in CANDIDATE_FORMATS {
        match configure(&pcm, format, rate, channels) {
            Ok(period_frames) => {
                log::info!(
                    "🎚️  ALSA exclusive : {hw_id} accepte {rate} Hz / {channels} ch en {format:?} \
                     (période {period_frames} frames)"
                );
                drop(pcm);
                drop(reservation);
                return Some(AlsaNegotiation {
                    format,
                    period_frames,
                });
            }
            Err(e) => log::debug!("🎚️  ALSA exclusive : {format:?} refusé sur {hw_id} — {e}"),
        }
    }

    log::info!(
        "🎚️  ALSA exclusive : aucun format accepté à {rate} Hz sur {hw_id} → CPAL partagé"
    );
    drop(pcm);
    drop(reservation);
    None
}

/// Sortie ALSA exclusive : un thread de rendu bloquant sur `writei`.
pub struct AlsaExclusiveOutput {
    handle: Option<JoinHandle<()>>,
    /// Armé au `Drop` pour que le thread sorte de sa boucle.
    is_stopped: Arc<AtomicBool>,
    /// Mis à `true` par le thread une fois le stream réellement ouvert.
    started: Arc<AtomicBool>,
    device_name: String,
    sample_rate: u32,
    channels: u16,
}

impl AlsaExclusiveOutput {
    /// Démarre le thread de rendu. La négociation ayant déjà été validée par
    /// [`probe`], l'échec d'ouverture ici est très improbable (carte reprise
    /// entre-temps) et se traduit par un silence journalisé, pas par un crash.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new<C>(
        hw_id: String,
        device_name: String,
        negotiation: AlsaNegotiation,
        rate: u32,
        channels: u16,
        consumer: C,
        atomics: PlaybackAtomics,
        shared: SymphoniaSharedState,
    ) -> Result<Self, AudioOutputError>
    where
        C: Consumer<Item = f32> + Send + 'static,
    {
        let is_stopped = atomics.is_stopped.clone();
        let started = Arc::new(AtomicBool::new(false));
        let started_thread = started.clone();

        let handle = std::thread::Builder::new()
            .name("rustmusic-alsa-render".into())
            .spawn(move || {
                if let Err(e) = render_loop(
                    &hw_id,
                    negotiation,
                    rate,
                    channels,
                    consumer,
                    atomics,
                    shared,
                    started_thread,
                ) {
                    log::error!("❌ [ALSA exclusive] {e}");
                }
            })
            .map_err(|e| AudioOutputError::BuildFailed(format!("spawn thread ALSA: {e}")))?;

        Ok(Self {
            handle: Some(handle),
            is_stopped,
            started,
            device_name,
            sample_rate: rate,
            channels,
        })
    }
}

impl AudioOutput for AlsaExclusiveOutput {
    fn start(&mut self) -> Result<(), AudioOutputError> {
        // Le thread s'ouvre et joue tout seul ; on attend juste la confirmation
        // que le stream est bien ouvert pour que l'appelant ne croie pas la
        // lecture démarrée alors que la carte a été reprise entre-temps.
        for _ in 0..30 {
            if self.started.load(Ordering::Relaxed) {
                return Ok(());
            }
            if self.is_stopped.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        if self.started.load(Ordering::Relaxed) {
            Ok(())
        } else {
            Err(AudioOutputError::StartFailed(
                "le thread ALSA n'a pas confirmé l'ouverture du stream".into(),
            ))
        }
    }

    fn output_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn output_channels(&self) -> u16 {
        self.channels
    }

    fn device_name(&self) -> &str {
        &self.device_name
    }

    fn backend(&self) -> AudioBackend {
        AudioBackend::AlsaExclusive
    }
}

impl Drop for AlsaExclusiveOutput {
    fn drop(&mut self) {
        // Le thread détient la réservation D-Bus et le `hw:` : il FAUT le
        // joindre avant de rendre la main, sinon la piste suivante trouve la
        // carte encore occupée.
        self.is_stopped.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Boucle de rendu : lit le ring buffer (ou le FullBuffer), applique volume et
/// Replay Gain, convertit vers le format négocié, écrit au DAC.
#[allow(clippy::too_many_arguments)]
fn render_loop<C>(
    hw_id: &str,
    negotiation: AlsaNegotiation,
    rate: u32,
    channels: u16,
    mut consumer: C,
    atomics: PlaybackAtomics,
    shared: SymphoniaSharedState,
    started: Arc<AtomicBool>,
) -> Result<(), String>
where
    C: Consumer<Item = f32> + Send + 'static,
{
    let ch = channels.max(1) as usize;

    // La réservation vit aussi longtemps que la boucle : PipeWire ne reprend la
    // carte qu'au retour de cette fonction.
    let _reservation = card_index_from_hw_id(hw_id).and_then(|idx| {
        DeviceReservation::acquire(idx)
            .map_err(|e| log::warn!("🔒 Réservation carte {idx} refusée : {e}"))
            .ok()
    });

    let pcm = open_hw_pcm(hw_id).map_err(|e| format!("ouverture {hw_id}: {e}"))?;
    let period_frames = configure(&pcm, negotiation.format, rate, channels)?;
    pcm.prepare().map_err(|e| format!("prepare: {e}"))?;

    log::info!(
        "🎚️  Stream ALSA exclusive ouvert : {hw_id} @ {rate} Hz / {channels} ch \
         ({:?}, bit-perfect)",
        negotiation.format
    );
    started.store(true, Ordering::Relaxed);

    let output_channels = shared.output_channels.max(1);
    let chunk_samples = period_frames * ch;
    let mut scratch: Vec<f32> = vec![0.0; chunk_samples];
    let mut buf_i32: Vec<i32> = Vec::with_capacity(chunk_samples);
    let mut buf_i16: Vec<i16> = Vec::with_capacity(chunk_samples);
    let mut fade_in_samples: usize = 0;

    loop {
        if atomics.is_stopped.load(Ordering::Relaxed) {
            break;
        }

        // ─── Seek : drainer et repositionner, exactement comme le chemin CPAL ───
        if shared.seek_flush.load(Ordering::Acquire) {
            let to_skip = consumer.occupied_len();
            consumer.skip(to_skip);

            let target = shared.pending_seek_frames.load(Ordering::Acquire);
            let new_frames = if target != usize::MAX {
                shared.pending_seek_frames.store(usize::MAX, Ordering::Relaxed);
                atomics.current_position_frames.store(target, Ordering::Relaxed);
                target
            } else {
                atomics.current_position_frames.load(Ordering::Relaxed)
            };
            shared
                .full_buffer_cursor
                .store(new_frames * output_channels as usize, Ordering::Relaxed);

            shared.seek_flush.store(false, Ordering::Release);
            fade_in_samples = 2048;
            let _ = pcm.prepare();
            continue;
        }

        // ─── Pause : silence, le stream reste ouvert (reprise instantanée) ───
        if atomics.is_paused.load(Ordering::Relaxed) {
            scratch.fill(0.0);
            write_chunk(
                &pcm,
                negotiation.format,
                &scratch,
                &mut buf_i32,
                &mut buf_i16,
                ch,
            )?;
            continue;
        }

        // ─── Remplissage depuis la source active ───
        let mut samples_read = 0usize;
        let source = shared.current_source.load(Ordering::Relaxed);

        if source == 0 {
            samples_read = consumer.pop_slice(&mut scratch);
        } else if shared.is_full_buffer_ready.load(Ordering::Relaxed) {
            if let Ok(fb) = shared.full_buffer_data.read() {
                let cursor = shared.full_buffer_cursor.load(Ordering::Relaxed);
                let available = fb.len().saturating_sub(cursor);
                samples_read = available.min(scratch.len());
                if samples_read > 0 {
                    scratch[..samples_read]
                        .copy_from_slice(&fb[cursor..cursor + samples_read]);
                    shared
                        .full_buffer_cursor
                        .fetch_add(samples_read, Ordering::Relaxed);
                }
            }
        }
        if samples_read < scratch.len() {
            scratch[samples_read..].fill(0.0);
        }

        // ─── Position ───
        if samples_read > 0 && !shared.seek_flush.load(Ordering::Relaxed) {
            if source == 1 {
                let cursor = shared.full_buffer_cursor.load(Ordering::Relaxed);
                atomics
                    .current_position_frames
                    .store(cursor / output_channels as usize, Ordering::Relaxed);
            } else {
                atomics
                    .current_position_frames
                    .fetch_add(samples_read / output_channels as usize, Ordering::Relaxed);
            }
        }

        // ─── Volume + Replay Gain + fade-in post-seek + clipping ───
        let vol = atomics.volume.load(Ordering::Relaxed) as f32 / 100.0
            * crate::core::audio_player::replay_gain::current_factor();
        for s in scratch.iter_mut() {
            let fade = if fade_in_samples > 0 {
                fade_in_samples -= 1;
                (2048 - fade_in_samples) as f32 / 2048.0
            } else {
                1.0
            };
            *s = (*s * vol * fade * 0.98).clamp(-1.0, 1.0);
        }

        write_chunk(
            &pcm,
            negotiation.format,
            &scratch,
            &mut buf_i32,
            &mut buf_i16,
            ch,
        )?;
    }

    // Laisse le DAC vider ce qui reste plutôt que de couper net.
    let _ = pcm.drain();
    log::debug!("🎚️  Stream ALSA exclusive fermé ({hw_id})");
    Ok(())
}

/// Convertit `samples` (f32 interleavé) vers le format négocié et l'écrit,
/// en récupérant les underruns.
fn write_chunk(
    pcm: &PCM,
    format: Format,
    samples: &[f32],
    buf_i32: &mut Vec<i32>,
    buf_i16: &mut Vec<i16>,
    ch: usize,
) -> Result<(), String> {
    match format {
        Format::S32LE => {
            buf_i32.clear();
            buf_i32.extend(
                samples
                    .iter()
                    // 2^31-1 : même mise à l'échelle que le chemin WASAPI.
                    .map(|s| (*s * 2_147_483_647.0).clamp(-2_147_483_648.0, 2_147_483_647.0) as i32),
            );
            let io = pcm.io_i32().map_err(|e| format!("io_i32: {e}"))?;
            let mut off = 0usize;
            while off < buf_i32.len() {
                match io.writei(&buf_i32[off..]) {
                    Ok(frames) => off += frames * ch,
                    Err(e) => {
                        if pcm.recover(e.errno(), true).is_err() {
                            return Err(format!("writei S32: {e}"));
                        }
                    }
                }
            }
        }
        _ => {
            buf_i16.clear();
            buf_i16.extend(
                samples
                    .iter()
                    .map(|s| (*s * 32_767.0).clamp(-32_768.0, 32_767.0) as i16),
            );
            let io = pcm.io_i16().map_err(|e| format!("io_i16: {e}"))?;
            let mut off = 0usize;
            while off < buf_i16.len() {
                match io.writei(&buf_i16[off..]) {
                    Ok(frames) => off += frames * ch,
                    Err(e) => {
                        if pcm.recover(e.errno(), true).is_err() {
                            return Err(format!("writei S16: {e}"));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
