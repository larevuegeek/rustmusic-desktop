//! Placement d'un tag ID3v2 dans un fichier MP3.
//!
//! # Ce que le conteneur MP3 impose
//! Rien, ou presque : le tag ID3v2 se pose **avant** le flux audio, sans que
//! rien ailleurs ne référence sa taille ni sa position. Remplacer le tag revient
//! donc à recomposer le fichier « nouveau tag + audio inchangé ». C'est le
//! conteneur le plus simple des trois — le DSF, lui, inscrit la position et la
//! taille de son chunk ID3 dans son en-tête.
//!
//! # L'ID3v1 de fin de fichier
//! Certains MP3 portent aussi un tag ID3v1 dans leurs 128 derniers octets —
//! très fréquent sur les fichiers rippés, souvent porteur d'un commentaire de
//! type watermark.
//!
//! On avait d'abord choisi de ne pas y toucher, en supposant que tout lecteur
//! moderne privilégie l'ID3v2. **C'est faux en pratique** : certains lecteurs
//! (dont Symphonia, donc notre propre affichage) remontent la valeur ID3v1
//! quand elle existe. Résultat, on écrivait bien le nouveau commentaire en
//! ID3v2 mais l'ancien restait visible — le tag semblait « ne pas
//! s'enregistrer ».
//!
//! On met donc les deux en cohérence : quand un bloc ID3v1 est présent, il est
//! réécrit à partir des valeurs finales. On ne le supprime pas — certains
//! vieux appareils ne lisent que lui.

use std::fs;
use std::path::Path;

use crate::core::audio_metadata::injector::atomic_write;
use crate::core::audio_metadata::injector::edit::TagEdit;
use crate::core::audio_metadata::injector::id3v2_writer;
use crate::core::audio_metadata::injector::injector::InjectError;
use crate::core::audio_metadata::tag_format::id3v2;
use crate::entity::audio::audio_tags::AudioTags;

/// Taille fixe d'un bloc ID3v1, toujours en toute fin de fichier.
const ID3V1_LEN: usize = 128;

fn has_id3v1(audio: &[u8]) -> bool {
    audio.len() >= ID3V1_LEN && &audio[audio.len() - ID3V1_LEN..][..3] == b"TAG"
}

/// Réécrit un bloc ID3v1 (128 octets) à partir des valeurs finales.
///
/// Le format est rigide : champs de taille fixe, ISO-8859-1, pas d'extension
/// possible. Ce qui dépasse est tronqué — c'est une limite du format, pas une
/// perte de notre fait : l'ID3v2 conserve la valeur complète.
fn sync_id3v1(block: &mut [u8], tags: &AudioTags) {
    /// Écrit `value` dans `field`, complété de zéros, tronqué si trop long.
    fn put(field: &mut [u8], value: Option<&str>) {
        field.fill(0);
        let Some(v) = value else { return };
        for (slot, ch) in field.iter_mut().zip(v.chars()) {
            // ISO-8859-1 : au-delà, on met un point d'interrogation plutôt que
            // des octets incohérents.
            *slot = if (ch as u32) <= 0xFF { ch as u8 } else { b'?' };
        }
    }

    put(&mut block[3..33], tags.title.as_deref());
    put(&mut block[33..63], tags.artist.as_deref());
    put(&mut block[63..93], tags.album.as_deref());
    put(&mut block[93..97], tags.year.as_deref());

    // ID3v1.1 : les deux derniers octets du commentaire portent le numéro de
    // piste (un zéro puis le numéro). On préserve cette convention.
    let track = tags.track_number.unwrap_or(0);
    if track > 0 && track <= 255 {
        put(&mut block[97..125], tags.comment.as_deref());
        block[125] = 0;
        block[126] = track as u8;
    } else {
        put(&mut block[97..127], tags.comment.as_deref());
    }
    // Le genre reste inchangé : l'ID3v1 ne connaît qu'une liste numérotée
    // figée, où « Hip-Hop » n'a pas toujours d'équivalent. Le réécrire mal
    // serait pire que de le laisser.
}

