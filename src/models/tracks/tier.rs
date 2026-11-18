/// Palier de qualité.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    Hires,
    Lossless,
    Lossy,
}
