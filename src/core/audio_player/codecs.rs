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
pub fn codec_registry() -> &'static CodecRegistry {
    &REGISTRE
}

/// Étalons ffmpeg d'un même PCM brut : décodés puis convertis comme en sortie exclusive,
/// chaque entier doit ressortir identique.
#[cfg(test)]
mod tests {
    use super::codec_registry;
    use crate::core::audio_player::audio_utils::convert_audio_buffer_to_interleaved;
    use crate::core::audio_player::output::samples::{to_i16, to_i24};
    use crate::core::audio_player::pipeline_info::{is_lossless_integer, source_bit_depth};
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;

    struct Decode {
        rate: u32,
        bits: Option<u32>,
        lossless: bool,
        samples: Vec<f32>,
    }

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join(name)
    }

    fn decoder(name: &str) -> Decode {
        let path_part = fixture(name);
        let mss = MediaSourceStream::new(Box::new(std::fs::File::open(&path_part).unwrap()), Default::default());
        let mut hint = Hint::new();
        hint.with_extension(path_part.extension().unwrap().to_str().unwrap());
        let mut reader = symphonia::default::get_probe()
            .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
            .unwrap_or_else(|e| panic!("{name} : probe {e}"));
        let track = reader.tracks().iter().find(|t| t.codec_params.as_ref().and_then(|c| c.audio()).is_some()).unwrap();
        let id = track.id;
        let params = track.codec_params.as_ref().unwrap().audio().unwrap().clone();
        let channels = params.channels.as_ref().unwrap().count();
        let mut dec = codec_registry()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())
            .unwrap_or_else(|e| panic!("{name} : aucun décodeur ({e:?})"));
        let mut samples = Vec::new();
        while let Ok(Some(packet)) = reader.next_packet() {
            if packet.track_id == id {
                let buf = dec.decode(&packet).unwrap_or_else(|e| panic!("{name} : décodage {e}"));
                samples.extend(convert_audio_buffer_to_interleaved(&buf, channels));
            }
        }
        Decode { rate: params.sample_rate.unwrap(), bits: source_bit_depth(&params), lossless: is_lossless_integer(params.codec), samples }
    }

    fn reference(name: &str, bytes: usize) -> Vec<i32> {
        std::fs::read(fixture(name))
            .unwrap()
            .chunks_exact(bytes)
            .map(|c| match bytes {
                2 => i16::from_le_bytes([c[0], c[1]]) as i32,
                _ => (i32::from_le_bytes([0, c[0], c[1], c[2]])) >> 8,
            })
            .collect()
    }

    fn assert_exact(name: &str, bits: u32) {
        let d = decoder(name);
        let expected = reference(if bits == 16 { "ref16.raw" } else { "ref24.raw" }, (bits / 8) as usize);
        assert!(d.lossless, "{name} : devrait être reconnu sans perte");
        assert_eq!(d.bits, Some(bits), "{name} : profondeur");
        assert_eq!(d.samples.len(), expected.len(), "{name} : nombre d'échantillons");
        for (i, (s, x)) in d.samples.iter().zip(&expected).enumerate() {
            let output = if bits == 16 { to_i16(*s) as i32 } else { to_i24(*s) };
            assert_eq!(output, *x, "{name} : échantillon {i} altéré");
        }
    }

    #[test]
    fn flac_16_and_24_bits_intact() {
        assert_exact("flac16.flac", 16);
        assert_exact("flac24.flac", 24);
    }

    #[test]
    fn aiff_16_and_24_bits_read_intact() {
        assert_exact("aiff16.aiff", 16);
        assert_exact("aiff24.aiff", 24);
    }

    #[test]
    fn alac_16_and_24_bits_read_intact() {
        assert_exact("alac16.m4a", 16);
        assert_exact("alac24.m4a", 24);
    }

    #[test]
    fn wav_24_bits_intact() {
        assert_exact("wav24.wav", 24);
    }

    #[test]
    fn opus_reads_but_is_not_lossless() {
        let d = decoder("opus.opus");
        assert!(!d.lossless);
        assert_eq!(d.rate, 48_000);
        // 0,3 s stéréo à 48 kHz, à un bloc près (préroll compris).
        assert!(d.samples.len() > 20_000, "Opus : {} échantillons", d.samples.len());
        assert!(d.samples.iter().any(|s| s.abs() > 0.1), "Opus : silence");
    }
}
