//! Sortie DoP (DSD over PCM) bit-perfect via CoreAudio — macOS uniquement.
//!
//! # Pourquoi le DoP est la SEULE voie DSD native sur macOS
//! CoreAudio n'a aucun transport DSD natif (pas d'équivalent ASIO Native DSD ni
//! de `SNDRV_PCM_FORMAT_DSD_*` comme ALSA). Un Mac ne peut envoyer que du PCM
//! linéaire à un DAC USB. Le DoP contourne ça en **transportant** le DSD dans
//! des trames PCM 24-bit que le DAC reconnaît (marqueurs `0x05/0xFA`) et rejoue
//! en DSD natif. C'est exactement ce que font Audirvana / Roon / HQPlayer sur
//! macOS. Corollaire : un DAC en mode **UAC (USB Audio Class, class-compliant)**
//! n'est pas un obstacle — c'est le mode *nominal* de macOS, qui ne charge
//! jamais de pilote propriétaire. La seule vraie contrainte est le débit : il
//! faut que le DAC accepte le rate porteur (DSD64 → 176,4 kHz, DSD128 → 352,8
//! kHz, DSD256 → 705,6 kHz), donc un DAC **UAC2**. Un DAC bloqué en **UAC1**
//! plafonne à 96 kHz → DoP impossible, on retombe sur DSD2PCM.
//!
//! # Pourquoi CPAL pour le stream, CoreAudio brut pour le reste
//! L'unité HAL que CPAL ouvre règle déjà le rate nominal du device
//! (`kAudioDevicePropertyNominalSampleRate`) sur celui demandé et ne
//! rééchantillonne pas quand les deux coïncident — c'est tout ce qu'il faut au
//! DoP. En revanche CPAL n'expose ni les `AudioDeviceID`, ni les rates
//! disponibles, ni le **hog mode**. On appelle donc l'API `AudioObject*`
//! directement pour ces trois points (cf. `HogGuard`).
//!
//! # Bit-perfect à travers du Float32 ?
//! Oui, et c'est exact — pas une approximation. CPAL sort du `f32` sur macOS,
//! et un mot DoP ne fait que **24 bits significatifs**, ce qui tient
//! exactement dans la mantisse d'un `f32` (24 bits). La reconversion
//! `f32 → entier` du HAL est une multiplication par une puissance de deux :
//! aller-retour exact, aucun bit perdu, marqueurs intacts. Les valeurs DoP
//! restent en outre confinées à ±0,05 pleine échelle (les marqueurs `0x05`
//! et `0xFA` bornent l'octet de poids fort) → aucun risque d'écrêtage.
//!
//! # Ce qui casserait le DoP, et comment on s'en protège
//! - **Rééchantillonnage** : on force le rate nominal = porteur et on le
//!   *revérifie* après ouverture du stream ; en cas d'écart on abandonne.
//! - **Mixage avec une autre app** : `HogGuard` prend le hog mode (accès
//!   exclusif au device pour notre process), équivalent macOS du WASAPI
//!   exclusive / ALSA `hw:`.
//! - **Volume logiciel** : on ne touche pas au volume système (sur un DAC USB
//!   macOS le relaie en UAC, donc *dans le DAC*, après décodage DSD — inoffensif),
//!   et le volume applicatif n'est pas appliqué sur ce chemin.
//!
//! # v1 : per-track
//! Un stream par piste, comme le DoP ALSA de Linux (pas encore de moteur
//! persistant façon `dop_engine.rs`). Le DAC se re-verrouille entre les
//! morceaux (~1-2 s). Un pré-roll de silence DoP couvre ce lock en début de
//! piste pour ne pas amputer les premières notes.

#![cfg(target_os = "macos")]

use std::ffi::c_void;
use std::ptr::{null, NonNull};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, StreamTrait};
use objc2_core_audio::{
    kAudioDevicePropertyAvailableNominalSampleRates, kAudioDevicePropertyHogMode,
    kAudioDevicePropertyNominalSampleRate, kAudioDevicePropertyStreamConfiguration,
    kAudioDevicePropertyStreams, kAudioHardwarePropertyDevices, kAudioObjectPropertyElementMain,
    kAudioObjectPropertyName, kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyScopeOutput,
    kAudioObjectSystemObject, kAudioStreamPropertyPhysicalFormat, AudioDeviceID,
    AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectID,
    AudioObjectPropertyAddress, AudioObjectSetPropertyData,
};
#[cfg(test)]
use objc2_core_audio::kAudioStreamPropertyAvailablePhysicalFormats;
use objc2_core_audio_types::{AudioBufferList, AudioStreamBasicDescription, AudioValueRange};
use objc2_core_foundation::{CFRetained, CFString};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;

