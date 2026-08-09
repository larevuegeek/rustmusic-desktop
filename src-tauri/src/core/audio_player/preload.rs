//! Préchargement de la piste suivante — le « troisième tampon ».
//!
//! # Rôle
//! Le lecteur possède déjà deux tampons : le **ring buffer** (démarrage) et le
//! **FullBuffer** (la piste en cours, entièrement décodée en RAM, ce qui donne
//! le seek instantané). Ce module en ajoute un troisième, **à côté** des deux
//! autres : une réserve contenant la piste **suivante**, décodée d'avance.
//!
//! Aucun des deux mécanismes existants n'est modifié : tant qu'aucune piste
//! n'est préchargée, la lecture se comporte exactement comme avant.
//!
//! # L'enchaînement sans blanc
//! Quand le curseur atteint la fin du FullBuffer, l'audio correspondant est
//! déjà parti dans le tampon du système : il reste 10 à 40 ms avant que le DAC
//! ne manque de données. C'est dans cette fenêtre que la **promotion** a lieu.
//!
//! Promouvoir ne copie rien : le `Vec` préchargé **remplace** celui du
//! FullBuffer (un déplacement, en temps constant) et le curseur repart à zéro.
//! L'opération se fait sous le verrou en écriture, donc de façon atomique
//! vis-à-vis du callback audio, qui lit données et curseur sous son propre
//! verrou de lecture. Le callback n'a pas une ligne à changer.
//!
//! # Ce qui est délibérément conservateur
//! - **Format identique exigé** (fréquence source et canaux) : un album est
//!   homogène, et accepter un format différent obligerait à rééchantillonner,
//!   ce qui ruinerait le bit-perfect sans prévenir.
//! - **Plafond mémoire** : au-delà, on renonce au préchargement plutôt que de
//!   faire gonfler la RAM (le FullBuffer en garde déjà une piste entière).
//! - **Annulation gratuite** : tant que la piste n'est pas promue, la jeter ne
//!   coûte rien et ne s'entend pas — c'est ce qui rend un changement de file
//!   d'attente inoffensif.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use symphonia::core::audio::GenericAudioBufferRef;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::{get_codecs, get_probe};

use crate::core::audio_player::audio_utils::{adapt_channels, convert_audio_buffer_to_interleaved};
use crate::core::audio_player::replay_gain;
use crate::core::audio_resampler::resampler::Resampler;

/// Plafond mémoire du préchargement. Au-delà, on n'enchaîne pas : mieux vaut
/// un court silence entre deux pistes qu'un doublement de RAM sur des fichiers
/// haute résolution multicanaux.
const MAX_PRELOAD_BYTES: usize = 256 * 1024 * 1024;

/// Format que la piste suivante doit respecter pour pouvoir être enchaînée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamFormat {
    /// Fréquence du **fichier source** (pas de la sortie) : c'est elle qui
    /// détermine si un rééchantillonnage serait nécessaire.
    pub source_rate: u32,
    /// Canaux du fichier source.
    pub source_channels: usize,
    /// Fréquence de sortie effective du stream ouvert.
    pub output_rate: u32,
    /// Canaux de sortie effectifs du stream ouvert.
    pub output_channels: u16,
}

/// Une piste décodée, prête à être promue.
pub struct PreloadedTrack {
    pub path: PathBuf,
    /// Échantillons f32 interleavés, déjà au format de sortie.
    pub samples: Vec<f32>,
    /// Durée en secondes, pour mettre à jour la barre de progression.
    pub duration_secs: f64,
    /// Facteur Replay Gain de CETTE piste, publié à la promotion.
    pub gain: f32,
}

/// État global du préchargement (une seule lecture à la fois dans l'app).
struct PreloadState {
    /// Chemin annoncé par le frontend comme étant la piste suivante.
    next_path: Option<PathBuf>,
    /// Piste décodée et prête à être promue.
    ready: Option<PreloadedTrack>,
}

static STATE: Mutex<PreloadState> = Mutex::new(PreloadState {
    next_path: None,
    ready: None,
});

/// Génération courante de l'annonce. Toute invalidation l'incrémente ; le
/// décodeur de préchargement capture la sienne au démarrage et abandonne dès
/// qu'elle change.
///
/// Un simple drapeau « annuler » ne suffisait pas : entre le lancement du
/// thread et sa première instruction, une annulation pouvait être écrasée par
/// la remise à zéro du drapeau, et le thread décodait alors une piste déjà
/// périmée.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Réglage utilisateur « lecture sans blanc ». Lu par les threads audio, donc
/// stocké en atomique et synchronisé avec la BDD au boot puis à chaque
/// changement (même schéma que la préférence de sortie exclusive).
static GAPLESS_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn gapless_enabled() -> bool {
    GAPLESS_ENABLED.load(Ordering::Relaxed)
}

pub fn set_gapless_enabled(enabled: bool) {
    GAPLESS_ENABLED.store(enabled, Ordering::Relaxed);
    log::info!(
        "🔗 Lecture sans blanc (gapless) : {}",
        if enabled { "ON" } else { "OFF" }
    );
    if !enabled {
        reset();
    }
}

