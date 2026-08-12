//! Placement d'un tag ID3v2 dans un fichier DFF (DSDIFF).
//!
//! # Ce que le conteneur impose
//! Le DFF est un format IFF : une suite de chunks autodescriptifs
//! `identifiant (4) | taille (8, gros-boutiste) | données`, chacun complété à
//! une frontière paire, le tout enveloppé dans un chunk racine `FRM8` dont la
//! taille couvre tous les autres.
//!
//! Deux conséquences pour l'écriture :
//! - **aucun décalage à propager** — contrairement au DSF, rien ne stocke la
//!   position absolue d'un chunk, donc déplacer le tag ne casse rien ;
//! - **une seule taille à corriger** — celle de `FRM8`, qui englobe tout. La
//!   laisser fausse tronquerait la lecture au chunk près.
//!
//! # Le chunk `ID3 `
//! Il ne figure pas dans la spécification DSDIFF, qui prévoit `DIIN` (titre,
//! artiste, copyright) et rien de plus. Mais dBpoweramp, foobar2000 et la
//! plupart des rippeurs SACD écrivent un chunk `ID3 ` porteur d'un tag ID3v2
//! complet, et c'est ce que tous les lecteurs lisent réellement. On écrit donc
//! là où l'écosystème lit.
//!
//! Le `DIIN` existant n'est pas touché : le réécrire demanderait de trancher
//! entre deux sources de vérité à chaque lecture, alors qu'il est aujourd'hui
//! ignoré au profit de l'ID3 quand les deux sont présents.
//!
//! # Recopie au fil de l'eau
//! Comme pour le DSF, l'audio est recopié par blocs sans passer par la mémoire.

use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::core::audio_metadata::injector::atomic_write;
use crate::core::audio_metadata::injector::edit::TagEdit;
use crate::core::audio_metadata::injector::id3v2_writer;
use crate::core::audio_metadata::injector::injector::InjectError;

/// En-tête `FRM8` : identifiant (4) + taille (8) + type de formulaire (4).
const FORM_HEADER_LEN: u64 = 16;
/// En-tête d'un chunk IFF : identifiant (4) + taille (8).
const CHUNK_HEADER_LEN: u64 = 12;

/// Un morceau du fichier reconstruit.
enum Part {
    /// Plage à recopier depuis l'original : (offset, longueur).
    Copy(u64, u64),
    /// Emplacement du nouveau chunk `ID3 `.
    Tag,
}

/// Applique `edit` au fichier DFF `path`.
pub fn apply(path: &Path, edit: &TagEdit) -> Result<(), InjectError> {
    let mut file = fs::File::open(path)?;

    let mut head = [0u8; FORM_HEADER_LEN as usize];
    file.read_exact(&mut head).map_err(|_| {
        InjectError::Malformed("fichier trop court pour un en-tête DFF".into())
    })?;
    if &head[0..4] != b"FRM8" {
        return Err(InjectError::Malformed(
            "signature DFF absente (« FRM8 » attendu)".into(),
        ));
    }
    if &head[12..16] != b"DSD " {
        return Err(InjectError::Malformed(
            "formulaire DFF inattendu (« DSD » attendu)".into(),
        ));
    }

    let file_len = fs::metadata(path)?.len();
    let form_size = u64::from_be_bytes(head[4..12].try_into().unwrap());
    // Une taille déclarée plus grande que le fichier arrive sur les fichiers
    // tronqués : on ne parcourt que ce qui existe vraiment.
    let body_end = (CHUNK_HEADER_LEN + form_size).min(file_len);

    let (mut parts, existing) = scan_chunks(&mut file, body_end, file_len)?;

    let new_tag = id3v2_writer::rewrite(existing.as_deref(), edit)?;
    let chunk = build_id3_chunk(&new_tag);

    // Aucun chunk `ID3 ` d'origine : on l'ajoute à la fin, là où le placent les
    // outils qui en écrivent.
    if !parts.iter().any(|p| matches!(p, Part::Tag)) {
        parts.push(Part::Tag);
    }

    let body_len: u64 = parts
        .iter()
        .map(|p| match p {
            Part::Copy(_, len) => *len,
            Part::Tag => chunk.len() as u64,
        })
        .sum();

    // La taille de `FRM8` couvre le type de formulaire (4 octets) et tous les
    // chunks — soit tout le fichier moins son propre en-tête de 12 octets.
    let form_size = FORM_HEADER_LEN - CHUNK_HEADER_LEN + body_len;
    head[4..12].copy_from_slice(&form_size.to_be_bytes());

    atomic_write::replace_file_with(path, move |out| {
        out.write_all(&head)?;
        for part in &parts {
            match part {
                Part::Copy(offset, len) => {
                    file.seek(SeekFrom::Start(*offset))?;
                    io::copy(&mut (&mut file).take(*len), out)?;
                }
                Part::Tag => out.write_all(&chunk)?,
            }
        }
        Ok(())
    })?;

    log::info!(
        "🏷  Tags DFF réécrits : {} (tag {} octets)",
        path.display(),
        new_tag.len()
    );
    Ok(())
}