/// Met le bloc ID3v1 de fin en cohérence avec le tag ID3v2 qu'on vient d'écrire.
///
/// Renvoie vrai si un bloc était présent et a été réécrit. Sans cette
/// synchronisation il continue d'être remonté par certains lecteurs — dont le
/// nôtre — et masque la modification : le tag semble « ne pas s'enregistrer ».
fn sync_trailing_id3v1(audio: &mut [u8], new_tag: &[u8]) -> bool {
    if !has_id3v1(audio) {
        return false;
    }
    let Ok(parsed) = id3v2::parse(new_tag) else {
        return false;
    };
    let start = audio.len() - ID3V1_LEN;
    sync_id3v1(&mut audio[start..], &id3v2::to_audio_tags(&parsed));
    true
}

/// Applique `edit` au fichier MP3 `path`.
pub fn apply(path: &Path, edit: &TagEdit) -> Result<(), InjectError> {
    let data = fs::read(path)?;

    let tag_len = id3v2_writer::tag_len(&data);
    let existing = if tag_len > 0 {
        Some(&data[..tag_len])
    } else {
        None
    };

    // Chemin rapide : si le nouveau tag tient dans la place déjà réservée, on
    // l'écrit par-dessus l'ancien et l'audio n'est jamais recopié. Mesuré à
    // vingt-trois fois plus rapide sur un fichier de 34 Mo via un partage
    // réseau — voir `atomic_write::write_in_place`.
    if tag_len > 0 {
        if let Some(tag) = id3v2_writer::rewrite_sized(existing, edit, tag_len)? {
            let mut audio = data[tag_len..].to_vec();
            let synced = sync_trailing_id3v1(&mut audio, &tag);

            atomic_write::write_in_place(path, 0, &tag)?;
            if synced {
                let offset = (tag_len + audio.len() - ID3V1_LEN) as u64;
                atomic_write::write_in_place(path, offset, &audio[audio.len() - ID3V1_LEN..])?;
            }

            log::info!("🏷  Tags MP3 réécrits sur place : {}", path.display());
            return Ok(());
        }
    }

    let new_tag = id3v2_writer::rewrite(existing, edit)?;

    let mut audio = data[tag_len..].to_vec();

    sync_trailing_id3v1(&mut audio, &new_tag);

    let mut out = Vec::with_capacity(new_tag.len() + audio.len());
    out.extend_from_slice(&new_tag);
    out.extend_from_slice(&audio);

    atomic_write::replace_file(path, &out)?;

    log::info!(
        "🏷  Tags MP3 réécrits : {} (tag {} → {} octets)",
        path.display(),
        tag_len,
        new_tag.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::audio_metadata::injector::edit::FieldEdit;
    use crate::core::audio_metadata::tag_format::id3v2;

    /// Une trame MP3 plausible : de quoi vérifier que l'audio n'est pas touché.
    /// Le contenu importe peu, seule son intégrité compte.
    const FAKE_AUDIO: &[u8] = &[
        0xFF, 0xFB, 0x90, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
    ];

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rustmusic-mp3-{tag}"));
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

    fn read_title(path: &Path) -> Option<String> {
        let data = fs::read(path).unwrap();
        let len = id3v2_writer::tag_len(&data);
        let tag = id3v2::parse(&data[..len]).unwrap();
        id3v2::to_audio_tags(&tag).title
    }

    #[test]
    fn adds_a_tag_to_a_file_that_had_none() {
        let dir = temp_dir("untagged");
        let file = dir.join("piste.mp3");
        fs::write(&file, FAKE_AUDIO).unwrap();

        apply(&file, &set_title("Starman")).unwrap();

        assert_eq!(read_title(&file).as_deref(), Some("Starman"));
        // L'audio doit se retrouver intact juste après le tag.
        let data = fs::read(&file).unwrap();
        let len = id3v2_writer::tag_len(&data);
        assert_eq!(&data[len..], FAKE_AUDIO);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replaces_an_existing_tag_without_stacking() {
        let dir = temp_dir("existing");
        let file = dir.join("piste.mp3");
        fs::write(&file, FAKE_AUDIO).unwrap();

        apply(&file, &set_title("Version 1")).unwrap();
        let after_first = fs::read(&file).unwrap().len();
        apply(&file, &set_title("Version 2")).unwrap();
        let after_second = fs::read(&file).unwrap().len();

        assert_eq!(read_title(&file).as_deref(), Some("Version 2"));
        // Un tag empilé ferait grossir le fichier à chaque édition.
        assert_eq!(after_first, after_second, "le tag s'est empilé");

        let data = fs::read(&file).unwrap();
        let len = id3v2_writer::tag_len(&data);
        assert_eq!(&data[len..], FAKE_AUDIO, "l'audio a bougé");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn keeps_a_trailing_id3v1_block_in_sync() {
        // Régression : un ID3v1 laissé tel quel continue d'être remonté par
        // certains lecteurs et masque la modification — l'édition semble alors
        // « ne pas s'enregistrer ».
        let dir = temp_dir("id3v1");
        let file = dir.join("piste.mp3");
        let mut content = FAKE_AUDIO.to_vec();
        let mut v1 = vec![0u8; 128];
        v1[0..3].copy_from_slice(b"TAG");
        v1[3..8].copy_from_slice(b"Vieux"); // ancien titre
        v1[97..104].copy_from_slice(b"HQRap.R"); // ancien commentaire
        content.extend_from_slice(&v1);
        fs::write(&file, &content).unwrap();

        apply(
            &file,
            &TagEdit {
                title: FieldEdit::Set("Ashes to Ashes".into()),
                comment: FieldEdit::Set("test".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let data = fs::read(&file).unwrap();
        let v1 = &data[data.len() - 128..];
        assert_eq!(&v1[..3], b"TAG", "le bloc ID3v1 a disparu");

        let title: String = v1[3..33].iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();
        let comment: String = v1[97..127].iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();
        assert_eq!(title, "Ashes to Ashes");
        assert_eq!(comment, "test", "l'ancien commentaire ID3v1 a survécu");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_edit_is_written_in_place() {
        // Même garantie qu'en FLAC : la seconde écriture tient dans le
        // remplissage posé par la première, donc l'audio ne bouge pas.
        let dir = temp_dir("in-place");
        let file = dir.join("piste.mp3");
        fs::write(&file, FAKE_AUDIO).unwrap();

        apply(&file, &set_title("Premier")).unwrap();
        let after_first = fs::metadata(&file).unwrap().len();

        apply(&file, &set_title("Second titre nettement plus long")).unwrap();

        assert_eq!(
            fs::metadata(&file).unwrap().len(),
            after_first,
            "la taille a changé : l'audio a été décalé"
        );
        let data = fs::read(&file).unwrap();
        let len = id3v2_writer::tag_len(&data);
        assert_eq!(&data[len..], FAKE_AUDIO, "l'audio a bougé");
        assert_eq!(read_title(&file).as_deref(), Some("Second titre nettement plus long"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_edit_leaves_the_file_byte_for_byte_identical() {
        let dir = temp_dir("noop");
        let file = dir.join("piste.mp3");
        fs::write(&file, FAKE_AUDIO).unwrap();

        // Passe par le routeur, seul endroit qui court-circuite les éditions
        // vides — le conteneur, lui, réécrirait pour rien.
        crate::core::audio_metadata::injector::injector::apply(&file, &TagEdit::default())
            .unwrap();

        assert_eq!(fs::read(&file).unwrap(), FAKE_AUDIO);
        let _ = fs::remove_dir_all(&dir);
    }
}