/// Une piste est-elle prête à être promue ? (sans la consommer)
pub fn is_ready() -> bool {
    lock().ready.is_some()
}

fn lock() -> std::sync::MutexGuard<'static, PreloadState> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Annonce la piste qui suivra. Appelé par le frontend à chaque fois que la
/// « suite » change : lecture d'un nouveau morceau, réordonnancement de la
/// file, shuffle, repeat, ajout ou retrait.
///
/// Toute annonce différente de la précédente **invalide** le préchargement en
/// cours : la piste décodée n'est plus la bonne. Comme elle n'a pas encore été
/// promue, la jeter est inaudible.
pub fn announce_next(path: Option<PathBuf>) {
    let mut st = lock();
    if st.next_path == path {
        return;
    }

    st.next_path = path.clone();
    st.ready = None;
    drop(st);

    // Toute annonce différente périme le décodage en cours.
    GENERATION.fetch_add(1, Ordering::Relaxed);

    match path {
        Some(p) => log::debug!("⏭  Piste suivante annoncée : {}", p.display()),
        None => log::debug!("⏭  Plus de piste suivante annoncée"),
    }
}

/// Chemin annoncé, s'il y en a un.
pub fn announced_next() -> Option<PathBuf> {
    lock().next_path.clone()
}

/// Récupère la piste préchargée si elle correspond toujours à l'annonce.
///
/// Consomme la réserve **et** l'annonce : la piste devient celle qui joue,
/// donc elle n'est plus « la suivante ». Sans ça, on relancerait aussitôt un
/// préchargement de la piste qu'on vient de promouvoir.
pub fn take_ready() -> Option<PreloadedTrack> {
    let mut st = lock();
    let expected = st.next_path.clone()?;
    let taken = match st.ready.take() {
        Some(t) if t.path == expected => Some(t),
        // Décalage entre l'annonce et ce qui a été décodé (la file a changé
        // pendant le décodage) : on jette.
        _ => None,
    };
    if taken.is_some() {
        st.next_path = None;
        drop(st);
        GENERATION.fetch_add(1, Ordering::Relaxed);
    }
    taken
}

/// Jette le décodage en cours mais **conserve** l'annonce : utilisé en fin de
/// piste, quand le frontend n'a pas encore eu le temps d'annoncer la suite.
pub fn abort_current_decode() {
    lock().ready = None;
    GENERATION.fetch_add(1, Ordering::Relaxed);
}

/// Vide entièrement la réserve (stop, changement manuel de piste).
pub fn reset() {
    let mut st = lock();
    st.next_path = None;
    st.ready = None;
    drop(st);
    GENERATION.fetch_add(1, Ordering::Relaxed);
}

/// Décode la piste annoncée au format du stream en cours et la met en réserve.
///
/// À lancer dans un thread dédié pendant la lecture de la piste courante. Tout
/// échec est silencieux et sans conséquence : on retombe simplement sur
/// l'enchaînement normal, avec son court silence.
pub fn run_preload(format: StreamFormat, is_stopped: Arc<AtomicBool>) {
    // Génération capturée AVANT de lire l'annonce : si elle change en cours de
    // route, c'est que la file a bougé et que ce décodage n'a plus d'objet.
    let generation = GENERATION.load(Ordering::Relaxed);

    let Some(path) = announced_next() else {
        return;
    };

    match decode_track(&path, format, &is_stopped, generation) {
        Ok(Some(track)) => {
            let mut st = lock();
            // La file a pu changer pendant le décodage : on ne met en réserve
            // que si l'annonce est toujours la même ET que la génération n'a
            // pas bougé.
            if st.next_path.as_deref() == Some(path.as_path())
                && GENERATION.load(Ordering::Relaxed) == generation
            {
                log::info!(
                    "📦 Piste suivante préchargée : {} ({:.1} Mo, {:.1}s)",
                    path.display(),
                    (track.samples.len() * 4) as f64 / (1024.0 * 1024.0),
                    track.duration_secs
                );
                st.ready = Some(track);
            }
        }
        Ok(None) => {} // Incompatible ou annulé : pas d'enchaînement, sans bruit.
        Err(e) => log::debug!("📦 Préchargement de {} abandonné : {e}", path.display()),
    }
}

