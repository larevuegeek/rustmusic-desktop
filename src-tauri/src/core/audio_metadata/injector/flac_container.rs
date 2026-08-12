//! Écriture des métadonnées d'un fichier FLAC.
//!
//! # Un format de tags entièrement différent
//! Les trois autres conteneurs se contentent de **placer** un blob ID3v2. Le
//! FLAC, lui, a son propre système : une suite de blocs de métadonnées, dont un
//! `VORBIS_COMMENT` porteur des champs texte et autant de blocs `PICTURE` qu'il
//! y a d'images. Il n'y a donc pas de blob à recopier : tout est encodé ici.
//!
//! ```text
//! "fLaC"
//! bloc*  : 1 octet (bit 7 = dernier bloc, bits 0-6 = type)
//!          3 octets de longueur (gros-boutiste)
//!          données
//!   type 0 STREAMINFO     — obligatoire, toujours en premier
//!   type 1 PADDING        — remplissage
//!   type 3 SEEKTABLE      — points de recherche
//!   type 4 VORBIS_COMMENT — les champs texte
//!   type 6 PICTURE        — une image
//! trames audio
//! ```
//!
//! # Le piège des boutismes
//! L'en-tête des blocs et le contenu de `PICTURE` sont **gros-boutistes**,
//! comme tout le reste du FLAC. Mais `VORBIS_COMMENT` vient d'Ogg Vorbis et
//! reste **petit-boutiste**. Les deux cohabitent dans le même fichier, à
//! quelques octets d'écart.
//!
//! Second piège du même bloc : en Ogg il se termine par un « bit de cadrage ».
//! En FLAC, **non**. L'ajouter décale d'un octet tout ce qui suit.
//!
//! # Les champs
//! Contrairement à l'ID3 où « 3/12 » tient dans une seule frame, Vorbis stocke
//! numéro et total dans deux champs distincts. Certains encodeurs écrivent
//! pourtant « 3/12 » dans `TRACKNUMBER` : on démêle cette forme avant d'éditer,
//! sinon corriger le total laisserait un `TRACKNUMBER` qui le contredit.

use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::core::audio_metadata::injector::atomic_write;
use crate::core::audio_metadata::injector::edit::{
    FieldEdit, ImagePlan, ImageSlot, ImageSource, TagEdit,
};
use crate::core::audio_metadata::injector::image_prep::image_id;
use crate::core::audio_metadata::injector::injector::InjectError;

const BLOCK_STREAMINFO: u8 = 0;
const BLOCK_PADDING: u8 = 1;
const BLOCK_VORBIS_COMMENT: u8 = 4;
const BLOCK_PICTURE: u8 = 6;

/// La longueur d'un bloc tient sur 24 bits — au-delà, il est inécrivable.
const MAX_BLOCK_LEN: usize = 0xFF_FFFF;
/// Remplissage laissé pour qu'un autre éditeur puisse retoucher les tags sans
/// réécrire le fichier entier.
const PADDING_LEN: usize = 1024;
/// Type d'image « pochette avant », même code qu'en ID3.
const PICTURE_TYPE_COVER_FRONT: u32 = 3;
const PICTURE_TYPE_OTHER: u32 = 0;

struct Block {
    kind: u8,
    data: Vec<u8>,
}

