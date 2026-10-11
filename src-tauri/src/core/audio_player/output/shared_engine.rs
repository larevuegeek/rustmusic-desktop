//! Moteur partagé persistant : un seul flux CPAL reste ouvert d'une piste à l'autre.
//!
//! Ouvrir un flux coûte jusqu'à 1,6 s sur certains DAC USB (Fosi K7). Chaque piste
//! branche ici sa fonction de rendu ; entre deux pistes, le flux joue du silence.
//! Il est rouvert si la sortie ou le format change, s'il est invalidé, et fermé
//! après un temps d'inactivité ou avant une sortie exclusive.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, StreamTrait};

use super::traits::AudioOutputError;

/// Rendu d'une piste : remplit le tampon demandé par la carte.
pub type RenderFn = Box<dyn FnMut(&mut [f32]) + Send>;

/// Sans piste branchée pendant ce délai, la sortie est rendue.
const IDLE_GRACE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq)]
struct EngineKey {
    device: String,
    config: String,
}

type Slot = Arc<Mutex<Option<(u64, RenderFn)>>>;

struct Engine {
    key: EngineKey,
    slot: Slot,
    broken: Arc<AtomicBool>,
    _stream: cpal::Stream,
}

static ENGINE: Mutex<Option<Engine>> = Mutex::new(None);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn engine() -> std::sync::MutexGuard<'static, Option<Engine>> {
    ENGINE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Branche une piste ; ne rouvre le flux que si la sortie ou le format a changé.
/// Rend l'identifiant de la piste, à passer à [`detach`].
pub fn attach(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    device_name: &str,
    render: RenderFn,
) -> Result<u64, AudioOutputError> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let key = EngineKey { device: device_name.to_string(), config: format!("{config:?}") };
    let mut guard = engine();

    if let Some(e) = guard.as_ref() {
        if e.key == key && !e.broken.load(Ordering::Acquire) {
            // L'ancienne piste est libérée hors du verrou du callback.
            let old = e.slot.lock().unwrap_or_else(|p| p.into_inner()).replace((id, render));
            drop(old);
            log::debug!("▶️  Sortie partagée réutilisée ({device_name})");
            return Ok(id);
        }
    }
    // Sortie ou format différents, ou flux invalidé : on repart d'un flux neuf.
    *guard = None;

    let slot: Slot = Arc::new(Mutex::new(Some((id, render))));
    let broken = Arc::new(AtomicBool::new(false));
    let (cb_slot, cb_broken) = (slot.clone(), broken.clone());
    let errors = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let stream = device
        .build_output_stream(
            config.clone(),
            move |output: &mut [f32], _| {
                // `try_lock` : le callback n'attend jamais ; au pire un tampon de silence au changement de piste.
                match cb_slot.try_lock() {
                    Ok(mut g) => match g.as_mut() {
                        Some((_, render)) => render(output),
                        None => output.fill(0.0),
                    },
                    Err(_) => output.fill(0.0),
                }
            },
            move |err: cpal::Error| {
                use cpal::ErrorKind;
                if !matches!(err.kind(), ErrorKind::Xrun | ErrorKind::DeviceChanged) {
                    cb_broken.store(true, Ordering::Release);
                }
                // 1re erreur, puis 1 sur 100, puis 1 sur 1000 (rafales de sous-débits sur VM).
                let n = errors.fetch_add(1, Ordering::Relaxed);
                if n == 0 || (n < 1000 && n % 100 == 0) || n % 1000 == 0 {
                    log::error!("❌ Erreur audio (#{}): {:?}", n + 1, err);
                }
            },
            None,
        )
        .map_err(|e| AudioOutputError::BuildFailed(format!("build_output_stream: {e}")))?;
    stream
        .play()
        .map_err(|e| AudioOutputError::StartFailed(format!("stream.play: {e}")))?;

    log::debug!("▶️  Sortie partagée ouverte ({device_name})");
    *guard = Some(Engine { key, slot, broken, _stream: stream });
    Ok(id)
}

/// Débranche la piste si c'est encore elle ; la sortie se ferme après [`IDLE_GRACE`] sans relève.
pub fn detach(id: u64) {
    let old = {
        let guard = engine();
        let Some(e) = guard.as_ref() else { return };
        let mut slot = e.slot.lock().unwrap_or_else(|p| p.into_inner());
        if slot.as_ref().map(|(i, _)| *i) != Some(id) {
            return;
        }
        slot.take()
    };
    drop(old);

    std::thread::spawn(move || {
        std::thread::sleep(IDLE_GRACE);
        let mut guard = engine();
        let idle = guard.as_ref().is_some_and(|e| {
            e.slot.lock().unwrap_or_else(|p| p.into_inner()).is_none()
                && NEXT_ID.load(Ordering::Relaxed) == id + 1
        });
        if idle {
            *guard = None;
            log::debug!("⏹  Sortie partagée rendue après inactivité");
        }
    });
}

/// Rend la sortie tout de suite : avant une sortie exclusive ou DSD sur le même DAC.
pub fn close() {
    if engine().take().is_some() {
        log::debug!("⏹  Sortie partagée fermée");
    }
}
