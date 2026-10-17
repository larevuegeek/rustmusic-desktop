//! Gain et conversion des échantillons, communs aux sorties. Gain neutre = rien n'est touché,
//! et la conversion inverse exactement celle du décodeur (÷ 2^15 ou 2^23) : bit-perfect.

/// Longueur du fondu d'entrée après un saut (anti-clic), en échantillons.
pub const FONDU: usize = 2048;

/// Marge des sorties partagées : un rééchantillonneur peut dépasser 0 dBFS de
/// quelques centièmes. Les sorties exclusives jouent au rate natif et s'en passent.
pub const MARGE_PARTAGEE: f32 = 0.98;

/// Volume × Replay Gain, fondu après un saut et marge. Tout neutre : rien n'est touché.
pub fn appliquer_gain(echantillons: &mut [f32], gain: f32, fondu_restant: &mut usize, marge: f32) {
    let total = gain * marge;
    if total == 1.0 && *fondu_restant == 0 {
        return;
    }
    for s in echantillons.iter_mut() {
        let fondu = if *fondu_restant > 0 {
            *fondu_restant -= 1;
            (FONDU - *fondu_restant) as f32 / FONDU as f32
        } else {
            1.0
        };
        *s = (*s * total * fondu).clamp(-1.0, 1.0);
    }
}

/// f32 → entier 16 bits ; exact pour toute valeur venue d'un entier 16 bits.
#[inline]
#[cfg_attr(target_os = "macos", allow(dead_code))] // macOS sort en f32
pub fn vers_i16(s: f32) -> i16 {
    (s * 32_768.0).round().clamp(-32_768.0, 32_767.0) as i16
}

/// f32 → entier 24 bits (dans un i32) ; exact pour toute valeur venue d'un entier ≤ 24 bits.
#[inline]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))] // WASAPI 24 bits
pub fn vers_i24(s: f32) -> i32 {
    (s * 8_388_608.0).round().clamp(-8_388_608.0, 8_388_607.0) as i32
}

/// f32 → entier 32 bits ; exact pour toute valeur venue d'un entier ≤ 24 bits (décalage de 8).
#[inline]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))] // sortie ALSA S32
pub fn vers_i32(s: f32) -> i32 {
    (s as f64 * 2_147_483_648.0).round().clamp(-2_147_483_648.0, 2_147_483_647.0) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use symphonia::core::audio::conv::FromSample;
    use symphonia::core::audio::sample::i24;

    // Le décodeur → la sortie : chaque entier doit ressortir identique.
    #[test]
    fn seize_bits_aller_retour_exact() {
        for x in i16::MIN..=i16::MAX {
            let f = <f32 as FromSample<i16>>::from_sample(x);
            assert_eq!(vers_i16(f), x, "16 bits : {x}");
        }
    }

    #[test]
    fn vingt_quatre_bits_aller_retour_exact() {
        for x in -8_388_608..=8_388_607 {
            let f = <f32 as FromSample<i24>>::from_sample(i24(x));
            assert_eq!(vers_i24(f), x, "24 bits : {x}");
        }
    }

    #[test]
    fn trente_deux_bits_porte_les_vingt_quatre_bits() {
        for x in (-8_388_608..=8_388_607).step_by(7919) {
            let f = <f32 as FromSample<i24>>::from_sample(i24(x));
            assert_eq!(vers_i32(f), x << 8, "S32 : {x}");
        }
        assert_eq!(vers_i32(-1.0), i32::MIN);
        assert_eq!(vers_i32(1.0), i32::MAX);
    }

    #[test]
    fn seize_bits_dans_une_sortie_vingt_quatre_bits() {
        for x in i16::MIN..=i16::MAX {
            let f = <f32 as FromSample<i16>>::from_sample(x);
            assert_eq!(vers_i24(f), (x as i32) << 8);
        }
    }

    #[test]
    fn neutre_ne_touche_rien() {
        let origine: Vec<f32> = (i16::MIN..=i16::MAX).map(|x| <f32 as FromSample<i16>>::from_sample(x)).collect();
        let mut v = origine.clone();
        let mut fondu = 0;
        appliquer_gain(&mut v, 1.0, &mut fondu, 1.0);
        assert!(v.iter().zip(&origine).all(|(a, b)| a.to_bits() == b.to_bits()));
    }

    #[test]
    fn la_marge_partagee_et_le_volume_s_appliquent() {
        let mut v = vec![0.5f32; 4];
        let mut fondu = 0;
        appliquer_gain(&mut v, 0.5, &mut fondu, MARGE_PARTAGEE);
        assert!(v.iter().all(|s| (*s - 0.5 * 0.5 * 0.98).abs() < 1e-6));
    }

    #[test]
    fn le_fondu_monte_puis_rend_la_main() {
        let mut v = vec![1.0f32; FONDU + 10];
        let mut fondu = FONDU;
        appliquer_gain(&mut v, 1.0, &mut fondu, 1.0);
        assert_eq!(fondu, 0);
        assert!(v[0] < 0.01 && v[FONDU / 2] < v[FONDU - 1]);
        assert_eq!(v[FONDU + 5], 1.0);
    }

    #[test]
    fn hors_bornes_ecrete_sans_deborder() {
        assert_eq!(vers_i16(1.5), i16::MAX);
        assert_eq!(vers_i16(-1.5), i16::MIN);
        assert_eq!(vers_i24(1.0), 8_388_607);
        assert_eq!(vers_i24(-1.0), -8_388_608);
    }
}