/// Applique `edit` au fichier FLAC `path`.
pub fn apply(path: &Path, edit: &TagEdit) -> Result<(), InjectError> {
    let mut file = fs::File::open(path)?;
    let file_len = fs::metadata(path)?.len();

    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).map_err(|_| {
        InjectError::Malformed("fichier trop court pour un en-tête FLAC".into())
    })?;
    if &magic != b"fLaC" {
        return Err(InjectError::Malformed(
            "signature FLAC absente (« fLaC » attendu)".into(),
        ));
    }

    let blocks = read_blocks(&mut file, file_len)?;
    if blocks.first().map(|b| b.kind) != Some(BLOCK_STREAMINFO) {
        return Err(InjectError::Malformed(
            "le premier bloc FLAC n'est pas un STREAMINFO".into(),
        ));
    }
    let audio_start = file.stream_position()?;

    // ─── Champs texte ───
    let mut comment = blocks
        .iter()
        .find(|b| b.kind == BLOCK_VORBIS_COMMENT)
        .and_then(|b| decode_comment(&b.data))
        .unwrap_or_else(new_comment);
    apply_fields(&mut comment.fields, edit);

    // ─── Images ───
    let existing_pictures: Vec<Picture> = blocks
        .iter()
        .filter(|b| b.kind == BLOCK_PICTURE)
        .filter_map(|b| decode_picture(&b.data))
        .collect();
    let pictures = apply_images(&existing_pictures, &edit.images)?;

    // ─── Reconstruction ───
    // STREAMINFO d'abord (la spécification l'impose), puis les blocs qu'on ne
    // touche pas, puis les nôtres. Le PADDING d'origine est écarté : on remet
    // le nôtre en dernier.
    let mut out = Vec::new();
    out.extend_from_slice(b"fLaC");

    let mut rebuilt: Vec<(u8, Vec<u8>)> = Vec::new();
    for block in &blocks {
        match block.kind {
            BLOCK_PADDING | BLOCK_VORBIS_COMMENT | BLOCK_PICTURE => {}
            kind => rebuilt.push((kind, block.data.clone())),
        }
    }
    rebuilt.push((BLOCK_VORBIS_COMMENT, encode_comment(&comment)));
    for picture in &pictures {
        rebuilt.push((BLOCK_PICTURE, encode_picture(picture)));
    }
    rebuilt.push((BLOCK_PADDING, vec![0u8; PADDING_LEN]));

    let last = rebuilt.len() - 1;
    for (index, (kind, data)) in rebuilt.iter().enumerate() {
        write_block(&mut out, *kind, data, index == last)?;
    }

    let metadata_len = out.len();
    atomic_write::replace_file_with(path, move |dest| {
        dest.write_all(&out)?;
        // Les trames audio sont recopiées par blocs : un FLAC 24/192 d'un
        // morceau long dépasse facilement les 200 Mo.
        file.seek(SeekFrom::Start(audio_start))?;
        io::copy(&mut file, dest)?;
        Ok(())
    })?;

    log::info!(
        "🏷  Tags FLAC réécrits : {} ({} champs, {} image(s), métadonnées {} octets)",
        path.display(),
        comment.fields.len(),
        pictures.len(),
        metadata_len
    );
    Ok(())
}

// ─── Blocs de métadonnées ────────────────────────────────────────────────

fn read_blocks(file: &mut fs::File, file_len: u64) -> Result<Vec<Block>, InjectError> {
    let mut blocks = Vec::new();
    loop {
        let mut header = [0u8; 4];
        file.read_exact(&mut header)
            .map_err(|_| InjectError::Malformed("bloc FLAC tronqué".into()))?;

        let is_last = header[0] & 0x80 != 0;
        let kind = header[0] & 0x7F;
        let len = u32::from_be_bytes([0, header[1], header[2], header[3]]) as u64;

        // Une longueur qui dépasse le fichier signale un fichier abîmé. Sans ce
        // garde-fou on tenterait d'allouer jusqu'à 16 Mo par bloc sur du bruit.
        let position = file.stream_position()?;
        if position + len > file_len {
            return Err(InjectError::Malformed(format!(
                "bloc FLAC de type {kind} annonce {len} octets, au-delà de la fin du fichier"
            )));
        }

        let mut data = vec![0u8; len as usize];
        file.read_exact(&mut data)
            .map_err(|_| InjectError::Malformed("bloc FLAC tronqué".into()))?;

        blocks.push(Block { kind, data });
        if is_last {
            return Ok(blocks);
        }
    }
}

fn write_block(out: &mut Vec<u8>, kind: u8, data: &[u8], is_last: bool) -> Result<(), InjectError> {
    if data.len() > MAX_BLOCK_LEN {
        return Err(InjectError::InvalidRequest(format!(
            "bloc FLAC de {} octets : la longueur ne tient pas sur 24 bits",
            data.len()
        )));
    }
    out.push(if is_last { kind | 0x80 } else { kind });
    out.push((data.len() >> 16) as u8);
    out.push((data.len() >> 8) as u8);
    out.push(data.len() as u8);
    out.extend_from_slice(data);
    Ok(())
}

// ─── VORBIS_COMMENT ──────────────────────────────────────────────────────

struct Comment {
    vendor: String,
    /// `(clé, valeur)` dans l'ordre du fichier. La clé garde sa casse d'origine ;
    /// les comparaisons se font sans casse, comme le veut la spécification.
    fields: Vec<(String, String)>,
}

