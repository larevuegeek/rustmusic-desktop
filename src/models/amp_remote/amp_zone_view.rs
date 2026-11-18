use crate::core::amp_remote::model::Zone;

/// Une zone : volume (affiché, avec le glissement en cours), sourdine, marche.
pub struct AmpZoneView {
    pub zone: Zone,
    pub power: bool,
    pub muted: bool,
    pub volume: f32,
    pub max: f32,
    pub step: f32,
    /// Part remplie du curseur (0 à 1).
    pub fill: f32,
    /// « NET · 42 » (zone 2) ou « En veille ».
    pub label: String,
}