/// Parcourt les chunks de premier niveau et découpe le fichier en morceaux.
///
/// Le chunk `ID3 ` existant est remplacé **à sa place**. Le déplacer serait sans
/// conséquence pour un lecteur conforme, mais laisser la disposition inchangée
/// évite de parier là-dessus.
fn scan_chunks(
    file: &mut fs::File,
    body_end: u64,
    file_len: u64,
) -> Result<(Vec<Part>, Option<Vec<u8>>), InjectError> {
    let mut parts = Vec::new();
    let mut existing = None;
    let mut cursor = FORM_HEADER_LEN;
    let mut keep_from = FORM_HEADER_LEN;

    while cursor + CHUNK_HEADER_LEN <= body_end {
        file.seek(SeekFrom::Start(cursor))?;
        let mut header = [0u8; CHUNK_HEADER_LEN as usize];
        if file.read_exact(&mut header).is_err() {
            break;
        }

        let size = u64::from_be_bytes(header[4..12].try_into().unwrap());
        if cursor + CHUNK_HEADER_LEN + size > body_end {
            // Chunk tronqué ou taille aberrante : on s'arrête là et le reste du
            // fichier sera recopié tel quel, sans tenter de l'interpréter.
            break;
        }
        // Les chunks sont alignés sur une frontière paire.
        let advance = CHUNK_HEADER_LEN + size + (size % 2);

        if &header[0..4] == b"ID3 " {
            if cursor > keep_from {
                parts.push(Part::Copy(keep_from, cursor - keep_from));
            }
            let mut blob = vec![0u8; size as usize];
            file.read_exact(&mut blob)?;
            existing = Some(blob);
            parts.push(Part::Tag);
            keep_from = cursor + advance;
        }

        cursor += advance;
    }

    // Tout ce qui suit — chunks restants, alignement, éventuelles données que
    // le parcours n'a pas su lire — est recopié sans interprétation.
    if file_len > keep_from {
        parts.push(Part::Copy(keep_from, file_len - keep_from));
    }

    Ok((parts, existing))
}