fn new_comment() -> Comment {
    Comment {
        vendor: format!("RustMusic {}", env!("CARGO_PKG_VERSION")),
        fields: Vec::new(),
    }
}

/// Décode un bloc `VORBIS_COMMENT`. Entiers **petit-boutistes**, et pas de bit
/// de cadrage final en FLAC.
fn decode_comment(data: &[u8]) -> Option<Comment> {
    let mut pos = 0usize;

    let mut take_u32 = |pos: &mut usize| -> Option<u32> {
        if *pos + 4 > data.len() {
            return None;
        }
        let v = u32::from_le_bytes(data[*pos..*pos + 4].try_into().ok()?);
        *pos += 4;
        Some(v)
    };

    let vendor_len = take_u32(&mut pos)? as usize;
    if pos + vendor_len > data.len() {
        return None;
    }
    let vendor = String::from_utf8_lossy(&data[pos..pos + vendor_len]).into_owned();
    pos += vendor_len;

    let count = take_u32(&mut pos)? as usize;
    let mut fields = Vec::with_capacity(count.min(1024));

    for _ in 0..count {
        let len = take_u32(&mut pos)? as usize;
        if pos + len > data.len() {
            return None;
        }
        let entry = String::from_utf8_lossy(&data[pos..pos + len]).into_owned();
        pos += len;

        // Une entrée sans « = » est hors spécification : on l'ignore plutôt
        // que d'inventer une clé.
        if let Some((key, value)) = entry.split_once('=') {
            fields.push((key.to_string(), value.to_string()));
        }
    }

    Some(Comment { vendor, fields })
}

fn encode_comment(comment: &Comment) -> Vec<u8> {
    let mut out = Vec::new();
    let vendor = comment.vendor.as_bytes();
    out.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    out.extend_from_slice(vendor);

    out.extend_from_slice(&(comment.fields.len() as u32).to_le_bytes());
    for (key, value) in &comment.fields {
        let entry = format!("{key}={value}");
        out.extend_from_slice(&(entry.len() as u32).to_le_bytes());
        out.extend_from_slice(entry.as_bytes());
    }
    // Pas de bit de cadrage : il n'existe qu'en Ogg.
    out
}

/// Clé principale et synonymes à purger, pour chaque champ éditable.
///
/// Purger les synonymes n'est pas du zèle : un `TOTALTRACKS` oublié à côté d'un
/// `TRACKTOTAL` fraîchement écrit fait que la valeur affichée dépend de l'ordre
/// de lecture — donc du lecteur.
const FIELD_KEYS: &[(&str, &[&str])] = &[
    ("TITLE", &[]),
    ("ARTIST", &[]),
    ("ALBUM", &[]),
    ("ALBUMARTIST", &["ALBUM ARTIST"]),
    ("DATE", &["YEAR"]),
    ("GENRE", &[]),
    ("COMMENT", &[]),
    ("COMPOSER", &[]),
    ("TRACKNUMBER", &["TRACK"]),
    ("TRACKTOTAL", &["TOTALTRACKS"]),
    ("DISCNUMBER", &["DISKNUMBER", "DISC", "DISK"]),
    ("DISCTOTAL", &["DISKTOTAL", "TOTALDISCS"]),
];

fn keys_for(index: usize) -> (&'static str, &'static [&'static str]) {
    FIELD_KEYS[index]
}

fn find(fields: &[(String, String)], key: &str) -> Option<String> {
    fields
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.clone())
}

fn remove(fields: &mut Vec<(String, String)>, index: usize) {
    let (primary, aliases) = keys_for(index);
    fields.retain(|(k, _)| {
        !k.eq_ignore_ascii_case(primary) && !aliases.iter().any(|a| k.eq_ignore_ascii_case(a))
    });
}

fn set(fields: &mut Vec<(String, String)>, index: usize, edit: &FieldEdit<String>) {
    match edit {
        FieldEdit::Keep => {}
        FieldEdit::Clear => remove(fields, index),
        FieldEdit::Set(value) => {
            remove(fields, index);
            fields.push((keys_for(index).0.to_string(), value.clone()));
        }
    }
}

