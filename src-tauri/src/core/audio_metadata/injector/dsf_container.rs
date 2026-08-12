//! Placement d'un tag ID3v2 dans un fichier DSF.
//!
//! # Ce que le conteneur DSF impose, et que le MP3 n'imposait pas
//! Le tag se place **à la fin**, après l'audio — mais l'en-tête du fichier
//! garde deux champs qui le décrivent :
//!
//! ```text
//! offset 12 (8 octets, LE) : taille totale du fichier
//! offset 20 (8 octets, LE) : position absolue du chunk de métadonnées
//! ```
//!
//! Les laisser tels quels après avoir réécrit le tag ne « dégrade » pas le
//! fichier, ça le **casse** : un lecteur qui suit l'ancien pointeur atterrit au
//! milieu de l'audio et interprète des échantillons DSD comme un en-tête ID3.
//! Ces deux champs sont donc réécrits à chaque fois, et c'est toute la
//! différence avec le MP3, où rien ne référence le tag.
//!
//! # Recopie au fil de l'eau
//! Un DSD64 stéréo pèse couramment 300 Mo. On ne le charge pas en mémoire pour
//! changer un titre : l'audio est recopié par blocs vers le fichier temporaire
//! (voir `atomic_write::replace_file_with`), seul le tag transite en RAM.

use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::core::audio_metadata::injector::atomic_write;
use crate::core::audio_metadata::injector::edit::TagEdit;
use crate::core::audio_metadata::injector::id3v2_writer;
use crate::core::audio_metadata::injector::injector::InjectError;

/// Taille du chunk `DSD ` d'en-tête, fixée par la spécification.
const HEADER_LEN: u64 = 28;

