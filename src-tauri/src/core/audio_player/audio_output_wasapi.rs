//! Backend audio WASAPI exclusive mode pour Windows.
//!
//! Compilé uniquement sur Windows (le `#![cfg(target_os = "windows")]` global
//! masque le module sur les autres OS).
//!
//! # Pourquoi WASAPI exclusive ?
//!
//! Le mode partagé (par défaut, utilisé par CPAL) passe par l'**Audio Engine**
//! de Windows qui resample tout vers le sample rate configuré du device
//! (souvent 24-bit 48 kHz par défaut sur Windows 10/11) et mixe avec les
//! autres sources sonores. Un FLAC 96 kHz est donc resamplé en 48 kHz par
//! Windows avant d'atteindre le DAC. Ce n'est PAS du bit-perfect.
//!
//! Le mode **exclusive** :
//! - Bypass complet de l'Audio Engine Windows
//! - L'app prend possession exclusive du DAC
//! - Le format envoyé au DAC correspond exactement au fichier source (si le
//!   DAC le supporte)
//! - Vraie sortie bit-perfect

#![cfg(target_os = "windows")]

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use ringbuf::traits::{Consumer, Observer};
use wasapi::{
    Direction, DeviceEnumerator, SampleType, ShareMode, StreamMode, WaveFormat,
    initialize_mta,
};

// ============================================================================
// Initialisation COM — par-thread (PAS process-wide)
// ============================================================================
//
// WASAPI utilise COM. Sous Windows, COM est initialisé **par thread** dans
// soit l'apartment MTA (Multi-Threaded), soit STA (Single-Threaded). Un
// thread ne peut PAS changer d'apartment après son init.
//
// Tauri initialise ses worker threads en STA. Si on appelle `initialize_mta`
// depuis une commande Tauri, on obtient `RPC_E_CHANGED_MODE (0x80010106)`.
//
// Solution :
// 1. **Stockage thread_local** : on garde un flag par thread pour savoir si
//    on a déjà tenté l'init, et son résultat.
// 2. **Helper `run_on_mta_thread`** : pour les opérations one-shot (énumération
//    devices, format negotiation), on spawn un thread frais qui peut init MTA
//    proprement, puis on join. Coût ~2 ms, négligeable pour ces ops.
// 3. **Render thread** : `run_wasapi_playback` est censé être appelé depuis
//    un thread dédié à la lecture — il init MTA sur lui-même au démarrage.

thread_local! {
    static COM_INIT_STATE: Cell<ComInitState> = const { Cell::new(ComInitState::Untried) };
}

#[derive(Debug, Clone, Copy)]
enum ComInitState {
    Untried,
    OkMta,
    Failed,
}

/// Init COM en MTA sur le thread courant. Idempotent : ne retente pas si
/// déjà tenté. À utiliser **uniquement sur un thread où on contrôle l'init**
/// (thread spawné, thread audio dédié) — JAMAIS sur un thread Tauri (déjà STA).
fn ensure_com_initialized_current_thread() -> Result<(), String> {
    COM_INIT_STATE.with(|cell| match cell.get() {
        ComInitState::OkMta => Ok(()),
        ComInitState::Failed => Err("COM init already failed on this thread".into()),
        ComInitState::Untried => {
            let hr = initialize_mta();
            if hr.is_ok() {
                cell.set(ComInitState::OkMta);
                Ok(())
            } else {
                cell.set(ComInitState::Failed);
                Err(format!(
                    "CoInitializeEx (MTA) failed: HRESULT 0x{:08X} (probably called from a thread already in STA)",
                    hr.0
                ))
            }
        }
    })
}

/// Exécute `f` sur un thread frais, init MTA d'abord. Renvoie le résultat.
/// Utilisé par les commandes Tauri (`wasapi_*_command`) qui sont appelées
/// depuis un thread Tauri en STA — on ne peut pas faire MTA sur ce thread,
/// donc on délègue à un thread propre.
fn run_on_mta_thread<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    std::thread::spawn(move || {
        ensure_com_initialized_current_thread()?;
        f()
    })
    .join()
    .map_err(|e| format!("WASAPI worker thread panic: {e:?}"))?
}

// ============================================================================
// Erreurs typées
// ============================================================================

#[derive(Debug)]
pub enum WasapiPlayerError {
    /// Init COM impossible (très rare).
    ComInit(String),
    /// Pas de device de sortie audio détecté.
    NoDevice(String),
    /// Aucun format compatible trouvé entre la source et le DAC.
    NoFormat(String),
    /// AudioClient::initialize() a échoué (device occupé, format rejeté in extremis…).
    InitFailed(String),
    /// Erreur runtime pendant la lecture (perte de device, etc.).
    Runtime(String),
}

impl std::fmt::Display for WasapiPlayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasapiPlayerError::ComInit(m) => write!(f, "WASAPI COM init: {m}"),
            WasapiPlayerError::NoDevice(m) => write!(f, "WASAPI device: {m}"),
            WasapiPlayerError::NoFormat(m) => write!(f, "WASAPI format: {m}"),
            WasapiPlayerError::InitFailed(m) => write!(f, "WASAPI init: {m}"),
            WasapiPlayerError::Runtime(m) => write!(f, "WASAPI runtime: {m}"),
        }
    }
}

impl std::error::Error for WasapiPlayerError {}

// ============================================================================
// Énumération des devices (utilitaires UI)
// ============================================================================

#[derive(Debug, Clone)]
pub struct WasapiDevice {
    pub id: String,
    pub friendly_name: String,
}

