use crate::core::amp_remote::model::{Command, NavKey, Zone};
use karbon_ui::ui::drag_drop::PanEvent;
use crate::models::amp_remote::AmpTab;

#[derive(Debug, Clone, PartialEq)]
pub enum AmpRemoteMsg {
    /// Languette : sortir ou ranger le panneau.
    Toggle,
    /// Mode mini : l'afficheur et le volume seuls.
    Mini,
    Tab(AmpTab),
    /// Commande envoyée telle quelle ; l'état revient par l'ampli.
    Send(Command),
    /// Curseur de volume : glissé (affiché tout de suite), puis relâché (envoyé).
    Drag(Zone, f64),
    Release(Zone, f64),
    /// −/+ : début ou fin d'appui (répétition tant qu'on appuie, après 380 ms).
    Step(Zone, bool, bool),
    /// −/+ cliqué (souris ou clavier) : un pas, si l'appui n'a rien répété.
    StepClick(Zone, bool),
    /// Répétition de −/+ (jeton de l'appui en cours).
    Repeat(u64),
    /// Marche : début ou fin d'appui (veille au bout de 0,8 s d'appui, ampli allumé).
    Power(bool),
    /// Marche cliquée : allume ; au clavier (sans appui), met aussi en veille.
    PowerClick,
    /// Appui sur la marche tenu jusqu'au bout (jeton de l'appui).
    PowerHeld(u64),
    /// Fin de l'indice « Maintenez pour éteindre » (jeton de l'indice).
    HintDone(u64),
    /// Source précédente ou suivante (mode mini).
    StepInput(i32),
    /// Molette sur l'afficheur ou le pavé : un pas de volume (`true` : monter).
    Wheel(bool),
    /// Molette sur le pavé : une direction de navigation.
    NavWheel(NavKey),
    /// Commandes de direction : molette ronde ou touches en ligne.
    Pad(bool),
    /// Les réglages de la télécommande.
    Settings,
    /// La languette glissée : le moteur déplace la télécommande ; au relâchement, elle se
    /// colle au bord s'il est tout près, sinon elle reste où on la pose.
    Pan(PanEvent),
}
