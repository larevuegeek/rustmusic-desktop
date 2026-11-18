use crate::models::player::*;

#[derive(Debug, Clone, PartialEq)]
pub enum MiniMsg {
    /// Bouton de la barre de titre ou raccourci clavier.
    Toggle,
    /// Retour au lecteur complet.
    Restore,
    /// Fermer : la fenêtre reprend sa taille normale (retenue au prochain lancement).
    Close,
    /// Épingler au-dessus des autres fenêtres, ou non.
    Pin,
    Micro,
    BackToMini,
    /// Recliquer l'onglet ouvert le replie.
    Tab(MiniTab),
    /// File : vider la suite, retirer un titre, jouer un titre (index dans la file).
    ClearUpcoming,
    Remove(String),
    Play(usize),
    /// Bulle du volume : ouvrir ou fermer, sourdine.
    VolumeMenu(bool),
    Mute,
    /// Molette (sur le volume du mini et du micro-lecteur) : ±5 ; `hint` affiche le niveau sur
    /// la pochette du micro le temps d'un tour de molette.
    Wheel { up: bool, hint: bool },
    /// Fin de l'indication du niveau (900 ms après le dernier cran ; numéro du tour).
    HintEnd(u32),
}
