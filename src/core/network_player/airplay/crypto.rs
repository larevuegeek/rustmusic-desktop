//! Chiffrement AirPlay 1 : AES-128-CBC paquet par paquet, clé envoyée chiffrée en RSA-OAEP.

use aws_lc_rs::cipher::{EncryptingKey, EncryptionContext, UnboundCipherKey, AES_128};
use aws_lc_rs::iv::FixedLength;
use aws_lc_rs::rand::{SecureRandom, SystemRandom};
use aws_lc_rs::rsa::{OaepPublicEncryptingKey, PublicEncryptingKey, OAEP_SHA1_MGF1SHA1};
use base64::Engine;

/// Clé publique des AirPort Express (module RSA 2048 bits, exposant 65537), connue de tous les émetteurs.
const AIRPORT_MODULUS: &str = "59dE8qLieItsH1WgjrcFRKj6eUWqi+bGLOX1HL3U3GhC/j0Qg90u3sG/1CUtwC5vOYvfDmFI6oSFXi5ELabWJmT2dKHzBJKa3k9ok+8t9ucRqMd6DZHJ2YCCLlDRKSKv6kDqnw4UwPdpOMXziC/AMj3Z/lUVX1G7WSHCAWKf1zNS1eLvqr+boEjXuBOitnZ/bDzPHrTOZz0Dew0uowxf/+sG+NCK3eQJVxqcaJ/vEHKIVd2M+5qL71yJQ+87X6oV3eaYvt3zWZYD6z5vYTcrtij2VZ9Zmni/UAaHqn9JdsBWLUEpVviYnhimNVvYFZeCXg/IdTQ+x4IRdiXNv5hEew==";

/// Clé et IV d'une session ; l'IV repart de zéro à chaque paquet.
pub struct Cipher {
    key: EncryptingKey,
    pub raw_key: [u8; 16],
    pub iv: [u8; 16],
}

impl Cipher {
    pub fn random() -> Result<Self, String> {
        let rng = SystemRandom::new();
        let (mut raw_key, mut iv) = ([0u8; 16], [0u8; 16]);
        rng.fill(&mut raw_key).map_err(|_| "aléa indisponible")?;
        rng.fill(&mut iv).map_err(|_| "aléa indisponible")?;
        Self::new(raw_key, iv)
    }

    fn new(raw_key: [u8; 16], iv: [u8; 16]) -> Result<Self, String> {
        let unbound = UnboundCipherKey::new(&AES_128, &raw_key).map_err(|_| "clé AES refusée")?;
        let key = EncryptingKey::cbc(unbound).map_err(|_| "AES-CBC indisponible")?;
        Ok(Self { key, raw_key, iv })
    }

    /// Chiffre les blocs entiers de `payload` ; le reste (< 16 octets) part en clair, comme l'attend le récepteur.
    pub fn encrypt(&self, payload: &mut [u8]) {
        let whole = payload.len() / 16 * 16;
        if whole == 0 {
            return;
        }
        let context = EncryptionContext::Iv128(FixedLength::from(self.iv));
        if self.key.less_safe_encrypt(&mut payload[..whole], context).is_err() {
            log::warn!("📡 AirPlay : chiffrement d'un paquet impossible");
        }
    }

    /// Clé AES chiffrée pour l'appareil (champ `rsaaeskey` de l'annonce).
    pub fn wrapped_key(&self) -> Result<Vec<u8>, String> {
        let der = airport_public_key_der()?;
        let public = PublicEncryptingKey::from_der(&der).map_err(|e| format!("clé RSA : {e:?}"))?;
        let oaep = OaepPublicEncryptingKey::new(public).map_err(|_| "RSA-OAEP indisponible")?;
        let mut out = vec![0u8; oaep.ciphertext_size()];
        let n = oaep
            .encrypt(&OAEP_SHA1_MGF1SHA1, &self.raw_key, &mut out, None)
            .map_err(|_| "chiffrement RSA impossible")?
            .len();
        out.truncate(n);
        Ok(out)
    }
}

/// `SubjectPublicKeyInfo` DER de la clé des AirPort.
fn airport_public_key_der() -> Result<Vec<u8>, String> {
    let modulus = base64::engine::general_purpose::STANDARD
        .decode(AIRPORT_MODULUS)
        .map_err(|e| e.to_string())?;
    let rsa_key = der(0x30, &[der_uint(&modulus), der_uint(&[1, 0, 1])].concat());
    // rsaEncryption (1.2.840.113549.1.1.1), paramètres NULL.
    let algorithm = der(0x30, &[der(0x06, &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01]), vec![0x05, 0x00]].concat());
    let bits = der(0x03, &[&[0u8][..], &rsa_key].concat());
    Ok(der(0x30, &[algorithm, bits].concat()))
}

fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let len = content.len();
    if len < 0x80 {
        out.push(len as u8);
    } else {
        let bytes: Vec<u8> = len.to_be_bytes().into_iter().skip_while(|b| *b == 0).collect();
        out.push(0x80 | bytes.len() as u8);
        out.extend_from_slice(&bytes);
    }
    out.extend_from_slice(content);
    out
}

/// Entier DER positif : un zéro devant si le bit de poids fort est levé.
fn der_uint(bytes: &[u8]) -> Vec<u8> {
    let mut content = Vec::with_capacity(bytes.len() + 1);
    if bytes.first().is_some_and(|b| b & 0x80 != 0) {
        content.push(0);
    }
    content.extend_from_slice(bytes);
    der(0x02, &content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_lc_rs::cipher::{DecryptingKey, DecryptionContext};

    #[test]
    fn encrypts_whole_blocks_only() {
        let cipher = Cipher::new([7; 16], [9; 16]).unwrap();
        let plain: Vec<u8> = (0..40u8).collect();
        let mut data = plain.clone();
        cipher.encrypt(&mut data);
        assert_ne!(&data[..32], &plain[..32]);
        assert_eq!(&data[32..], &plain[32..]);

        let key = DecryptingKey::cbc(UnboundCipherKey::new(&AES_128, &[7; 16]).unwrap()).unwrap();
        let ctx = DecryptionContext::Iv128(FixedLength::from([9; 16]));
        let back = key.decrypt(&mut data[..32], ctx).unwrap();
        assert_eq!(back, &plain[..32]);
    }

    #[test]
    fn wraps_the_key_for_an_airport() {
        let modulus = base64::engine::general_purpose::STANDARD.decode(AIRPORT_MODULUS).unwrap();
        assert_eq!(modulus.len(), 256);
        let cipher = Cipher::random().unwrap();
        assert_eq!(cipher.wrapped_key().unwrap().len(), 256);
    }
}
