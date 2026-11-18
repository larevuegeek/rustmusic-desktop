use std::sync::Arc;
use karbon_ui::ui::scene::Image;
use crate::models::amp_remote::*;
use crate::models::app::Msg;

/// Le panneau de la télécommande.
pub struct AmpRemoteView {
    pub open: bool,
    /// Où se tient la télécommande.
    pub dock: AmpDock,
    /// Hauteur maximale du panneau, selon sa position (unités logiques).
    pub panel_max: f32,
    /// En cours de déplacement : la languette prend le curseur de saisie.
    pub panning: bool,
    /// Sortie ou rangement demandé : le panneau glisse.
    pub slide: bool,
    pub mini: bool,
    /// Panneau court (mini, ampli injoignable) : la languette se cale en haut, plus courte.
    pub compact: bool,
    pub connected: bool,
    /// Connecté et allumé.
    pub on: bool,
    pub name: String,
    /// « TX-NR686 · 192.168.1.20 », « En veille » ou « Reconnexion… ».
    pub status: String,
    /// Voyant de la languette et de l'en-tête : « on », « off » ou « warn ».
    pub led: &'static str,
    /// « Maintenez pour éteindre » : appui en cours, puis indice.
    pub holding: bool,
    pub hint: bool,
    pub unreachable: String,
    /// Afficheur : voyants, entrée (et sa taille), mode, volume en 7 segments.
    pub lights: Vec<(&'static str, bool)>,
    pub code: String,
    pub code_size: f32,
    pub mode: String,
    /// Mode mini, seconde ligne : nom de l'ampli, MUTING ou le mode.
    pub mini_line: String,
    /// Volume en 7 segments (chiffres éteints compris) ; `lit` : allumés en clair, sinon
    /// en sourdine.
    pub digits: Vec<AmpDigit>,
    pub lit: bool,
    /// « VOL / 80 » ou « MUTING ».
    pub volume_tag: String,
    pub main: AmpZoneView,
    pub input_name: String,
    pub input_icon: &'static str,
    pub tab: AmpTab,
    /// Sources par famille (titre vide s'il n'y en a qu'une).
    pub families: Vec<(String, Vec<AmpChoice>)>,
    pub modes: Vec<AmpChoice>,
    pub tone: Vec<AmpToneRow>,
    pub zone2: Option<AmpZoneView>,
    pub zone2_inputs: Vec<AmpChoice>,
    /// Navigation : sur NET ou USB, la liste de l'ampli ; sinon le menu à l'écran.
    pub on_net: bool,
    pub net_title: String,
    pub net_count: String,
    pub net_lines: Vec<AmpNetLine>,
    pub net_loading: bool,
    pub playing: bool,
    pub track: String,
    pub track_line: String,
    pub time: String,
    pub paused: bool,
    pub cover: Option<Arc<Image>>,
    /// « Passer sur NET » quand l'ampli a cette entrée.
    pub go_net: Option<Msg>,
    /// Molette ronde (sinon touches en ligne).
    pub wheel: bool,
}
