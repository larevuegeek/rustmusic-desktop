//! Point d'entrée public de l'écriture de tags.
//!
//! Miroir en écriture de `audio_metadata::extractor` : celui-ci détecte le
//! format puis délègue à un module de lecture, celui-là détecte le format puis
//! délègue à un module d'écriture.
//!
//! # Ce que ce module garantit à l'appelant
//! - **Rien n'est écrit si rien ne change** (cf. [`TagEdit::is_empty`]).
//! - **Un format non géré échoue proprement**, sans jamais toucher au fichier.
//! - **Le remplacement est atomique** (cf. `atomic_write`) : à aucun moment le
//!   fichier de l'utilisateur n'existe dans un état partiel.
//!
//! # Formats
//! L'écriture ne peut pas s'appuyer sur Symphonia, qui est strictement en
//! lecture. Chaque format écrit demande donc son propre encodeur, écrit ici —
//! exactement comme les parsers de `tag_format` et `file_format`.

use std::io::Read;
use std::path::Path;

use crate::core::audio_metadata::injector::dff_container;
use crate::core::audio_metadata::injector::dsf_container;
use crate::core::audio_metadata::injector::edit::TagEdit;
use crate::core::audio_metadata::injector::flac_container;
use crate::core::audio_metadata::injector::mp3_container;

/// Formats dont on sait réécrire les métadonnées.
///
/// Détection propre à l'écriture, distincte de `extractor::format_sniffer` :
/// « ce que je sais écrire » et « ce que je sais lire nativement » sont deux
/// questions différentes. Un MP3 se lit via Symphonia mais s'écrit ici ; le
/// sniffer de lecture le classe volontairement en `Unknown` pour laisser
/// Symphonia faire son travail, et il n'y a aucune raison de perturber ça.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritableFormat {
    Mp3,
    Dsf,
    Dff,
    Flac,
    /// Reconnu mais pas encore réinscriptible — le message dit lequel.
    Unsupported,
}

/// Échecs possibles d'une écriture de tags.
#[derive(Debug)]
pub enum InjectError {
    /// Le format du fichier ne sait pas encore recevoir de tags.
    UnsupportedFormat(String),
    /// Le fichier est illisible, mal formé, ou son bloc de tags est invalide.
    Malformed(String),
    /// La modification demandée est incohérente — typiquement une image
    /// référencée qui n'existe pas dans le fichier. Faute de l'appelant, pas
    /// du fichier : on refuse plutôt que d'écrire un résultat approximatif.
    InvalidRequest(String),
    /// Échec d'entrée/sortie (droits, disque plein, fichier verrouillé…).
    Io(String),
}

impl std::fmt::Display for InjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InjectError::UnsupportedFormat(m) => {
                write!(f, "Écriture de tags non prise en charge pour ce format : {m}")
            }
            InjectError::Malformed(m) => write!(f, "Fichier illisible ou mal formé : {m}"),
            InjectError::InvalidRequest(m) => write!(f, "Modification impossible : {m}"),
            InjectError::Io(m) => write!(f, "Erreur d'écriture : {m}"),
        }
    }
}

impl std::error::Error for InjectError {}

impl From<std::io::Error> for InjectError {
    fn from(e: std::io::Error) -> Self {
        InjectError::Io(e.to_string())
    }
}

/// Applique `edit` au fichier `path`.
///
/// Ne fait rien — et ne renvoie pas d'erreur — quand `edit` ne demande aucune
/// modification : c'est le cas courant quand l'utilisateur ouvre la fenêtre
/// d'édition et la referme sans rien changer.
pub fn apply(path: &Path, edit: &TagEdit) -> Result<(), InjectError> {
    if edit.is_empty() {
        log::debug!("🏷  Aucune modification demandée sur {}", path.display());
        return Ok(());
    }

    if !path.is_file() {
        return Err(InjectError::Io(format!(
            "fichier introuvable : {}",
            path.display()
        )));
    }

    match detect_writable(path) {
        WritableFormat::Mp3 => mp3_container::apply(path, edit),
        WritableFormat::Dsf => dsf_container::apply(path, edit),
        WritableFormat::Dff => dff_container::apply(path, edit),
        WritableFormat::Flac => flac_container::apply(path, edit),
        // Refus explicite plutôt qu'échec silencieux : l'utilisateur doit
        // savoir que son édition n'a pas été enregistrée.
        WritableFormat::Unsupported => Err(InjectError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("inconnu")
                .to_string(),
        )),
    }
}

/// Le fichier est-il réinscriptible, et dans quel format ?
///
/// Extension puis octets magiques : l'extension oriente, les octets tranchent.
/// Un fichier renommé en `.mp3` mais qui n'en est pas un doit être refusé
/// **avant** qu'on n'y écrive quoi que ce soit.
pub fn detect_writable(path: &Path) -> WritableFormat {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());

    match ext.as_deref() {
        Some("mp3") if looks_like_mp3(path) => WritableFormat::Mp3,
        Some("dsf") if starts_with(path, b"DSD ") => WritableFormat::Dsf,
        Some("dff") if starts_with(path, b"FRM8") => WritableFormat::Dff,
        Some("flac") if starts_with(path, b"fLaC") => WritableFormat::Flac,
        _ => WritableFormat::Unsupported,
    }
}

/// Le fichier commence-t-il par cette signature ?
fn starts_with(path: &Path, magic: &[u8; 4]) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut head = [0u8; 4];
    file.read_exact(&mut head).is_ok() && &head == magic
}

/// Un MP3 commence soit par un tag ID3v2 (`ID3`), soit directement par une
/// trame audio, dont les onze premiers bits sont à 1 (`0xFF` puis `0xEx`/`0xFx`).
fn looks_like_mp3(path: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut head = [0u8; 3];
    if file.read_exact(&mut head).is_err() {
        return false;
    }
    &head == b"ID3" || (head[0] == 0xFF && (head[1] & 0xE0) == 0xE0)
}

/// Sert à l'interface : ne pas proposer « Modifier les tags » sur un fichier
/// qu'on ne saura pas réenregistrer.
pub fn is_writable(path: &Path) -> bool {
    detect_writable(path) != WritableFormat::Unsupported
}
