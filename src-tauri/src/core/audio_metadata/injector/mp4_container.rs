//! Tags iTunes (`moov/udta/meta/ilst`) d'un fichier MP4 : M4A (AAC, ALAC), M4B.
//!
//! # Le piège du conteneur
//! Le `moov` décrit aussi l'audio : ses tables `stco`/`co64` donnent la position
//! **absolue** de chaque bloc audio. Si le `moov` grossit et précède l'audio,
//! tout l'audio recule et ces positions doivent suivre, sinon le fichier ne se lit plus.
//!
//! # Remplissage
//! Comme iTunes, un atome `free` reste dans `meta`, après `ilst` : l'édition
//! suivante qui y tient réécrit le seul `moov`, sur place, sans toucher à l'audio.

use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::core::audio_metadata::injector::atomic_write;
use crate::core::audio_metadata::injector::edit::{
    ExistingImage, FieldEdit, ImageSlot, ImageSource, TagEdit, PICTURE_TYPE_COVER_FRONT,
};
use crate::core::audio_metadata::injector::image_prep::image_id;
use crate::core::audio_metadata::injector::injector::InjectError;

/// Remplissage laissé après une réécriture complète, comme le PADDING du FLAC.
const PADDING_LEN: usize = 1024;
/// Au-delà, ce n'est plus un `moov` de fichier audio mais un fichier abîmé.
const MAX_MOOV_LEN: u64 = 256 * 1024 * 1024;

// Clés iTunes ; « © » s'écrit 0xA9.
const NAM: [u8; 4] = [0xA9, b'n', b'a', b'm'];
const ART: [u8; 4] = [0xA9, b'A', b'R', b'T'];
const ALB: [u8; 4] = [0xA9, b'a', b'l', b'b'];
const AART: [u8; 4] = *b"aART";
const DAY: [u8; 4] = [0xA9, b'd', b'a', b'y'];
const GEN: [u8; 4] = [0xA9, b'g', b'e', b'n'];
/// Genre ID3v1 numéroté : remplacé par `©gen` dès qu'on touche au genre.
const GNRE: [u8; 4] = *b"gnre";
const CMT: [u8; 4] = [0xA9, b'c', b'm', b't'];
const WRT: [u8; 4] = [0xA9, b'w', b'r', b't'];
const COM: [u8; 4] = [0xA9, b'c', b'o', b'm'];
const TRKN: [u8; 4] = *b"trkn";
const DISK: [u8; 4] = *b"disk";
const COVR: [u8; 4] = *b"covr";

// Types de l'atome `data`.
const TYPE_IMPLICIT: u32 = 0;
const TYPE_UTF8: u32 = 1;
const TYPE_GIF: u32 = 12;
const TYPE_JPEG: u32 = 13;
const TYPE_PNG: u32 = 14;
const TYPE_BMP: u32 = 27;

#[derive(Debug, Clone)]
struct Atom {
    kind: [u8; 4],
    body: Body,
}

#[derive(Debug, Clone)]
enum Body {
    Leaf(Vec<u8>),
    /// `prefix` : version et drapeaux d'une « full box » (`meta`) ;
    /// `tail` : octets de fin trop courts pour un atome (zéro final QuickTime).
    Container { prefix: Vec<u8>, children: Vec<Atom>, tail: Vec<u8> },
}

