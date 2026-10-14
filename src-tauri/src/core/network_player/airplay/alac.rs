//! Trame ALAC « non compressée » : l'en-tête ALAC puis les échantillons 16 bits bruts.
//!
//! AirPlay 1 n'exige pas de vraie compression ; c'est ce qu'envoient les émetteurs
//! libres (libraop, OwnTones). Le contenu est bit pour bit le PCM 16 bits.

/// Échantillons par trame et par canal, valeur convenue dans l'annonce SDP.
pub const FRAMES_PER_PACKET: usize = 352;

struct BitWriter<'a> {
    out: &'a mut Vec<u8>,
    acc: u64,
    bits: u32,
}

impl BitWriter<'_> {
    fn put(&mut self, value: u32, n: u32) {
        self.acc = (self.acc << n) | (value as u64 & ((1u64 << n) - 1));
        self.bits += n;
        while self.bits >= 8 {
            self.bits -= 8;
            self.out.push((self.acc >> self.bits) as u8);
        }
    }

    fn finish(self) {
        if self.bits > 0 {
            self.out.push((self.acc << (8 - self.bits)) as u8);
        }
    }
}

/// Encode des échantillons stéréo entrelacés (G, D, G, D…) en une trame ALAC.
pub fn encode_stereo(pcm: &[i16], out: &mut Vec<u8>) {
    let frames = pcm.len() / 2;
    out.clear();
    out.reserve(8 + frames * 4);
    let mut w = BitWriter { out, acc: 0, bits: 0 };
    w.put(1, 3); // élément stéréo (CPE)
    w.put(0, 4); // instance
    w.put(0, 12); // inutilisé
    w.put(1, 1); // nombre d'échantillons présent
    w.put(0, 2); // octets décalés
    w.put(1, 1); // non compressé
    w.put(frames as u32, 32);
    for s in &pcm[..frames * 2] {
        w.put(*s as u16 as u32, 16);
    }
    w.put(7, 3); // fin de trame
    w.finish();
}

#[cfg(test)]
mod tests {
    use super::*;

    struct BitReader<'a> {
        data: &'a [u8],
        pos: usize,
    }

    impl BitReader<'_> {
        fn get(&mut self, n: usize) -> u32 {
            let mut v = 0u32;
            for _ in 0..n {
                let bit = (self.data[self.pos / 8] >> (7 - self.pos % 8)) & 1;
                v = (v << 1) | bit as u32;
                self.pos += 1;
            }
            v
        }
    }

    #[test]
    fn writes_a_readable_uncompressed_frame() {
        let pcm: Vec<i16> = (0..FRAMES_PER_PACKET * 2).map(|i| (i as i16).wrapping_mul(97).wrapping_sub(12000)).collect();
        let mut out = Vec::new();
        encode_stereo(&pcm, &mut out);
        // 55 bits d'en-tête, 32 par trame, 3 de fin : 1416 octets pour 352 trames.
        assert_eq!(out.len(), (55 + 32 * FRAMES_PER_PACKET + 3).div_ceil(8));
        // Mêmes premiers octets que libraop.
        assert_eq!(&out[..3], &[0x20, 0x00, 0x12]);

        let mut r = BitReader { data: &out, pos: 0 };
        assert_eq!(r.get(3), 1);
        assert_eq!(r.get(4 + 12), 0);
        assert_eq!(r.get(1), 1);
        assert_eq!(r.get(2), 0);
        assert_eq!(r.get(1), 1);
        assert_eq!(r.get(32) as usize, FRAMES_PER_PACKET);
        for s in &pcm {
            assert_eq!(r.get(16) as u16 as i16, *s);
        }
        assert_eq!(r.get(3), 7);
    }
}
