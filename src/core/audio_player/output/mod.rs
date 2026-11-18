//! Module `output` — backends de sortie audio pour la voie Symphonia.
//!
//! Architecture :
//! - `types.rs` : types partagés (atomics, état partagé Symphonia, choix backend)
//! - `traits.rs` : trait `AudioOutput` + erreurs typées
//! - `cpal_symphonia.rs` : implémentation CPAL shared mode (default, cross-platform)
//! - `wasapi_exclusive.rs` : implémentation WASAPI exclusive (Windows, bit-perfect)
//!
//! L'entry point public est `create_symphonia_output()` qui :
//!   1. Tente le backend demandé en premier (WASAPI si activé + sur Windows)
//!   2. Si WASAPI échoue à négocier le format, fallback automatique sur CPAL
//!   3. Si CPAL échoue aussi, propagation de l'erreur
//!
//! Le caller (`audio_player.rs`) ne connaît que le trait `AudioOutput`, il
//! n'a aucune dépendance sur cpal ou wasapi en direct.

mod airplay;
mod cpal_symphonia;
pub mod preference;
mod traits;
mod types;
#[cfg(target_os = "windows")]
mod wasapi_exclusive;
#[cfg(target_os = "windows")]
pub use wasapi_exclusive::close_engine;
#[cfg(target_os = "linux")]
mod exclusive_alsa;
#[cfg(target_os = "macos")]
mod exclusive_coreaudio;
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub mod dop_engine;
#[cfg(target_os = "linux")]
pub mod dop_alsa;
#[cfg(target_os = "linux")]
pub mod device_reservation;
#[cfg(target_os = "linux")]
pub mod pipewire_profile;
#[cfg(target_os = "macos")]
pub mod dop_coreaudio;
pub mod samples;
pub mod device_lookup;
pub mod shared_engine;

pub use preference::{current_preference, dop_enabled, set_dop_enabled, set_shared_only, set_wasapi_exclusive};
pub use traits::{AudioOutput, AudioOutputError};
pub use types::{AudioBackend, PlaybackAtomics, SymphoniaSharedState};

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod exclusive;
mod create;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use exclusive::*;
pub use create::*;
