//! DoP (DSD over PCM) encoder — standard DoP v1.1.
//!
//! Empaquette un flux DSD 1-bit brut dans des trames PCM 24-bit pour
//! l'envoyer **tel quel** à un DAC compatible DoP. Le DAC reconnaît les
//! marqueurs et rejoue du **DSD natif** (pas de conversion PCM).
//!
//! # Format d'une trame DoP (24 bits, un échantillon PCM)
//! ```text
//!   bits 23..16 : marqueur (0x05 ou 0xFA, alterné à chaque trame)
//!   bits 15..8  : 8 bits DSD (échantillons les plus anciens, MSB-first)
//!   bits  7..0  : 8 bits DSD suivants (MSB-first)
//! ```
//! Chaque trame porte donc **16 bits DSD** (2 octets) par canal. Le rate
//! porteur PCM = `dsd_rate / 16` :
//! - DSD64  (2 822 400 Hz) → 176 400 Hz
//! - DSD128 (5 644 800 Hz) → 352 800 Hz
//! - DSD256 (11 289 600 Hz) → 705 600 Hz
//!
//! # Ordre des bits
//! DoP attend le DSD en **MSB-first** (échantillon le plus ancien = bit de
//! poids fort). Les fichiers **DSF** stockent en LSB-first → il faut inverser
//! les bits de chaque octet. Les **DFF** sont déjà MSB-first → pas d'inversion.
//!
//! # Sortie
//! On produit des `i32` interleavés par canal contenant **uniquement le
//! payload DSD** (2 octets) cadré pour WASAPI 24-bit :
//! ```text
//!   bits 31..24 : 0 (marqueur posé PAR LE BACKEND, pas ici)
//!   bits 23..16 : octet DSD ancien (hi)
//!   bits 15..8  : octet DSD suivant (lo)
//!   bits  7..0  : 0
//! ```
//! Le **marqueur DoP est posé par le backend render** (`run_wasapi_dop_playback`)
//! sur CHAQUE trame — silence comme musique — avec un compteur unique et
//! continu. C'est indispensable : deux générateurs de marqueurs séparés
//! (silence vs musique) se désynchroniseraient à la jonction → DoP invalide →
//! clic. Une seule source garantit l'alternance 0x05/0xFA ininterrompue.

/// Nombre d'octets DSD consommés par trame DoP et par canal (16 bits = 2 octets).
pub const DSD_BYTES_PER_DOP_FRAME: usize = 2;

/// Rate porteur PCM pour un rate DSD donné (dsd_rate / 16).
/// Ex. DSD64 2 822 400 → 176 400.
pub fn dop_carrier_rate(dsd_rate: u32) -> u32 {
    dsd_rate / 16
}

/// Table d'inversion de bits (256 entrées) : `REVERSE[b]` = octet `b` avec
/// l'ordre des 8 bits inversé. Sert à convertir DSF (LSB-first) → MSB-first.
static REVERSE: [u8; 256] = build_reverse_table();

const fn build_reverse_table() -> [u8; 256] {
    let mut t = [0u8; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut b = i as u8;
        let mut r = 0u8;
        let mut k = 0;
        while k < 8 {
            r = (r << 1) | (b & 1);
            b >>= 1;
            k += 1;
        }
        t[i] = r;
        i += 1;
    }
    t
}

/// Encodeur DoP (sans état de marqueur : le marqueur est posé par le backend).
/// Ne conserve que la parité d'inversion de bits selon le format source.
pub struct DopEncoder {
    channels: usize,
    /// `true` si la source stocke les bits en LSB-first (DSF) → inversion.
    lsb_first: bool,
}

impl DopEncoder {
    pub fn new(channels: u8, lsb_first: bool) -> Self {
        Self {
            channels: channels as usize,
            lsb_first,
        }
    }

    /// No-op conservé pour compat (plus d'état de marqueur à réinitialiser).
    pub fn reset(&mut self) {}

    fn normalize(&self, byte: u8) -> u32 {
        (if self.lsb_first { REVERSE[byte as usize] } else { byte }) as u32
    }

