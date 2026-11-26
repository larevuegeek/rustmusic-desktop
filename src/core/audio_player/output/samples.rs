//! Gain et conversion des échantillons, communs aux sorties. Gain neutre = rien n'est touché,
//! et la conversion inverse exactement celle du décodeur (÷ 2^15 ou 2^23) : bit-perfect.

/// Longueur du fondu d'entrée après un saut (anti-clic), en échantillons.
pub const FADE: usize = 2048;

/// Marge des sorties partagées : un rééchantillonneur peut dépasser 0 dBFS de
/// quelques centièmes. Les sorties exclusives jouent au rate natif et s'en passent.
pub const SHARED_MARGIN: f32 = 0.98;

/// Volume × Replay Gain, fondu après un saut et marge. Tout neutre : rien n'est touché.
pub fn apply_gain(samples: &mut [f32], gain: f32, fade_remaining: &mut usize, margin: f32) {
    let total = gain * margin;
    if total == 1.0 && *fade_remaining == 0 {
        return;
    }
    for s in samples.iter_mut() {
        let fade = if *fade_remaining > 0 {
            *fade_remaining -= 1;
            (FADE - *fade_remaining) as f32 / FADE as f32
        } else {
            1.0
        };
        *s = (*s * total * fade).clamp(-1.0, 1.0);
    }
}

/// f32 → entier 16 bits ; exact pour toute valeur venue d'un entier 16 bits.
#[inline]
#[cfg_attr(target_os = "macos", allow(dead_code))] // macOS sort en f32
pub fn to_i16(s: f32) -> i16 {
    (s * 32_768.0).round().clamp(-32_768.0, 32_767.0) as i16
}

/// f32 → entier 24 bits (dans un i32) ; exact pour toute valeur venue d'un entier ≤ 24 bits.
#[inline]
#[cfg_attr(target_os = "macos", allow(dead_code))] // WASAPI 24 bits, ALSA S24_3
pub fn to_i24(s: f32) -> i32 {
    (s * 8_388_608.0).round().clamp(-8_388_608.0, 8_388_607.0) as i32
}

/// f32 → entier 32 bits ; exact pour toute valeur venue d'un entier ≤ 24 bits (décalage de 8).
#[inline]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))] // sortie ALSA S32
pub fn to_i32(s: f32) -> i32 {
    (s as f64 * 2_147_483_648.0).round().clamp(-2_147_483_648.0, 2_147_483_647.0) as i32
}

/// Échantillons en entiers de `bytes` octets (2, 3 ou 4), petit ou grand boutiste : de quoi
/// alimenter un DAC en 24 bits packé (S24_3LE) ou en big-endian (S16_BE, S24_3BE, S32_BE).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))] // sortie ALSA
pub fn encode_pcm(samples: &[f32], bytes: usize, big_endian: bool, out: &mut Vec<u8>) {
    out.clear();
    out.reserve(samples.len() * bytes);
    for &s in samples {
        let value = match bytes {
            2 => to_i16(s) as i32,
            3 => to_i24(s),
            _ => to_i32(s),
        };
        let word = &value.to_le_bytes()[..bytes.min(4)];
        if big_endian {
            out.extend(word.iter().rev());
        } else {
            out.extend_from_slice(word);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symphonia::core::audio::conv::FromSample;
    use symphonia::core::audio::sample::i24;

    // Le décodeur → la sortie : chaque entier doit ressortir identique.
    #[test]
    fn sixteen_bits_round_trip_exactly() {
        for x in i16::MIN..=i16::MAX {
            let f = <f32 as FromSample<i16>>::from_sample(x);
            assert_eq!(to_i16(f), x, "16 bits : {x}");
        }
    }

    #[test]
    fn twenty_four_bits_round_trip_exactly() {
        for x in -8_388_608..=8_388_607 {
            let f = <f32 as FromSample<i24>>::from_sample(i24(x));
            assert_eq!(to_i24(f), x, "24 bits : {x}");
        }
    }

    #[test]
    fn thirty_two_bits_carry_the_twenty_four_bits() {
        for x in (-8_388_608..=8_388_607).step_by(7919) {
            let f = <f32 as FromSample<i24>>::from_sample(i24(x));
            assert_eq!(to_i32(f), x << 8, "S32 : {x}");
        }
        assert_eq!(to_i32(-1.0), i32::MIN);
        assert_eq!(to_i32(1.0), i32::MAX);
    }

    #[test]
    fn sixteen_bits_in_a_twenty_four_bit_output() {
        for x in i16::MIN..=i16::MAX {
            let f = <f32 as FromSample<i16>>::from_sample(x);
            assert_eq!(to_i24(f), (x as i32) << 8);
        }
    }

    #[test]
    fn neutral_touches_nothing() {
        let origin: Vec<f32> = (i16::MIN..=i16::MAX).map(|x| <f32 as FromSample<i16>>::from_sample(x)).collect();
        let mut v = origin.clone();
        let mut fade = 0;
        apply_gain(&mut v, 1.0, &mut fade, 1.0);
        assert!(v.iter().zip(&origin).all(|(a, b)| a.to_bits() == b.to_bits()));
    }

    #[test]
    fn shared_headroom_and_volume_apply() {
        let mut v = vec![0.5f32; 4];
        let mut fade = 0;
        apply_gain(&mut v, 0.5, &mut fade, SHARED_MARGIN);
        assert!(v.iter().all(|s| (*s - 0.5 * 0.5 * 0.98).abs() < 1e-6));
    }

    #[test]
    fn the_fade_rises_then_hands_over() {
        let mut v = vec![1.0f32; FADE + 10];
        let mut fade = FADE;
        apply_gain(&mut v, 1.0, &mut fade, 1.0);
        assert_eq!(fade, 0);
        assert!(v[0] < 0.01 && v[FADE / 2] < v[FADE - 1]);
        assert_eq!(v[FADE + 5], 1.0);
    }

    #[test]
    fn out_of_range_clips_without_overflow() {
        assert_eq!(to_i16(1.5), i16::MAX);
        assert_eq!(to_i16(-1.5), i16::MIN);
        assert_eq!(to_i24(1.0), 8_388_607);
        assert_eq!(to_i24(-1.0), -8_388_608);
    }

    #[test]
    fn pcm_words_are_packed_in_the_byte_order_the_dac_asks_for() {
        let samples = [<f32 as FromSample<i24>>::from_sample(i24(0x123456)), <f32 as FromSample<i24>>::from_sample(i24(-2))];
        let mut out = Vec::new();
        encode_pcm(&samples, 3, false, &mut out);
        assert_eq!(out, [0x56, 0x34, 0x12, 0xFE, 0xFF, 0xFF], "S24_3LE");
        encode_pcm(&samples, 3, true, &mut out);
        assert_eq!(out, [0x12, 0x34, 0x56, 0xFF, 0xFF, 0xFE], "S24_3BE");
        let half = [<f32 as FromSample<i16>>::from_sample(0x1234i16)];
        encode_pcm(&half, 2, true, &mut out);
        assert_eq!(out, [0x12, 0x34], "S16_BE");
        encode_pcm(&samples[..1], 4, true, &mut out);
        assert_eq!(out, [0x12, 0x34, 0x56, 0x00], "S32_BE");
        encode_pcm(&samples[..1], 4, false, &mut out);
        assert_eq!(out, [0x00, 0x56, 0x34, 0x12], "S32_LE");
    }
}
