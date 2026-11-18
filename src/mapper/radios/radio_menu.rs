use crate::core::radio::directory::StreamVariant;

/// « MP3 · 128 kbit/s ».
pub fn variant_label(v: &StreamVariant) -> String {
    [Some(v.codec.clone().unwrap_or_else(|| "?".into())), v.bitrate.map(|b| format!("{b} kbit/s"))].into_iter().flatten().collect::<Vec<_>>().join(" · ")
}

impl crate::models::radios::RadioMenu {
    /// Plusieurs flux : le choix de la qualité.
    pub fn has_variants(&self) -> bool {
        self.variants.len() > 1
    }
}