pub fn list_output_devices() -> Result<Vec<WasapiDevice>, String> {
    run_on_mta_thread(|| {
        let enumerator = DeviceEnumerator::new()
            .map_err(|e| format!("DeviceEnumerator::new: {:?}", e))?;
        let collection = enumerator
            .get_device_collection(&Direction::Render)
            .map_err(|e| format!("get_device_collection: {:?}", e))?;
        let count = collection
            .get_nbr_devices()
            .map_err(|e| format!("get_nbr_devices: {:?}", e))?;

        let mut devices = Vec::with_capacity(count as usize);
        for i in 0..count {
            if let Ok(dev) = collection.get_device_at_index(i) {
                let id = dev.get_id().unwrap_or_else(|_| format!("device-{i}"));
                let name = dev
                    .get_friendlyname()
                    .unwrap_or_else(|_| format!("Device {i}"));
                devices.push(WasapiDevice {
                    id,
                    friendly_name: name,
                });
            }
        }
        Ok(devices)
    })
}

pub fn default_output_device_name() -> Result<String, String> {
    run_on_mta_thread(|| {
        let enumerator = DeviceEnumerator::new()
            .map_err(|e| format!("DeviceEnumerator::new: {:?}", e))?;
        let dev = enumerator
            .get_default_device(&Direction::Render)
            .map_err(|e| format!("get_default_device: {:?}", e))?;
        dev.get_friendlyname()
            .map_err(|e| format!("get_friendlyname: {:?}", e))
    })
}

// ============================================================================
// Capacités réelles par device (probing WASAPI)
// ============================================================================

/// Résultat du probing WASAPI d'un device de sortie.
///
/// Contrairement à CPAL sur Windows (qui expose la table shared-mode uniforme
/// pour tous les endpoints), ce probing interroge le driver du DAC via
/// `AudioClient::is_supported()` pour chaque combinaison (rate, bit_depth,
/// mode). Résultat : capacités réelles de l'endpoint sélectionné.
#[derive(Debug, Clone)]
pub struct WasapiCapabilities {
    /// Sample rates acceptés en mode Exclusive (bypass mixer Windows).
    pub exclusive_rates: Vec<u32>,
    /// Bit-depths acceptés en mode Exclusive (16/24/32).
    pub exclusive_bit_depths: Vec<u16>,
    /// Sample rates acceptés en mode Shared (mixer Windows actif).
    pub shared_rates: Vec<u32>,
    /// Format "mix" par défaut du device (celui utilisé par Windows quand
    /// une app ne demande rien de spécifique).
    pub mix_rate: Option<u32>,
    pub mix_bit_depth: Option<u16>,
    pub mix_channels: Option<u16>,
}

/// Sample rates PCM audiophiles standard qu'on teste. Focalisé musique —
/// pas la peine d'itérer sur 8000 Hz et autres rates telephonie.
const PROBE_RATES: &[u32] = &[
    44_100, 48_000, 88_200, 96_000, 176_400, 192_000, 352_800, 384_000, 705_600, 768_000,
];

/// Bit-depths PCM entiers testés. On skip 8-bit (aucun DAC musical) et le
/// float (rarement exposé en exclusive par les DACs USB).
const PROBE_BIT_DEPTHS: &[u16] = &[16, 24, 32];

/// Probing des capacités d'un device identifié par son ID WASAPI.
///
/// Le `device_id` provient de `WasapiDevice::id` (chaîne interne renvoyée
/// par `IMMDevice::GetId`). On itère l'énumération pour retrouver le device
/// avec cet ID, puis on probe chaque combinaison.
///
/// Coût typique : ~200-500 ms par device (10 rates × 3 bit-depths × 2 modes
/// = 60 probes, chacune ~5 ms).
pub fn probe_device_capabilities(device_id: String) -> Result<WasapiCapabilities, String> {
    run_on_mta_thread(move || {
        let enumerator = DeviceEnumerator::new()
            .map_err(|e| format!("DeviceEnumerator::new: {:?}", e))?;
        let collection = enumerator
            .get_device_collection(&Direction::Render)
            .map_err(|e| format!("get_device_collection: {:?}", e))?;
        let count = collection
            .get_nbr_devices()
            .map_err(|e| format!("get_nbr_devices: {:?}", e))?;

        // Retrouve le device par ID.
        let mut device = None;
        for i in 0..count {
            if let Ok(dev) = collection.get_device_at_index(i) {
                if let Ok(id) = dev.get_id() {
                    if id == device_id {
                        device = Some(dev);
                        break;
                    }
                }
            }
        }
        let device = device.ok_or_else(|| format!("Device ID not found: {device_id}"))?;
        let mut audio_client = device
            .get_iaudioclient()
            .map_err(|e| format!("get_iaudioclient: {:?}", e))?;

        // Mix format par défaut — celui négocié par Windows en shared mode.
        // C'est aussi une bonne base pour le nombre de canaux natif.
        let (mix_rate, mix_bit_depth, mix_channels) = match audio_client.get_mixformat() {
            Ok(fmt) => (
                Some(fmt.get_samplespersec()),
                Some(fmt.get_bitspersample() as u16),
                Some(fmt.get_nchannels()),
            ),
            Err(_) => (None, None, None),
        };

        // Canaux à tester : toujours stereo d'abord (99% des DACs musicaux
        // n'acceptent QUE le stereo en exclusive, même si Windows mixe en 8ch
        // pour HDMI). Ajoute mix_channels s'il diffère (pour couvrir les vrais
        // surround et le mono).
        let mut probe_channels_list: Vec<u16> = vec![2];
        if let Some(mc) = mix_channels {
            if mc != 2 {
                probe_channels_list.push(mc);
            }
        }

        let mut exclusive_rates_set = std::collections::BTreeSet::<u32>::new();
        let mut exclusive_bit_depths_set = std::collections::BTreeSet::<u16>::new();
        let mut shared_rates_set = std::collections::BTreeSet::<u32>::new();

        for &channels in &probe_channels_list {
            for &rate in PROBE_RATES {
                for &bits in PROBE_BIT_DEPTHS {
                    // Storage vs valid bits : le WASAPI convention est de
                    // stocker le 24-bit sur 32 bits (8 bits de padding). Le
                    // 16 et 32 sont storage = valid.
                    let (storage_bits, valid_bits) = match bits {
                        24 => (32usize, 24usize),
                        other => (other as usize, other as usize),
                    };
                    let fmt = WaveFormat::new(
                        storage_bits,
                        valid_bits,
                        &SampleType::Int,
                        rate as usize,
                        channels as usize,
                        None,
                    );

                    if audio_client.is_supported(&fmt, &ShareMode::Exclusive).is_ok() {
                        exclusive_rates_set.insert(rate);
                        exclusive_bit_depths_set.insert(bits);
                    }
                    // Shared mode : seuls les rates avec le mix bit-depth
                    // passent en pratique (Windows resample tout).
                    if Some(bits) == mix_bit_depth
                        && audio_client.is_supported(&fmt, &ShareMode::Shared).is_ok()
                    {
                        shared_rates_set.insert(rate);
                    }
                }
            }
        }

        Ok(WasapiCapabilities {
            exclusive_rates: exclusive_rates_set.into_iter().collect(),
            exclusive_bit_depths: exclusive_bit_depths_set.into_iter().collect(),
            shared_rates: shared_rates_set.into_iter().collect(),
            mix_rate,
            mix_bit_depth,
            mix_channels,
        })
    })
}

