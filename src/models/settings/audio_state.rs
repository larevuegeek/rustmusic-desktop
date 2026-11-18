use crate::models::settings::*;
use std::collections::HashMap;

/// État de la page Audio.
#[derive(Default)]
pub struct AudioState {
    pub status: Option<AudioStatus>,
    /// Capacités exclusives sondées, par nom affiché.
    pub caps: HashMap<String, OutputCaps>,
    /// Préampli affiché pendant le glissement.
    pub preamp: Option<f32>,
    pub refreshing: bool,
    /// `DacTest` : sortie testée, en cours, résultat ou erreur.
    pub dac_device: Option<String>,
    pub dac_running: bool,
    pub dac_caps: Option<OutputCaps>,
    pub dac_error: Option<String>,
}