use crate::core::audio_decoder::dsd::dop_encoder::{
    dop_silence_payload, stamp_marker, DopEncoder,
};
use crate::core::audio_decoder::dsd::dsd_container::DsdContainerReader;

// ===========================================================================
// Helpers CoreAudio (API `AudioObject*`)
// ===========================================================================

fn prop(selector: u32, scope: u32, element: u32) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: element,
    }
}

/// Lit une propriété de taille fixe (`T`) sur un AudioObject.
///
/// # Safety
/// `T` doit correspondre au type que CoreAudio écrit pour ce sélecteur.
unsafe fn get_fixed<T>(id: AudioObjectID, addr: &AudioObjectPropertyAddress, out: &mut T) -> bool {
    let mut size = std::mem::size_of::<T>() as u32;
    let status = AudioObjectGetPropertyData(
        id,
        NonNull::from(addr),
        0,
        null(),
        NonNull::from(&mut size),
        NonNull::from(out).cast(),
    );
    status == 0
}

/// Lit une propriété de taille variable et renvoie les octets bruts, alignés
/// sur 8 (les structures CoreAudio contiennent des pointeurs).
unsafe fn get_variable(
    id: AudioObjectID,
    addr: &AudioObjectPropertyAddress,
) -> Option<(Vec<u64>, usize)> {
    let mut size = 0u32;
    if AudioObjectGetPropertyDataSize(id, NonNull::from(addr), 0, null(), NonNull::from(&mut size))
        != 0
        || size == 0
    {
        return None;
    }
    let mut buf = vec![0u64; (size as usize).div_ceil(8)];
    let status = AudioObjectGetPropertyData(
        id,
        NonNull::from(addr),
        0,
        null(),
        NonNull::from(&mut size),
        NonNull::new(buf.as_mut_ptr())?.cast::<c_void>(),
    );
    if status != 0 {
        return None;
    }
    Some((buf, size as usize))
}