    /// Encode un super-bloc DSD (un bloc d'octets par canal, tous de même
    /// longueur) en échantillons DoP interleavés (PAYLOAD seul, sans marqueur).
    ///
    /// Layout de chaque i32 : `[0][hi][lo][0]` (bits 23..8 utiles). Le marqueur
    /// (bits 31..24) est posé par le backend render sur chaque trame.
    ///
    /// `blocks[ch]` = octets DSD du canal `ch`. Tous les canaux doivent avoir
    /// la même longueur (garanti par les readers DSF/DFF).
    pub fn encode_blocks(&mut self, blocks: &[Vec<u8>]) -> Vec<i32> {
        debug_assert_eq!(blocks.len(), self.channels, "channel count mismatch");
        if blocks.is_empty() {
            return Vec::new();
        }
        let bytes_per_channel = blocks[0].len();
        let frames = bytes_per_channel / DSD_BYTES_PER_DOP_FRAME;
        let mut out: Vec<i32> = Vec::with_capacity(frames * self.channels);

        for f in 0..frames {
            let byte_idx = f * DSD_BYTES_PER_DOP_FRAME;
            for ch in 0..self.channels {
                let block = &blocks[ch];
                let hi = self.normalize(block[byte_idx]);
                let lo = self.normalize(block[byte_idx + 1]);
                // Payload 24-bit sans marqueur : [0][hi][lo], cadré <<8 pour WASAPI.
                let payload = (hi << 8) | lo;
                out.push((payload << 8) as i32);
            }
        }

        out
    }
}

/// Pose le marqueur DoP (0x05/0xFA) sur un échantillon payload cadré WASAPI.
/// `marker_b=false` → 0x05, `true` → 0xFA. Le backend appelle ceci sur CHAQUE
/// trame (silence comme musique) avec un compteur continu.
#[inline]
pub fn stamp_marker(payload_sample: i32, marker_b: bool) -> i32 {
    let marker: u32 = if marker_b { 0xFA } else { 0x05 };
    ((payload_sample as u32) | (marker << 24)) as i32
}

/// Échantillon DoP « silence » (payload DSD idle 0x69), sans marqueur.
/// Le backend y ajoutera le marqueur via `stamp_marker`.
#[inline]
pub fn dop_silence_payload() -> i32 {
    let payload = (0x69u32 << 8) | 0x69;
    (payload << 8) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carrier_rates() {
        assert_eq!(dop_carrier_rate(2_822_400), 176_400); // DSD64
        assert_eq!(dop_carrier_rate(5_644_800), 352_800); // DSD128
        assert_eq!(dop_carrier_rate(11_289_600), 705_600); // DSD256
    }

    #[test]
    fn reverse_table() {
        assert_eq!(REVERSE[0b0000_0001], 0b1000_0000);
        assert_eq!(REVERSE[0b1000_0000], 0b0000_0001);
        assert_eq!(REVERSE[0xFF], 0xFF);
        assert_eq!(REVERSE[0x00], 0x00);
    }

    #[test]
    fn payload_layout_no_marker() {
        // 2 canaux, MSB-first (DFF), 4 octets par canal → 2 trames.
        let mut enc = DopEncoder::new(2, false);
        let blocks = vec![vec![0xAA, 0xBB, 0xCC, 0xDD], vec![0x11, 0x22, 0x33, 0x44]];
        let out = enc.encode_blocks(&blocks);
        assert_eq!(out.len(), 2 * 2);

        // Payload sans marqueur : [0][hi][lo][0]
        assert_eq!(out[0] as u32, 0x00AABB00); // ch0 trame0 : hi=AA lo=BB
        assert_eq!(out[1] as u32, 0x00112200); // ch1 trame0 : hi=11 lo=22
        assert_eq!(out[2] as u32, 0x00CCDD00); // ch0 trame1 : hi=CC lo=DD
        assert_eq!(out[3] as u32, 0x00334400); // ch1 trame1 : hi=33 lo=44
    }

    #[test]
    fn marker_stamping() {
        let payload = 0x00AABB00i32;
        assert_eq!(stamp_marker(payload, false) as u32, 0x05AABB00); // 0x05
        assert_eq!(stamp_marker(payload, true) as u32, 0xFAAABB00);  // 0xFA
    }

    #[test]
    fn silence_payload_idle() {
        assert_eq!(dop_silence_payload() as u32, 0x00696900);
        assert_eq!(stamp_marker(dop_silence_payload(), false) as u32, 0x05696900);
    }
}
