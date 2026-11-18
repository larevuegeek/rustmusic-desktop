//! Backend WASAPI exclusive (Windows uniquement).
//!
//! Chaque piste est confiée au moteur exclusif persistant (`run_moteur_exclusif`) : le DAC
//! reste ouvert tant que le format et la sortie ne changent pas, ce qui évite 1,5 à 2,5 s
//! de réouverture à chaque morceau.

#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ringbuf::traits::Consumer;

use super::traits::{AudioOutput, AudioOutputError};
use super::types::{AudioBackend, PlaybackAtomics, SymphoniaSharedState};
use crate::core::audio_player::audio_output_wasapi::{
    default_output_device_name, run_exclusive_engine, try_negotiate_exclusive_format, TrackStream,
    NegotiatedFormat, WasapiSymphoniaState,
};

/// Le moteur vivant : son format, sa sortie, et la piste qu'on lui passe.
struct Engine {
    format: NegotiatedFormat,
    output: Option<String>,
    next: Arc<Mutex<Option<TrackStream>>>,
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

static ENGINE: Mutex<Option<Engine>> = Mutex::new(None);

fn engine_lock() -> std::sync::MutexGuard<'static, Option<Engine>> {
    ENGINE.lock().unwrap_or_else(|e| e.into_inner())
}

fn stop_engine(engine: Engine) {
    engine.stop.store(true, Ordering::SeqCst);
    if engine.thread.join().is_err() {
        log::warn!("WASAPI : le fil du moteur exclusif a paniqué");
    }
}

/// Rend le DAC : avant une lecture partagée, DSD ou sur une autre sortie.
pub fn close_engine() {
    let engine = engine_lock().take();
    if let Some(m) = engine {
        stop_engine(m);
    }
}

/// Confie une piste au moteur ; le relance seulement si le format ou la sortie change.
fn hand_over(format: NegotiatedFormat, output: Option<String>, stream: TrackStream) {
    let mut guard = engine_lock();
    if let Some(m) = guard.as_ref() {
        if !m.thread.is_finished() && m.format == format && m.output == output {
            *m.next.lock().unwrap_or_else(|e| e.into_inner()) = Some(stream);
            log::debug!("▶️  WASAPI exclusive : moteur réutilisé ({} Hz)", format.sample_rate);
            return;
        }
    }
    if let Some(old_path) = guard.take() {
        stop_engine(old_path);
    }
    let next = Arc::new(Mutex::new(Some(stream)));
    let stop = Arc::new(AtomicBool::new(false));
    let (p, a, s) = (next.clone(), stop.clone(), output.clone());
    let thread = std::thread::Builder::new()
        .name("rustmusic-wasapi-render".into())
        .spawn(move || {
            if let Err(e) = run_exclusive_engine(format, s, p, a) {
                log::error!("❌ WASAPI exclusive : {e}");
            }
        })
        .expect("fil du moteur WASAPI");
    *guard = Some(Engine { format, output, next, stop, thread });
}

/// Backend WASAPI exclusive pour la voie Symphonia (LiveDecode + FullBuffer).
pub struct WasapiExclusiveOutput {
    format: NegotiatedFormat,
    device_name: String,
    atomics: PlaybackAtomics,
    /// Levé par le moteur au premier tampon de cette piste.
    stream_ready: Arc<AtomicBool>,
    /// Fin de cette piste pour le moteur.
    end: Arc<AtomicBool>,
}

/// Délai maximal d'attente du premier tampon, pour ne pas figer l'interface.
const READY_TIMEOUT: Duration = Duration::from_secs(3);

/// Au-delà, on journalise le temps de réponse du DAC.
const SLOW_START: Duration = Duration::from_millis(500);

/// Rend le `Consumer` si la négociation échoue, pour un repli CPAL.
pub struct WasapiBuildError<C> {
    pub consumer: C,
    pub error: AudioOutputError,
}

impl WasapiExclusiveOutput {
    pub fn try_new<C>(
        source_sample_rate: u32,
        source_channels: u16,
        preferred_device_name: Option<String>,
        atomics: PlaybackAtomics,
        shared: SymphoniaSharedState,
        consumer: C,
    ) -> Result<Self, WasapiBuildError<C>>
    where
        C: Consumer<Item = f32> + Send + 'static,
    {
        let format = match try_negotiate_exclusive_format(source_sample_rate, source_channels, preferred_device_name.clone()) {
            Ok(f) => f,
            Err(e) => {
                return Err(WasapiBuildError { consumer, error: AudioOutputError::NegotiationFailed(e.to_string()) });
            }
        };

        let device_name = preferred_device_name
            .clone()
            .or_else(|| default_output_device_name().ok())
            .unwrap_or_else(|| "Device WASAPI inconnu".to_string());

        let stream_ready = Arc::new(AtomicBool::new(false));
        let end = Arc::new(AtomicBool::new(false));
        hand_over(format, preferred_device_name, TrackStream {
            source: Box::new(consumer),
            is_paused: atomics.is_paused.clone(),
            is_stopped: atomics.is_stopped.clone(),
            end: end.clone(),
            volume: atomics.volume.clone(),
            current_position_frames: atomics.current_position_frames.clone(),
            seek_flush: shared.seek_flush.clone(),
            sym: WasapiSymphoniaState {
                current_source: shared.current_source.clone(),
                full_buffer_data: shared.full_buffer_data.clone(),
                full_buffer_cursor: shared.full_buffer_cursor.clone(),
                is_full_buffer_ready: shared.is_full_buffer_ready.clone(),
                pending_seek_frames: shared.pending_seek_frames.clone(),
                output_channels: shared.output_channels,
            },
            stream_ready: stream_ready.clone(),
        });

        Ok(Self { format, device_name, atomics, stream_ready, end })
    }
}

impl AudioOutput for WasapiExclusiveOutput {
    /// Attend le premier tampon réellement écrit : la lecture ne commence pas avant.
    fn start(&mut self) -> Result<(), AudioOutputError> {
        let began = Instant::now();
        while !self.stream_ready.load(Ordering::Acquire) {
            if self.atomics.is_stopped.load(Ordering::Relaxed) {
                return Ok(());
            }
            if began.elapsed() >= READY_TIMEOUT {
                log::warn!("WASAPI exclusive : aucun tampon écrit après {} s, on continue sans attendre", READY_TIMEOUT.as_secs());
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let waited = began.elapsed();
        if waited >= SLOW_START {
            log::warn!("WASAPI exclusive : premier tampon après {} ms", waited.as_millis());
        } else {
            log::debug!("▶️  WASAPI exclusive : son effectif après {} ms", waited.as_millis());
        }
        Ok(())
    }

    fn output_sample_rate(&self) -> u32 {
        self.format.sample_rate
    }

    fn bits_exacts(&self) -> u32 {
        u32::from(self.format.bits_per_sample).min(24)
    }

    fn output_channels(&self) -> u16 {
        self.format.channels
    }

    fn device_name(&self) -> &str {
        &self.device_name
    }

    fn backend(&self) -> AudioBackend {
        AudioBackend::WasapiExclusive
    }
}

impl Drop for WasapiExclusiveOutput {
    /// La piste s'arrête ; le moteur garde le DAC ouvert pour la suivante.
    fn drop(&mut self) {
        self.end.store(true, Ordering::SeqCst);
        self.atomics.is_stopped.store(true, Ordering::SeqCst);
    }
}
