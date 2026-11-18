// ============================================================================
// FONCTIONS UTILITAIRES
// ============================================================================

use symphonia::core::audio::GenericAudioBufferRef;

/// Convertit un GenericAudioBufferRef Symphonia en Vec<f32> interleaved.
/// Depuis symphonia 0.6, la conversion de format de sample (U8/S16/S24/S32/F64
/// → f32 normalisé [-1.0, 1.0]) est intégrée : `copy_to_vec_interleaved`
/// gère tous les formats source, y compris ceux que l'ancien match manuel
/// ne couvrait pas (S24, U16, U24, U32...).
pub fn convert_audio_buffer_to_interleaved(
    audio_buf: &GenericAudioBufferRef,
    channels: usize,
) -> Vec<f32> {
    let mut samples: Vec<f32> = Vec::with_capacity(audio_buf.frames() * channels);
    audio_buf.copy_to_vec_interleaved(&mut samples);
    samples
}

pub fn adapt_channels(
    input: &[f32],
    input_channels: usize,
    output: &mut [f32],
    output_channels: usize,
) {
    let frames: usize = input.len() / input_channels;

    for frame in 0..frames {
        let base_in: usize = frame * input_channels;
        let base_out: usize = frame * output_channels;

        match (input_channels, output_channels) {
            // Mono vers stéréo
            (1, 2) => {
                let m: f32 = input[base_in];
                output[base_out] = m;
                output[base_out + 1] = m;
            }

            // Stéréo vers 7.1 (upmix cohérent)
            (2, 8) => {
                let l: f32 = input[base_in];
                let r: f32 = input[base_in + 1];

                // On crée un mix léger pour chaque canal (valeurs équilibrées)
                output[base_out + 0] = l * 0.48; // FL
                output[base_out + 1] = r * 0.48; // FR

                // Center : réduit pour éviter l'effet "bouché"
                output[base_out + 2] = (l + r) * 0.25; // C (moins de médiums)

                // LFE : atténué (moins de graves bourdonnants)
                output[base_out + 3] = (l + r) * 0.12; // LFE

                // Surrounds : un peu moins forts et plus diffus
                output[base_out + 4] = (l * 0.25) - (r * 0.05); // RL
                output[base_out + 5] = (r * 0.25) - (l * 0.05); // RR
                output[base_out + 6] = l * 0.20; // SL
                output[base_out + 7] = r * 0.20; // SR

                if output_channels >= 6 {
                    for ch in 2..output_channels {
                        // Applique un léger déphasage (-0.003) et un filtrage haut adouci
                        output[base_out + ch] *= 0.95; // réduit les aigus corrélés
                        output[base_out + ch] -= output[base_out + ch] * 0.0025;
                    }
                }
            }

            // Stéréo vers 5.1 (plus classique)
            (2, 6) => {
                let l: f32 = input[base_in];
                let r: f32 = input[base_in + 1];
                output[base_out + 0] = l * 0.48; // L
                output[base_out + 1] = r * 0.48; // R
                output[base_out + 2] = (l + r) * 0.25; // C (moins de médiums)
                output[base_out + 3] = (l + r) * 0.12; // LFE
                output[base_out + 4] = (l * 0.25) - (r * 0.05); // RL
                output[base_out + 5] = (r * 0.25) - (l * 0.05); // RR

                for ch in 2..output_channels {
                    output[base_out + ch] *= 0.97;
                    output[base_out + ch] -= output[base_out + ch] * 0.0025;
                }
            }

            // 3 canaux (L R C) vers stéréo (ITU-R BS.775)
            (3, 2) => {
                let fl = input[base_in];
                let fr = input[base_in + 1];
                let c = input[base_in + 2];
                output[base_out] = (fl + 0.707 * c) * 0.707;
                output[base_out + 1] = (fr + 0.707 * c) * 0.707;
            }

            // 4 canaux (FL FR LS RS = quad) vers stéréo
            (4, 2) => {
                let fl = input[base_in];
                let fr = input[base_in + 1];
                let ls = input[base_in + 2];
                let rs = input[base_in + 3];
                output[base_out] = (fl + 0.707 * ls) * 0.707;
                output[base_out + 1] = (fr + 0.707 * rs) * 0.707;
            }

            // 5.0 surround (FL FR C LS RS) vers stéréo — typique des SACD multicanal.
            // Downmix ITU-R BS.775 : Lo = FL + 0.707·C + 0.707·LS, Ro = FR + 0.707·C + 0.707·RS,
            // puis -6 dB global pour éviter le clipping quand les canaux sont corrélés.
            (5, 2) => {
                let fl = input[base_in];
                let fr = input[base_in + 1];
                let c = input[base_in + 2];
                let ls = input[base_in + 3];
                let rs = input[base_in + 4];
                output[base_out] = (fl + 0.707 * c + 0.707 * ls) * 0.5;
                output[base_out + 1] = (fr + 0.707 * c + 0.707 * rs) * 0.5;
            }

            // 5.1 (FL FR C LFE LS RS) vers stéréo
            (6, 2) => {
                let fl = input[base_in];
                let fr = input[base_in + 1];
                let c = input[base_in + 2];
                let lfe = input[base_in + 3];
                let ls = input[base_in + 4];
                let rs = input[base_in + 5];
                output[base_out] = (fl + 0.707 * c + 0.707 * ls + 0.5 * lfe) * 0.5;
                output[base_out + 1] = (fr + 0.707 * c + 0.707 * rs + 0.5 * lfe) * 0.5;
            }

            // Même nombre de canaux → copie directe
            (x, y) if x == y => {
                for ch in 0..x {
                    output[base_out + ch] = input[base_in + ch];
                }
            }

            // Par défaut : tronquer ou dupliquer sans distorsion
            _ => {
                let min_ch = input_channels.min(output_channels);
                for ch in 0..min_ch {
                    output[base_out + ch] = input[base_in + ch];
                }
                for ch in min_ch..output_channels {
                    output[base_out + ch] = 0.0;
                }
            }
        }
    }
}