/// Tous les AudioDeviceID connus du système (entrées et sorties confondues).
fn all_device_ids() -> Vec<AudioDeviceID> {
    let addr = prop(
        kAudioHardwarePropertyDevices,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    unsafe {
        match get_variable(kAudioObjectSystemObject as AudioObjectID, &addr) {
            Some((buf, size)) => {
                let count = size / std::mem::size_of::<AudioDeviceID>();
                std::slice::from_raw_parts(buf.as_ptr() as *const AudioDeviceID, count).to_vec()
            }
            None => Vec::new(),
        }
    }
}

/// Nom lisible du device (`kAudioObjectPropertyName`, ex. "Fosi Audio K7").
fn device_name(id: AudioDeviceID) -> Option<String> {
    let addr = prop(
        kAudioObjectPropertyName,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    unsafe {
        let mut cf: *mut CFString = std::ptr::null_mut();
        if !get_fixed(id, &addr, &mut cf) {
            return None;
        }
        // `kAudioObjectPropertyName` suit la Create Rule (+1 retain) → on
        // reprend la propriété de la CFString, libérée au drop du CFRetained.
        let cf = NonNull::new(cf)?;
        Some(CFRetained::from_raw(cf).to_string())
    }
}

/// Nombre total de canaux de SORTIE du device (0 si c'est une entrée pure).
fn output_channel_count(id: AudioDeviceID) -> u32 {
    let addr = prop(
        kAudioDevicePropertyStreamConfiguration,
        kAudioObjectPropertyScopeOutput,
        kAudioObjectPropertyElementMain,
    );
    unsafe {
        let Some((buf, _)) = get_variable(id, &addr) else {
            return 0;
        };
        let list = &*(buf.as_ptr() as *const AudioBufferList);
        let n = list.mNumberBuffers as usize;
        if n == 0 {
            return 0;
        }
        std::slice::from_raw_parts(list.mBuffers.as_ptr(), n)
            .iter()
            .map(|b| b.mNumberChannels)
            .sum()
    }
}

/// Rate nominal courant du device, en Hz.
pub(super) fn nominal_rate(id: AudioDeviceID) -> Option<f64> {
    let addr = prop(
        kAudioDevicePropertyNominalSampleRate,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    let mut rate = 0.0f64;
    unsafe { get_fixed(id, &addr, &mut rate) }.then_some(rate)
}

/// Profondeur du format PHYSIQUE le plus fin parmi les flux de sortie du
/// device, en bits par canal (`None` si illisible).
///
/// C'est ce format-là qui part réellement sur le câble USB : le HAL y convertit
/// notre `f32`. Un mot DoP occupant 24 bits significatifs, un format physique
/// plus court tronquerait les marqueurs — donc bruit blanc à fort niveau.
fn physical_bit_depth(id: AudioDeviceID) -> Option<u32> {
    let streams_addr = prop(
        kAudioDevicePropertyStreams,
        kAudioObjectPropertyScopeOutput,
        kAudioObjectPropertyElementMain,
    );
    let streams: Vec<AudioObjectID> = unsafe {
        let (buf, size) = get_variable(id, &streams_addr)?;
        let count = size / std::mem::size_of::<AudioObjectID>();
        std::slice::from_raw_parts(buf.as_ptr() as *const AudioObjectID, count).to_vec()
    };

    let fmt_addr = prop(
        kAudioStreamPropertyPhysicalFormat,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    streams
        .iter()
        .filter_map(|stream| {
            let mut asbd: AudioStreamBasicDescription = unsafe { std::mem::zeroed() };
            unsafe { get_fixed(*stream, &fmt_addr, &mut asbd) }.then_some(asbd.mBitsPerChannel)
        })
        .max()
}

/// `AudioStreamRangedDescription` : un format physique proposé par un flux,
/// assorti de sa plage de rates. Absent d'`objc2-core-audio-types` 0.3, on le
/// redéclare à l'identique (ABI stable, cf. `<CoreAudio/AudioHardwareBase.h>`).
#[cfg(test)]
#[repr(C)]
#[derive(Clone, Copy)]
struct StreamRangedDescription {
    format: AudioStreamBasicDescription,
    sample_rate_range: AudioValueRange,
}

/// Tous les formats physiques que les flux de sortie du device savent prendre.
///
/// Sert au diagnostic : c'est la liste exhaustive de ce que le DAC annonce à
/// macOS, donc la preuve directe de son profil UAC.
#[cfg(test)]
fn available_physical_formats(id: AudioDeviceID) -> Vec<AudioStreamBasicDescription> {
    let streams_addr = prop(
        kAudioDevicePropertyStreams,
        kAudioObjectPropertyScopeOutput,
        kAudioObjectPropertyElementMain,
    );
    let streams: Vec<AudioObjectID> = unsafe {
        match get_variable(id, &streams_addr) {
            Some((buf, size)) => {
                let count = size / std::mem::size_of::<AudioObjectID>();
                std::slice::from_raw_parts(buf.as_ptr() as *const AudioObjectID, count).to_vec()
            }
            None => Vec::new(),
        }
    };

    let fmt_addr = prop(
        kAudioStreamPropertyAvailablePhysicalFormats,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    let mut out = Vec::new();
    for stream in streams {
        unsafe {
            let Some((buf, size)) = get_variable(stream, &fmt_addr) else {
                continue;
            };
            let count = size / std::mem::size_of::<StreamRangedDescription>();
            let descs =
                std::slice::from_raw_parts(buf.as_ptr() as *const StreamRangedDescription, count);
            out.extend(descs.iter().map(|d| d.format));
        }
    }
    out
}

/// Le device peut-il tourner EXACTEMENT à `rate` Hz ?
///
/// C'est ici que se joue la limite UAC1 / UAC2 : un DAC UAC1 ne déclare rien
/// au-delà de 96 kHz, donc aucun porteur DoP n'est atteignable.
fn available_nominal_rates(id: AudioDeviceID) -> Vec<AudioValueRange> {
    // ATTENTION : cette propriété vit sur le scope GLOBAL, pas Output (elle
    // décrit l'horloge du device, pas un sens de transfert). L'interroger en
    // scope Output renvoie « unknown property » → liste vide → tout refusé.
    let addr = prop(
        kAudioDevicePropertyAvailableNominalSampleRates,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    unsafe {
        match get_variable(id, &addr) {
            Some((buf, size)) => {
                let count = size / std::mem::size_of::<AudioValueRange>();
                std::slice::from_raw_parts(buf.as_ptr() as *const AudioValueRange, count).to_vec()
            }
            None => Vec::new(),
        }
    }
}

pub(super) fn supports_nominal_rate(id: AudioDeviceID, rate: u32) -> bool {
    let want = rate as f64;
    // Tolérance ±0,5 Hz : les DAC déclarent souvent des bornes discrètes
    // (min == max) en flottant, pas toujours arrondies à l'entier près.
    available_nominal_rates(id)
        .iter()
        .any(|r| want >= r.mMinimum - 0.5 && want <= r.mMaximum + 0.5)
}

// ===========================================================================
// Énumération / résolution de device
// ===========================================================================

/// Un périphérique de sortie CoreAudio candidat au DoP.
#[derive(Debug, Clone)]
pub struct CoreAudioDopDevice {
    /// Identifiant CoreAudio du device.
    pub id: AudioDeviceID,
    /// Nom lisible, pour le matching avec le nom affiché dans l'UI.
    pub display: String,
}

/// Énumère les devices CoreAudio ayant au moins un canal de sortie.
pub fn list_dop_devices() -> Vec<CoreAudioDopDevice> {
    all_device_ids()
        .into_iter()
        .filter(|id| output_channel_count(*id) > 0)
        .filter_map(|id| {
            device_name(id).map(|display| CoreAudioDopDevice { id, display })
        })
        .collect()
}

/// Mappe le nom d'affichage sélectionné dans l'UI (ex. "Fosi Audio K7 (…)")
/// vers l'`AudioDeviceID` correspondant, par sous-chaîne insensible à la casse
/// dans les deux sens — même stratégie que `dop_alsa::resolve_hw_id`.
pub fn resolve_device_id(selected_display: &str) -> Option<AudioDeviceID> {
    let sel = selected_display.to_lowercase();
    if sel.is_empty() {
        return None;
    }
    for dev in list_dop_devices() {
        let name = dev.display.to_lowercase();
        if name.is_empty() {
            continue;
        }
        if sel.contains(&name) || name.contains(&sel) {
            return Some(dev.id);
        }
    }
    None
}

/// Le DAC accepte-t-il le format DoP au porteur donné ?
///
/// Contrairement à ALSA on ne peut pas « répéter » l'ouverture exclusive à
/// blanc sans couper le son des autres apps : on se contente donc de vérifier
/// les canaux de sortie et les rates nominaux déclarés. La vérification dure
/// (rate réellement appliqué) est refaite après ouverture du stream.
pub fn dop_format_supported(id: AudioDeviceID, carrier_rate: u32, channels: u16) -> bool {
    if output_channel_count(id) < channels as u32 {
        log::debug!(
            "🎚️  DoP CoreAudio : device {id} n'a pas {channels} canaux de sortie"
        );
        return false;
    }
    if !supports_nominal_rate(id, carrier_rate) {
        // On logue les rates RÉELLEMENT déclarés : c'est la seule information
        // qui permette à l'utilisateur de trancher entre « mon DAC est bridé »
        // et « rustmusic interroge mal CoreAudio ».
        let declared: Vec<String> = available_nominal_rates(id)
            .iter()
            .map(|r| {
                if (r.mMinimum - r.mMaximum).abs() < 0.5 {
                    format!("{:.0}", r.mMinimum)
                } else {
                    format!("{:.0}–{:.0}", r.mMinimum, r.mMaximum)
                }
            })
            .collect();
        log::warn!(
            "🎚️  DoP CoreAudio : device {id} ne déclare pas {carrier_rate} Hz — \
             rates annoncés à macOS : [{}]",
            declared.join(", ")
        );
        return false;
    }
    true
}

// ===========================================================================
// Hog mode (accès exclusif)
// ===========================================================================

/// Prend le **hog mode** sur le device et le relâche au drop.
///
/// C'est l'équivalent macOS du WASAPI exclusive : tant qu'on le détient, le HAL
/// n'accepte aucun autre client sur ce device — donc plus personne ne peut
/// injecter d'audio qui serait mixé dans notre flux (ce qui détruirait
/// instantanément les marqueurs DoP).
///
/// L'échec n'est pas fatal : sans hog, le DoP fonctionne tant qu'aucune autre
/// application ne joue sur le même DAC. On se contente donc d'avertir.
pub(super) struct HogGuard {
    id: AudioDeviceID,
    owned: bool,
}

impl HogGuard {
    pub(super) fn acquire(id: AudioDeviceID) -> Self {
        let addr = prop(
            kAudioDevicePropertyHogMode,
            kAudioObjectPropertyScopeGlobal,
            kAudioObjectPropertyElementMain,
        );
        let me = std::process::id() as i32;

        let mut owner: i32 = -1;
        unsafe { get_fixed(id, &addr, &mut owner) };
        if owner != -1 && owner != me {
            log::warn!(
                "🔒 DoP CoreAudio : device déjà en hog mode par le process {owner} — \
                 lecture sans exclusivité (risque si une autre app joue dessus)"
            );
            return Self { id, owned: false };
        }

        let mut want = me;
        let status = unsafe {
            AudioObjectSetPropertyData(
                id,
                NonNull::from(&addr),
                0,
                null(),
                std::mem::size_of::<i32>() as u32,
                NonNull::from(&mut want).cast(),
            )
        };

        let mut after: i32 = -1;
        unsafe { get_fixed(id, &addr, &mut after) };
        let owned = status == 0 && after == me;
        if owned {
            log::info!("🔒 DoP CoreAudio : hog mode acquis (accès exclusif au DAC)");
        } else {
            log::warn!(
                "🔒 DoP CoreAudio : hog mode refusé (status {status}) — \
                 lecture sans exclusivité"
            );
        }
        Self { id, owned }
    }
}

impl Drop for HogGuard {
    fn drop(&mut self) {
        if !self.owned {
            return;
        }
        let addr = prop(
            kAudioDevicePropertyHogMode,
            kAudioObjectPropertyScopeGlobal,
            kAudioObjectPropertyElementMain,
        );
        let mut release: i32 = -1;
        let status = unsafe {
            AudioObjectSetPropertyData(
                self.id,
                NonNull::from(&addr),
                0,
                null(),
                std::mem::size_of::<i32>() as u32,
                NonNull::from(&mut release).cast(),
            )
        };
        if status == 0 {
            log::debug!("🔓 DoP CoreAudio : hog mode relâché");
        } else {
            log::warn!("🔓 DoP CoreAudio : échec du relâchement du hog mode (status {status})");
        }
    }
}

// ===========================================================================
// Lecture
// ===========================================================================

/// Pilotage du render DoP par le thread de lecture (miroir de `AlsaDopControl`).
pub struct CoreAudioDopControl {
    /// Pause globale : on écrit du silence DSD (le DAC reste locké).
    pub is_paused: Arc<AtomicBool>,
    /// Arrêt demandé : la boucle sort dès que possible.
    pub is_stopped: Arc<AtomicBool>,
    /// Position courante en secondes (bits f64). Dérivée des trames jouées.
    pub current_position: Arc<AtomicU64>,
    /// Seek demandé en secondes (bits f64) ; `u64::MAX` = aucun.
    pub seek_position: Arc<AtomicU64>,
    /// Durée totale en secondes (bits f64), pour clamper le seek.
    pub total_duration: Arc<AtomicU64>,
}

/// Convertit un échantillon DoP 24-bit cadré `[marqueur][hi][lo][0]` en `f32`.
///
/// `>> 8` ramène le mot DoP sur 24 bits signés (exactement représentable dans
/// la mantisse d'un `f32`), la division par 2^23 est exacte (puissance de
/// deux). Le HAL refait l'opération inverse vers le format entier du DAC :
/// aller-retour bit à bit, marqueurs préservés.
#[inline]
fn dop_to_f32(sample: i32) -> f32 {
    (sample >> 8) as f32 / 8_388_608.0
}

/// Durée du pré-roll de silence DoP avant la musique. Le DAC mute pendant
/// qu'il acquiert le lock DSD ; sans ce pré-roll les premières notes sont
/// avalées.
const WARMUP: Duration = Duration::from_millis(800);

/// Ouvre le stream CoreAudio et joue la piste DSD en DoP jusqu'à EOF ou stop.
///
/// Le **marqueur DoP est posé dans le callback audio**, sur chaque trame,
/// musique comme silence, avec un compteur unique et continu : c'est la seule
/// façon de garantir l'alternance `0x05`/`0xFA` ininterrompue à la jonction
/// silence↔musique (deux compteurs séparés se désynchroniseraient → le DAC
/// perdrait le lock). Le ring buffer ne transporte donc que le *payload*.
///
/// Retour : `Ok(true)` = fin naturelle (EOF), `Ok(false)` = stop utilisateur.
pub fn run_coreaudio_dop_playback(
    device: &cpal::Device,
    device_id: AudioDeviceID,
    carrier_rate: u32,
    channels: u16,
    mut decoder: Box<dyn DsdContainerReader + Send>,
    lsb_first: bool,
    ctl: CoreAudioDopControl,
) -> Result<bool, String> {
    let ch = channels as usize;
    if ch == 0 {
        return Err("nombre de canaux nul".into());
    }

    // Exclusivité d'abord : si une autre app tient le device, autant le savoir
    // avant d'ouvrir le stream. Relâché automatiquement au retour de la fonction.
    let _hog = HogGuard::acquire(device_id);

    // ─── Ring buffer de PAYLOAD DoP (sans marqueur) ───
    // ~500 ms : assez pour absorber les hoquets du décodeur DSD, assez court
    // pour que pause/seek réagissent sans latence perceptible.
    let capacity = (carrier_rate as usize / 2) * ch;
    let (mut producer, mut consumer) = HeapRb::<i32>::new(capacity).split();

    let flush = Arc::new(AtomicBool::new(false));
    let frames_played = Arc::new(AtomicU64::new(0));

    let cb_flush = flush.clone();
    let cb_played = frames_played.clone();
    let cb_paused = ctl.is_paused.clone();

    // État propre au callback.
    let silence = dop_silence_payload();
    let mut marker_b = false;
    let mut scratch: Vec<i32> = Vec::new();

    let config = cpal::StreamConfig {
        channels,
        sample_rate: carrier_rate,
        buffer_size: cpal::BufferSize::Default,
    };

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let frames = data.len() / ch;

                // Seek : le producteur a repositionné le décodeur, on jette ce
                // qui restait de l'ancienne position.
                if cb_flush.swap(false, Ordering::Acquire) {
                    consumer.clear();
                }

                // En pause on ne consomme rien : le ring garde son contenu
                // (reprise instantanée) et on émet du silence DoP pour que le
                // DAC conserve son lock DSD.
                let ready_frames = if cb_paused.load(Ordering::Relaxed) {
                    0
                } else {
                    consumer.occupied_len() / ch
                };

                let music_frames = ready_frames.min(frames);
                let mut done_frames = 0usize;

                if music_frames > 0 {
                    let wanted = music_frames * ch;
                    if scratch.len() < wanted {
                        scratch.resize(wanted, 0);
                    }
                    // Le producteur ne pousse que des trames entières et on a
                    // vérifié `occupied_len >= wanted` → `pop_slice` rend tout.
                    let popped = consumer.pop_slice(&mut scratch[..wanted]);
                    done_frames = popped / ch;

                    for f in 0..done_frames {
                        let marker = marker_b;
                        marker_b = !marker_b;
                        let base = f * ch;
                        for c in 0..ch {
                            data[base + c] = dop_to_f32(stamp_marker(scratch[base + c], marker));
                        }
                    }
                    cb_played.fetch_add(done_frames as u64, Ordering::Relaxed);
                }

                // Trames non couvertes (pause, sous-alimentation, pré-roll) :
                // silence DoP avec le MÊME compteur de marqueur.
                for f in done_frames..frames {
                    let marker = marker_b;
                    marker_b = !marker_b;
                    let s = dop_to_f32(stamp_marker(silence, marker));
                    let base = f * ch;
                    for c in 0..ch {
                        data[base + c] = s;
                    }
                }

                // Reliquat si `data.len()` n'est pas un multiple des canaux.
                let covered = frames * ch;
                if covered < data.len() {
                    let s = dop_to_f32(stamp_marker(silence, marker_b));
                    data[covered..].fill(s);
                }
            },
            {
                // Même throttling que le chemin CPAL classique : un DAC en
                // exclusive peut cracher des centaines d'erreurs/s sans que la
                // lecture soit réellement compromise.
                let errors = Arc::new(AtomicUsize::new(0));
                move |err| {
                    let n = errors.fetch_add(1, Ordering::Relaxed);
                    if n == 0 || (n < 1000 && n % 100 == 0) || n % 1000 == 0 {
                        log::error!("❌ [DoP CoreAudio] erreur stream (#{}): {:?}", n + 1, err);
                    }
                }
            },
            None,
        )
        .map_err(|e| format!("ouverture stream CoreAudio @ {carrier_rate} Hz: {e}"))?;

    stream
        .play()
        .map_err(|e| format!("démarrage stream CoreAudio: {e}"))?;

    // Garde-fou : si le HAL n'a pas réellement basculé le device sur le
    // porteur, il y a un rééchantillonnage quelque part → le DoP sortirait en
    // bruit blanc très fort. On refuse plutôt que de risquer les oreilles.
    match nominal_rate(device_id) {
        Some(actual) if (actual - carrier_rate as f64).abs() < 0.5 => {}
        Some(actual) => {
            return Err(format!(
                "rate appliqué {actual} Hz ≠ porteur {carrier_rate} Hz (rééchantillonnage)"
            ));
        }
        None => {
            return Err("rate nominal du device illisible".into());
        }
    }

    // Second garde-fou : le format physique doit porter au moins les 24 bits
    // d'un mot DoP. En dessous, les marqueurs seraient tronqués → bruit blanc
    // à niveau élevé. On refuse plutôt que d'envoyer ça dans des enceintes.
    match physical_bit_depth(device_id) {
        Some(bits) if bits >= 24 => {
            log::debug!("🎚️  DoP CoreAudio : format physique {bits} bits");
        }
        Some(bits) => {
            return Err(format!(
                "format physique {bits} bits < 24 bits requis par le DoP"
            ));
        }
        // Illisible : on ne bloque pas (certains devices agrégés n'exposent pas
        // leurs flux), le garde-fou sur le rate reste actif.
        None => log::warn!("🎚️  DoP CoreAudio : profondeur du format physique illisible"),
    }

    log::info!(
        "🎚️  Stream CoreAudio DoP ouvert : device {device_id} @ {carrier_rate} Hz / \
         {channels} ch (f32 → 24-bit exact, bit-perfect)"
    );

    // Pré-roll : le callback tourne déjà et émet du silence DoP, ce qui laisse
    // au DAC le temps d'acquérir le lock DSD avant la première note.
    let warmup_deadline = std::time::Instant::now() + WARMUP;
    while std::time::Instant::now() < warmup_deadline {
        if ctl.is_stopped.load(Ordering::Relaxed) {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    // ─── Boucle producteur : décode → encode → pousse ───
    let mut encoder = DopEncoder::new(channels as u8, lsb_first);
    let mut pending: Vec<i32> = Vec::new();
    let mut pending_off = 0usize;

    // Seules sorties de la boucle : `break` sur EOF, ou `return` sur stop /
    // erreur de décodage. Franchir le `break` signifie donc fin naturelle.
    loop {
        if ctl.is_stopped.load(Ordering::Relaxed) {
            return Ok(false);
        }

        // ─── Seek ───
        let seek_bits = ctl.seek_position.load(Ordering::Relaxed);
        if seek_bits != u64::MAX {
            ctl.seek_position.store(u64::MAX, Ordering::Relaxed);
            let total = f64::from_bits(ctl.total_duration.load(Ordering::Relaxed)).max(0.0);
            let secs = f64::from_bits(seek_bits).clamp(0.0, total);
            if let Ok(actual) = decoder.seek_to_seconds(secs) {
                encoder.reset();
                pending.clear();
                pending_off = 0;
                frames_played.store((actual * carrier_rate as f64) as u64, Ordering::Relaxed);
                ctl.current_position.store(actual.to_bits(), Ordering::Relaxed);
                // Après le compteur, sinon le callback pourrait créditer des
                // trames de l'ancienne position par-dessus la nouvelle base.
                flush.store(true, Ordering::Release);
            }
            continue;
        }

        // Position : source de vérité = trames réellement consommées par le
        // callback (et non poussées), donc alignée sur ce qu'on entend.
        let pos = frames_played.load(Ordering::Relaxed) as f64 / carrier_rate as f64;
        ctl.current_position.store(pos.to_bits(), Ordering::Relaxed);

        // ─── Décodage à la demande ───
        if pending_off >= pending.len() {
            match decoder.read_next_blocks() {
                Ok(Some(blocks)) => {
                    pending = encoder.encode_blocks(&blocks);
                    pending_off = 0;
                }
                Ok(None) => break, // EOF
                Err(e) => return Err(format!("décodage DSD: {e:?}")),
            }
            if pending.is_empty() {
                continue;
            }
        }

        // ─── Push par TRAMES ENTIÈRES ───
        // Pousser une trame partielle décalerait l'appariement canal/échantillon
        // dans le callback pour tout le reste de la piste.
        let free_frames = producer.vacant_len() / ch;
        let want = ((pending.len() - pending_off) / ch).min(free_frames) * ch;
        if want == 0 {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        pending_off += producer.push_slice(&pending[pending_off..pending_off + want]);
    }

    // ─── Fin naturelle : laisser le ring puis le buffer matériel se vider ───
    while producer.occupied_len() >= ch {
        if ctl.is_stopped.load(Ordering::Relaxed) {
            return Ok(false);
        }
        let pos = frames_played.load(Ordering::Relaxed) as f64 / carrier_rate as f64;
        ctl.current_position.store(pos.to_bits(), Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(10));
    }
    // Le callback a tout consommé mais le DAC a encore quelques trames en vol :
    // on laisse passer avant de couper le stream.
    std::thread::sleep(Duration::from_millis(150));

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::audio_decoder::dsd::dop_encoder::dop_silence_payload;

    /// Le cœur de la promesse bit-perfect : un mot DoP passé en `f32` puis
    /// reconverti en entier 24-bit doit être stritement identique.
    #[test]
    fn f32_roundtrip_is_exact() {
        for hi in [0x00u32, 0x69, 0xAA, 0xFF] {
            for lo in [0x00u32, 0x69, 0x55, 0xFF] {
                for marker_b in [false, true] {
                    let payload = (((hi << 8) | lo) << 8) as i32;
                    let sample = stamp_marker(payload, marker_b);
                    let f = dop_to_f32(sample);
                    // Reconversion telle que la fait le HAL vers un entier 24-bit.
                    let back = (f * 8_388_608.0) as i32;
                    assert_eq!(back, sample >> 8, "hi={hi:#04x} lo={lo:#04x} b={marker_b}");
                }
            }
        }
    }

    /// Diagnostic manuel (`cargo test dop_capability_report -- --ignored
    /// --nocapture`) : liste les sorties CoreAudio et dit, pour chacune, quels
    /// porteurs DoP elle accepte. C'est l'outil pour trancher « mon DAC est-il
    /// UAC1 ou UAC2 ? » — un DAC UAC1 n'affichera aucun ✓.
    #[test]
    #[ignore = "dépend du matériel branché"]
    fn dop_capability_report() {
        let devices = list_dop_devices();
        assert!(
            !devices.is_empty(),
            "aucune sortie CoreAudio énumérée — le FFI a échoué"
        );
        for dev in devices {
            println!(
                "\n{} (id {}, {} ch, format physique COURANT {:?} bits)",
                dev.display,
                dev.id,
                output_channel_count(dev.id),
                physical_bit_depth(dev.id)
            );
            let rates: Vec<String> = available_nominal_rates(dev.id)
                .iter()
                .map(|r| {
                    if (r.mMinimum - r.mMaximum).abs() < 0.5 {
                        format!("{:.0}", r.mMinimum)
                    } else {
                        format!("{:.0}–{:.0}", r.mMinimum, r.mMaximum)
                    }
                })
                .collect();
            println!("   rates déclarés : {}", rates.join(", "));
            for f in available_physical_formats(dev.id) {
                println!(
                    "   format physique dispo : {:.0} Hz / {} bits / {} ch",
                    f.mSampleRate, f.mBitsPerChannel, f.mChannelsPerFrame
                );
            }
            for (label, dsd_rate) in [
                ("DSD64", 2_822_400u32),
                ("DSD128", 5_644_800),
                ("DSD256", 11_289_600),
            ] {
                let carrier = dsd_rate / 16;
                let ok = dop_format_supported(dev.id, carrier, 2);
                println!(
                    "   {} {label:6} → porteur {carrier} Hz",
                    if ok { "✓" } else { "✗" }
                );
            }
        }
    }

    /// Les mots DoP restent très loin de la pleine échelle → jamais d'écrêtage
    /// dans le chemin flottant de CoreAudio.
    #[test]
    fn dop_samples_never_clip() {
        for marker_b in [false, true] {
            for payload in [0i32, dop_silence_payload(), 0x00FFFF00u32 as i32] {
                let f = dop_to_f32(stamp_marker(payload, marker_b));
                assert!(f.abs() < 0.06, "amplitude inattendue: {f}");
            }
        }
    }
}
