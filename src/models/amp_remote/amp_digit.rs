/// Un chiffre de l'afficheur 7 segments, dessiné segment par segment.
#[derive(Debug, Clone, PartialEq)]
pub struct AmpDigit {
    /// Segments allumés, dans l'ordre a (haut), b, c, d (bas), e, f, g (milieu).
    pub segments: [bool; 7],
    /// Point décimal après le chiffre (volume au demi-point).
    pub dot: bool,
}
