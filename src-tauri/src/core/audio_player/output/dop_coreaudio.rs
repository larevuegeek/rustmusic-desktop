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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
use ringbuf::traits::{Consumer, Observer};
use ringbuf::HeapCons;

use super::dop_engine::{DopRenderCtl, PRECHAUFFAGE_S};

use crate::core::audio_decoder::dsd::dop_encoder::{dop_silence_payload, stamp_marker};

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
pub(super) fn physical_bit_depth(id: AudioDeviceID) -> Option<u32> {
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

/// Render persistant du moteur DoP (hog mode, rate porteur) jusqu'au `render_stop`.
/// Marqueur posé sur chaque trame, musique ou silence : le DAC reste verrouillé.
pub fn run_coreaudio_dop_render(
    device: &cpal::Device,
    device_id: AudioDeviceID,
    carrier_rate: u32,
    channels: u16,
    mut consumer: HeapCons<i32>,
    ctl: DopRenderCtl,
) -> Result<(), String> {
    let ch = channels as usize;
    if ch == 0 {
        return Err("nombre de canaux nul".into());
    }

    // Exclusivité tenue aussi longtemps que le moteur (relâchée au retour).
    let _hog = HogGuard::acquire(device_id);

    let cb = ctl.clone();
    // Arrêt demandé : le callback ne joue plus que du silence le temps de couper proprement.
    let silence_seul = Arc::new(AtomicBool::new(false));
    let cb_silence = silence_seul.clone();

    let silence = dop_silence_payload();
    let mut marker_b = false;
    let mut scratch: Vec<i32> = Vec::new();
    let warmup_frames = (carrier_rate as f64 * PRECHAUFFAGE_S) as usize;
    let mut warmed = 0usize;

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
                let mut musique = 0usize;

                // Vidage et saut d'abord, warm-up compris (sinon le début de la
                // piste déjà bufferisé serait jeté à la fin du warm-up).
                if cb.drain_request.swap(false, Ordering::AcqRel) {
                    consumer.clear();
                } else if cb.seek_flush.swap(false, Ordering::AcqRel) {
                    consumer.clear();
                } else if warmed < warmup_frames {
                    warmed += frames;
                    if warmed >= warmup_frames {
                        cb.audio_started.store(true, Ordering::Relaxed);
                    }
                } else if !cb.is_paused.load(Ordering::Relaxed) && !cb_silence.load(Ordering::Relaxed) {
                    // Trames entières seulement : l'appariement canal/échantillon tient.
                    let dispo = (consumer.occupied_len() / ch).min(frames);
                    if scratch.len() < dispo * ch {
                        scratch.resize(dispo * ch, 0);
                    }
                    musique = consumer.pop_slice(&mut scratch[..dispo * ch]) / ch;
                }

                for f in 0..frames {
                    let marker = marker_b;
                    marker_b = !marker_b;
                    let base = f * ch;
                    if f < musique {
                        for c in 0..ch {
                            data[base + c] = dop_to_f32(stamp_marker(scratch[base + c], marker));
                        }
                    } else {
                        data[base..base + ch].fill(dop_to_f32(stamp_marker(silence, marker)));
                    }
                }
                // Reliquat si `data.len()` n'est pas un multiple des canaux.
                let couvert = frames * ch;
                if couvert < data.len() {
                    data[couvert..].fill(dop_to_f32(stamp_marker(silence, marker_b)));
                }
                if musique > 0 {
                    cb.music_frames_played.fetch_add(musique, Ordering::Relaxed);
                }
            },
            {
                // Même throttling que le chemin CPAL classique : un DAC en
                // exclusif peut cracher des centaines d'erreurs/s sans que la
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
        None => return Err("rate nominal du device illisible".into()),
    }

    // Second garde-fou : le format physique doit porter au moins les 24 bits
    // d'un mot DoP. En dessous, les marqueurs seraient tronqués → bruit blanc.
    match physical_bit_depth(device_id) {
        Some(bits) if bits >= 24 => log::debug!("🎚️  DoP CoreAudio : format physique {bits} bits"),
        Some(bits) => return Err(format!("format physique {bits} bits < 24 bits requis par le DoP")),
        // Illisible : on ne bloque pas (certains devices agrégés n'exposent pas
        // leurs flux), le garde-fou sur le rate reste actif.
        None => log::warn!("🎚️  DoP CoreAudio : profondeur du format physique illisible"),
    }

    ctl.started_ok.store(true, Ordering::Relaxed);
    log::info!(
        "🎚️  Stream CoreAudio DoP ouvert : device {device_id} @ {carrier_rate} Hz / \
         {channels} ch (f32 → 24-bit exact, moteur persistant)"
    );

    while !ctl.render_stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(20));
    }

    // Un peu de silence avant de couper : le DAC quitte le DSD proprement.
    silence_seul.store(true, Ordering::Relaxed);
    std::thread::sleep(Duration::from_millis(60));
    drop(stream);
    log::info!("⏹  CoreAudio DoP : stream fermé (device {device_id})");
    Ok(())
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
