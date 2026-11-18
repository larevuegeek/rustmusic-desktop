/// Une part de la barre segmentée.
pub struct Segment {
    /// Rang de la teinte catégorielle (0 bleu, 1 orange, 2 aqua).
    pub tone: usize,
    pub name: String,
    /// Nombre de morceaux, formaté.
    pub count: String,
    /// « 86 % ».
    pub share: String,
    /// Part en pourcentage (largeur du bloc).
    pub pct: f32,
    /// Le bloc est assez large pour porter sa part ; sinon la légende la donne.
    pub labeled: bool,
}
