//! Moteur partagé persistant : un seul flux CPAL reste ouvert d'une piste à l'autre.
//!
//! Ouvrir un flux coûte jusqu'à 1,6 s sur certains DAC USB (Fosi K7). Chaque piste
//! branche ici sa fonction de rendu ; entre deux pistes, le flux joue du silence.
//! Il est rouvert si la sortie ou le format change, et fermé après un temps
//! d'inactivité ou avant une sortie exclusive. Invalidé en pleine lecture (DAC USB
//! réinitialisé…), il est rouvert et la piste reprend où elle était : sur la même
//! sortie si elle revient, sinon sur celle du système, sinon la lecture s'arrête.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tauri::{AppHandle, Emitter};

use super::traits::AudioOutputError;

/// Rendu d'une piste : remplit le tampon demandé par la carte.
pub type RenderFn = Box<dyn FnMut(&mut [f32]) + Send>;

/// Sans piste branchée pendant ce délai, la sortie est rendue.
const IDLE_GRACE: Duration = Duration::from_secs(30);
/// Flux invalidé : un essai de réouverture toutes les 500 ms, pendant 15 s au plus.
const RECOVER_EVERY: Duration = Duration::from_millis(500);
const RECOVER_TRIES: u32 = 30;
/// Essais réservés à la même sortie (5 s) avant d'accepter celle du système.
const SAME_DEVICE_TRIES: u32 = 10;

static APP: OnceLock<AppHandle> = OnceLock::new();

/// Pour prévenir l'interface d'une sortie perdue.
pub fn set_app(app: AppHandle) {
    let _ = APP.set(app);
}

/// Sortie perdue : `fallback` est la sortie qui a pris le relais, `None` si la lecture s'arrête.
#[derive(Clone, serde::Serialize)]
struct OutputLost {
    device: String,
    fallback: Option<String>,
}

fn notify(device: &str, fallback: Option<String>) {
    if let Some(app) = APP.get() {
        let _ = app.emit("audio-output-lost", OutputLost { device: device.to_string(), fallback });
    }
}

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
    /// Change à chaque flux ouvert : une reprise visant un flux remplacé s'arrête.
    generation: u64,
    _stream: cpal::Stream,
}

static ENGINE: Mutex<Option<Engine>> = Mutex::new(None);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

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
    let (stream, broken, generation) = open(device, config, device_name, slot.clone())?;
    log::debug!("▶️  Sortie partagée ouverte ({device_name})");
    *guard = Some(Engine { key, slot, broken, generation, _stream: stream });
    Ok(id)
}

/// Ouvre et démarre un flux qui joue ce que contient `slot`.
fn open(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    device_name: &str,
    slot: Slot,
) -> Result<(cpal::Stream, Arc<AtomicBool>, u64), AudioOutputError> {
    let generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
    let broken = Arc::new(AtomicBool::new(false));
    let cb_broken = broken.clone();
    let errors = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (name, cfg) = (device_name.to_string(), config.clone());

    let stream = device
        .build_output_stream(
            config.clone(),
            move |output: &mut [f32], _| {
                // `try_lock` : le callback n'attend jamais ; au pire un tampon de silence au changement de piste.
                match slot.try_lock() {
                    Ok(mut g) => match g.as_mut() {
                        Some((_, render)) => render(output),
                        None => output.fill(0.0),
                    },
                    Err(_) => output.fill(0.0),
                }
            },
            move |err: cpal::Error| {
                use cpal::ErrorKind;
                // Flux perdu (0x88890004…) : rouvert à côté, une seule fois par flux.
                if !matches!(err.kind(), ErrorKind::Xrun | ErrorKind::DeviceChanged)
                    && !cb_broken.swap(true, Ordering::AcqRel)
                {
                    let (name, cfg) = (name.clone(), cfg.clone());
                    std::thread::spawn(move || recover(generation, &name, &cfg));
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
    Ok((stream, broken, generation))
}

/// Rouvre un flux invalidé en gardant la piste branchée : même sortie d'abord,
/// celle du système après 5 s ; rien après 15 s, l'interface arrête la lecture.
fn recover(generation: u64, device_name: &str, config: &cpal::StreamConfig) {
    for attempt in 1..=RECOVER_TRIES {
        std::thread::sleep(RECOVER_EVERY);
        let mut guard = engine();
        // Fermé, ou déjà rouvert par une nouvelle piste : plus rien à faire.
        let Some(e) = guard.as_mut().filter(|e| e.generation == generation) else { return };
        if e.slot.lock().unwrap_or_else(|p| p.into_inner()).is_none() {
            *guard = None;
            return;
        }
        // Le DAC peut mettre quelques secondes à réapparaître (câble, réinitialisation USB).
        let host = cpal::default_host();
        let target = match super::device_lookup::find(&host, device_name) {
            Some(device) => Some((device, device_name.to_string())),
            None if attempt > SAME_DEVICE_TRIES => host
                .default_output_device()
                .and_then(|d| super::device_lookup::full_name(&d).map(|n| (d, n))),
            None => None,
        };
        let Some((device, name)) = target else { continue };
        match open(&device, config, &name, e.slot.clone()) {
            Ok((stream, broken, generation)) => {
                (e._stream, e.broken, e.generation) = (stream, broken, generation);
                if name == device_name {
                    log::warn!("🔁 Sortie partagée rouverte après invalidation ({name}, essai {attempt})");
                } else {
                    // La clé suit la sortie réelle : la piste suivante retentera la sortie choisie.
                    e.key.device = name.clone();
                    log::warn!("🔁 Sortie '{device_name}' perdue : lecture sur la sortie système '{name}'");
                    notify(device_name, Some(name));
                }
                return;
            }
            Err(err) => log::debug!("Réouverture de la sortie, essai {attempt} : {err:?}"),
        }
    }
    log::error!("❌ Sortie '{device_name}' perdue, aucune sortie de relais : lecture arrêtée");
    notify(device_name, None);
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
