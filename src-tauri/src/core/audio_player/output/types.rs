//! Types partagés par toutes les implémentations d'`AudioOutput`.
//!
//! Regroupe :
//! - les atomics de pilotage de la lecture (pause/stop/volume/position)
//! - l'état partagé spécifique à la voie Symphonia (FullBuffer + LiveDecode)
//! - le choix de backend audio résolu

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize};
use std::sync::{Arc, RwLock};

// ============================================================================
// Atomics communs à tous les backends
// ============================================================================

/// Atomics partagés entre le décodeur, le backend output, et la boucle Phase 6.
/// Cloned côté caller (via Arc), donc construire avec `Arc::new(AtomicBool::new(false))`.
#[derive(Clone)]
pub struct PlaybackAtomics {
    /// `true` quand l'utilisateur a mis pause.
    pub is_paused: Arc<AtomicBool>,
    /// `true` quand l'utilisateur demande l'arrêt (le backend doit se terminer).
    pub is_stopped: Arc<AtomicBool>,
    /// Volume 0..=100.
    pub volume: Arc<AtomicU8>,
    /// Position courante en frames depuis le début du morceau. Écrit par le
    /// backend output au fil de la lecture, lu par Phase 6 pour le frontend
    /// et par le décodeur sur seek.
    pub current_position_frames: Arc<AtomicUsize>,
}

// ============================================================================
// État partagé pour la voie Symphonia
// ============================================================================
//
// La voie Symphonia gère deux modes :
//   - **LiveDecode** (source=0) : le décodeur pousse dans le RingBuffer, le
//     backend audio le consomme.
//   - **FullBuffer** (source=1) : le fichier est décodé entièrement en RAM,
//     le backend audio lit dans un Vec<f32> indexé par un curseur.
//
// Le seek coordonne les deux via `seek_flush` + `pending_seek_frames`.

/// Wrapper du Vec<f32> du FullBuffer. RwLock pour permettre la croissance
/// pendant le pre-fill (write occasionnel) et la lecture concurrente (read
/// fréquent par le backend audio).
pub type FullBufferData = Arc<RwLock<Vec<f32>>>;

/// État spécifique à la voie Symphonia. À passer au backend output qui doit
/// gérer le seek_flush et le switch LiveDecode ↔ FullBuffer.
#[derive(Clone)]
pub struct SymphoniaSharedState {
    /// Vec<f32> contenant l'audio en mode FullBuffer (Minimal profile).
    pub full_buffer_data: FullBufferData,
    /// Curseur de lecture dans `full_buffer_data` (en samples, pas en frames).
    pub full_buffer_cursor: Arc<AtomicUsize>,
    /// `true` quand le FullBuffer est prêt à être lu.
    pub is_full_buffer_ready: Arc<AtomicBool>,
    /// Source audio : 0=RingBuffer (LiveDecode), 1=FullBuffer.
    pub current_source: Arc<AtomicU8>,
    /// Signal du décodeur vers le backend output après un seek : le backend
    /// doit drainer son buffer interne + appliquer un fade-in.
    pub seek_flush: Arc<AtomicBool>,
    /// Frame target d'un seek en attente. Le backend lit cette valeur dans
    /// la branche seek_flush pour repositionner le curseur FullBuffer.
    /// Valeur sentinelle `usize::MAX` = pas de seek en attente.
    pub pending_seek_frames: Arc<AtomicUsize>,
    /// Nombre de canaux de sortie (typiquement 2 pour stereo).
    pub output_channels: u16,
}

// ============================================================================
// Choix de backend
// ============================================================================

/// Backend audio sélectionné. Le frontend exprime sa préférence (toggle
/// "WASAPI exclusive" dans les réglages), la factory du module `output`
/// fait le choix final en tenant compte de la plateforme et de la
/// négociation de format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioBackend {
    /// CPAL en mode partagé. Path par défaut, marche partout.
    CpalShared,
    /// WASAPI en mode exclusive (Windows uniquement). Bit-perfect, bypass mixer.
    #[cfg(target_os = "windows")]
    WasapiExclusive,
    /// ALSA `hw:` en accès direct (Linux uniquement). Bit-perfect, bypass
    /// PipeWire/Pulse/dmix — équivalent Linux de WASAPI exclusive.
    #[cfg(target_os = "linux")]
    AlsaExclusive,
    /// CoreAudio en hog mode + rate nominal forcé (macOS uniquement).
    /// Bit-perfect, aucun autre client ne peut mixer dans notre flux.
    #[cfg(target_os = "macos")]
    CoreAudioExclusive,
}

impl AudioBackend {
    pub fn display_name(self) -> &'static str {
        match self {
            AudioBackend::CpalShared => "CPAL shared",
            #[cfg(target_os = "windows")]
            AudioBackend::WasapiExclusive => "WASAPI exclusive",
            #[cfg(target_os = "linux")]
            AudioBackend::AlsaExclusive => "ALSA exclusive (hw:)",
            #[cfg(target_os = "macos")]
            AudioBackend::CoreAudioExclusive => "CoreAudio exclusive (hog)",
        }
    }

    /// Vrai si le backend peut garantir une sortie bit-perfect (bypass mixer
    /// OS, format natif au DAC). La chaîne complète reste bit-perfect
    /// uniquement si le source rate == output rate (pas de resampler actif).
    pub fn is_bit_perfect_capable(self) -> bool {
        match self {
            AudioBackend::CpalShared => false,
            #[cfg(target_os = "windows")]
            AudioBackend::WasapiExclusive => true,
            #[cfg(target_os = "linux")]
            AudioBackend::AlsaExclusive => true,
            #[cfg(target_os = "macos")]
            AudioBackend::CoreAudioExclusive => true,
        }
    }

    /// Backend exclusif de la plateforme courante, s'il en existe un.
    /// `None` sur les OS sans chemin bit-perfect implémenté.
    pub fn platform_exclusive() -> Option<Self> {
        #[cfg(target_os = "windows")]
        {
            Some(AudioBackend::WasapiExclusive)
        }
        #[cfg(target_os = "linux")]
        {
            Some(AudioBackend::AlsaExclusive)
        }
        #[cfg(target_os = "macos")]
        {
            Some(AudioBackend::CoreAudioExclusive)
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }
}
