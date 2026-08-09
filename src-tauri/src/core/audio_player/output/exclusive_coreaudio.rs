//! Sortie PCM bit-perfect via CoreAudio — macOS uniquement.
//!
//! # Ce que ce backend ajoute par rapport à CPAL partagé
//! CPAL ouvre déjà une unité HAL qui **règle le rate nominal du device** sur
//! celui demandé et ne rééchantillonne pas quand les deux coïncident. Il ne
//! manque donc que deux choses pour du bit-perfect :
//!
//! 1. **L'exclusivité** — le *hog mode*, équivalent macOS du WASAPI exclusive
//!    et de l'ALSA `hw:`. Sans lui, une autre application peut ouvrir le même
//!    DAC et le HAL mixe les deux flux (donc plus de bit-perfect).
//! 2. **La garantie du rate** — on refuse d'ouvrir si le device ne déclare pas
//!    le rate source, et on **revérifie** le rate nominal après ouverture. En
//!    cas d'écart, l'appelant retombe sur CPAL partagé plutôt que de laisser
//!    croire à un bit-perfect qui n'en est pas un.
//!
//! Le reste (callback, seek, FullBuffer/LiveDecode, volume, Replay Gain) est
//! rigoureusement identique au chemin CPAL : ce module **compose** avec
//! [`CpalSymphoniaOutput`] au lieu de dupliquer sa logique.
//!
//! # Pourquoi du f32 reste bit-perfect
//! Même raisonnement que le DoP (cf. `dop_coreaudio`) : CPAL sort du `f32` sur
//! macOS, et un échantillon PCM 24 bits tient exactement dans la mantisse d'un
//! `f32` (24 bits). La reconversion du HAL vers l'entier du DAC est une
//! multiplication par une puissance de deux — aller-retour exact.
//!
//! ⚠️ Le volume logiciel et le Replay Gain, eux, modifient les échantillons.
//! Pour du bit-perfect strict il faut volume à 100 % et Replay Gain désactivé,
//! exactement comme sur le chemin WASAPI exclusive de Windows.

#![cfg(target_os = "macos")]

use ringbuf::traits::Consumer;

use super::cpal_symphonia::CpalSymphoniaOutput;
use super::dop_coreaudio::{nominal_rate, resolve_device_id, supports_nominal_rate, HogGuard};
use super::traits::{AudioOutput, AudioOutputError};
use super::types::{AudioBackend, PlaybackAtomics, SymphoniaSharedState};

/// Sortie CPAL encadrée par un hog mode CoreAudio.
///
/// Le `HogGuard` est stocké ici pour que l'exclusivité vive exactement aussi
/// longtemps que le stream : il est relâché au `Drop`, donc au changement de
/// piste ou à l'arrêt, et les autres applications récupèrent le DAC.
pub struct CoreAudioExclusiveOutput {
    inner: CpalSymphoniaOutput,
    /// Ordre des champs volontaire : `inner` est droppé avant `_hog`, donc le
    /// stream est fermé **avant** que l'exclusivité soit relâchée.
    _hog: HogGuard,
    device_id: u32,
    source_rate: u32,
}

impl CoreAudioExclusiveOutput {
    /// Vérifie qu'un chemin exclusif est possible **avant** de consommer quoi
    /// que ce soit : device identifiable et rate source déclaré par le DAC.
    ///
    /// Appelé par la factory pour décider sans risque : tant que cette
    /// fonction n'a pas répondu `Some`, le `Consumer` du ring buffer n'a pas
    /// bougé et le repli sur CPAL reste possible.
    pub fn probe(device_name: &str, source_rate: u32) -> Option<u32> {
        let id = resolve_device_id(device_name)?;
        if !supports_nominal_rate(id, source_rate) {
            log::info!(
                "🎚️  CoreAudio exclusive : {device_name} ne déclare pas {source_rate} Hz \
                 → repli sur CPAL partagé"
            );
            return None;
        }
        Some(id)
    }

    /// Construit la sortie exclusive. `device_id` vient de [`Self::probe`].
    ///
    /// Le stream CPAL est demandé **au rate source** : c'est ce qui pousse le
    /// HAL à régler le rate nominal du DAC et à ne pas rééchantillonner.
    pub fn try_new<C>(
        device: cpal::Device,
        device_id: u32,
        device_name: String,
        source_rate: u32,
        channels: u16,
        consumer: C,
        atomics: PlaybackAtomics,
        shared: SymphoniaSharedState,
    ) -> Result<Self, AudioOutputError>
    where
        C: Consumer<Item = f32> + Send + 'static,
    {
        // Exclusivité d'abord : si une autre app tient le DAC, autant le savoir
        // avant d'ouvrir le stream. L'échec n'est pas fatal (cf. HogGuard) mais
        // il est journalisé — l'utilisateur doit pouvoir comprendre pourquoi sa
        // sortie n'est pas réellement exclusive.
        let hog = HogGuard::acquire(device_id);

        let config = cpal::StreamConfig {
            channels,
            sample_rate: source_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let inner = CpalSymphoniaOutput::try_new(
            device,
            config,
            device_name,
            consumer,
            atomics,
            shared,
        )?;

        Ok(Self {
            inner,
            _hog: hog,
            device_id,
            source_rate,
        })
    }
}

impl AudioOutput for CoreAudioExclusiveOutput {
    fn start(&mut self) -> Result<(), AudioOutputError> {
        self.inner.start()?;

        // Vérification dure : le rate nominal réellement appliqué par le HAL.
        // Un écart signifie qu'un rééchantillonnage a lieu quelque part — on ne
        // peut plus parler de bit-perfect, donc on le dit franchement plutôt
        // que d'afficher un badge mensonger.
        match nominal_rate(self.device_id) {
            Some(actual) if (actual - self.source_rate as f64).abs() < 0.5 => {
                log::info!(
                    "🎚️  CoreAudio exclusive : rate nominal confirmé à {} Hz (bit-perfect)",
                    self.source_rate
                );
            }
            Some(actual) => {
                log::warn!(
                    "🎚️  CoreAudio exclusive : rate nominal {actual:.0} Hz ≠ source {} Hz — \
                     le HAL rééchantillonne, sortie NON bit-perfect",
                    self.source_rate
                );
            }
            None => {
                log::warn!("🎚️  CoreAudio exclusive : rate nominal illisible après ouverture");
            }
        }
        Ok(())
    }

    fn output_sample_rate(&self) -> u32 {
        self.inner.output_sample_rate()
    }

    fn output_channels(&self) -> u16 {
        self.inner.output_channels()
    }

    fn device_name(&self) -> &str {
        self.inner.device_name()
    }

    fn backend(&self) -> AudioBackend {
        AudioBackend::CoreAudioExclusive
    }
}
