//! Pochettes d'une vignette : mosaïque de quatre ou image seule.

/// Les quatre pochettes d'une mosaïque, dès qu'il y en a quatre (les suivantes sont
/// ignorées) ; sinon la vignette montre une seule pochette.
pub fn mosaic(covers: &[String]) -> Option<&[String]> {
    covers.get(..4)
}