fn set_num(fields: &mut Vec<(String, String)>, index: usize, edit: &FieldEdit<u16>) {
    match edit {
        FieldEdit::Keep => {}
        FieldEdit::Clear => remove(fields, index),
        FieldEdit::Set(value) => {
            remove(fields, index);
            fields.push((keys_for(index).0.to_string(), value.to_string()));
        }
    }
}

// Positions dans FIELD_KEYS, nommées pour que les appels restent lisibles.
const F_TITLE: usize = 0;
const F_ARTIST: usize = 1;
const F_ALBUM: usize = 2;
const F_ALBUM_ARTIST: usize = 3;
const F_DATE: usize = 4;
const F_GENRE: usize = 5;
const F_COMMENT: usize = 6;
const F_COMPOSER: usize = 7;
const F_TRACK_NUM: usize = 8;
const F_TRACK_TOTAL: usize = 9;
const F_DISC_NUM: usize = 10;
const F_DISC_TOTAL: usize = 11;

fn apply_fields(fields: &mut Vec<(String, String)>, edit: &TagEdit) {
    // Certains encodeurs écrivent « 3/12 » dans TRACKNUMBER, à la mode ID3.
    // On démêle avant d'éditer, sinon corriger le total laisserait un numéro
    // qui le contredit dans le même fichier.
    if edit.track_number.is_change() || edit.total_tracks.is_change() {
        split_pair(fields, F_TRACK_NUM, F_TRACK_TOTAL);
    }
    if edit.disc_number.is_change() || edit.total_discs.is_change() {
        split_pair(fields, F_DISC_NUM, F_DISC_TOTAL);
    }

    set(fields, F_TITLE, &edit.title);
    set(fields, F_ARTIST, &edit.artist);
    set(fields, F_ALBUM, &edit.album);
    set(fields, F_ALBUM_ARTIST, &edit.album_artist);
    set(fields, F_DATE, &edit.year);
    set(fields, F_GENRE, &edit.genre);
    set(fields, F_COMMENT, &edit.comment);
    set(fields, F_COMPOSER, &edit.composer);
    set_num(fields, F_TRACK_NUM, &edit.track_number);
    set_num(fields, F_TRACK_TOTAL, &edit.total_tracks);
    set_num(fields, F_DISC_NUM, &edit.disc_number);
    set_num(fields, F_DISC_TOTAL, &edit.total_discs);
}

/// Transforme un `NUMBER` valant « n/total » en deux champs distincts.
fn split_pair(fields: &mut Vec<(String, String)>, num: usize, total: usize) {
    let Some(value) = find(fields, keys_for(num).0) else {
        return;
    };
    let Some((left, right)) = value.split_once('/') else {
        return;
    };
    let (left, right) = (left.trim().to_string(), right.trim().to_string());

    remove(fields, num);
    fields.push((keys_for(num).0.to_string(), left));

    // Un total déjà présent ailleurs fait foi : il a été écrit explicitement.
    if find(fields, keys_for(total).0).is_none() && !right.is_empty() {
        fields.push((keys_for(total).0.to_string(), right));
    }
}

// ─── PICTURE ─────────────────────────────────────────────────────────────

/// Un bloc `PICTURE`. Tous les entiers y sont **gros-boutistes**, contrairement
/// au bloc de commentaires voisin.
#[derive(Debug, Clone)]
struct Picture {
    picture_type: u32,
    mime: String,
    description: String,
    width: u32,
    height: u32,
    /// Profondeur en bits par pixel. Informative : les lecteurs décodent
    /// l'image elle-même.
    depth: u32,
    /// Nombre de couleurs pour une image indexée, 0 sinon.
    colors: u32,
    data: Vec<u8>,
}

fn decode_picture(data: &[u8]) -> Option<Picture> {
    let mut pos = 0usize;

    let take_u32 = |data: &[u8], pos: &mut usize| -> Option<u32> {
        if *pos + 4 > data.len() {
            return None;
        }
        let v = u32::from_be_bytes(data[*pos..*pos + 4].try_into().ok()?);
        *pos += 4;
        Some(v)
    };
    let take_str = |data: &[u8], pos: &mut usize| -> Option<String> {
        let len = take_u32(data, pos)? as usize;
        if *pos + len > data.len() {
            return None;
        }
        let s = String::from_utf8_lossy(&data[*pos..*pos + len]).into_owned();
        *pos += len;
        Some(s)
    };

    let picture_type = take_u32(data, &mut pos)?;
    let mime = take_str(data, &mut pos)?;
    let description = take_str(data, &mut pos)?;
    let width = take_u32(data, &mut pos)?;
    let height = take_u32(data, &mut pos)?;
    let depth = take_u32(data, &mut pos)?;
    let colors = take_u32(data, &mut pos)?;
    let len = take_u32(data, &mut pos)? as usize;
    if pos + len > data.len() {
        return None;
    }

    Some(Picture {
        picture_type,
        mime,
        description,
        width,
        height,
        depth,
        colors,
        data: data[pos..pos + len].to_vec(),
    })
}