/// Applique `edit` au fichier DSF `path`.
pub fn apply(path: &Path, edit: &TagEdit) -> Result<(), InjectError> {
    let mut file = fs::File::open(path)?;

    let mut header = [0u8; HEADER_LEN as usize];
    file.read_exact(&mut header).map_err(|_| {
        InjectError::Malformed("fichier trop court pour un en-tête DSF".into())
    })?;
    if &header[0..4] != b"DSD " {
        return Err(InjectError::Malformed(
            "signature DSF absente (« DSD » attendu)".into(),
        ));
    }

    let file_len = fs::metadata(path)?.len();
    let metadata_offset = u64::from_le_bytes(header[20..28].try_into().unwrap());

    // Un pointeur nul signifie « aucune métadonnée ». Un pointeur incohérent —
    // hors du fichier, ou empiétant sur l'en-tête — vient d'un fichier abîmé :
    // on le traite comme une absence plutôt que d'aller lire n'importe où.
    let audio_len = if metadata_offset >= HEADER_LEN && metadata_offset < file_len {
        metadata_offset
    } else {
        file_len
    };

    let existing = if audio_len < file_len {
        file.seek(SeekFrom::Start(audio_len))?;
        let mut blob = Vec::new();
        file.read_to_end(&mut blob)?;

        // Le pointeur annonce un tag, mais ce n'en est pas un. Réécrire
        // reviendrait à jeter des octets qu'on ne sait pas identifier ; on
        // préfère refuser et laisser le fichier intact.
        if blob.len() < 10 || &blob[0..3] != b"ID3" {
            return Err(InjectError::Malformed(
                "le chunk de métadonnées DSF ne contient pas de tag ID3v2".into(),
            ));
        }
        Some(blob)
    } else {
        None
    };

    let new_tag = id3v2_writer::rewrite(existing.as_deref(), edit)?;
    let tag_len = new_tag.len() as u64;
    let total_len = audio_len + tag_len;

    // Les deux champs que le DSF impose de tenir à jour.
    header[12..20].copy_from_slice(&total_len.to_le_bytes());
    header[20..28].copy_from_slice(&audio_len.to_le_bytes());

    atomic_write::replace_file_with(path, move |out| {
        out.write_all(&header)?;
        file.seek(SeekFrom::Start(HEADER_LEN))?;
        io::copy(&mut file.take(audio_len - HEADER_LEN), out)?;
        out.write_all(&new_tag)
    })?;

    log::info!(
        "🏷  Tags DSF réécrits : {} (audio {} octets, tag {} octets)",
        path.display(),
        audio_len,
        tag_len
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::audio_metadata::injector::edit::FieldEdit;
    use crate::core::audio_metadata::tag_format::id3v2;

    /// Fabrique un DSF minimal mais structurellement valide.
    ///
    /// Le contenu du chunk `fmt ` importe peu ici : ce module ne le lit pas et
    /// ne doit jamais y toucher. Ce qu'on vérifie, c'est qu'il ressort intact.
    fn build_dsf(audio: &[u8], tag: Option<&[u8]>) -> Vec<u8> {
        let mut fmt = vec![0u8; 52];
        fmt[0..4].copy_from_slice(b"fmt ");
        fmt[4..12].copy_from_slice(&52u64.to_le_bytes());

        let mut data = Vec::new();
        data.extend_from_slice(b"data");
        data.extend_from_slice(&((12 + audio.len()) as u64).to_le_bytes());
        data.extend_from_slice(audio);

        let audio_len = (HEADER_LEN as usize + fmt.len() + data.len()) as u64;
        let total = audio_len + tag.map_or(0, |t| t.len()) as u64;

        let mut out = Vec::new();
        out.extend_from_slice(b"DSD ");
        out.extend_from_slice(&28u64.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&(if tag.is_some() { audio_len } else { 0 }).to_le_bytes());
        out.extend_from_slice(&fmt);
        out.extend_from_slice(&data);
        if let Some(t) = tag {
            out.extend_from_slice(t);
        }
        out
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rustmusic-dsf-{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn set_title(value: &str) -> TagEdit {
        TagEdit {
            title: FieldEdit::Set(value.to_string()),
            ..Default::default()
        }
    }

    /// Relit le fichier en suivant **le pointeur de l'en-tête**, comme le ferait
    /// n'importe quel lecteur — c'est le seul moyen de prouver qu'il est juste.
    fn read_via_header(path: &Path) -> (u64, u64, Option<String>) {
        let data = fs::read(path).unwrap();
        let total = u64::from_le_bytes(data[12..20].try_into().unwrap());
        let offset = u64::from_le_bytes(data[20..28].try_into().unwrap());
        let title = if offset > 0 && (offset as usize) < data.len() {
            id3v2::parse(&data[offset as usize..])
                .ok()
                .and_then(|t| id3v2::to_audio_tags(&t).title)
        } else {
            None
        };
        (total, offset, title)
    }

    #[test]
    fn adds_a_tag_to_a_file_that_had_none() {
        let dir = temp_dir("untagged");
        let file = dir.join("piste.dsf");
        fs::write(&file, build_dsf(b"AUDIO-DSD", None)).unwrap();
        let audio_len = fs::metadata(&file).unwrap().len();

        apply(&file, &set_title("Blue Train")).unwrap();

        let (total, offset, title) = read_via_header(&file);
        assert_eq!(title.as_deref(), Some("Blue Train"));
        assert_eq!(offset, audio_len, "le pointeur ne vise pas la fin de l'audio");
        assert_eq!(
            total,
            fs::metadata(&file).unwrap().len(),
            "la taille annoncée ne correspond pas au fichier"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_header_pointer_follows_the_new_tag() {
        // Le vrai risque du DSF : un tag qui change de taille sans que
        // l'en-tête suive. Un lecteur atterrirait alors en plein audio.
        let dir = temp_dir("pointer");
        let file = dir.join("piste.dsf");
        fs::write(&file, build_dsf(b"AUDIO-DSD", None)).unwrap();

        apply(&file, &set_title("Court")).unwrap();
        let (_, first_offset, _) = read_via_header(&file);

        apply(
            &file,
            &TagEdit {
                comment: FieldEdit::Set("un commentaire nettement plus long".repeat(40)),
                ..Default::default()
            },
        )
        .unwrap();

        let (total, offset, title) = read_via_header(&file);
        assert_eq!(offset, first_offset, "l'audio a bougé");
        assert_eq!(total, fs::metadata(&file).unwrap().len());
        assert_eq!(title.as_deref(), Some("Court"), "le titre a été perdu");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_audio_comes_out_untouched() {
        let dir = temp_dir("audio");
        let file = dir.join("piste.dsf");
        let before = build_dsf(b"AUDIO-DSD-INTACT", None);
        fs::write(&file, &before).unwrap();
        let audio_len = before.len();

        apply(&file, &set_title("Peu importe")).unwrap();

        let after = fs::read(&file).unwrap();
        // Tout sauf les deux champs de l'en-tête doit être identique.
        assert_eq!(&after[28..audio_len], &before[28..], "l'audio a été modifié");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replaces_an_existing_tag_without_stacking() {
        let dir = temp_dir("existing");
        let file = dir.join("piste.dsf");
        fs::write(&file, build_dsf(b"AUDIO", None)).unwrap();

        apply(&file, &set_title("Version 1")).unwrap();
        let first = fs::metadata(&file).unwrap().len();
        apply(&file, &set_title("Version 2")).unwrap();
        let second = fs::metadata(&file).unwrap().len();

        assert_eq!(first, second, "le tag s'est empilé");
        assert_eq!(read_via_header(&file).2.as_deref(), Some("Version 2"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_a_metadata_chunk_that_is_not_id3() {
        // Réécrire jetterait des octets qu'on ne sait pas identifier.
        let dir = temp_dir("bogus");
        let file = dir.join("piste.dsf");
        fs::write(&file, build_dsf(b"AUDIO", Some(b"CECI N'EST PAS UN TAG ID3"))).unwrap();
        let before = fs::read(&file).unwrap();

        assert!(apply(&file, &set_title("x")).is_err());
        assert_eq!(fs::read(&file).unwrap(), before, "le fichier a été touché");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_inconsistent_pointer_is_treated_as_no_tag() {
        // Fichier abîmé : le pointeur vise hors du fichier. On écrit un tag
        // neuf à la fin plutôt que d'aller lire n'importe où.
        let dir = temp_dir("garbage-pointer");
        let file = dir.join("piste.dsf");
        let mut content = build_dsf(b"AUDIO", None);
        content[20..28].copy_from_slice(&999_999u64.to_le_bytes());
        let audio_len = content.len() as u64;
        fs::write(&file, &content).unwrap();

        apply(&file, &set_title("Récupéré")).unwrap();

        let (_, offset, title) = read_via_header(&file);
        assert_eq!(offset, audio_len);
        assert_eq!(title.as_deref(), Some("Récupéré"));
        let _ = fs::remove_dir_all(&dir);
    }
}