// ============================================================================
// Format negotiation
// ============================================================================

/// Format audio négocié et accepté par le device en mode exclusive.
#[derive(Debug, Clone, Copy)]
pub struct NegotiatedFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub sample_type: WasapiSampleType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasapiSampleType {
    /// PCM entier signé (16 ou 24 bits stockés sur 32, etc.)
    Int,
    /// PCM flottant (rare en exclusive, certains DAC l'acceptent)
    Float,
}

/// Tente de trouver un format que le DAC supporte en exclusive mode.
/// Stratégie hiérarchique :
///   1. Format natif du fichier (source_rate, 24-bit Int) — idéal bit-perfect
///   2. Sample rates audiophiles standards en cascade : 96k, 88.2k, 48k, 44.1k
///   3. Bit depths : 24 puis 16 (16 si DAC ne supporte pas 24)
///
/// Renvoie le PREMIER format accepté, ou `NoFormat` si rien ne passe.
/// Cherche un device WASAPI par friendly_name (substring match).
/// Si `preferred_name` est None ou aucun match, retombe sur le default Windows.
/// À appeler DANS un thread MTA.
fn resolve_wasapi_device(
    enumerator: &DeviceEnumerator,
    preferred_name: Option<&str>,
) -> Result<wasapi::Device, String> {
    if let Some(target) = preferred_name {
        if let Ok(collection) = enumerator.get_device_collection(&Direction::Render) {
            if let Ok(count) = collection.get_nbr_devices() {
                // Passe 1 : match EXACT (le nom CPAL complet "Nom (Fabricant)"
                // correspond typiquement exactement au friendly_name WASAPI).
                for i in 0..count {
                    if let Ok(dev) = collection.get_device_at_index(i) {
                        if let Ok(friendly) = dev.get_friendlyname() {
                            if friendly == target {
                                log::info!("🎚️  WASAPI: device ciblé (exact) '{}'", friendly);
                                return Ok(dev);
                            }
                        }
                    }
                }
                // Passe 2 : substring bidirectionnel (fallback si les noms
                // CPAL/WASAPI diffèrent légèrement). On garde le PREMIER match
                // mais seulement après avoir épuisé les matchs exacts, ce qui
                // évite qu'un nom ambigu ("Haut-parleurs") capture le mauvais
                // endpoint avant le bon.
                for i in 0..count {
                    if let Ok(dev) = collection.get_device_at_index(i) {
                        if let Ok(friendly) = dev.get_friendlyname() {
                            if friendly.contains(target) || target.contains(friendly.as_str()) {
                                log::info!(
                                    "🎚️  WASAPI: device ciblé (substring) '{}' pour '{}'",
                                    friendly, target
                                );
                                return Ok(dev);
                            }
                        }
                    }
                }
            }
        }
        log::warn!(
            "🎚️  WASAPI: device '{}' non trouvé, fallback default Windows",
            target
        );
    }
    enumerator
        .get_default_device(&Direction::Render)
        .map_err(|e| format!("get_default_device: {e:?}"))
}