impl Atom {
    fn leaf(kind: [u8; 4], data: Vec<u8>) -> Self {
        Atom { kind, body: Body::Leaf(data) }
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Atom>> {
        match &mut self.body {
            Body::Container { children, .. } => Some(children),
            Body::Leaf(_) => None,
        }
    }
}

/// Atome de premier niveau, repéré sans être chargé (l'audio peut peser des centaines de Mo).
#[derive(Debug, Clone, Copy)]
struct TopAtom {
    kind: [u8; 4],
    offset: u64,
    size: u64,
}

/// Applique `edit` au fichier MP4 `path`.
pub fn apply(path: &Path, edit: &TagEdit) -> Result<(), InjectError> {
    let mut file = fs::File::open(path)?;
    let file_len = file.metadata()?.len();

    let top = read_top_level(&mut file, file_len)?;
    if top.iter().any(|a| &a.kind == b"moof") {
        return Err(InjectError::UnsupportedFormat("MP4 fragmenté".into()));
    }
    let mut moovs = top.iter().enumerate().filter(|(_, a)| &a.kind == b"moov");
    let (moov_index, moov_at) = match (moovs.next(), moovs.next()) {
        (Some((i, a)), None) => (i, *a),
        _ => return Err(InjectError::Malformed("un seul atome « moov » attendu".into())),
    };
    if moov_at.size > MAX_MOOV_LEN {
        return Err(InjectError::Malformed(format!("« moov » de {} octets", moov_at.size)));
    }

    // Les `free` collés au `moov` font partie de la place disponible.
    let absorbed: Vec<usize> = top[moov_index + 1..]
        .iter()
        .take_while(|a| matches!(&a.kind, b"free" | b"skip"))
        .enumerate()
        .map(|(i, _)| moov_index + 1 + i)
        .collect();
    let region = moov_at.size + absorbed.iter().map(|&i| top[i].size).sum::<u64>();

    let mut raw = vec![0u8; moov_at.size as usize];
    file.seek(SeekFrom::Start(moov_at.offset))?;
    file.read_exact(&mut raw)?;
    let mut moov = parse_atom(&raw, b"")?
        .ok_or_else(|| InjectError::Malformed("« moov » illisible".into()))?
        .0;
    if moov.children_mut().is_some_and(|c| c.iter().any(|a| &a.kind == b"mvex")) {
        return Err(InjectError::UnsupportedFormat("MP4 fragmenté".into()));
    }

    // ─── Tags ───
    {
        let meta = meta_of(&mut moov)?;
        let children = meta.children_mut().expect("meta est un conteneur");
        children.retain(|a| !matches!(&a.kind, b"free" | b"skip"));
        let ilst = children.iter_mut().find(|a| &a.kind == b"ilst").expect("ilst créé par meta_of");
        let Body::Leaf(data) = &ilst.body else { unreachable!("ilst est lu comme une feuille") };
        let mut items = parse_children(data, b"ilst")?.0;
        apply_fields(&mut items, edit);
        apply_images(&mut items, edit)?;
        ilst.body = Body::Leaf(serialize_all(&items));
    }

    // ─── Sur place : le `moov` réécrit tient dans sa zone, `free` compris ───
    let base_len = serialized_len(&moov);
    let slack = region.checked_sub(base_len);
    if let Some(slack) = slack.filter(|&s| s == 0 || s >= 8) {
        if slack > 0 {
            push_padding(&mut moov, slack as usize)?;
        }
        let out = serialize(&moov);
        debug_assert_eq!(out.len() as u64, region, "le moov réécrit décalerait l'audio");
        atomic_write::write_in_place(path, moov_at.offset, &out)?;
        log::info!("🏷  Tags MP4 réécrits sur place : {}", path.display());
        return Ok(());
    }

    // ─── Réécriture complète : l'audio qui suit le `moov` recule ───
    push_padding(&mut moov, PADDING_LEN)?;
    let new_len = serialized_len(&moov);
    let delta = new_len as i64 - region as i64;
    shift_chunk_offsets(&mut moov, moov_at.offset + region, delta)?;
    let out = serialize(&moov);

    atomic_write::replace_file_with(path, move |dest| {
        for (i, atom) in top.iter().enumerate() {
            if i == moov_index {
                dest.write_all(&out)?;
            } else if !absorbed.contains(&i) {
                file.seek(SeekFrom::Start(atom.offset))?;
                io::copy(&mut (&mut file).take(atom.size), dest)?;
            }
        }
        Ok(())
    })?;

    log::info!("🏷  Tags MP4 réécrits : {} (moov {} → {} octets)", path.display(), region, new_len);
    Ok(())
}

// ─── Lecture des atomes ──────────────────────────────────────────────────

fn read_top_level(file: &mut fs::File, file_len: u64) -> Result<Vec<TopAtom>, InjectError> {
    let mut atoms = Vec::new();
    let mut offset = 0u64;
    while offset + 8 <= file_len {
        let mut header = [0u8; 16];
        file.seek(SeekFrom::Start(offset))?;
        let n = file.read(&mut header)?;
        let (size, _) = atom_size(&header[..n], file_len - offset)?;
        if size < 8 || offset + size > file_len {
            return Err(InjectError::Malformed(format!("atome de {size} octets à la position {offset}")));
        }
        atoms.push(TopAtom { kind: header[4..8].try_into().unwrap(), offset, size });
        offset += size;
    }
    Ok(atoms)
}

/// Taille totale et longueur d'en-tête ; une taille 0 court jusqu'à la fin (`remaining`).
fn atom_size(header: &[u8], remaining: u64) -> Result<(u64, usize), InjectError> {
    if header.len() < 8 {
        return Err(InjectError::Malformed("en-tête d'atome tronqué".into()));
    }
    match u32::from_be_bytes(header[0..4].try_into().unwrap()) {
        0 => Ok((remaining, 8)),
        1 => {
            let large = header.get(8..16).ok_or_else(|| InjectError::Malformed("taille 64 bits tronquée".into()))?;
            Ok((u64::from_be_bytes(large.try_into().unwrap()), 16))
        }
        n => Ok((n as u64, 8)),
    }
}

/// Ne descend que sur le chemin des tags et des tables de positions ; le reste est recopié tel quel.
fn is_container(parent: &[u8], kind: &[u8; 4]) -> bool {
    matches!(
        (parent, kind),
        (b"", b"moov")
            | (b"moov", b"trak" | b"udta")
            | (b"trak", b"mdia")
            | (b"mdia", b"minf")
            | (b"minf", b"stbl")
            | (b"udta", b"meta")
    )
}

/// Lit un atome en tête de `data` ; `None` s'il reste moins qu'un en-tête.
fn parse_atom(data: &[u8], parent: &[u8]) -> Result<Option<(Atom, usize)>, InjectError> {
    if data.len() < 8 {
        return Ok(None);
    }
    let (size, header) = atom_size(data, data.len() as u64)?;
    if size < header as u64 || size > data.len() as u64 {
        return Err(InjectError::Malformed(format!("atome de {size} octets dans {} disponibles", data.len())));
    }
    let size = size as usize;
    let kind: [u8; 4] = data[4..8].try_into().unwrap();
    let body = &data[header..size];

    let atom = if is_container(parent, &kind) {
        // `meta` est une full box chez iTunes, mais pas toujours chez QuickTime.
        let prefix_len = if &kind == b"meta" && body.get(4..8) != Some(b"hdlr") { 4 } else { 0 };
        let prefix = body.get(..prefix_len).ok_or_else(|| InjectError::Malformed("« meta » tronqué".into()))?;
        let (children, tail) = parse_children(&body[prefix_len..], &kind)?;
        Atom { kind, body: Body::Container { prefix: prefix.to_vec(), children, tail } }
    } else {
        Atom::leaf(kind, body.to_vec())
    };
    Ok(Some((atom, size)))
}

fn parse_children(mut data: &[u8], parent: &[u8]) -> Result<(Vec<Atom>, Vec<u8>), InjectError> {
    let mut children = Vec::new();
    while let Some((atom, size)) = parse_atom(data, parent)? {
        children.push(atom);
        data = &data[size..];
    }
    Ok((children, data.to_vec()))
}

// ─── Écriture des atomes ─────────────────────────────────────────────────

fn body_len(atom: &Atom) -> u64 {
    match &atom.body {
        Body::Leaf(data) => data.len() as u64,
        Body::Container { prefix, children, tail } => {
            (prefix.len() + tail.len()) as u64 + children.iter().map(serialized_len).sum::<u64>()
        }
    }
}

fn serialized_len(atom: &Atom) -> u64 {
    let body = body_len(atom);
    body + if body + 8 > u32::MAX as u64 { 16 } else { 8 }
}

fn write_atom(atom: &Atom, out: &mut Vec<u8>) {
    let total = serialized_len(atom);
    if total > u32::MAX as u64 {
        out.extend_from_slice(&1u32.to_be_bytes());
        out.extend_from_slice(&atom.kind);
        out.extend_from_slice(&total.to_be_bytes());
    } else {
        out.extend_from_slice(&(total as u32).to_be_bytes());
        out.extend_from_slice(&atom.kind);
    }
    match &atom.body {
        Body::Leaf(data) => out.extend_from_slice(data),
        Body::Container { prefix, children, tail } => {
            out.extend_from_slice(prefix);
            for child in children {
                write_atom(child, out);
            }
            out.extend_from_slice(tail);
        }
    }
}

fn serialize(atom: &Atom) -> Vec<u8> {
    let mut out = Vec::with_capacity(serialized_len(atom) as usize);
    write_atom(atom, &mut out);
    out
}

fn serialize_all(atoms: &[Atom]) -> Vec<u8> {
    let mut out = Vec::new();
    for atom in atoms {
        write_atom(atom, &mut out);
    }
    out
}

// ─── moov/udta/meta ──────────────────────────────────────────────────────

/// `moov/udta/meta`, créés au besoin avec leur `hdlr` et un `ilst` vide.
fn meta_of(moov: &mut Atom) -> Result<&mut Atom, InjectError> {
    let children = moov.children_mut().ok_or_else(|| InjectError::Malformed("« moov » vide".into()))?;
    if !children.iter().any(|a| &a.kind == b"udta") {
        children.push(Atom { kind: *b"udta", body: Body::Container { prefix: vec![], children: vec![], tail: vec![] } });
    }
    let udta = children.iter_mut().find(|a| &a.kind == b"udta").unwrap().children_mut().unwrap();
    if !udta.iter().any(|a| &a.kind == b"meta") {
        let mut hdlr = vec![0u8; 8];
        hdlr.extend_from_slice(b"mdirappl");
        hdlr.extend_from_slice(&[0u8; 9]);
        udta.push(Atom {
            kind: *b"meta",
            body: Body::Container { prefix: vec![0; 4], children: vec![Atom::leaf(*b"hdlr", hdlr)], tail: vec![] },
        });
    }
    let meta = udta.iter_mut().find(|a| &a.kind == b"meta").unwrap();
    let list = meta.children_mut().unwrap();
    if !list.iter().any(|a| &a.kind == b"ilst") {
        let at = list.iter().position(|a| &a.kind == b"hdlr").map_or(0, |i| i + 1);
        list.insert(at, Atom::leaf(*b"ilst", vec![]));
    }
    Ok(meta)
}

/// Pose un `free` de `len` octets (en-tête compris) à la fin de `meta`.
fn push_padding(moov: &mut Atom, len: usize) -> Result<(), InjectError> {
    let meta = meta_of(moov)?;
    meta.children_mut().unwrap().push(Atom::leaf(*b"free", vec![0; len - 8]));
    Ok(())
}

/// Décale de `delta` les positions `stco`/`co64` qui pointent au-delà de `from`.
fn shift_chunk_offsets(atom: &mut Atom, from: u64, delta: i64) -> Result<(), InjectError> {
    match &mut atom.body {
        Body::Container { children, .. } => {
            for child in children {
                shift_chunk_offsets(child, from, delta)?;
            }
        }
        Body::Leaf(data) if &atom.kind == b"stco" || &atom.kind == b"co64" => {
            let width = if &atom.kind == b"stco" { 4 } else { 8 };
            let count = data.get(4..8).map_or(0, |c| u32::from_be_bytes(c.try_into().unwrap())) as usize;
            if data.len() < 8 + count * width {
                return Err(InjectError::Malformed("table de positions tronquée".into()));
            }
            for entry in data[8..8 + count * width].chunks_exact_mut(width) {
                let old = if width == 4 {
                    u32::from_be_bytes(entry.try_into().unwrap()) as u64
                } else {
                    u64::from_be_bytes(entry.try_into().unwrap())
                };
                if old < from {
                    continue;
                }
                let new = old.checked_add_signed(delta).ok_or_else(|| InjectError::Malformed("position négative".into()))?;
                if width == 4 {
                    let new = u32::try_from(new).map_err(|_| {
                        InjectError::InvalidRequest("audio au-delà de 4 Go : positions 32 bits saturées".into())
                    })?;
                    entry.copy_from_slice(&new.to_be_bytes());
                } else {
                    entry.copy_from_slice(&new.to_be_bytes());
                }
            }
        }
        Body::Leaf(_) => {}
    }
    Ok(())
}

// ─── Éléments de `ilst` ──────────────────────────────────────────────────

/// Les `data` d'un élément : (type, contenu).
fn item_values(item: &Atom) -> Vec<(u32, Vec<u8>)> {
    let Body::Leaf(body) = &item.body else { return vec![] };
    parse_children(body, b"")
        .map(|(atoms, _)| atoms)
        .unwrap_or_default()
        .into_iter()
        .filter(|a| &a.kind == b"data")
        .filter_map(|a| match a.body {
            Body::Leaf(d) if d.len() >= 8 => Some((u32::from_be_bytes(d[0..4].try_into().unwrap()) & 0xFF_FFFF, d[8..].to_vec())),
            _ => None,
        })
        .collect()
}

fn item(kind: [u8; 4], values: &[(u32, &[u8])]) -> Atom {
    let datas: Vec<Atom> = values
        .iter()
        .map(|(kind, payload)| {
            let mut d = Vec::with_capacity(8 + payload.len());
            d.extend_from_slice(&kind.to_be_bytes());
            d.extend_from_slice(&[0; 4]);
            d.extend_from_slice(payload);
            Atom::leaf(*b"data", d)
        })
        .collect();
    Atom::leaf(kind, serialize_all(&datas))
}

/// Remplace l'élément à sa place (sinon à la fin), ou le retire avec `None`.
fn put(items: &mut Vec<Atom>, kind: [u8; 4], new: Option<Atom>) {
    let at = items.iter().position(|a| a.kind == kind);
    items.retain(|a| a.kind != kind);
    if let Some(new) = new {
        items.insert(at.unwrap_or(items.len()).min(items.len()), new);
    }
}

fn set_text(items: &mut Vec<Atom>, kind: [u8; 4], edit: &FieldEdit<String>) {
    match edit {
        FieldEdit::Keep => {}
        FieldEdit::Clear => put(items, kind, None),
        FieldEdit::Set(v) => put(items, kind, Some(item(kind, &[(TYPE_UTF8, v.as_bytes())]))),
    }
}

/// `trkn` (8 octets) et `disk` (6) : réservé, numéro, total sur 16 bits.
fn set_pair(items: &mut Vec<Atom>, kind: [u8; 4], len: usize, number: &FieldEdit<u16>, total: &FieldEdit<u16>) {
    if !number.is_change() && !total.is_change() {
        return;
    }
    let current = items.iter().find(|a| a.kind == kind).and_then(|a| item_values(a).into_iter().next());
    let read = |range: std::ops::Range<usize>| {
        current.as_ref().and_then(|(_, d)| d.get(range)).map(|b| u16::from_be_bytes(b.try_into().unwrap())).filter(|&n| n > 0)
    };
    let number = number.clone().apply(read(2..4));
    let total = total.clone().apply(read(4..6));
    if number.is_none() && total.is_none() {
        return put(items, kind, None);
    }
    let mut payload = vec![0u8; len];
    payload[2..4].copy_from_slice(&number.unwrap_or(0).to_be_bytes());
    payload[4..6].copy_from_slice(&total.unwrap_or(0).to_be_bytes());
    put(items, kind, Some(item(kind, &[(TYPE_IMPLICIT, &payload)])));
}

fn apply_fields(items: &mut Vec<Atom>, edit: &TagEdit) {
    set_text(items, NAM, &edit.title);
    set_text(items, ART, &edit.artist);
    set_text(items, ALB, &edit.album);
    set_text(items, AART, &edit.album_artist);
    set_text(items, DAY, &edit.year);
    if edit.genre.is_change() {
        // Un `gnre` laissé à côté contredirait le nouveau genre.
        put(items, GNRE, None);
    }
    set_text(items, GEN, &edit.genre);
    set_text(items, CMT, &edit.comment);
    // Compositeur : `©wrt` chez iTunes ; un `©com` resté à côté le masquerait.
    if edit.composer.is_change() {
        put(items, COM, None);
    }
    set_text(items, WRT, &edit.composer);
    set_pair(items, TRKN, 8, &edit.track_number, &edit.total_tracks);
    set_pair(items, DISK, 6, &edit.disc_number, &edit.total_discs);
}

// ─── Pochettes (`covr`) ──────────────────────────────────────────────────

/// `covr` ne connaît ni type ni description : la première image est la pochette.
fn apply_images(items: &mut Vec<Atom>, edit: &TagEdit) -> Result<(), InjectError> {
    let existing: Vec<(u32, Vec<u8>)> = items.iter().find(|a| a.kind == COVR).map(item_values).unwrap_or_default();
    let inventory: Vec<ExistingImage> = existing
        .iter()
        .enumerate()
        .map(|(i, (_, data))| ExistingImage {
            id: image_id(data),
            picture_type: if i == 0 { PICTURE_TYPE_COVER_FRONT } else { 0 },
            description: String::new(),
        })
        .collect();
    let Some(mut slots) = edit.images.resolve(&inventory) else { return Ok(()) };

    // La pochette avant passe en tête : c'est la seule façon de la désigner en MP4.
    if let Some(i) = slots.iter().position(|s| s.picture_type == PICTURE_TYPE_COVER_FRONT) {
        let cover = slots.remove(i);
        slots.insert(0, cover);
    }
    let images = slots.iter().map(|slot| resolve_slot(&existing, slot)).collect::<Result<Vec<_>, _>>()?;
    let values: Vec<(u32, &[u8])> = images.iter().map(|(t, d)| (*t, d.as_slice())).collect();
    put(items, COVR, (!values.is_empty()).then(|| item(COVR, &values)));
    Ok(())
}

fn resolve_slot(existing: &[(u32, Vec<u8>)], slot: &ImageSlot) -> Result<(u32, Vec<u8>), InjectError> {
    match &slot.source {
        ImageSource::Existing(id) => existing
            .iter()
            .find(|(_, data)| image_id(data) == *id)
            .cloned()
            .ok_or_else(|| InjectError::InvalidRequest(format!("image introuvable dans le fichier : {id}"))),
        ImageSource::New { data, mime_type, .. } => {
            let kind = match mime_type.to_ascii_lowercase().as_str() {
                "image/jpeg" | "image/jpg" => TYPE_JPEG,
                "image/png" => TYPE_PNG,
                "image/bmp" => TYPE_BMP,
                "image/gif" => TYPE_GIF,
                _ => TYPE_IMPLICIT,
            };
            Ok((kind, data.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::audio_analyser::audio_analyser::AudioAnalyser;
    use crate::core::audio_metadata::injector::edit::ImagePlan;
    use crate::core::audio_player::audio_utils::convert_audio_buffer_to_interleaved;
    use crate::core::audio_player::codecs::codec_registry;
    use crate::entity::audio::audio_tags::AudioTags;
    use std::path::PathBuf;
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::formats::probe::Hint;

    fn fixture(name: &str) -> Vec<u8> {
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap()
    }

    fn temp_file(tag: &str, content: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rustmusic-mp4-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("piste.m4a");
        fs::write(&file, content).unwrap();
        file
    }

    fn cleanup(file: &Path) {
        let _ = fs::remove_dir_all(file.parent().unwrap());
    }

    /// Échantillons décodés : la preuve que l'audio est toujours lu au bon endroit.
    fn decode(path: &Path) -> Vec<f32> {
        let mss = symphonia::core::io::MediaSourceStream::new(Box::new(fs::File::open(path).unwrap()), Default::default());
        let mut hint = Hint::new();
        hint.with_extension("m4a");
        let mut reader = symphonia::default::get_probe().probe(&hint, mss, Default::default(), Default::default()).unwrap();
        let track = reader.tracks().iter().find(|t| t.codec_params.as_ref().and_then(|c| c.audio()).is_some()).unwrap();
        let (id, params) = (track.id, track.codec_params.as_ref().unwrap().audio().unwrap().clone());
        let channels = params.channels.as_ref().unwrap().count();
        let mut decoder = codec_registry().make_audio_decoder(&params, &AudioDecoderOptions::default()).unwrap();
        let mut samples = Vec::new();
        while let Ok(Some(packet)) = reader.next_packet() {
            if packet.track_id == id {
                samples.extend(convert_audio_buffer_to_interleaved(&decoder.decode(&packet).unwrap(), channels));
            }
        }
        samples
    }

    fn tags(path: &Path) -> AudioTags {
        AudioAnalyser::analyse_audio_file(&path.to_path_buf()).unwrap().tags
    }

    /// Les éléments de `ilst`, relus par notre propre lecteur.
    fn ilst_kinds(path: &Path) -> Vec<[u8; 4]> {
        let data = fs::read(path).unwrap();
        let mut file = fs::File::open(path).unwrap();
        let top = read_top_level(&mut file, data.len() as u64).unwrap();
        let at = top.iter().find(|a| &a.kind == b"moov").unwrap();
        let mut moov = parse_atom(&data[at.offset as usize..(at.offset + at.size) as usize], b"").unwrap().unwrap().0;
        let meta = meta_of(&mut moov).unwrap();
        let ilst = meta.children_mut().unwrap().iter().find(|a| &a.kind == b"ilst").unwrap().clone();
        let Body::Leaf(body) = ilst.body else { panic!() };
        parse_children(&body, b"ilst").unwrap().0.iter().map(|a| a.kind).collect()
    }

    /// iTunes range le `moov` avant l'audio : on reconstruit la fixture ainsi, positions corrigées à la main.
    fn moov_first(name: &str) -> Vec<u8> {
        let data = fixture(name);
        let mut file_top = Vec::new();
        let mut at = 0usize;
        while at + 8 <= data.len() {
            let size = u32::from_be_bytes(data[at..at + 4].try_into().unwrap()) as usize;
            file_top.push((data[at + 4..at + 8].to_vec(), at, size));
            at += size;
        }
        let find = |k: &[u8]| file_top.iter().find(|(kind, ..)| kind == k).unwrap().clone();
        let (_, moov_at, moov_len) = find(b"moov");
        let mut moov = data[moov_at..moov_at + moov_len].to_vec();
        let stco = moov.windows(4).position(|w| w == b"stco").unwrap() + 4;
        let count = u32::from_be_bytes(moov[stco + 4..stco + 8].try_into().unwrap()) as usize;
        for i in 0..count {
            let p = stco + 8 + i * 4;
            let v = u32::from_be_bytes(moov[p..p + 4].try_into().unwrap()) + moov_len as u32;
            moov[p..p + 4].copy_from_slice(&v.to_be_bytes());
        }
        let mut out = Vec::new();
        for (kind, offset, size) in &file_top {
            if kind == b"moov" {
                continue;
            }
            if kind == b"mdat" {
                out.extend_from_slice(&moov);
            }
            out.extend_from_slice(&data[*offset..offset + size]);
        }
        out
    }

    fn all_fields() -> TagEdit {
        TagEdit {
            title: FieldEdit::Set("Life on Mars?".into()),
            artist: FieldEdit::Set("David Bowie".into()),
            album: FieldEdit::Set("Hunky Dory".into()),
            album_artist: FieldEdit::Set("Bowie".into()),
            year: FieldEdit::Set("1971".into()),
            genre: FieldEdit::Set("Glam rock".into()),
            comment: FieldEdit::Set("Remaster 2015 — été".into()),
            composer: FieldEdit::Set("D. Bowie".into()),
            track_number: FieldEdit::Set(4),
            total_tracks: FieldEdit::Set(11),
            disc_number: FieldEdit::Set(1),
            total_discs: FieldEdit::Set(2),
            ..Default::default()
        }
    }

    fn title(value: &str) -> TagEdit {
        TagEdit { title: FieldEdit::Set(value.into()), ..Default::default() }
    }

    fn assert_all_fields(path: &Path) {
        let t = tags(path);
        assert_eq!(t.title.as_deref(), Some("Life on Mars?"));
        assert_eq!(t.artist.as_deref(), Some("David Bowie"));
        assert_eq!(t.album.as_deref(), Some("Hunky Dory"));
        assert_eq!(t.album_artist.as_deref(), Some("Bowie"));
        assert_eq!(t.year.as_deref(), Some("1971"));
        assert_eq!(t.genre.as_deref(), Some("Glam rock"));
        assert_eq!(t.comment.as_deref(), Some("Remaster 2015 — été"));
        assert_eq!(t.composer.as_deref(), Some("D. Bowie"));
        assert_eq!((t.track_number, t.total_tracks, t.disc_number, t.total_discs), (Some(4), Some(11), Some(1), Some(2)));
    }

    #[test]
    fn writes_every_field_on_alac_16_and_24_bits() {
        for name in ["alac16.m4a", "alac24.m4a"] {
            let file = temp_file(&format!("champs-{name}"), &fixture(name));
            let before = decode(&file);
            apply(&file, &all_fields()).unwrap();
            assert_all_fields(&file);
            assert_eq!(decode(&file), before, "{name} : audio altéré");
            cleanup(&file);
        }
    }

    #[test]
    fn moov_before_the_audio_moves_the_chunk_offsets() {
        let original = moov_first("alac24.m4a");
        let file = temp_file("moov-devant", &original);
        let before = decode(&file);
        assert!(!before.is_empty());

        apply(&file, &all_fields()).unwrap();

        assert!(fs::metadata(&file).unwrap().len() > original.len() as u64, "le moov aurait dû grossir");
        assert_all_fields(&file);
        assert_eq!(decode(&file), before, "positions stco non suivies : audio illisible ou décalé");
        cleanup(&file);
    }

    #[test]
    fn a_second_edit_is_written_in_place() {
        for (tag, content) in [("fin", fixture("alac16.m4a")), ("devant", moov_first("alac16.m4a"))] {
            let file = temp_file(&format!("sur-place-{tag}"), &content);
            let before = decode(&file);
            apply(&file, &title("Court")).unwrap();
            let len = fs::metadata(&file).unwrap().len();
            let mdat = |p: &Path| {
                let d = fs::read(p).unwrap();
                let at = d.windows(4).position(|w| w == b"mdat").unwrap();
                (at, d[at..at + 1000].to_vec())
            };
            let audio = mdat(&file);

            apply(&file, &title("Un titre nettement plus long qu'avant")).unwrap();

            assert_eq!(fs::metadata(&file).unwrap().len(), len, "{tag} : la taille a changé");
            assert_eq!(mdat(&file), audio, "{tag} : l'audio a bougé");
            assert_eq!(tags(&file).title.as_deref(), Some("Un titre nettement plus long qu'avant"));
            assert_eq!(decode(&file), before);
            cleanup(&file);
        }
    }

    #[test]
    fn padding_too_small_falls_back_to_a_full_rewrite() {
        let file = temp_file("debordement", &moov_first("alac16.m4a"));
        let before = decode(&file);
        apply(&file, &title("x")).unwrap();
        // Plus long que tout le remplissage : le moov ne tient plus, l'audio recule.
        apply(&file, &title(&"y".repeat(PADDING_LEN * 2))).unwrap();
        assert_eq!(tags(&file).title.map(|t| t.len()), Some(PADDING_LEN * 2));
        assert_eq!(decode(&file), before);
        cleanup(&file);
    }

    #[test]
    fn clearing_removes_and_unknown_items_survive() {
        let file = temp_file("effacer", &fixture("alac16.m4a"));
        apply(&file, &all_fields()).unwrap();
        apply(&file, &TagEdit { comment: FieldEdit::Clear, total_tracks: FieldEdit::Clear, ..Default::default() }).unwrap();

        let t = tags(&file);
        assert_eq!(t.comment, None);
        assert_eq!((t.track_number, t.total_tracks), (Some(4), None), "le numéro doit rester sans son total");
        assert_eq!(t.title.as_deref(), Some("Life on Mars?"));
        // `©too` (l'encodeur), que l'édition ne connaît pas, est intact.
        assert!(ilst_kinds(&file).contains(&[0xA9, b't', b'o', b'o']));
        cleanup(&file);
    }

    #[test]
    fn freeform_items_like_replaygain_are_kept() {
        let file = temp_file("libre", &fixture("alac16.m4a"));
        // Un `----` (ReplayGain) posé à la main dans ilst.
        let mut items = vec![];
        let mean = Atom::leaf(*b"mean", [&[0u8; 4][..], b"com.apple.iTunes"].concat());
        let name = Atom::leaf(*b"name", [&[0u8; 4][..], b"replaygain_track_gain"].concat());
        let data = Atom::leaf(*b"data", [&1u32.to_be_bytes()[..], &[0; 4], b"-6.20 dB"].concat());
        items.push(Atom::leaf(*b"----", serialize_all(&[mean, name, data])));
        inject_items(&file, &items);

        apply(&file, &title("Avec ReplayGain")).unwrap();

        assert!(ilst_kinds(&file).contains(b"----"), "l'élément libre a disparu");
        let mss = symphonia::core::io::MediaSourceStream::new(Box::new(fs::File::open(&file).unwrap()), Default::default());
        let mut reader = symphonia::default::get_probe().probe(&Hint::new(), mss, Default::default(), Default::default()).unwrap();
        let gain = crate::core::audio_player::replay_gain::from_metadata(reader.metadata().current().unwrap());
        assert_eq!(gain.track_gain_db, Some(-6.2), "ReplayGain illisible après édition");
        cleanup(&file);
    }

    /// Ajoute des éléments bruts dans `ilst`, en passant par une réécriture complète.
    fn inject_items(file: &Path, extra: &[Atom]) {
        let data = fs::read(file).unwrap();
        let mut f = fs::File::open(file).unwrap();
        let top = read_top_level(&mut f, data.len() as u64).unwrap();
        let at = *top.iter().find(|a| &a.kind == b"moov").unwrap();
        let mut moov = parse_atom(&data[at.offset as usize..(at.offset + at.size) as usize], b"").unwrap().unwrap().0;
        let meta = meta_of(&mut moov).unwrap();
        let ilst = meta.children_mut().unwrap().iter_mut().find(|a| &a.kind == b"ilst").unwrap();
        let Body::Leaf(body) = &mut ilst.body else { panic!() };
        body.extend_from_slice(&serialize_all(extra));
        // La fixture a son moov en fin de fichier : rien ne le suit.
        assert_eq!(at.offset + at.size, data.len() as u64);
        let mut out = data[..at.offset as usize].to_vec();
        out.extend_from_slice(&serialize(&moov));
        fs::write(file, out).unwrap();
    }

    #[test]
    fn setting_the_genre_drops_the_numeric_one() {
        let file = temp_file("gnre", &fixture("alac16.m4a"));
        inject_items(&file, &[item(GNRE, &[(TYPE_IMPLICIT, &[0, 18])])]);
        apply(&file, &TagEdit { genre: FieldEdit::Set("Trip-hop".into()), ..Default::default() }).unwrap();
        let kinds = ilst_kinds(&file);
        assert!(!kinds.contains(&GNRE) && kinds.contains(&GEN));
        assert_eq!(tags(&file).genre.as_deref(), Some("Trip-hop"));
        cleanup(&file);
    }

    fn jpeg(seed: u8) -> Vec<u8> {
        let mut d = vec![0xFF, 0xD8, 0xFF, 0xE0, seed];
        d.extend(std::iter::repeat_n(seed, 300));
        d
    }

    fn new_cover(data: Vec<u8>) -> ImageSlot {
        ImageSlot {
            source: ImageSource::New { data, mime_type: "image/jpeg".into(), width: 10, height: 10 },
            picture_type: PICTURE_TYPE_COVER_FRONT,
            description: String::new(),
        }
    }

    fn images(path: &Path) -> Vec<Vec<u8>> {
        tags(path).attached_images.into_iter().map(|i| i.image_data).collect()
    }

    #[test]
    fn covers_are_added_replaced_and_removed() {
        let file = temp_file("pochettes", &moov_first("alac16.m4a"));
        let before = decode(&file);

        // Deux images : la pochette, puis une seconde.
        apply(&file, &TagEdit { images: ImagePlan::SetCover(new_cover(jpeg(1))), ..Default::default() }).unwrap();
        let second = ImageSlot { picture_type: 4, ..new_cover(jpeg(2)) };
        let keep_first = ImageSlot { source: ImageSource::Existing(image_id(&jpeg(1))), ..new_cover(vec![]) };
        apply(&file, &TagEdit { images: ImagePlan::Replace(vec![second, keep_first]), ..Default::default() }).unwrap();
        assert_eq!(images(&file), vec![jpeg(1), jpeg(2)], "la pochette avant doit passer en tête");

        // Nouvelle pochette en édition multiple : la seconde image reste.
        apply(&file, &TagEdit { images: ImagePlan::SetCover(new_cover(jpeg(3))), ..Default::default() }).unwrap();
        assert_eq!(images(&file), vec![jpeg(3), jpeg(2)]);

        apply(&file, &TagEdit { images: ImagePlan::Replace(vec![]), ..Default::default() }).unwrap();
        assert!(images(&file).is_empty());
        assert!(!ilst_kinds(&file).contains(&COVR));
        assert_eq!(decode(&file), before);
        cleanup(&file);
    }

    #[test]
    fn a_file_without_any_metadata_gets_a_full_tree() {
        // Fixture privée de son `udta` : udta, meta, hdlr et ilst sont à créer.
        let data = fixture("alac16.m4a");
        let src = temp_file("nu-src", &data);
        let top = read_top_level(&mut fs::File::open(&src).unwrap(), data.len() as u64).unwrap();
        cleanup(&src);
        let at = *top.iter().find(|a| &a.kind == b"moov").unwrap();
        let mut moov = parse_atom(&data[at.offset as usize..], b"").unwrap().unwrap().0;
        moov.children_mut().unwrap().retain(|a| &a.kind != b"udta");
        let mut bare = data[..at.offset as usize].to_vec();
        bare.extend_from_slice(&serialize(&moov));
        let file = temp_file("nu", &bare);
        let before = decode(&file);

        apply(&file, &all_fields()).unwrap();

        assert_all_fields(&file);
        assert_eq!(decode(&file), before);
        cleanup(&file);
    }

    #[test]
    fn fragmented_files_are_refused_untouched() {
        let mut data = fixture("alac16.m4a");
        data.extend_from_slice(&[0, 0, 0, 8, b'm', b'o', b'o', b'f']);
        let file = temp_file("fragmente", &data);
        assert!(matches!(apply(&file, &title("x")), Err(InjectError::UnsupportedFormat(_))));
        assert_eq!(fs::read(&file).unwrap(), data);
        cleanup(&file);
    }

    #[test]
    fn co64_offsets_are_shifted_too() {
        let mut body = vec![0u8; 4];
        body.extend_from_slice(&2u32.to_be_bytes());
        body.extend_from_slice(&100u64.to_be_bytes());
        body.extend_from_slice(&5_000_000_000u64.to_be_bytes());
        let mut atom = Atom::leaf(*b"co64", body);
        shift_chunk_offsets(&mut atom, 200, 64).unwrap();
        let Body::Leaf(d) = atom.body else { panic!() };
        // Avant le moov : intouché ; après : décalé, même au-delà de 4 Go.
        assert_eq!(u64::from_be_bytes(d[8..16].try_into().unwrap()), 100);
        assert_eq!(u64::from_be_bytes(d[16..24].try_into().unwrap()), 5_000_000_064);
    }

    /// Sur de vrais fichiers (copies !) : `RUSTMUSIC_MP4_DIR=… cargo test --lib real_m4a -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_m4a_files_survive_an_edit_and_its_undo() {
        let dir = PathBuf::from(std::env::var("RUSTMUSIC_MP4_DIR").expect("RUSTMUSIC_MP4_DIR"));
        for entry in fs::read_dir(dir).unwrap() {
            let src = entry.unwrap().path();
            if src.extension().and_then(|e| e.to_str()) != Some("m4a") {
                continue;
            }
            let name = src.file_name().unwrap().to_string_lossy().to_string();
            let file = temp_file(&format!("reel-{}", name.len()), &fs::read(&src).unwrap());
            let (audio, before, kinds, pictures) = (decode(&file), tags(&file), ilst_kinds(&file), images(&file));

            let mut edit = all_fields();
            edit.images = ImagePlan::SetCover(new_cover(jpeg(9)));
            apply(&file, &edit).unwrap();
            assert_all_fields(&file);
            assert_eq!(decode(&file), audio, "{name} : audio altéré");
            let after = ilst_kinds(&file);
            for k in kinds.iter().filter(|k| ![GNRE, COM].contains(k)) {
                assert!(after.contains(k), "{name} : {} perdu", String::from_utf8_lossy(k));
            }
            let mut expected = vec![jpeg(9)];
            expected.extend(pictures.iter().skip(1).cloned());
            assert_eq!(images(&file), expected, "{name} : pochettes");
            // Pour un contrôle par un autre lecteur (Explorateur, ffprobe…).
            if let Ok(out) = std::env::var("RUSTMUSIC_MP4_OUT") {
                fs::copy(&file, Path::new(&out).join(&name)).unwrap();
            }

            let len = fs::metadata(&file).unwrap().len();
            apply(&file, &title("Encore un autre titre, plus long que le premier")).unwrap();
            assert_eq!(fs::metadata(&file).unwrap().len(), len, "{name} : seconde édition pas sur place");

            // Annulation : l'état d'origine revient, champ par champ.
            let back = |v: &Option<String>| v.clone().map_or(FieldEdit::Clear, FieldEdit::Set);
            let num = |v: Option<u16>| v.map_or(FieldEdit::Clear, FieldEdit::Set);
            let slots = pictures
                .iter()
                .enumerate()
                .map(|(i, d)| ImageSlot {
                    source: ImageSource::Existing(image_id(d)),
                    picture_type: if i == 0 { PICTURE_TYPE_COVER_FRONT } else { 0 },
                    description: String::new(),
                })
                .collect();
            let undo = TagEdit {
                title: back(&before.title),
                artist: back(&before.artist),
                album: back(&before.album),
                album_artist: back(&before.album_artist),
                year: back(&before.year),
                genre: back(&before.genre),
                comment: back(&before.comment),
                composer: back(&before.composer),
                track_number: num(before.track_number),
                total_tracks: num(before.total_tracks),
                disc_number: num(before.disc_number),
                total_discs: num(before.total_discs),
                images: ImagePlan::Replace(slots),
            };
            // Les images d'origine ont été retirées par SetCover : on les remet d'abord telles quelles.
            if !pictures.is_empty() {
                let restore_first = ImageSlot { source: ImageSource::New { data: pictures[0].clone(), mime_type: "image/jpeg".into(), width: 0, height: 0 }, ..new_cover(vec![]) };
                apply(&file, &TagEdit { images: ImagePlan::SetCover(restore_first), ..Default::default() }).unwrap();
            }
            apply(&file, &undo).unwrap();
            let t = tags(&file);
            assert_eq!(
                (&t.title, &t.artist, &t.album, &t.album_artist, &t.year, &t.genre, &t.comment, &t.composer),
                (&before.title, &before.artist, &before.album, &before.album_artist, &before.year, &before.genre, &before.comment, &before.composer),
                "{name} : annulation incomplète"
            );
            assert_eq!((t.track_number, t.total_tracks, t.disc_number, t.total_discs), (before.track_number, before.total_tracks, before.disc_number, before.total_discs));
            assert_eq!(images(&file), pictures, "{name} : pochettes non restaurées");
            assert_eq!(decode(&file), audio);
            println!("ok  {name} : {} échantillons, {} élément(s) ilst, {} image(s)", audio.len(), kinds.len(), pictures.len());
            cleanup(&file);
        }
    }

    #[test]
    fn the_router_accepts_m4a_and_refuses_impostors() {
        use crate::core::audio_metadata::injector::injector::{detect_writable, WritableFormat};
        let ok = temp_file("routeur", &fixture("alac16.m4a"));
        assert_eq!(detect_writable(&ok), WritableFormat::Mp4);
        let fake = ok.with_extension("mp4");
        fs::write(&fake, b"pas un mp4 du tout").unwrap();
        assert_eq!(detect_writable(&fake), WritableFormat::Unsupported);
        let adts = ok.with_extension("aac");
        fs::copy(&ok, &adts).unwrap();
        assert_eq!(detect_writable(&adts), WritableFormat::Unsupported, ".aac est de l'ADTS, pas du MP4");
        cleanup(&ok);
    }
}
