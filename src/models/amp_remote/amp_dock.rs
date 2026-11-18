/// Où se tient la télécommande : côté de la fenêtre, distance à ce bord, hauteur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmpDock {
    /// Collée au bord gauche (sinon au bord droit).
    pub left: bool,
    /// Distance au bord, unités logiques (20 une fois collée).
    pub inset: f32,
    /// Haut de la télécommande dans la fenêtre, unités logiques.
    pub top: f32,
}