pub fn try_negotiate_exclusive_format(
    source_rate: u32,
    channels: u16,
    preferred_device_name: Option<String>,
) -> Result<NegotiatedFormat, WasapiPlayerError> {
    // Délégué à un thread frais (Tauri thread = STA, incompatible MTA).
    run_on_mta_thread(move || {
        let enumerator = DeviceEnumerator::new()
            .map_err(|e| format!("DeviceEnumerator: {e:?}"))?;
        let device = resolve_wasapi_device(&enumerator, preferred_device_name.as_deref())?;
        let mut audio_client = device
            .get_iaudioclient()
            .map_err(|e| format!("get_iaudioclient: {e:?}"))?;

        // Construit la liste des candidats : sample rate source d'abord pour
        // viser le bit-perfect, puis fallback hiérarchique sur les rates
        // audiophiles standards. Doublons déduplés.
        let mut candidates_rates = vec![source_rate];
        for &rate in &[96_000u32, 88_200, 48_000, 44_100] {
            if !candidates_rates.contains(&rate) {
                candidates_rates.push(rate);
            }
        }

        // Pour chaque rate : 24-bit puis 16-bit (Int PCM uniquement).
        for rate in &candidates_rates {
            for bits in &[24u16, 16] {
                let (storage_bits, valid_bits) = if *bits == 24 { (32, 24) } else { (16, 16) };
                let fmt = WaveFormat::new(
                    storage_bits as usize,
                    valid_bits as usize,
                    &SampleType::Int,
                    *rate as usize,
                    channels as usize,
                    None,
                );
                if audio_client.is_supported(&fmt, &ShareMode::Exclusive).is_ok() {
                    log::info!(
                        "🎚️  WASAPI exclusive: format négocié {} Hz / {}-bit / {} canaux",
                        rate, bits, channels
                    );
                    // On encode le format dans un Result<NegotiatedFormat, String>
                    // car le helper run_on_mta_thread renvoie String en erreur.
                    return Ok(NegotiatedFormat {
                        sample_rate: *rate,
                        channels,
                        bits_per_sample: *bits,
                        sample_type: WasapiSampleType::Int,
                    });
                }
            }
        }
        Err(format!(
            "Aucun format exclusive trouvé pour {channels} canaux (essayés : {} Hz × 24/16-bit)",
            candidates_rates
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })
    .map_err(WasapiPlayerError::NoFormat)
}

// ============================================================================
// Player WASAPI — Render thread event-driven
// ============================================================================

/// État partagé de la voie Symphonia dont le render loop WASAPI a besoin pour
/// gérer les deux modes de lecture (comme le fait CPAL) :
/// - **LiveDecode** (`current_source == 0`) : consomme le ring buffer.
/// - **FullBuffer** (`current_source == 1`) : lit dans `full_buffer_data`.
///
/// Le décodeur bascule en FullBuffer après un court pré-remplissage (~2000
/// samples) pour TOUS les profils, donc WASAPI DOIT gérer ce mode sous peine
/// de ne rien lire (ring buffer figé après le switch).
pub struct WasapiSymphoniaState {
    pub current_source: Arc<AtomicU8>,
    pub full_buffer_data: Arc<RwLock<Vec<f32>>>,
    pub full_buffer_cursor: Arc<AtomicUsize>,
    pub is_full_buffer_ready: Arc<AtomicBool>,
    pub pending_seek_frames: Arc<AtomicUsize>,
    pub output_channels: u16,
}

/// Boucle de rendu WASAPI exclusive. À spawn dans un thread dédié.
///
/// Cette fonction :
/// 1. Ouvre l'AudioClient en exclusive mode avec le format négocié
/// 2. Crée un event handle (timing event-driven, CPU-friendly vs polling)
/// 3. Démarre le stream
/// 4. Boucle : attend l'event → pop des samples du ring buffer → écrit au DAC
/// 5. Réagit à `is_stopped` / `is_paused` / `volume`
///
/// Le `Consumer<f32>` est le côté lecture du même ring buffer alimenté par
/// le decoder thread. Pour rester drop-in avec CPAL, on consomme exactement
/// `available_frames * channels` samples par cycle.
pub fn run_wasapi_playback<C>(
    format: NegotiatedFormat,
    consumer: C,
    is_paused: Arc<AtomicBool>,
    is_stopped: Arc<AtomicBool>,
    volume: Arc<AtomicU8>,
    current_position_frames: Arc<AtomicUsize>,
    seek_flush: Arc<AtomicBool>,
    preferred_device_name: Option<String>,
    sym: WasapiSymphoniaState,
    stream_ready: Arc<AtomicBool>,
) -> Result<(), WasapiPlayerError>
where
    C: Consumer<Item = f32>,
{
    // Le signal est levé quoi qu'il arrive.
    //
    // L'initialisation du flux compte une demi-douzaine de sorties d'erreur —
    // périphérique occupé, format refusé, poignée d'événement indisponible.
    // Chacune quittait la fonction sans lever le drapeau, et l'appelant
    // attendait alors son délai complet : cinq secondes de gel pour une panne
    // connue dès la première milliseconde.
    //
    // L'enveloppe le garantit sur **toutes** les portes de sortie, y compris
    // celles qu'on ajoutera plus tard.
    let outcome = run_wasapi_playback_inner(
        format,
        consumer,
        is_paused,
        is_stopped,
        volume,
        current_position_frames,
        seek_flush,
        preferred_device_name,
        sym,
        stream_ready.clone(),
    );
    stream_ready.store(true, Ordering::Release);
    outcome
}

fn run_wasapi_playback_inner<C>(
    format: NegotiatedFormat,
    mut consumer: C,
    is_paused: Arc<AtomicBool>,
    is_stopped: Arc<AtomicBool>,
    volume: Arc<AtomicU8>,
    current_position_frames: Arc<AtomicUsize>,
    seek_flush: Arc<AtomicBool>,
    preferred_device_name: Option<String>,
    sym: WasapiSymphoniaState,
    // Levé au **premier tampon réellement écrit au périphérique**. Pas à
    // l'ouverture du flux : en mode exclusif, `start_stream` rend la main bien
    // avant que le DAC se soit verrouillé sur la fréquence. Entre les deux il
    // s'écoule une à deux secondes pendant lesquelles rien ne sort — et c'est
    // précisément l'intervalle où l'interface croyait la lecture commencée.
    stream_ready: Arc<AtomicBool>,
) -> Result<(), WasapiPlayerError>
where
    C: Consumer<Item = f32>,
{
    // À appeler depuis un thread dédié au playback. Si ce thread est dans STA
    // (par ex. accidentellement réutilisé), l'init MTA va échouer ici proprement.
    // Chronomètre interne au fil de rendu.
    //
    // Vu du dehors, l'ouverture du périphérique prend deux millisecondes : ce
    // n'est que le lancement de ce fil. Tout le délai réel — près de deux
    // secondes mesurées — se passe ici, et il fallait le découper pour savoir
    // laquelle de ces six étapes le porte.
    let mut phases = crate::core::audio_player::startup_timer::StartupTimer::new();

    ensure_com_initialized_current_thread().map_err(WasapiPlayerError::ComInit)?;
    phases.mark("init COM");

    // ─── 1. Ouvrir le device + AudioClient ───
    // On cible le device sélectionné par l'utilisateur si possible (matching
    // par friendly_name substring), sinon fallback sur le default Windows.
    let enumerator = DeviceEnumerator::new()
        .map_err(|e| WasapiPlayerError::NoDevice(format!("DeviceEnumerator: {e:?}")))?;
    let device = resolve_wasapi_device(&enumerator, preferred_device_name.as_deref())
        .map_err(WasapiPlayerError::NoDevice)?;
    let mut audio_client = device
        .get_iaudioclient()
        .map_err(|e| WasapiPlayerError::NoDevice(format!("get_iaudioclient: {e:?}")))?;

    phases.mark("ouverture du device");

    // ─── 2. WaveFormat exclusive ───
    let (storage_bits, valid_bits) = if format.bits_per_sample == 24 {
        (32, 24)
    } else {
        (format.bits_per_sample, format.bits_per_sample)
    };
    let wave_fmt = WaveFormat::new(
        storage_bits as usize,
        valid_bits as usize,
        &SampleType::Int,
        format.sample_rate as usize,
        format.channels as usize,
        None,
    );
    let bytes_per_frame = wave_fmt.get_blockalign() as usize;

    // ─── 3. Période device ───
    // En exclusive event-driven, period_hns = buffer_duration. On vise ~20 ms
    // (équilibre entre latence et stabilité). calculate_aligned_period_near
    // ajuste à la période minimale supportée + alignement bytes (souvent 128
    // bytes pour Intel HD Audio).
    let desired_period_hns: i64 = 200_000; // 20 ms en unités de 100 ns
    let period_hns = audio_client
        .calculate_aligned_period_near(desired_period_hns, Some(128), &wave_fmt)
        .map_err(|e| WasapiPlayerError::InitFailed(format!("calculate_aligned_period: {e:?}")))?;

    phases.mark("format et période");

    // ─── 4. Initialize en exclusive event-driven ───
    let stream_mode = StreamMode::EventsExclusive { period_hns };
    audio_client
        .initialize_client(&wave_fmt, &Direction::Render, &stream_mode)
        .map_err(|e| WasapiPlayerError::InitFailed(format!(
            "initialize_client (likely device busy or format rejected): {e:?}"
        )))?;

    // C'est ici que Windows reprend l'endpoint à son mixeur : s'il est déjà
    // ouvert en mode partagé par une autre application, il faut d'abord l'en
    // déloger.
    phases.mark("prise exclusive du périphérique");

    // ─── 5. Event handle + render client + démarrage ───
    let event_handle = audio_client
        .set_get_eventhandle()
        .map_err(|e| WasapiPlayerError::InitFailed(format!("set_get_eventhandle: {e:?}")))?;
    let render_client = audio_client
        .get_audiorenderclient()
        .map_err(|e| WasapiPlayerError::InitFailed(format!("get_audiorenderclient: {e:?}")))?;

    audio_client
        .start_stream()
        .map_err(|e| WasapiPlayerError::InitFailed(format!("start_stream: {e:?}")))?;

    phases.mark("démarrage du flux");

    log::info!(
        "▶️  WASAPI exclusive: stream démarré ({} Hz, {}-bit, {} ch, period {} ms)",
        format.sample_rate,
        format.bits_per_sample,
        format.channels,
        period_hns / 10_000
    );

    // ─── 6. Boucle de rendu ───
    let mut interleaved_f32: Vec<f32> = Vec::new();
    let mut output_bytes: Vec<u8> = Vec::new();
    let mut fade_in_samples: usize = 0; // fade-in post-seek (anti-clic)

    // Diagnostic (premières ~2 s) : compteurs pour comprendre un éventuel
    // silence. On log une synthèse après ~100 cycles puis on arrête.
    let mut first_buffer_written = false;
    let mut diag_cycles: u32 = 0;
    let mut diag_frames_written: u64 = 0;
    let mut diag_underruns: u32 = 0;
    let mut diag_peak: f32 = 0.0;
    let mut diag_done = false;

    loop {
        // Sortie propre si l'utilisateur stop la lecture
        if is_stopped.load(Ordering::Relaxed) {
            break;
        }

        // Attend que le DAC réclame des samples (timeout 100 ms pour permettre
        // le check de is_stopped). Quand l'event est signalé, on a typiquement
        // ~20 ms (= period) pour remplir le buffer.
        if event_handle.wait_for_event(100).is_err() {
            // Timeout ou event invalidé → on re-check les flags et on continue
            continue;
        }

        // Combien de frames le DAC est prêt à accepter ?
        let available = match audio_client.get_available_space_in_frames() {
            Ok(n) => n as usize,
            Err(e) => {
                log::warn!("get_available_space_in_frames failed: {e:?}");
                continue;
            }
        };
        if available == 0 {
            continue;
        }

        let needed_samples = available * format.channels as usize;

        // En pause : on remplit avec du silence pour maintenir la stream
        // vivante (sinon le DAC peut nous virer). Les samples décodés
        // s'accumulent dans le ring buffer en attendant.
        if is_paused.load(Ordering::Relaxed) {
            output_bytes.clear();
            output_bytes.resize(available * bytes_per_frame, 0);
            if let Err(e) = render_client.write_to_device(available, &output_bytes, None) {
                log::warn!("write silence: {e:?}");
            }
            continue;
        }

        // --- Seek flush : drainer + repositionner + silence + fade-in ---
        // Le decoder thread a déjà fait format.seek + resampler.reset. Ici
        // on jette le ring buffer, on repositionne le curseur FullBuffer
        // depuis pending_seek_frames (comme CPAL), puis silence + fade-in.
        if seek_flush.load(Ordering::Acquire) {
            let to_skip = consumer.occupied_len();
            consumer.skip(to_skip);

            let target = sym.pending_seek_frames.load(Ordering::Acquire);
            let new_frames = if target != usize::MAX {
                sym.pending_seek_frames.store(usize::MAX, Ordering::Relaxed);
                current_position_frames.store(target, Ordering::Relaxed);
                target
            } else {
                current_position_frames.load(Ordering::Relaxed)
            };
            let new_cursor = new_frames * sym.output_channels as usize;
            sym.full_buffer_cursor.store(new_cursor, Ordering::Relaxed);

            seek_flush.store(false, Ordering::Release);
            fade_in_samples = 2048;
            output_bytes.clear();
            output_bytes.resize(available * bytes_per_frame, 0);
            if let Err(e) = render_client.write_to_device(available, &output_bytes, None) {
                log::warn!("write silence post-seek: {e:?}");
            }
            continue;
        }

        // --- Lecture des samples selon le mode source (comme CPAL) ---
        // source==0 : LiveDecode (ring buffer). source==1 : FullBuffer (le
        // décodeur y bascule après pré-remplissage — c'est le mode nominal).
        interleaved_f32.clear();
        interleaved_f32.resize(needed_samples, 0.0);
        let source_mode = sym.current_source.load(Ordering::Relaxed);
        let popped: usize = if source_mode == 0 {
            // LiveDecode : consomme le ring buffer.
            let n = consumer.pop_slice(&mut interleaved_f32);
            if n < needed_samples {
                for s in interleaved_f32[n..].iter_mut() {
                    *s = 0.0;
                }
            }
            n
        } else {
            // FullBuffer : lit dans full_buffer_data[cursor..].
            if sym.is_full_buffer_ready.load(Ordering::Relaxed) {
                if let Ok(fb) = sym.full_buffer_data.read() {
                    let cursor = sym.full_buffer_cursor.load(Ordering::Relaxed);
                    let available_samples = fb.len().saturating_sub(cursor);
                    let n = available_samples.min(needed_samples);
                    if n > 0 {
                        interleaved_f32[..n].copy_from_slice(&fb[cursor..cursor + n]);
                        sym.full_buffer_cursor.fetch_add(n, Ordering::Relaxed);
                    }
                    if n < needed_samples {
                        for s in interleaved_f32[n..].iter_mut() {
                            *s = 0.0;
                        }
                    }
                    n
                } else {
                    0
                }
            } else {
                0
            }
        };

        // Mise à jour de la position (source-aware) — voir plus bas.
        if popped > 0 {
            if source_mode == 1 {
                // FullBuffer : la position dérive du curseur (source de vérité).
                let cursor = sym.full_buffer_cursor.load(Ordering::Relaxed);
                current_position_frames
                    .store(cursor / sym.output_channels as usize, Ordering::Relaxed);
            }
        }

        // --- Diagnostic silence (premiers cycles) ---
        if !diag_done {
            diag_cycles += 1;
            diag_frames_written += available as u64;
            if popped == 0 {
                diag_underruns += 1;
            }
            for s in interleaved_f32.iter().take(popped) {
                let a = s.abs();
                if a > diag_peak {
                    diag_peak = a;
                }
            }
            if diag_cycles >= 100 {
                log::info!(
                    "🔍 WASAPI diag: {} cycles, {} frames écrites, {} underruns, peak amplitude {:.4}, vol={}, available/cycle≈{}",
                    diag_cycles,
                    diag_frames_written,
                    diag_underruns,
                    diag_peak,
                    volume.load(Ordering::Relaxed),
                    available,
                );
                diag_done = true;
            }
        }

        // Mise à jour position en LiveDecode uniquement (le FullBuffer a déjà
        // mis à jour depuis le curseur plus haut).
        if popped > 0 && source_mode == 0 {
            let frames_played = popped / format.channels as usize;
            current_position_frames.fetch_add(frames_played, Ordering::Relaxed);
        }

        // Applique volume + Replay Gain + fade-in post-seek + clipping.
        // NB : ce chemin PCM applique déjà le volume logiciel ; le Replay Gain
        // suit la même règle. La voie DoP (plus bas) n'applique ni l'un ni
        // l'autre et reste bit-perfect.
        let vol = volume.load(Ordering::Relaxed) as f32 / 100.0
            * crate::core::audio_player::replay_gain::current_factor();
        for s in interleaved_f32.iter_mut() {
            let fade = if fade_in_samples > 0 {
                fade_in_samples -= 1;
                (2048 - fade_in_samples) as f32 / 2048.0
            } else {
                1.0
            };
            *s = (*s * vol * fade * 0.98).clamp(-1.0, 1.0);
        }

        // Convertit f32 → format cible (Int 16 ou 24-stored-on-32)
        output_bytes.clear();
        output_bytes.reserve(available * bytes_per_frame);
        match (format.bits_per_sample, format.sample_type) {
            (16, WasapiSampleType::Int) => {
                for s in &interleaved_f32 {
                    let v = (*s * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    output_bytes.extend_from_slice(&v.to_le_bytes());
                }
            }
            (24, WasapiSampleType::Int) => {
                // Stockage 32-bit, valid bits = 24. On shift de 8 pour mettre
                // les 24 bits significatifs dans les bits hauts du i32.
                for s in &interleaved_f32 {
                    let v = (*s * 8_388_607.0).clamp(-8_388_608.0, 8_388_607.0) as i32;
                    let v32 = v << 8;
                    output_bytes.extend_from_slice(&v32.to_le_bytes());
                }
            }
            _ => {
                return Err(WasapiPlayerError::Runtime(format!(
                    "Format inattendu : {}-bit {:?}",
                    format.bits_per_sample, format.sample_type
                )));
            }
        }

        match render_client.write_to_device(available, &output_bytes, None) {
            Ok(()) => {
                // Le premier tampon est parti : à cet instant, et pas avant, la
                // lecture a réellement commencé.
                if !first_buffer_written {
                    first_buffer_written = true;
                    stream_ready.store(true, Ordering::Release);
                    phases.mark("attente du premier événement");
                    phases.finish("fil de rendu WASAPI");
                }
            }
            Err(e) => log::warn!("write_to_device: {e:?}"),
        }
    }

    // ─── 7. Arrêt propre ───
    if let Err(e) = audio_client.stop_stream() {
        log::warn!("stop_stream: {e:?}");
    }
    log::info!("⏹  WASAPI exclusive: stream arrêté");

    // Best-effort drain pour ne pas laisser des samples coincés en buffer.
    std::thread::sleep(Duration::from_millis(50));

    Ok(())
}

// ============================================================================
// Player WASAPI — Render thread DoP (DSD over PCM), passthrough bit-perfect
// ============================================================================

/// Écrit un bloc de `frames` trames DoP au DAC en posant le marqueur
/// (0x05/0xFA) sur CHAQUE trame via un compteur unique et continu `marker_b`.
///
/// `payloads` = échantillons payload interleavés (sans marqueur, sortie de
/// l'encodeur). S'il est plus court que `frames * channels`, le reste est
/// complété par du **silence DoP** — c'est ce qui garantit la continuité du
/// marqueur à la jonction silence↔musique (une seule source de marqueur pour
/// tout le stream → jamais deux marqueurs identiques consécutifs → pas de clic).
#[inline]
fn write_dop_block(
    render_client: &wasapi::AudioRenderClient,
    output_bytes: &mut Vec<u8>,
    payloads: &[i32],
    channels: usize,
    marker_b: &mut bool,
    frames: usize,
) {
    use crate::core::audio_decoder::dsd::dop_encoder::{dop_silence_payload, stamp_marker};
    output_bytes.clear();
    output_bytes.reserve(frames * channels * 4);
    let mut idx = 0usize;
    for _ in 0..frames {
        let mk = *marker_b;
        for _ in 0..channels {
            let p = if idx < payloads.len() {
                payloads[idx]
            } else {
                dop_silence_payload()
            };
            output_bytes.extend_from_slice(&stamp_marker(p, mk).to_le_bytes());
            idx += 1;
        }
        *marker_b = !*marker_b; // le marqueur alterne à CHAQUE trame
    }
    let _ = render_client.write_to_device(frames, output_bytes, None);
}

/// Boucle de rendu WASAPI exclusive en mode **DoP** (DSD over PCM).
///
/// Contrairement au chemin f32, les échantillons sont déjà encodés en DoP par
/// le décodeur (marqueurs + DSD 1-bit dans des trames 24-bit). Le rôle de
/// cette boucle est de les écrire **verbatim** au DAC — **aucun** volume,
/// fade ou clipping (ça corromprait les marqueurs et le DAC perdrait le DSD).
///
/// Le volume logiciel est donc inopérant en DoP : le réglage se fait
/// physiquement sur le DAC/ampli (bit-perfect obligatoire).
/// Contrôle partagé du render DoP persistant (moteur gapless).
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
    /// `true` dès que le stream WASAPI a démarré (init réussi). Sert au moteur
    /// pour confirmer que `new()` a bien ouvert le device.
    pub started_ok: Arc<AtomicBool>,
}

pub fn run_wasapi_dop_playback<C>(
    carrier_rate: u32,
    channels: u16,
    mut consumer: C,
    preferred_device_name: Option<String>,
    ctl: DopRenderCtl,
) -> Result<(), WasapiPlayerError>
where
    C: Consumer<Item = i32>,
{
    let DopRenderCtl {
        is_paused,
        render_stop: is_stopped,
        music_frames_played: current_position_frames,
        seek_flush,
        drain_request,
        audio_started,
        started_ok,
    } = ctl;
    ensure_com_initialized_current_thread().map_err(WasapiPlayerError::ComInit)?;

    // ─── 1. Device + AudioClient (device sélectionné, fallback default) ───
    let enumerator = DeviceEnumerator::new()
        .map_err(|e| WasapiPlayerError::NoDevice(format!("DeviceEnumerator: {e:?}")))?;
    let device = resolve_wasapi_device(&enumerator, preferred_device_name.as_deref())
        .map_err(WasapiPlayerError::NoDevice)?;
    let mut audio_client = device
        .get_iaudioclient()
        .map_err(|e| WasapiPlayerError::NoDevice(format!("get_iaudioclient: {e:?}")))?;

    // ─── 2. WaveFormat 24-bit (stocké sur 32) au rate porteur DoP ───
    let wave_fmt = WaveFormat::new(32, 24, &SampleType::Int, carrier_rate as usize, channels as usize, None);
    let bytes_per_frame = wave_fmt.get_blockalign() as usize;

    // ─── 3. Période + init exclusive event-driven ───
    let desired_period_hns: i64 = 200_000; // ~20 ms
    let period_hns = audio_client
        .calculate_aligned_period_near(desired_period_hns, Some(128), &wave_fmt)
        .map_err(|e| WasapiPlayerError::InitFailed(format!("calculate_aligned_period: {e:?}")))?;
    let stream_mode = StreamMode::EventsExclusive { period_hns };
    audio_client
        .initialize_client(&wave_fmt, &Direction::Render, &stream_mode)
        .map_err(|e| WasapiPlayerError::InitFailed(format!(
            "initialize_client DoP (device busy ou rate {carrier_rate} rejeté): {e:?}"
        )))?;

    let event_handle = audio_client
        .set_get_eventhandle()
        .map_err(|e| WasapiPlayerError::InitFailed(format!("set_get_eventhandle: {e:?}")))?;
    let render_client = audio_client
        .get_audiorenderclient()
        .map_err(|e| WasapiPlayerError::InitFailed(format!("get_audiorenderclient: {e:?}")))?;

    // ─── 4. Buffers réutilisés + marqueur DoP unique et continu ───
    let _ = bytes_per_frame; // (info seulement — write_dop_block gère l'encodage)
    let mut interleaved: Vec<i32> = Vec::new();
    let mut output_bytes: Vec<u8> = Vec::new();
    // Compteur de marqueur DoP UNIQUE pour tout le stream (silence + musique).
    // Garantit l'alternance 0x05/0xFA ininterrompue → aucune jonction invalide.
    let mut marker_b = false;

    // ─── PRÉ-FILL AVANT start_stream (crucial anti-clic) ───
    // En WASAPI exclusive event-driven, le buffer doit être rempli AVANT
    // `start_stream`, sinon le DAC joue le contenu indéfini du buffer pendant
    // le 1er cycle → clic au lock DSD. On le remplit de silence DoP valide :
    // le DAC verrouille proprement le DSD sur ce silence dès la 1re frame.
    if let Ok(buffer_frames) = audio_client.get_available_space_in_frames() {
        let bf = buffer_frames as usize;
        if bf > 0 {
            write_dop_block(&render_client, &mut output_bytes, &[], channels as usize, &mut marker_b, bf);
        }
    }

    audio_client
        .start_stream()
        .map_err(|e| WasapiPlayerError::InitFailed(format!("start_stream: {e:?}")))?;
    started_ok.store(true, Ordering::Relaxed);

    log::info!(
        "▶️  WASAPI DoP: stream démarré (porteur {} Hz, 24-bit, {} ch, period {} ms) — DSD natif",
        carrier_rate, channels, period_hns / 10_000
    );

    // ─── Warm-up : silence DSD le temps que le DAC verrouille ───
    // Un DAC DSD se MUTE pendant qu'il acquiert le lock (jusqu'à ~1-2 s selon le
    // modèle). Si on envoie la musique tout de suite, ce début est perdu
    // (inaudible). On écrit donc du silence DSD pendant `warmup_frames` SANS
    // consommer la musique, le temps que le DAC verrouille. La musique n'est
    // consommée qu'ensuite → aucun début perdu. Pendant ce temps le compteur
    // reste figé (le thread orchestrateur garde `playback-preparing: true`
    // jusqu'à ce que `audio_started` passe à true).
    let warmup_frames: usize = (carrier_rate as f64 * 2.5) as usize; // ~2.5 s (lock DAC)
    let mut warmed: usize = 0;

    loop {
        if is_stopped.load(Ordering::Relaxed) {
            // Settle : ~40 ms de silence DoP AVANT de couper, pour que le DAC
            // quitte le mode DSD proprement (stop abrupt en plein flux → clic).
            for _ in 0..2 {
                if event_handle.wait_for_event(100).is_err() {
                    break;
                }
                if let Ok(avail) = audio_client.get_available_space_in_frames() {
                    let avail = avail as usize;
                    if avail > 0 {
                        write_dop_block(&render_client, &mut output_bytes, &[], channels as usize, &mut marker_b, avail);
                    }
                }
            }
            break;
        }
        if event_handle.wait_for_event(100).is_err() {
            continue;
        }
        let available = match audio_client.get_available_space_in_frames() {
            Ok(n) => n as usize,
            Err(e) => {
                log::warn!("DoP get_available_space: {e:?}");
                continue;
            }
        };
        if available == 0 {
            continue;
        }
        let needed = available * channels as usize;

        // Warm-up : silence DSD tant que le DAC n'a pas verrouillé. On ne
        // consomme PAS la musique → le début n'est pas perdu. `audio_started`
        // signale la fin du warm-up (dé-fige le compteur côté frontend).
        if warmed < warmup_frames {
            write_dop_block(&render_client, &mut output_bytes, &[], channels as usize, &mut marker_b, available);
            warmed += available;
            if warmed >= warmup_frames {
                audio_started.store(true, Ordering::Relaxed);
                log::debug!("🎚️  [DoP] Warm-up terminé ({} frames) — lecture musique", warmed);
            }
            continue;
        }

        // Pause : silence DoP (le DAC reste locké en DSD).
        if is_paused.load(Ordering::Relaxed) {
            write_dop_block(&render_client, &mut output_bytes, &[], channels as usize, &mut marker_b, available);
            continue;
        }

        // Drain (changement de piste) : jeter la queue de la piste précédente,
        // écrire du silence. Le stream reste vivant (DAC locké).
        if drain_request.load(Ordering::Acquire) {
            let to_skip = consumer.occupied_len();
            consumer.skip(to_skip);
            drain_request.store(false, Ordering::Release);
            write_dop_block(&render_client, &mut output_bytes, &[], channels as usize, &mut marker_b, available);
            continue;
        }

        // Seek flush : drainer + silence le temps du re-remplissage.
        if seek_flush.load(Ordering::Acquire) {
            let to_skip = consumer.occupied_len();
            consumer.skip(to_skip);
            seek_flush.store(false, Ordering::Release);
            write_dop_block(&render_client, &mut output_bytes, &[], channels as usize, &mut marker_b, available);
            continue;
        }

        // Pop des payloads DoP (sans marqueur — posé ci-dessous par write_dop_block).
        interleaved.clear();
        interleaved.resize(needed, 0);
        let popped = consumer.pop_slice(&mut interleaved);
        // On aligne sur une frame complète (pop_slice peut s'arrêter au milieu).
        let popped_frames = popped / channels as usize;
        let popped_aligned = popped_frames * channels as usize;

        // Position : chaque trame porteuse = 16 samples DSD ; frames/carrier = secondes.
        if popped_frames > 0 {
            current_position_frames.fetch_add(popped_frames, Ordering::Relaxed);
        }

        // Écriture : les frames réelles + complétion silence si underrun. Le
        // marqueur est posé en continu par write_dop_block (une seule source).
        write_dop_block(
            &render_client,
            &mut output_bytes,
            &interleaved[..popped_aligned],
            channels as usize,
            &mut marker_b,
            available,
        );
    }

    if let Err(e) = audio_client.stop_stream() {
        log::warn!("DoP stop_stream: {e:?}");
    }
    log::info!("⏹  WASAPI DoP: stream arrêté");
    std::thread::sleep(Duration::from_millis(50));
    Ok(())
}

/// Vérifie qu'un DAC accepte le format porteur DoP (24-bit au `carrier_rate`)
/// en mode exclusive. Sert à décider DoP vs fallback DSD2PCM avant lecture.
pub fn dop_format_supported(carrier_rate: u32, channels: u16, device_name: Option<String>) -> bool {
    run_on_mta_thread(move || {
        let enumerator = DeviceEnumerator::new()
            .map_err(|e| format!("DeviceEnumerator: {e:?}"))?;
        let device = resolve_wasapi_device(&enumerator, device_name.as_deref())?;
        let mut audio_client = device
            .get_iaudioclient()
            .map_err(|e| format!("get_iaudioclient: {e:?}"))?;
        let fmt = WaveFormat::new(32, 24, &SampleType::Int, carrier_rate as usize, channels as usize, None);
        Ok(audio_client.is_supported(&fmt, &ShareMode::Exclusive).is_ok())
    })
    .unwrap_or(false)
}