fn encode_picture(picture: &Picture) -> Vec<u8> {
    let mut out = Vec::with_capacity(64 + picture.data.len());
    out.extend_from_slice(&picture.picture_type.to_be_bytes());

    let mime = picture.mime.as_bytes();
    out.extend_from_slice(&(mime.len() as u32).to_be_bytes());
    out.extend_from_slice(mime);

    let description = picture.description.as_bytes();
    out.extend_from_slice(&(description.len() as u32).to_be_bytes());
    out.extend_from_slice(description);

    out.extend_from_slice(&picture.width.to_be_bytes());
    out.extend_from_slice(&picture.height.to_be_bytes());
    out.extend_from_slice(&picture.depth.to_be_bytes());
    out.extend_from_slice(&picture.colors.to_be_bytes());

    out.extend_from_slice(&(picture.data.len() as u32).to_be_bytes());
    out.extend_from_slice(&picture.data);
    out
}

/// Construit la liste finale des images, mêmes règles qu'en ID3.
fn apply_images(existing: &[Picture], plan: &ImagePlan) -> Result<Vec<Picture>, InjectError> {
    let ImagePlan::Replace(slots) = plan else {
        return Ok(existing.to_vec());
    };

    let mut rebuilt = Vec::with_capacity(slots.len());
    let mut cover_taken = false;

    for slot in slots {
        let mut picture = resolve_slot(existing, slot)?;
        if picture.picture_type == PICTURE_TYPE_COVER_FRONT {
            if cover_taken {
                picture.picture_type = PICTURE_TYPE_OTHER;
            } else {
                cover_taken = true;
            }
        }
        rebuilt.push(picture);
    }

    Ok(rebuilt)
}

