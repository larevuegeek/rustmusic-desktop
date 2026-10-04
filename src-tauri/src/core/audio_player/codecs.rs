//! Décodeurs disponibles : ceux de Symphonia, plus Opus (libopus, le décodeur de
//! référence), que Symphonia ne fournit pas.

use std::sync::LazyLock;

use symphonia::core::codecs::registry::CodecRegistry;

static REGISTRE: LazyLock<CodecRegistry> = LazyLock::new(|| {
    let mut registre = CodecRegistry::new();
    symphonia::default::register_enabled_codecs(&mut registre);
    registre.register_audio_decoder::<symphonia_adapter_libopus::OpusDecoder>();
    registre
});

/// À utiliser partout à la place de `symphonia::default::get_codecs()`.
pub fn registre() -> &'static CodecRegistry {
    &REGISTRE
}

/// Étalons ffmpeg d'un même PCM brut : décodés puis convertis comme en sortie exclusive,
/// chaque entier doit ressortir identique.
#[cfg(test)]
mod tests {
    use super::registre;
    use crate::core::audio_player::audio_utils::convert_audio_buffer_to_interleaved;
    use crate::core::audio_player::output::echantillons::{vers_i16, vers_i24};
    use crate::core::audio_player::pipeline_info::{est_entier_sans_perte, profondeur_source};
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;

    struct Decode {
        rate: u32,
        bits: Option<u32>,
        sans_perte: bool,
        echantillons: Vec<f32>,
    }

    fn fixture(nom: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join(nom)
    }

    fn decoder(nom: &str) -> Decode {
        let chemin = fixture(nom);
        let mss = MediaSourceStream::new(Box::new(std::fs::File::open(&chemin).unwrap()), Default::default());
        let mut hint = Hint::new();
        hint.with_extension(chemin.extension().unwrap().to_str().unwrap());
        let mut reader = symphonia::default::get_probe()
            .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
            .unwrap_or_else(|e| panic!("{nom} : probe {e}"));
        let piste = reader.tracks().iter().find(|t| t.codec_params.as_ref().and_then(|c| c.audio()).is_some()).unwrap();
        let id = piste.id;
        let params = piste.codec_params.as_ref().unwrap().audio().unwrap().clone();
        let canaux = params.channels.as_ref().unwrap().count();
        let mut dec = registre()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())
            .unwrap_or_else(|e| panic!("{nom} : aucun décodeur ({e:?})"));
        let mut echantillons = Vec::new();
        while let Ok(Some(paquet)) = reader.next_packet() {
            if paquet.track_id == id {
                let buf = dec.decode(&paquet).unwrap_or_else(|e| panic!("{nom} : décodage {e}"));
                echantillons.extend(convert_audio_buffer_to_interleaved(&buf, canaux));
            }
        }
        Decode { rate: params.sample_rate.unwrap(), bits: profondeur_source(&params), sans_perte: est_entier_sans_perte(params.codec), echantillons }
    }

    fn reference(nom: &str, octets: usize) -> Vec<i32> {
        std::fs::read(fixture(nom))
            .unwrap()
            .chunks_exact(octets)
            .map(|c| match octets {
                2 => i16::from_le_bytes([c[0], c[1]]) as i32,
                _ => (i32::from_le_bytes([0, c[0], c[1], c[2]])) >> 8,
            })
            .collect()
    }

    fn verifier_exact(nom: &str, bits: u32) {
        let d = decoder(nom);
        let attendu = reference(if bits == 16 { "ref16.raw" } else { "ref24.raw" }, (bits / 8) as usize);
        assert!(d.sans_perte, "{nom} : devrait être reconnu sans perte");
        assert_eq!(d.bits, Some(bits), "{nom} : profondeur");
        assert_eq!(d.echantillons.len(), attendu.len(), "{nom} : nombre d'échantillons");
        for (i, (s, x)) in d.echantillons.iter().zip(&attendu).enumerate() {
            let sortie = if bits == 16 { vers_i16(*s) as i32 } else { vers_i24(*s) };
            assert_eq!(sortie, *x, "{nom} : échantillon {i} altéré");
        }
    }

    #[test]
    fn flac_16_et_24_bits_intacts() {
        verifier_exact("flac16.flac", 16);
        verifier_exact("flac24.flac", 24);
    }

    #[test]
    fn aiff_16_et_24_bits_lus_et_intacts() {
        verifier_exact("aiff16.aiff", 16);
        verifier_exact("aiff24.aiff", 24);
    }

    #[test]
    fn alac_16_et_24_bits_lus_et_intacts() {
        verifier_exact("alac16.m4a", 16);
        verifier_exact("alac24.m4a", 24);
    }

    #[test]
    fn wav_24_bits_intact() {
        verifier_exact("wav24.wav", 24);
    }

    #[test]
    fn opus_se_lit_mais_n_est_pas_sans_perte() {
        let d = decoder("opus.opus");
        assert!(!d.sans_perte);
        assert_eq!(d.rate, 48_000);
        // 0,3 s stéréo à 48 kHz, à un bloc près (préroll compris).
        assert!(d.echantillons.len() > 20_000, "Opus : {} échantillons", d.echantillons.len());
        assert!(d.echantillons.iter().any(|s| s.abs() > 0.1), "Opus : silence");
    }
}