/// Décode un fichier entier en mémoire, au format de sortie donné.
///
/// Retourne `Ok(None)` quand la piste ne peut pas être enchaînée : format
/// différent, trop volumineuse, ou décodage interrompu.
fn decode_track(
    path: &Path,
    fmt: StreamFormat,
    is_stopped: &AtomicBool,
    generation: u64,
) -> Result<Option<PreloadedTrack>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("ouverture : {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut reader: Box<dyn FormatReader> = get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| format!("probe : {e}"))?;

    // Replay Gain : calculé ici, publié seulement à la promotion.
    let gain = reader
        .metadata()
        .current()
        .map(replay_gain::from_metadata)
        .map(replay_gain::factor_for)
        .unwrap_or(1.0);

    let track = reader
        .tracks()
        .iter()
        .find(|t| {
            t.codec_params
                .as_ref()
                .and_then(|c| c.audio())
                .map_or(false, |a| a.sample_rate.is_some())
        })
        .ok_or("aucune piste audio")?;

    let track_id = track.id;
    let codec_params = track
        .codec_params
        .as_ref()
        .and_then(|c| c.audio())
        .cloned()
        .ok_or("paramètres codec absents")?;

    let source_rate = codec_params.sample_rate.ok_or("pas de sample rate")?;
    let channels = codec_params
        .channels
        .as_ref()
        .ok_or("pas d'info canaux")?
        .count();

    // Exigence de format : un écart imposerait un rééchantillonnage silencieux,
    // donc la perte du bit-perfect. On préfère renoncer à l'enchaînement.
    if source_rate != fmt.source_rate || channels != fmt.source_channels {
        log::info!(
            "⏭  Pas d'enchaînement : {} est en {} Hz / {} ch (piste en cours : {} Hz / {} ch)",
            path.display(),
            source_rate,
            channels,
            fmt.source_rate,
            fmt.source_channels
        );
        return Ok(None);
    }

    // Estimation de l'empreinte avant de décoder quoi que ce soit.
    let duration_secs = estimate_duration(&reader, track_id, source_rate);
    let estimated_bytes =
        (duration_secs * fmt.output_rate as f64 * fmt.output_channels as f64 * 4.0) as usize;
    if estimated_bytes > MAX_PRELOAD_BYTES {
        log::info!(
            "⏭  Pas d'enchaînement : {} pèserait {:.0} Mo décodée (plafond {:.0} Mo)",
            path.display(),
            estimated_bytes as f64 / (1024.0 * 1024.0),
            MAX_PRELOAD_BYTES as f64 / (1024.0 * 1024.0)
        );
        return Ok(None);
    }

    let mut decoder: Box<dyn AudioDecoder> = get_codecs()
        .make_audio_decoder(&codec_params, &AudioDecoderOptions::default())
        .map_err(|e| format!("création décodeur : {e:?}"))?;

    let mut resampler: Option<Resampler> = Resampler::maybe_new_with_profile(
        source_rate,
        fmt.output_rate,
        channels,
        crate::core::audio_quality::current_profile(),
    )?;

    let mut samples: Vec<f32> = Vec::with_capacity(estimated_bytes / 4);

    loop {
        if GENERATION.load(Ordering::Relaxed) != generation
            || is_stopped.load(Ordering::Relaxed)
        {
            return Ok(None);
        }

        let packet = match reader.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break, // fin du fichier
            Err(e) => {
                log::debug!("📦 Préchargement interrompu (lecture packet) : {e:?}");
                break;
            }
        };
        if packet.track_id != track_id {
            continue;
        }

        let buf: GenericAudioBufferRef<'_> = match decoder.decode(&packet) {
            Ok(b) => b,
            Err(_) => continue, // paquet abîmé : on l'ignore, comme le lecteur
        };

        let decoded = convert_audio_buffer_to_interleaved(&buf, channels);
        if decoded.is_empty() {
            continue;
        }

        let resampled = match resampler.as_mut() {
            Some(rs) => rs.process_interleaved(&decoded),
            None => decoded,
        };

        if channels as u16 != fmt.output_channels {
            let mut out =
                vec![0.0f32; (resampled.len() / channels) * fmt.output_channels as usize];
            adapt_channels(&resampled, channels, &mut out, fmt.output_channels.into());
            samples.extend_from_slice(&out);
        } else {
            samples.extend_from_slice(&resampled);
        }

        // Garde-fou : une durée mal estimée ne doit pas faire exploser la RAM.
        if samples.len() * 4 > MAX_PRELOAD_BYTES {
            log::info!(
                "⏭  Préchargement de {} abandonné : dépasse le plafond mémoire",
                path.display()
            );
            return Ok(None);
        }
    }

    if samples.is_empty() {
        return Ok(None);
    }

    let actual_secs = samples.len() as f64
        / (fmt.output_rate as f64 * fmt.output_channels as f64).max(1.0);

    Ok(Some(PreloadedTrack {
        path: path.to_path_buf(),
        samples,
        duration_secs: actual_secs,
        gain,
    }))
}

/// Durée du média, en secondes, d'après les métadonnées du conteneur.
fn estimate_duration(reader: &Box<dyn FormatReader>, track_id: u32, source_rate: u32) -> f64 {
    let Some(track) = reader.tracks().iter().find(|t| t.id == track_id) else {
        return 0.0;
    };
    if let (Some(dur), Some(tb)) = (track.duration, track.time_base) {
        if let Some(time) = tb.calc_time(symphonia::core::units::Timestamp::from(dur.get() as i64))
        {
            return time.as_secs_f64();
        }
    }
    track
        .num_frames
        .map(|n| n as f64 / source_rate as f64)
        .unwrap_or(0.0)
}