fn build_id3_chunk(tag: &[u8]) -> Vec<u8> {
    let mut chunk = Vec::with_capacity(CHUNK_HEADER_LEN as usize + tag.len() + 1);
    chunk.extend_from_slice(b"ID3 ");
    chunk.extend_from_slice(&(tag.len() as u64).to_be_bytes());
    chunk.extend_from_slice(tag);
    // Alignement pair : sans lui, le chunk suivant démarrerait sur une position
    // impaire et le parcours se désynchroniserait.
    if tag.len() % 2 == 1 {
        chunk.push(0);
    }
    chunk
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::audio_metadata::injector::edit::FieldEdit;
    use crate::core::audio_metadata::tag_format::id3v2;

    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(id);
        out.extend_from_slice(&(body.len() as u64).to_be_bytes());
        out.extend_from_slice(body);
        if body.len() % 2 == 1 {
            out.push(0);
        }
        out
    }

    /// Fabrique un DFF minimal : FVER, une charge audio, et éventuellement un
    /// chunk `ID3 ` à la position demandée.
    fn build_dff(audio: &[u8], tag: Option<&[u8]>, tag_first: bool) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&chunk(b"FVER", &[1, 5, 0, 0]));
        if tag_first {
            if let Some(t) = tag {
                body.extend_from_slice(&chunk(b"ID3 ", t));
            }
        }
        body.extend_from_slice(&chunk(b"DSD ", audio));
        if !tag_first {
            if let Some(t) = tag {
                body.extend_from_slice(&chunk(b"ID3 ", t));
            }
        }

        let mut out = Vec::new();
        out.extend_from_slice(b"FRM8");
        out.extend_from_slice(&((4 + body.len()) as u64).to_be_bytes());
        out.extend_from_slice(b"DSD ");
        out.extend_from_slice(&body);
        out
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rustmusic-dff-{tag}"));
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

    /// Relit le fichier comme un lecteur : parcours des chunks depuis `FRM8`,
    /// en s'appuyant sur la taille déclarée. Renvoie les chunks trouvés.
    fn walk(path: &Path) -> Vec<([u8; 4], Vec<u8>)> {
        let data = fs::read(path).unwrap();
        let form_size = u64::from_be_bytes(data[4..12].try_into().unwrap());
        let end = (12 + form_size) as usize;
        assert!(end <= data.len(), "FRM8 déclare plus que le fichier ne contient");

        let mut found = Vec::new();
        let mut cursor = 16usize;
        while cursor + 12 <= end {
            let mut id = [0u8; 4];
            id.copy_from_slice(&data[cursor..cursor + 4]);
            let size = u64::from_be_bytes(data[cursor + 4..cursor + 12].try_into().unwrap()) as usize;
            assert!(cursor + 12 + size <= end, "chunk {id:?} déborde de FRM8");
            found.push((id, data[cursor + 12..cursor + 12 + size].to_vec()));
            cursor += 12 + size + (size % 2);
        }
        assert_eq!(cursor, end, "le parcours ne retombe pas sur la fin de FRM8");
        found
    }

    fn title_of(path: &Path) -> Option<String> {
        walk(path)
            .into_iter()
            .find(|(id, _)| id == b"ID3 ")
            .and_then(|(_, blob)| id3v2::parse(&blob).ok())
            .and_then(|t| id3v2::to_audio_tags(&t).title)
    }

    #[test]
    fn adds_a_tag_to_a_file_that_had_none() {
        let dir = temp_dir("untagged");
        let file = dir.join("piste.dff");
        fs::write(&file, build_dff(b"AUDIO-DSD", None, false)).unwrap();

        apply(&file, &set_title("A Love Supreme")).unwrap();

        assert_eq!(title_of(&file).as_deref(), Some("A Love Supreme"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_form_size_matches_the_new_file() {
        // Une taille FRM8 fausse tronque la lecture au chunk près : `walk`
        // échoue si elle ne retombe pas exactement sur la fin.
        let dir = temp_dir("form-size");
        let file = dir.join("piste.dff");
        fs::write(&file, build_dff(b"AUDIO-DSD", None, false)).unwrap();

        apply(&file, &set_title("Court")).unwrap();
        apply(
            &file,
            &TagEdit {
                comment: FieldEdit::Set("un commentaire bien plus long".repeat(50)),
                ..Default::default()
            },
        )
        .unwrap();

        let chunks = walk(&file);
        let data = fs::read(&file).unwrap();
        let form_size = u64::from_be_bytes(data[4..12].try_into().unwrap());
        assert_eq!(12 + form_size, data.len() as u64);
        assert!(chunks.iter().any(|(id, _)| id == b"DSD "));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_audio_chunk_comes_out_untouched() {
        let dir = temp_dir("audio");
        let file = dir.join("piste.dff");
        fs::write(&file, build_dff(b"AUDIO-DSD-INTACT", None, false)).unwrap();

        apply(&file, &set_title("Peu importe")).unwrap();

        let audio = walk(&file)
            .into_iter()
            .find(|(id, _)| id == b"DSD ")
            .map(|(_, b)| b);
        assert_eq!(audio.as_deref(), Some(&b"AUDIO-DSD-INTACT"[..]));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn keeps_the_other_chunks() {
        let dir = temp_dir("others");
        let file = dir.join("piste.dff");
        fs::write(&file, build_dff(b"AUDIO", None, false)).unwrap();

        apply(&file, &set_title("x")).unwrap();

        let ids: Vec<_> = walk(&file).into_iter().map(|(id, _)| id).collect();
        assert!(ids.contains(b"FVER"), "le chunk FVER a disparu");
        assert!(ids.contains(b"DSD "), "le chunk audio a disparu");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replaces_an_existing_tag_in_place() {
        // Le chunk ID3 est ici AVANT l'audio : il doit y rester, sans quoi on
        // parierait sur la tolérance des lecteurs au réordonnancement.
        let dir = temp_dir("in-place");
        let file = dir.join("piste.dff");
        let seed = id3v2_writer::rewrite(None, &set_title("Ancien")).unwrap();
        fs::write(&file, build_dff(b"AUDIO", Some(&seed), true)).unwrap();

        apply(&file, &set_title("Nouveau")).unwrap();

        let ids: Vec<_> = walk(&file).into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids[0], *b"FVER");
        assert_eq!(ids[1], *b"ID3 ", "le tag a été déplacé");
        assert_eq!(ids[2], *b"DSD ");
        assert_eq!(title_of(&file).as_deref(), Some("Nouveau"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn does_not_stack_tags() {
        let dir = temp_dir("stack");
        let file = dir.join("piste.dff");
        fs::write(&file, build_dff(b"AUDIO", None, false)).unwrap();

        apply(&file, &set_title("Version 1")).unwrap();
        let first = fs::metadata(&file).unwrap().len();
        apply(&file, &set_title("Version 2")).unwrap();

        let tags = walk(&file).into_iter().filter(|(id, _)| id == b"ID3 ").count();
        assert_eq!(tags, 1, "deux chunks ID3 cohabitent");
        assert_eq!(fs::metadata(&file).unwrap().len(), first);
        assert_eq!(title_of(&file).as_deref(), Some("Version 2"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pads_an_odd_sized_tag() {
        // Sans l'octet d'alignement, le chunk suivant démarrerait sur une
        // position impaire et tout le parcours se désynchroniserait.
        let dir = temp_dir("padding");
        let file = dir.join("piste.dff");
        fs::write(&file, build_dff(b"AUDIO", None, true)).unwrap();

        apply(&file, &set_title("Titre de longueur impaire ?")).unwrap();

        // `walk` vérifie déjà l'alignement de bout en bout.
        assert!(walk(&file).iter().any(|(id, _)| id == b"DSD "));
        let _ = fs::remove_dir_all(&dir);
    }
}
