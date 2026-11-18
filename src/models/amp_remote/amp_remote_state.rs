use std::sync::Arc;
use std::time::Instant;
use karbon_ui::ui::scene::Image;
use crate::core::amp_remote::model::Zone;
use crate::core::amp_remote::session::View;
use crate::models::amp_remote::{AmpDock, AmpTab};

/// La télécommande : l'ampli piloté tel que la session le publie, et l'état du panneau.
#[derive(Default)]
pub struct AmpRemoteState {
    pub view: Option<View>,
    /// Pochette de l'écran réseau, décodée.
    pub cover: Option<Arc<Image>>,
    pub tab: AmpTab,
    /// Volume en cours de glissement : affiché tout de suite, envoyé au relâché.
    pub drag: Option<(Zone, f32)>,
    /// −/+ maintenu : la zone et le sens répétés.
    pub repeat: Option<(Zone, bool)>,
    /// Pas déjà répétés par l'appui en cours : le clic qui le termine n'en ajoute pas.
    pub repeats: u32,
    /// La marche a été appuyée à la souris : son clic ne fait rien de plus.
    pub power_pressed: bool,
    /// Appui en cours sur la marche (anneau rouge qui se remplit).
    pub holding: bool,
    /// « Maintenez pour éteindre », après un appui trop court.
    pub hint: bool,
    /// Jeton des minuteries (répétition, marche, indice) : un appui plus récent les périme.
    pub token: u64,
    /// Dernière entrée vue : le passage sur NET ou USB ouvre l'onglet Navigation.
    pub last_input: String,
    /// Dernier pas donné à la molette (un pas toutes les 70 ms au plus).
    pub last_wheel: Option<Instant>,
    /// La languette a servi : le panneau sort et se range en glissant (pas au lancement).
    pub slid: bool,
    /// Position imposée (aperçu) ; `None` : celle des réglages.
    pub dock: Option<AmpDock>,
    /// Position au début du geste (le moteur déplace la télécommande pendant le geste).
    pub pan_from: Option<AmpDock>,
}
