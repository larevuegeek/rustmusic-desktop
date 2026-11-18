use crate::core::metadata_source::matching::Confidence;

#[derive(Debug, Clone)]
pub struct Match {
    pub local: usize,
    pub remote: Option<u16>,
    pub score: f32,
    pub confidence: Confidence,
}