fn resolve_slot(existing: &[Picture], slot: &ImageSlot) -> Result<Picture, InjectError> {
    match &slot.source {
        ImageSource::Existing(id) => {
            let found = existing
                .iter()
                .find(|p| image_id(&p.data) == *id)
                .ok_or_else(|| {
                    InjectError::InvalidRequest(format!(
                        "image introuvable dans le fichier : {id}"
                    ))
                })?;
            Ok(Picture {
                picture_type: slot.picture_type as u32,
                description: slot.description.clone(),
                ..found.clone()
            })
        }
        ImageSource::New {
            data,
            mime_type,
            width,
            height,
        } => Ok(Picture {
            picture_type: slot.picture_type as u32,
            mime: mime_type.clone(),
            description: slot.description.clone(),
            width: *width,
            height: *height,
            // Profondeur conventionnelle pour une image en couleurs vraies, et
            // zéro couleur indexée. Ces deux champs sont informatifs : aucun
            // lecteur ne s'en sert pour décoder.
            depth: 24,
            colors: 0,
            data: data.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAKE_AUDIO: &[u8] = b"TRAMES-AUDIO-FLAC";

    /// Fabrique un FLAC minimal mais structurellement valide.
    fn build_flac(extra: &[(u8, Vec<u8>)]) -> Vec<u8> {
        let mut blocks: Vec<(u8, Vec<u8>)> = vec![(BLOCK_STREAMINFO, vec![0u8; 34])];
        blocks.extend_from_slice(extra);

        let mut out = Vec::new();
        out.extend_from_slice(b"fLaC");
        let last = blocks.len() - 1;
        for (i, (kind, data)) in blocks.iter().enumerate() {
            write_block(&mut out, *kind, data, i == last).unwrap();
        }
        out.extend_from_slice(FAKE_AUDIO);
        out
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rustmusic-flac-{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Relit le fichier écrit et renvoie ses blocs, en vérifiant au passage la
    /// cohérence du drapeau « dernier bloc ».
    fn reread(path: &Path) -> (Vec<Block>, Vec<u8>) {
        let data = fs::read(path).unwrap();
        assert_eq!(&data[0..4], b"fLaC");

        let mut blocks = Vec::new();
        let mut pos = 4usize;
        loop {
            let is_last = data[pos] & 0x80 != 0;
            let kind = data[pos] & 0x7F;
            let len =
                u32::from_be_bytes([0, data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
            pos += 4;
            blocks.push(Block {
                kind,
                data: data[pos..pos + len].to_vec(),
            });
            pos += len;
            if is_last {
                break;
            }
        }
        (blocks, data[pos..].to_vec())
    }

    fn fields_of(path: &Path) -> Vec<(String, String)> {
        let (blocks, _) = reread(path);
        blocks
            .iter()
            .find(|b| b.kind == BLOCK_VORBIS_COMMENT)
            .and_then(|b| decode_comment(&b.data))
            .map(|c| c.fields)
            .unwrap_or_default()
    }

    fn value(path: &Path, key: &str) -> Option<String> {
        find(&fields_of(path), key)
    }

    fn set_title(v: &str) -> TagEdit {
        TagEdit {
            title: FieldEdit::Set(v.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn writes_fields_into_a_file_that_had_none() {
        let dir = temp_dir("empty");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();

        apply(
            &file,
            &TagEdit {
                title: FieldEdit::Set("Blue in Green".into()),
                artist: FieldEdit::Set("Miles Davis".into()),
                year: FieldEdit::Set("1959".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(value(&file, "TITLE").as_deref(), Some("Blue in Green"));
        assert_eq!(value(&file, "ARTIST").as_deref(), Some("Miles Davis"));
        assert_eq!(value(&file, "DATE").as_deref(), Some("1959"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_audio_frames_come_out_untouched() {
        let dir = temp_dir("audio");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();

        apply(&file, &set_title("Peu importe")).unwrap();

        let (_, audio) = reread(&file);
        assert_eq!(audio, FAKE_AUDIO, "les trames audio ont bougé");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn keeps_the_blocks_it_does_not_understand() {
        // Même garantie qu'en ID3 : une table de recherche ou une feuille de
        // repères ne doit pas disparaître parce qu'on corrige un titre.
        let dir = temp_dir("unknown");
        let file = dir.join("piste.flac");
        fs::write(
            &file,
            build_flac(&[(3, b"TABLE-DE-RECHERCHE".to_vec()), (5, b"CUESHEET".to_vec())]),
        )
        .unwrap();

        apply(&file, &set_title("x")).unwrap();

        let (blocks, _) = reread(&file);
        let seek = blocks.iter().find(|b| b.kind == 3).unwrap();
        assert_eq!(seek.data, b"TABLE-DE-RECHERCHE");
        assert!(blocks.iter().any(|b| b.kind == 5), "la CUESHEET a disparu");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn streaminfo_stays_first_and_only_the_last_block_is_flagged() {
        let dir = temp_dir("layout");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();

        apply(&file, &set_title("x")).unwrap();

        let data = fs::read(&file).unwrap();
        assert_eq!(data[4] & 0x7F, BLOCK_STREAMINFO, "STREAMINFO n'est plus en tête");
        assert_eq!(data[4] & 0x80, 0, "STREAMINFO est marqué comme dernier bloc");
        // `reread` échouerait si le drapeau était mal placé : il suit la chaîne.
        let (blocks, _) = reread(&file);
        assert_eq!(blocks.last().unwrap().kind, BLOCK_PADDING);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn does_not_stack_comment_blocks() {
        let dir = temp_dir("stack");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();

        apply(&file, &set_title("Version 1")).unwrap();
        let first = fs::metadata(&file).unwrap().len();
        apply(&file, &set_title("Version 2")).unwrap();

        let (blocks, _) = reread(&file);
        let count = blocks.iter().filter(|b| b.kind == BLOCK_VORBIS_COMMENT).count();
        assert_eq!(count, 1, "deux blocs de commentaires cohabitent");
        assert_eq!(fs::metadata(&file).unwrap().len(), first);
        assert_eq!(value(&file, "TITLE").as_deref(), Some("Version 2"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn clearing_removes_the_field() {
        let dir = temp_dir("clear");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();

        apply(&file, &set_title("À effacer")).unwrap();
        apply(
            &file,
            &TagEdit {
                title: FieldEdit::Clear,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(value(&file, "TITLE"), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn keeps_fields_it_was_not_asked_to_change() {
        let dir = temp_dir("keep");
        let file = dir.join("piste.flac");
        let comment = encode_comment(&Comment {
            vendor: "un autre encodeur".into(),
            fields: vec![
                ("ARTIST".into(), "Bill Evans".into()),
                ("MUSICBRAINZ_TRACKID".into(), "abc-123".into()),
            ],
        });
        fs::write(&file, build_flac(&[(BLOCK_VORBIS_COMMENT, comment)])).unwrap();

        apply(&file, &set_title("Waltz for Debby")).unwrap();

        assert_eq!(value(&file, "ARTIST").as_deref(), Some("Bill Evans"));
        assert_eq!(
            value(&file, "MUSICBRAINZ_TRACKID").as_deref(),
            Some("abc-123"),
            "un champ inconnu a été perdu"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn purges_synonyms_of_the_field_it_writes() {
        // Un TOTALTRACKS oublié à côté d'un TRACKTOTAL neuf ferait dépendre la
        // valeur affichée de l'ordre de lecture, donc du lecteur.
        let dir = temp_dir("synonyms");
        let file = dir.join("piste.flac");
        let comment = encode_comment(&Comment {
            vendor: "x".into(),
            fields: vec![("TOTALTRACKS".into(), "99".into())],
        });
        fs::write(&file, build_flac(&[(BLOCK_VORBIS_COMMENT, comment)])).unwrap();

        apply(
            &file,
            &TagEdit {
                total_tracks: FieldEdit::Set(12),
                ..Default::default()
            },
        )
        .unwrap();

        let fields = fields_of(&file);
        assert_eq!(find(&fields, "TRACKTOTAL").as_deref(), Some("12"));
        assert_eq!(find(&fields, "TOTALTRACKS"), None, "le synonyme a survécu");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn untangles_a_track_number_written_as_a_pair() {
        // « 3/12 » est une habitude venue de l'ID3. Corriger le total sans
        // démêler laisserait un numéro qui le contredit dans le même fichier.
        let dir = temp_dir("pair");
        let file = dir.join("piste.flac");
        let comment = encode_comment(&Comment {
            vendor: "x".into(),
            fields: vec![("TRACKNUMBER".into(), "3/12".into())],
        });
        fs::write(&file, build_flac(&[(BLOCK_VORBIS_COMMENT, comment)])).unwrap();

        apply(
            &file,
            &TagEdit {
                total_tracks: FieldEdit::Set(14),
                ..Default::default()
            },
        )
        .unwrap();

        let fields = fields_of(&file);
        assert_eq!(find(&fields, "TRACKNUMBER").as_deref(), Some("3"));
        assert_eq!(find(&fields, "TRACKTOTAL").as_deref(), Some("14"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn round_trips_utf8_values() {
        let dir = temp_dir("utf8");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();

        apply(&file, &set_title("東京は夜の七時")).unwrap();

        assert_eq!(value(&file, "TITLE").as_deref(), Some("東京は夜の七時"));
        let _ = fs::remove_dir_all(&dir);
    }

    // ─── Images ───

    fn added(bytes: &[u8], picture_type: u8) -> ImageSlot {
        ImageSlot {
            source: ImageSource::New {
                data: bytes.to_vec(),
                mime_type: "image/jpeg".into(),
                width: 1200,
                height: 1200,
            },
            picture_type,
            description: String::new(),
        }
    }

    fn kept(bytes: &[u8], picture_type: u8) -> ImageSlot {
        ImageSlot {
            source: ImageSource::Existing(image_id(bytes)),
            picture_type,
            description: String::new(),
        }
    }

    fn with_images(slots: Vec<ImageSlot>) -> TagEdit {
        TagEdit {
            images: ImagePlan::Replace(slots),
            ..Default::default()
        }
    }

    fn pictures_of(path: &Path) -> Vec<Picture> {
        let (blocks, _) = reread(path);
        blocks
            .iter()
            .filter(|b| b.kind == BLOCK_PICTURE)
            .filter_map(|b| decode_picture(&b.data))
            .collect()
    }

    #[test]
    fn embeds_a_new_cover() {
        let dir = temp_dir("cover");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();

        apply(&file, &with_images(vec![added(b"OCTETS-POCHETTE", 3)])).unwrap();

        let pictures = pictures_of(&file);
        assert_eq!(pictures.len(), 1);
        assert_eq!(pictures[0].data, b"OCTETS-POCHETTE");
        assert_eq!(pictures[0].picture_type, 3);
        assert_eq!(pictures[0].mime, "image/jpeg");
        assert_eq!((pictures[0].width, pictures[0].height), (1200, 1200));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn promoting_an_image_does_not_re_encode_it() {
        let dir = temp_dir("promote");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();

        apply(
            &file,
            &with_images(vec![added(b"AAA-avant", 3), added(b"BBB-arriere", 4)]),
        )
        .unwrap();
        apply(&file, &with_images(vec![kept(b"BBB-arriere", 3)])).unwrap();

        let pictures = pictures_of(&file);
        assert_eq!(pictures.len(), 1);
        assert_eq!(pictures[0].data, b"BBB-arriere", "les octets ont changé");
        assert_eq!(pictures[0].picture_type, 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_order_of_the_slots_is_the_order_in_the_file() {
        let dir = temp_dir("order");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();

        apply(
            &file,
            &with_images(vec![added(b"un", 3), added(b"deux", 0), added(b"trois", 0)]),
        )
        .unwrap();
        apply(
            &file,
            &with_images(vec![kept(b"trois", 3), kept(b"un", 0), kept(b"deux", 0)]),
        )
        .unwrap();

        let order: Vec<_> = pictures_of(&file).into_iter().map(|p| p.data).collect();
        assert_eq!(
            order,
            vec![b"trois".to_vec(), b"un".to_vec(), b"deux".to_vec()]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_the_first_front_cover_keeps_its_type() {
        let dir = temp_dir("single-cover");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();

        apply(&file, &with_images(vec![added(b"a", 3), added(b"b", 3)])).unwrap();

        let pictures = pictures_of(&file);
        assert_eq!(pictures.len(), 2, "une image a disparu");
        assert_eq!(
            pictures.iter().filter(|p| p.picture_type == 3).count(),
            1,
            "deux pochettes avant coexistent"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_list_removes_every_picture() {
        let dir = temp_dir("no-picture");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();

        apply(&file, &with_images(vec![added(b"a", 3)])).unwrap();
        apply(&file, &with_images(Vec::new())).unwrap();

        assert!(pictures_of(&file).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unknown_image_reference_is_refused() {
        let dir = temp_dir("unknown-image");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();
        apply(&file, &with_images(vec![added(b"a", 3)])).unwrap();
        let before = fs::read(&file).unwrap();

        let result = apply(
            &file,
            &with_images(vec![ImageSlot {
                source: ImageSource::Existing("0123456789abcdef".into()),
                picture_type: 3,
                description: String::new(),
            }]),
        );

        assert!(result.is_err(), "référence inconnue acceptée en silence");
        assert_eq!(fs::read(&file).unwrap(), before, "le fichier a été touché");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn editing_text_leaves_the_pictures_untouched() {
        let dir = temp_dir("text-only");
        let file = dir.join("piste.flac");
        fs::write(&file, build_flac(&[])).unwrap();

        apply(&file, &with_images(vec![added(b"pochette", 3)])).unwrap();
        apply(&file, &set_title("Nouveau titre")).unwrap();

        let pictures = pictures_of(&file);
        assert_eq!(pictures.len(), 1);
        assert_eq!(pictures[0].data, b"pochette");
        assert_eq!(value(&file, "TITLE").as_deref(), Some("Nouveau titre"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_a_file_whose_first_block_is_not_streaminfo() {
        let dir = temp_dir("no-streaminfo");
        let file = dir.join("piste.flac");
        let mut data = Vec::new();
        data.extend_from_slice(b"fLaC");
        write_block(&mut data, BLOCK_PADDING, &[0u8; 8], true).unwrap();
        data.extend_from_slice(FAKE_AUDIO);
        fs::write(&file, &data).unwrap();

        assert!(apply(&file, &set_title("x")).is_err());
        assert_eq!(fs::read(&file).unwrap(), data, "le fichier a été touché");
        let _ = fs::remove_dir_all(&dir);
    }
}
