//! Caractéristiques techniques d'un fichier audio en texte : fréquence, débit et nom d'un
//! débit DSD, avec le séparateur décimal de la langue de l'utilisateur.

use karbon_ui::intl;

/// Fréquence d'échantillonnage : « 44.1 kHz », « 44,1 kHz », « 96 kHz ».
pub fn khz(rate: u32) -> String {
    format!("{} kHz", intl::decimal(f64::from(rate) / 1000., 1))
}

/// Débit : « 833 kb/s » (un débit déjà en kb/s est gardé tel quel).
pub fn bitrate(bps: u32) -> String {
    let kbps = if bps > 10_000 { bps / 1000 } else { bps };
    format!("{} kb/s", intl::number(i64::from(kbps)))
}

/// Nom d'un débit DSD d'après sa fréquence : « DSD64 », « DSD128 »… pour les multiples de
/// 44,1 kHz, « DSD64 (48k base) » pour ceux de 48 kHz, « DSD » pour une fréquence hors norme
/// ou inconnue (0). Une fréquence arrondie par le fichier (2 822 000 Hz) garde son nom.
pub fn dsd_label(rate: u32) -> String {
    const MULTIPLES: [u32; 5] = [64, 128, 256, 512, 1024];
    for (base, suffix) in [(44_100, ""), (48_000, " (48k base)")] {
        let multiple = (f64::from(rate) / f64::from(base)).round() as u32;
        if MULTIPLES.contains(&multiple) && rate.abs_diff(multiple * base) < 4_000 {
            return format!("DSD{multiple}{suffix}");
        }
    }
    "DSD".into()
}

#[cfg(test)]
mod tests {
    use karbon_ui::intl::Locale;

    #[test]
    fn rates_use_the_decimal_separator_of_the_language() {
        assert_eq!(format!("{} kHz", Locale::French.decimal(44.1, 1)), "44,1 kHz");
        assert_eq!(super::bitrate(833_000), "833 kb/s");
    }

    #[test]
    fn dsd_rates_are_named_by_their_multiple() {
        use super::dsd_label;
        assert_eq!(dsd_label(2_822_400), "DSD64");
        assert_eq!(dsd_label(2_822_000), "DSD64");
        assert_eq!(dsd_label(11_289_600), "DSD256");
        assert_eq!(dsd_label(45_158_400), "DSD1024");
        assert_eq!(dsd_label(3_072_000), "DSD64 (48k base)");
        assert_eq!(dsd_label(0), "DSD");
        assert_eq!(dsd_label(5_000_000), "DSD");
    }
}
