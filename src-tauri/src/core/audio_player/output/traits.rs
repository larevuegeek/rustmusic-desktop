//! Trait `AudioOutput` — interface unifiée pour les backends audio.
//!
//! Les deux implémentations actuelles (`CpalSymphoniaOutput` et
//! `WasapiExclusiveOutput`) en sortent. Le code appelant (`audio_player.rs`)
//! ne connaît que cette interface, ce qui permet d'ajouter des backends
//! supplémentaires (ASIO, Pulse exclusive, etc.) sans modifier le pipeline
//! de lecture.

use super::types::AudioBackend;

// ============================================================================
// Erreurs typées
// ============================================================================

#[derive(Debug)]
pub enum AudioOutputError {
    /// Aucun device de sortie disponible.
    DeviceNotFound(String),
    /// Création du stream/render thread a échoué.
    BuildFailed(String),
    /// Le démarrage du stream a échoué.
    StartFailed(String),
    /// La négociation de format avec le DAC a échoué (WASAPI exclusive).
    NegotiationFailed(String),
}

impl std::fmt::Display for AudioOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioOutputError::DeviceNotFound(m) => write!(f, "Device not found: {m}"),
            AudioOutputError::BuildFailed(m) => write!(f, "Build failed: {m}"),
            AudioOutputError::StartFailed(m) => write!(f, "Start failed: {m}"),
            AudioOutputError::NegotiationFailed(m) => write!(f, "Format negotiation failed: {m}"),
        }
    }
}

impl std::error::Error for AudioOutputError {}

// ============================================================================
// Trait public
// ============================================================================

/// Backend de sortie audio.
///
/// Cycle de vie :
///   1. Construit via `output::create_symphonia_output()`
///   2. `start()` lance la lecture
///   3. La boucle Phase 6 polle les atomics
///   4. Drop arrête proprement (RAII)
///
/// Le trait est `Send` car l'implémentation peut être stockée dans une
/// variable locale du thread de playback. Il n'a PAS besoin d'être `Sync`
/// car un seul thread possède l'output à la fois.
pub trait AudioOutput: Send {
    /// Démarre le stream / spawn le render thread. Doit être appelé après
    /// la construction et avant la boucle Phase 6.
    fn start(&mut self) -> Result<(), AudioOutputError>;

    /// Sample rate effectif de la sortie (peut différer du source rate en
    /// cas de resampling — typiquement seul WASAPI exclusive matche le
    /// source rate exactement).
    fn output_sample_rate(&self) -> u32;

    /// Nombre de canaux de sortie (typiquement 2).
    fn output_channels(&self) -> u16;

    /// Nom du device pour affichage UI (ex. "Speakers (FiiO K7)").
    fn device_name(&self) -> &str;

    /// Backend effectivement utilisé (peut différer du backend demandé en
    /// cas de fallback automatique).
    fn backend(&self) -> AudioBackend;

    /// Bits par échantillon que la sortie transporte sans les altérer (une source
    /// plus profonde y perd des bits). Le f32 en porte 24.
    fn bits_exacts(&self) -> u32 {
        24
    }
}
