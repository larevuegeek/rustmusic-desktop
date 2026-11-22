use crate::models::player::*;
use std::time::Instant;
use karbon_ui::window::WindowState;

#[derive(Debug)]
pub struct MiniState {
    pub active: bool,
    pub micro: bool,
    /// Épinglé au-dessus des autres fenêtres (par défaut en entrant en mode mini).
    pub pinned: bool,
    pub tab: Option<MiniTab>,
    pub volume_open: bool,
    /// Niveau du volume affiché sur la pochette du micro, et numéro du
    /// tour de molette qui l'a affiché.
    pub volume_hint: bool,
    pub(crate) hint_turn: u32,
    pub(crate) saved: Option<Saved>,
    /// Dernière géométrie normale : ni agrandie, ni plein écran, ni réduite, hors mode mini.
    pub(crate) normal: Option<(i32, i32, i32, i32)>,
    /// Dernier état de la fenêtre.
    pub window: WindowState,
    pub(crate) micro_size: i32,
    /// Hauteur courante du mini (suivie pour animer depuis la bonne valeur).
    pub(crate) height: i32,
    /// Animation de hauteur : départ, arrivée, début.
    pub(crate) anim: Option<(i32, i32, Instant)>,
}
