//! Encodeur ID3v2 — miroir en écriture de `tag_format::id3v2`.
//!
//! Produit un blob ID3v2 complet (en-tête + frames + padding), prêt à être
//! placé dans un conteneur : au début d'un MP3, dans le chunk `ID3` d'un DSF,
//! dans le chunk `ID3` d'un DFF. Le placement ne le regarde pas.
//!
//! # Principe directeur : ne jamais perdre ce qu'on ne comprend pas
//! Un fichier contient bien plus que les champs éditables — pochette (`APIC`),
//! note (`POPM`), tags personnalisés (`TXXX`), identifiants MusicBrainz… Si
//! corriger un titre effaçait la pochette, l'éditeur ferait plus de dégâts
//! qu'il n'en répare.
//!
//! L'encodeur ne repasse donc **pas** par le parser sémantique. Il parcourt
//! les frames au niveau octet et ne remplace que celles qui sont éditées ;
//! tout le reste est recopié **tel quel**, y compris ce qu'on ne sait pas
//! interpréter.
//!
//! # Version écrite
//! On réécrit dans la **même version majeure que le tag d'origine** (2.3 par
//! défaut quand il n'y en a pas). Ce n'est pas un détail de confort : entre
//! 2.3 et 2.4 la taille des frames ne s'encode pas pareil (entier simple
//! contre entier « synchsafe »), et les bits d'options des frames n'ont pas le
//! même sens. Recopier une frame 2.4 dans un en-tête 2.3 la rendrait
//! illisible. Conserver la version évite tout le problème.

use std::collections::HashMap;

use crate::core::audio_metadata::injector::edit::{
    FieldEdit, ImagePlan, ImageSlot, ImageSource, TagEdit,
};
use crate::core::audio_metadata::injector::image_prep::image_id;
use crate::core::audio_metadata::injector::injector::InjectError;

/// Octets de remplissage laissés après les frames.
///
/// Ils permettront plus tard de réécrire un tag légèrement plus gros **sans
/// toucher au reste du fichier** — sur un DSF de 300 Mo, c'est la différence
/// entre instantané et plusieurs secondes.
const PADDING: usize = 1024;

/// Une frame telle qu'on la manipule ici : un identifiant, ses bits d'options
/// et son corps brut. Le corps n'est jamais interprété pour les frames qu'on
/// se contente de recopier.
struct RawFrame {
    id: [u8; 4],
    flags: u16,
    body: Vec<u8>,
}

/// Réécrit un tag ID3v2 en appliquant `edit`.
///
/// `existing` est le blob ID3v2 actuel du fichier, ou `None` s'il n'en a pas
/// (on en crée alors un de toutes pièces).
pub fn rewrite(existing: Option<&[u8]>, edit: &TagEdit) -> Result<Vec<u8>, InjectError> {
    let (version, mut frames) = match existing {
        Some(blob) => read_frames(blob)?,
        None => (3, Vec::new()),
    };

    apply_edit(version, &mut frames, edit)?;
    Ok(encode(version, &frames))
}

/// Longueur totale du tag ID3v2 placé en tête de `data`, `0` s'il n'y en a pas.
///
/// Sert aux conteneurs pour savoir où commence réellement l'audio. Le pied de
/// tag optionnel de la 2.4 est compté : l'oublier laisserait 10 octets
/// parasites au début du flux audio.
pub fn tag_len(data: &[u8]) -> usize {
    if data.len() < 10 || &data[0..3] != b"ID3" {
        return 0;
    }
    let size = read_synchsafe(&data[6..10]) as usize;
    let footer = if data[5] & 0b0001_0000 != 0 { 10 } else { 0 };
    (10 + size + footer).min(data.len())
}

// ─── Lecture bas niveau ──────────────────────────────────────────────────

/// Parcourt les frames d'un blob existant sans interpréter leur contenu.
///
/// Tolérant par choix : un fichier réel se termine souvent par des octets
/// parasites après les frames. On s'arrête proprement plutôt que d'échouer,
/// exactement comme le fait le parser en lecture.
fn read_frames(blob: &[u8]) -> Result<(u8, Vec<RawFrame>), InjectError> {
    if blob.len() < 10 || &blob[0..3] != b"ID3" {
        return Err(InjectError::Malformed("en-tête ID3v2 absent".into()));
    }

    let version = blob[3];
    if version != 3 && version != 4 {
        return Err(InjectError::Malformed(format!(
            "version ID3v2.{version} non prise en charge"
        )));
    }

    // Un tag globalement « désynchronisé » demanderait de défaire puis refaire
    // cette transformation sur chaque frame recopiée. C'est rare (surtout du
    // v2.3 ancien) et le risque de corruption ne vaut pas la peine : on
    // refuse, l'utilisateur garde son fichier intact.
    if blob[5] & 0b1000_0000 != 0 {
        return Err(InjectError::Malformed(
            "tag désynchronisé : réécriture non prise en charge".into(),
        ));
    }

    let declared = read_synchsafe(&blob[6..10]) as usize;
    let end = (10 + declared).min(blob.len());
    let body = &blob[10..end];

    let mut pos = 0usize;

    // En-tête étendu : on le saute, comme le parser en lecture.
    if blob[5] & 0b0100_0000 != 0 {
        if body.len() < 4 {
            return Err(InjectError::Malformed("en-tête étendu tronqué".into()));
        }
        let ext = if version == 4 {
            read_synchsafe(&body[0..4]) as usize
        } else {
            u32::from_be_bytes([body[0], body[1], body[2], body[3]]) as usize + 4
        };
        if ext > body.len() {
            return Err(InjectError::Malformed("en-tête étendu incohérent".into()));
        }
        pos += ext;
    }

    let mut frames = Vec::new();
    while pos + 10 <= body.len() {
        if body[pos] == 0 {
            break; // début du padding
        }

        let mut id = [0u8; 4];
        id.copy_from_slice(&body[pos..pos + 4]);

        let size = if version == 4 {
            read_synchsafe(&body[pos + 4..pos + 8]) as usize
        } else {
            u32::from_be_bytes([
                body[pos + 4],
                body[pos + 5],
                body[pos + 6],
                body[pos + 7],
            ]) as usize
        };
        let flags = u16::from_be_bytes([body[pos + 8], body[pos + 9]]);
        pos += 10;

        if pos + size > body.len() {
            break; // frame tronquée : on garde ce qu'on a lu jusque-là
        }

        frames.push(RawFrame {
            id,
            flags,
            body: body[pos..pos + size].to_vec(),
        });
        pos += size;
    }

    Ok((version, frames))
}

// ─── Application des modifications ───────────────────────────────────────

/// Identifiant de frame porteur de l'année, selon la version.
///
/// La 2.3 utilise `TYER` (année seule), la 2.4 l'a remplacé par `TDRC` (date
/// complète). On écrit celui de la version courante et on retire l'autre, pour
/// qu'un lecteur ne tombe pas sur deux années contradictoires.
fn year_frame_id(version: u8) -> [u8; 4] {
    if version == 4 {
        *b"TDRC"
    } else {
        *b"TYER"
    }
}

fn apply_edit(version: u8, frames: &mut Vec<RawFrame>, edit: &TagEdit) -> Result<(), InjectError> {
    // Champs texte simples : un champ, une frame.
    let simple: [(&FieldEdit<String>, [u8; 4]); 7] = [
        (&edit.title, *b"TIT2"),
        (&edit.artist, *b"TPE1"),
        (&edit.album, *b"TALB"),
        (&edit.album_artist, *b"TPE2"),
        (&edit.genre, *b"TCON"),
        (&edit.composer, *b"TCOM"),
        (&edit.year, year_frame_id(version)),
    ];

    for (field, id) in simple {
        if !field.is_change() {
            continue;
        }
        remove_frames(frames, &id);
        if id == *b"TYER" || id == *b"TDRC" {
            // Purge l'équivalent de l'autre version, sinon deux années
            // cohabiteraient dans le même tag.
            remove_frames(frames, b"TYER");
            remove_frames(frames, b"TDRC");
        }
        if let FieldEdit::Set(value) = field {
            frames.push(text_frame(version, id, value));
        }
    }

    // Commentaire : structure particulière (langue + description).
    if edit.comment.is_change() {
        remove_frames(frames, b"COMM");
        if let FieldEdit::Set(value) = &edit.comment {
            frames.push(comment_frame(version, value));
        }
    }

    // Numéro de piste et total partagent UNE SEULE frame, au format
    // « piste/total ». Modifier l'un sans écraser l'autre oblige donc à relire
    // la valeur existante avant de la reconstruire.
    apply_pair(
        version,
        frames,
        b"TRCK",
        &edit.track_number,
        &edit.total_tracks,
    );
    apply_pair(
        version,
        frames,
        b"TPOS",
        &edit.disc_number,
        &edit.total_discs,
    );

    apply_images(version, frames, &edit.images)
}

// ─── Images intégrées ────────────────────────────────────────────────────

/// Type ID3 de la pochette avant. Une image n'est pas la pochette parce
/// qu'elle arrive en premier, mais parce qu'elle porte ce type.
const PICTURE_TYPE_COVER_FRONT: u8 = 3;
/// Type de repli quand on doit rétrograder une seconde pochette avant.
const PICTURE_TYPE_OTHER: u8 = 0;

/// Une frame `APIC` décomposée.
#[derive(Debug, Clone)]
struct Apic {
    mime: String,
    picture_type: u8,
    description: String,
    data: Vec<u8>,
}

/// Réécrit toutes les frames `APIC` pour refléter la liste visée.
///
/// Les images conservées sont recopiées **octet pour octet** : promouvoir une
/// image en pochette ou la déplacer ne la fait jamais repasser par un
/// encodeur, donc jamais perdre de qualité. Seuls le type et la description
/// sont réécrits, et ils ne coûtent rien.
fn apply_images(
    version: u8,
    frames: &mut Vec<RawFrame>,
    plan: &ImagePlan,
) -> Result<(), InjectError> {
    let ImagePlan::Replace(slots) = plan else {
        return Ok(());
    };

    // Index par contenu : c'est la clé que l'interface renvoie, et la seule
    // qui ne puisse pas dériver (cf. `image_prep::image_id`).
    let existing: HashMap<String, Apic> = frames
        .iter()
        .filter(|f| &f.id == b"APIC")
        .filter_map(|f| decode_apic(&f.body))
        .map(|a| (image_id(&a.data), a))
        .collect();

    let mut rebuilt = Vec::with_capacity(slots.len());
    // Invariant ID3 : une seule pochette avant. Si l'appelant en désigne
    // plusieurs, seule la première garde le type 3 — sinon chaque lecteur
    // tranche à sa façon et l'affichage devient imprévisible.
    let mut cover_taken = false;

    for slot in slots {
        let mut apic = resolve_slot(&existing, slot)?;
        if apic.picture_type == PICTURE_TYPE_COVER_FRONT {
            if cover_taken {
                apic.picture_type = PICTURE_TYPE_OTHER;
            } else {
                cover_taken = true;
            }
        }
        rebuilt.push(encode_apic(version, &apic));
    }

    remove_frames(frames, b"APIC");
    frames.extend(rebuilt);
    Ok(())
}

/// Construit l'image d'un emplacement, ou échoue si elle est introuvable.
///
/// Ignorer une référence inconnue serait le pire des comportements : elle
/// disparaîtrait de l'enregistrement sans que rien ne le signale, et
/// l'utilisateur perdrait une pochette en croyant l'avoir déplacée.
fn resolve_slot(existing: &HashMap<String, Apic>, slot: &ImageSlot) -> Result<Apic, InjectError> {
    match &slot.source {
        ImageSource::Existing(id) => {
            let found = existing.get(id).ok_or_else(|| {
                InjectError::InvalidRequest(format!("image introuvable dans le fichier : {id}"))
            })?;
            Ok(Apic {
                picture_type: slot.picture_type,
                description: slot.description.clone(),
                ..found.clone()
            })
        }
        // Les dimensions ne servent qu'au FLAC : l'ID3 ne les stocke nulle part.
        ImageSource::New {
            data, mime_type, ..
        } => Ok(Apic {
            mime: mime_type.clone(),
            picture_type: slot.picture_type,
            description: slot.description.clone(),
            data: data.clone(),
        }),
    }
}

/// Décompose une frame `APIC` : encodage, type MIME, type d'image,
/// description, puis les octets de l'image.
///
/// Renvoie `None` sur une frame tronquée plutôt que d'échouer : une image
/// illisible ne doit pas empêcher d'éditer le reste du fichier. Elle ne sera
/// simplement pas référençable — et comme l'appelant ne peut pas produire son
/// identifiant, il ne risque pas de la désigner par erreur.
fn decode_apic(body: &[u8]) -> Option<Apic> {
    let encoding = *body.first()?;
    let rest = &body[1..];

    // Le type MIME est toujours en ISO-8859-1, quel que soit l'encodage
    // déclaré : celui-ci ne concerne que la description.
    let end = rest.iter().position(|&b| b == 0)?;
    let mime: String = rest[..end].iter().map(|&b| b as char).collect();
    let rest = &rest[end + 1..];

    let picture_type = *rest.first()?;
    let rest = &rest[1..];

    let (description, used) = read_terminated(encoding, rest)?;

    Some(Apic {
        mime,
        picture_type,
        description,
        data: rest[used..].to_vec(),
    })
}

fn encode_apic(version: u8, apic: &Apic) -> RawFrame {
    let mut body = Vec::new();

    let latin1 = apic.description.chars().all(|c| (c as u32) <= 0xFF);
    let encoding: u8 = if latin1 {
        0
    } else if version == 4 {
        3
    } else {
        1
    };

    body.push(encoding);
    body.extend(apic.mime.chars().map(|c| c as u8));
    body.push(0);
    body.push(apic.picture_type);

    match encoding {
        0 => {
            body.extend(apic.description.chars().map(|c| c as u8));
            body.push(0);
        }
        3 => {
            body.extend_from_slice(apic.description.as_bytes());
            body.push(0);
        }
        _ => {
            body.extend_from_slice(&[0xFF, 0xFE]); // BOM little-endian
            for unit in apic.description.encode_utf16() {
                body.extend_from_slice(&unit.to_le_bytes());
            }
            body.extend_from_slice(&[0, 0]); // terminateur sur deux octets
        }
    }

    body.extend_from_slice(&apic.data);

    RawFrame {
        id: *b"APIC",
        flags: 0,
        body,
    }
}

/// Lit une chaîne terminée par un octet nul, et renvoie combien d'octets elle
/// occupe **terminateur compris** — c'est cette longueur qui dit où commencent
/// les données de l'image.
///
/// En UTF-16 le terminateur fait deux octets et doit être cherché sur des
/// positions paires : un octet nul isolé fait partie d'un caractère ASCII
/// encodé sur deux octets, et s'arrêter dessus couperait la chaîne en plein
/// milieu.
fn read_terminated(encoding: u8, data: &[u8]) -> Option<(String, usize)> {
    match encoding {
        0 | 3 => {
            let end = data.iter().position(|&b| b == 0)?;
            let text = if encoding == 0 {
                data[..end].iter().map(|&b| b as char).collect()
            } else {
                String::from_utf8_lossy(&data[..end]).to_string()
            };
            Some((text, end + 1))
        }
        1 | 2 => {
            let mut i = 0;
            while i + 1 < data.len() {
                if data[i] == 0 && data[i + 1] == 0 {
                    return Some((decode_utf16(encoding, &data[..i]), i + 2));
                }
                i += 2;
            }
            None
        }
        _ => None,
    }
}

fn decode_utf16(encoding: u8, bytes: &[u8]) -> String {
    // Encodage 1 : UTF-16 précédé d'une marque d'ordre des octets.
    // Encodage 2 : UTF-16 gros-boutiste, sans marque.
    let (body, little) = if encoding == 1 && bytes.len() >= 2 {
        match (bytes[0], bytes[1]) {
            (0xFF, 0xFE) => (&bytes[2..], true),
            (0xFE, 0xFF) => (&bytes[2..], false),
            _ => (bytes, true),
        }
    } else {
        (bytes, false)
    };

    let units: Vec<u16> = body
        .chunks_exact(2)
        .map(|c| {
            if little {
                u16::from_le_bytes([c[0], c[1]])
            } else {
                u16::from_be_bytes([c[0], c[1]])
            }
        })
        .collect();
    String::from_utf16_lossy(&units)
}

/// Reconstruit une frame « valeur/total » en préservant la partie non éditée.
fn apply_pair(
    version: u8,
    frames: &mut Vec<RawFrame>,
    id: &[u8; 4],
    number: &FieldEdit<u16>,
    total: &FieldEdit<u16>,
) {
    if !number.is_change() && !total.is_change() {
        return;
    }

    let (cur_number, cur_total) = frames
        .iter()
        .find(|f| &f.id == id)
        .map(|f| parse_pair(&decode_text_body(&f.body)))
        .unwrap_or((None, None));

    let new_number = number.clone().apply(cur_number);
    let new_total = total.clone().apply(cur_total);

    remove_frames(frames, id);

    let value = match (new_number, new_total) {
        (None, None) => return, // les deux effacés : plus de frame du tout
        (Some(n), Some(t)) => format!("{n}/{t}"),
        (Some(n), None) => format!("{n}"),
        // Un total sans numéro n'a pas de sens dans ce format ; on écrit
        // « 0/total » plutôt que de perdre l'information.
        (None, Some(t)) => format!("0/{t}"),
    };
    frames.push(text_frame(version, *id, &value));
}

fn remove_frames(frames: &mut Vec<RawFrame>, id: &[u8; 4]) {
    frames.retain(|f| &f.id != id);
}

/// Extrait « 3/12 » en `(Some(3), Some(12))`.
fn parse_pair(s: &str) -> (Option<u16>, Option<u16>) {
    let mut it = s.split('/');
    let a = it.next().and_then(|v| v.trim().parse::<u16>().ok());
    let b = it.next().and_then(|v| v.trim().parse::<u16>().ok());
    (a, b)
}

/// Décode le corps d'une frame texte, uniquement pour relire une valeur
/// existante (numéro de piste). Volontairement minimal : les cas exotiques
/// retournent une chaîne vide, ce qui revient à repartir de zéro.
fn decode_text_body(body: &[u8]) -> String {
    if body.is_empty() {
        return String::new();
    }
    match body[0] {
        0 => body[1..].iter().map(|&b| b as char).collect(),
        3 => String::from_utf8_lossy(&body[1..]).to_string(),
        _ => String::new(), // UTF-16 : inutile pour un numéro de piste
    }
}

// ─── Construction des frames ─────────────────────────────────────────────

/// Encode une frame texte avec l'encodage le plus économe qui préserve le
/// texte à l'identique.
///
/// L'ISO-8859-1 suffit à l'immense majorité des titres, y compris accentués,
/// et reste ce que tous les lecteurs savent lire. Au-delà (cyrillique,
/// japonais, emoji) on bascule : UTF-8 en 2.4, UTF-16 en 2.3 qui ne connaît
/// pas l'UTF-8.
fn text_frame(version: u8, id: [u8; 4], value: &str) -> RawFrame {
    let mut body = Vec::new();

    if value.chars().all(|c| (c as u32) <= 0xFF) {
        body.push(0); // ISO-8859-1
        body.extend(value.chars().map(|c| c as u8));
    } else if version == 4 {
        body.push(3); // UTF-8
        body.extend_from_slice(value.as_bytes());
    } else {
        body.push(1); // UTF-16 avec BOM
        body.extend_from_slice(&[0xFF, 0xFE]); // little-endian
        for unit in value.encode_utf16() {
            body.extend_from_slice(&unit.to_le_bytes());
        }
    }

    RawFrame { id, flags: 0, body }
}

/// Frame `COMM` : encodage, langue, description terminée par un octet nul,
/// puis le texte. Description vide = le commentaire principal, celui que les
/// lecteurs affichent.
fn comment_frame(version: u8, value: &str) -> RawFrame {
    let mut body = Vec::new();
    let ascii = value.chars().all(|c| (c as u32) <= 0xFF);

    let encoding: u8 = if ascii {
        0
    } else if version == 4 {
        3
    } else {
        1
    };
    body.push(encoding);
    // « XXX » = langue non précisée (valide selon la spécification).
    body.extend_from_slice(b"XXX");

    match encoding {
        0 => {
            body.push(0); // description vide
            body.extend(value.chars().map(|c| c as u8));
        }
        3 => {
            body.push(0);
            body.extend_from_slice(value.as_bytes());
        }
        _ => {
            // En UTF-16 le terminateur de la description fait deux octets.
            body.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x00]);
            body.extend_from_slice(&[0xFF, 0xFE]);
            for unit in value.encode_utf16() {
                body.extend_from_slice(&unit.to_le_bytes());
            }
        }
    }

    RawFrame {
        id: *b"COMM",
        flags: 0,
        body,
    }
}

// ─── Sérialisation ───────────────────────────────────────────────────────

fn encode(version: u8, frames: &[RawFrame]) -> Vec<u8> {
    let mut body = Vec::new();

    for frame in frames {
        body.extend_from_slice(&frame.id);
        let size = frame.body.len() as u32;
        if version == 4 {
            body.extend_from_slice(&write_synchsafe(size));
        } else {
            body.extend_from_slice(&size.to_be_bytes());
        }
        body.extend_from_slice(&frame.flags.to_be_bytes());
        body.extend_from_slice(&frame.body);
    }

    body.extend(std::iter::repeat(0u8).take(PADDING));

    let mut out = Vec::with_capacity(10 + body.len());
    out.extend_from_slice(b"ID3");
    out.push(version);
    out.push(0); // révision mineure
    out.push(0); // aucune option globale : ni désynchronisation, ni en-tête étendu
    out.extend_from_slice(&write_synchsafe(body.len() as u32));
    out.extend_from_slice(&body);
    out
}

/// Entier « synchsafe » : 7 bits utiles par octet, le bit de poids fort
/// toujours à zéro. C'est ce qui garantit qu'aucune séquence du tag ne peut
/// être confondue avec un début de trame audio.
fn read_synchsafe(bytes: &[u8]) -> u32 {
    ((bytes[0] as u32 & 0x7F) << 21)
        | ((bytes[1] as u32 & 0x7F) << 14)
        | ((bytes[2] as u32 & 0x7F) << 7)
        | (bytes[3] as u32 & 0x7F)
}

fn write_synchsafe(value: u32) -> [u8; 4] {
    [
        ((value >> 21) & 0x7F) as u8,
        ((value >> 14) & 0x7F) as u8,
        ((value >> 7) & 0x7F) as u8,
        (value & 0x7F) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::audio_metadata::tag_format::id3v2;
    use crate::entity::audio::audio_tags::ImageType;

    fn edit_title(value: &str) -> TagEdit {
        TagEdit {
            title: FieldEdit::Set(value.to_string()),
            ..Default::default()
        }
    }

    /// Relit le blob produit avec le parser de production : c'est la seule
    /// vérification qui compte vraiment — ce qu'on écrit doit être relisible
    /// par ce qui lit déjà les fichiers des utilisateurs.
    fn reparse(blob: &[u8]) -> crate::entity::audio::audio_tags::AudioTags {
        let tag = id3v2::parse(blob).expect("le blob produit doit être relisible");
        id3v2::to_audio_tags(&tag)
    }

    #[test]
    fn creates_a_tag_from_nothing() {
        let blob = rewrite(None, &edit_title("Life on Mars?")).unwrap();
        assert_eq!(&blob[0..3], b"ID3");
        assert_eq!(reparse(&blob).title.as_deref(), Some("Life on Mars?"));
    }

    #[test]
    fn round_trips_accented_text() {
        let blob = rewrite(None, &edit_title("Où sont les femmes ?")).unwrap();
        assert_eq!(reparse(&blob).title.as_deref(), Some("Où sont les femmes ?"));
    }

    #[test]
    fn round_trips_text_beyond_latin1() {
        // Bascule en UTF-16 (on écrit du 2.3 par défaut).
        let blob = rewrite(None, &edit_title("東京は夜の七時")).unwrap();
        assert_eq!(reparse(&blob).title.as_deref(), Some("東京は夜の七時"));
    }

    #[test]
    fn preserves_frames_it_does_not_understand() {
        // Une frame inconnue de l'encodeur ne doit pas disparaître : c'est la
        // garantie qui protège pochettes, notes et tags personnalisés.
        let first = rewrite(None, &edit_title("Titre")).unwrap();
        let (version, mut frames) = read_frames(&first).unwrap();
        frames.push(RawFrame {
            id: *b"ZZZZ",
            flags: 0,
            body: b"charge utile opaque".to_vec(),
        });
        let with_unknown = encode(version, &frames);

        let after = rewrite(Some(&with_unknown), &edit_title("Autre titre")).unwrap();

        let (_, after_frames) = read_frames(&after).unwrap();
        let kept = after_frames.iter().find(|f| &f.id == b"ZZZZ").unwrap();
        assert_eq!(kept.body, b"charge utile opaque");
        assert_eq!(reparse(&after).title.as_deref(), Some("Autre titre"));
    }

    #[test]
    fn replaces_instead_of_duplicating() {
        let first = rewrite(None, &edit_title("Version 1")).unwrap();
        let second = rewrite(Some(&first), &edit_title("Version 2")).unwrap();

        let (_, frames) = read_frames(&second).unwrap();
        let titles = frames.iter().filter(|f| &f.id == b"TIT2").count();
        assert_eq!(titles, 1, "le titre a été dupliqué");
        assert_eq!(reparse(&second).title.as_deref(), Some("Version 2"));
    }

    #[test]
    fn clearing_removes_the_frame() {
        let first = rewrite(None, &edit_title("À effacer")).unwrap();
        let cleared = rewrite(
            Some(&first),
            &TagEdit {
                title: FieldEdit::Clear,
                ..Default::default()
            },
        )
        .unwrap();

        let (_, frames) = read_frames(&cleared).unwrap();
        assert!(frames.iter().all(|f| &f.id != b"TIT2"));
    }

    #[test]
    fn editing_the_track_number_keeps_the_total() {
        let base = rewrite(
            None,
            &TagEdit {
                track_number: FieldEdit::Set(3),
                total_tracks: FieldEdit::Set(12),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(reparse(&base).track_number, Some(3));
        assert_eq!(reparse(&base).total_tracks, Some(12));

        // On ne change QUE le numéro : le total doit survivre.
        let edited = rewrite(
            Some(&base),
            &TagEdit {
                track_number: FieldEdit::Set(4),
                ..Default::default()
            },
        )
        .unwrap();

        let tags = reparse(&edited);
        assert_eq!(tags.track_number, Some(4));
        assert_eq!(tags.total_tracks, Some(12), "le total a été perdu");
    }

    #[test]
    fn an_empty_edit_still_produces_a_valid_tag() {
        let base = rewrite(None, &edit_title("Intact")).unwrap();
        let untouched = rewrite(Some(&base), &TagEdit::default()).unwrap();
        assert_eq!(reparse(&untouched).title.as_deref(), Some("Intact"));
    }

    #[test]
    fn synchsafe_round_trips() {
        for value in [0u32, 1, 127, 128, 255, 4096, 1_000_000, 0x0FFF_FFFF] {
            assert_eq!(read_synchsafe(&write_synchsafe(value)), value);
        }
    }

    #[test]
    fn refuses_a_desynchronised_tag_rather_than_risking_corruption() {
        let mut blob = rewrite(None, &edit_title("x")).unwrap();
        blob[5] |= 0b1000_0000; // marque le tag comme désynchronisé
        assert!(rewrite(Some(&blob), &edit_title("y")).is_err());
    }

    // ─── Images intégrées ────────────────────────────────────────────────
    //
    // Les octets utilisés en guise d'images ne sont pas de vraies images :
    // l'encodeur ne les interprète jamais, c'est `image_prep` qui valide et
    // prépare en amont. Des charges utiles reconnaissables rendent les échecs
    // beaucoup plus lisibles qu'un vrai JPEG.

    fn with_images(slots: Vec<ImageSlot>) -> TagEdit {
        TagEdit {
            images: ImagePlan::Replace(slots),
            ..Default::default()
        }
    }

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

    fn payloads(tags: &crate::entity::audio::audio_tags::AudioTags) -> Vec<Vec<u8>> {
        tags.attached_images
            .iter()
            .map(|i| i.image_data.clone())
            .collect()
    }

    #[test]
    fn embeds_a_new_cover() {
        let blob = rewrite(None, &with_images(vec![added(b"OCTETS-POCHETTE", 3)])).unwrap();

        let tags = reparse(&blob);
        assert_eq!(tags.attached_images.len(), 1);
        assert_eq!(tags.attached_images[0].image_data, b"OCTETS-POCHETTE");
        assert_eq!(tags.attached_images[0].mime_type, "image/jpeg");
        assert!(matches!(
            tags.attached_images[0].image_type,
            Some(ImageType::CoverFront)
        ));
    }

    #[test]
    fn promoting_an_image_does_not_re_encode_it() {
        // Le cœur du contrat : changer le rôle d'une image ne doit jamais la
        // faire repasser par un encodeur, donc jamais lui coûter de qualité.
        let base = rewrite(
            None,
            &with_images(vec![added(b"AAA-avant", 3), added(b"BBB-arriere", 4)]),
        )
        .unwrap();

        let promoted = rewrite(Some(&base), &with_images(vec![kept(b"BBB-arriere", 3)])).unwrap();

        let tags = reparse(&promoted);
        assert_eq!(tags.attached_images.len(), 1);
        assert_eq!(
            tags.attached_images[0].image_data, b"BBB-arriere",
            "les octets de l'image ont changé"
        );
        assert!(matches!(
            tags.attached_images[0].image_type,
            Some(ImageType::CoverFront)
        ));
    }

    #[test]
    fn the_order_of_the_slots_is_the_order_in_the_file() {
        let base = rewrite(
            None,
            &with_images(vec![added(b"un", 3), added(b"deux", 0), added(b"trois", 0)]),
        )
        .unwrap();

        let reordered = rewrite(
            Some(&base),
            &with_images(vec![kept(b"trois", 3), kept(b"un", 0), kept(b"deux", 0)]),
        )
        .unwrap();

        assert_eq!(
            payloads(&reparse(&reordered)),
            vec![b"trois".to_vec(), b"un".to_vec(), b"deux".to_vec()]
        );
    }

    #[test]
    fn omitting_a_slot_deletes_the_image() {
        let base = rewrite(None, &with_images(vec![added(b"garde", 3), added(b"jette", 0)])).unwrap();
        let after = rewrite(Some(&base), &with_images(vec![kept(b"garde", 3)])).unwrap();

        assert_eq!(payloads(&reparse(&after)), vec![b"garde".to_vec()]);
    }

    #[test]
    fn an_empty_list_removes_every_image() {
        let base = rewrite(None, &with_images(vec![added(b"a", 3), added(b"b", 0)])).unwrap();
        let after = rewrite(Some(&base), &with_images(Vec::new())).unwrap();

        assert!(reparse(&after).attached_images.is_empty());
    }

    #[test]
    fn only_the_first_front_cover_keeps_its_type() {
        // Deux images de type 3 rendraient l'affichage imprévisible : chaque
        // lecteur choisirait la sienne.
        let blob = rewrite(None, &with_images(vec![added(b"a", 3), added(b"b", 3)])).unwrap();

        let tags = reparse(&blob);
        let covers = tags
            .attached_images
            .iter()
            .filter(|i| matches!(i.image_type, Some(ImageType::CoverFront)))
            .count();
        assert_eq!(covers, 1, "deux pochettes avant coexistent");
        assert_eq!(tags.attached_images.len(), 2, "la seconde image a disparu");
    }

    #[test]
    fn an_unknown_image_reference_is_refused() {
        // Silencieusement ignorée, elle disparaîtrait de l'enregistrement et
        // l'utilisateur perdrait une image en croyant l'avoir déplacée.
        let base = rewrite(None, &with_images(vec![added(b"a", 3)])).unwrap();

        let result = rewrite(
            Some(&base),
            &with_images(vec![ImageSlot {
                source: ImageSource::Existing("0123456789abcdef".into()),
                picture_type: 3,
                description: String::new(),
            }]),
        );

        assert!(result.is_err(), "référence inconnue acceptée en silence");
    }

    #[test]
    fn editing_text_leaves_the_images_untouched() {
        let base = rewrite(None, &with_images(vec![added(b"pochette", 3)])).unwrap();
        let after = rewrite(Some(&base), &edit_title("Nouveau titre")).unwrap();

        let tags = reparse(&after);
        assert_eq!(payloads(&tags), vec![b"pochette".to_vec()]);
        assert_eq!(tags.title.as_deref(), Some("Nouveau titre"));
    }

    #[test]
    fn round_trips_a_description_beyond_latin1() {
        let blob = rewrite(
            None,
            &with_images(vec![ImageSlot {
                source: ImageSource::New {
                    data: b"CONTENU-IMAGE".to_vec(),
                    mime_type: "image/png".into(),
                    width: 600,
                    height: 600,
                },
                picture_type: 3,
                description: "ジャケット".into(),
            }]),
        )
        .unwrap();

        let tags = reparse(&blob);
        assert_eq!(
            tags.attached_images[0].description.as_deref(),
            Some("ジャケット")
        );
        assert_eq!(tags.attached_images[0].image_data, b"CONTENU-IMAGE");
    }

    #[test]
    fn finds_the_image_data_after_a_utf16_description() {
        // Le terminateur d'une description UTF-16 fait DEUX octets. Le chercher
        // comme un octet nul isolé s'arrêterait au milieu du premier caractère
        // ASCII venu et ferait commencer l'image au mauvais endroit — l'image
        // conservée serait alors silencieusement corrompue.
        let base = rewrite(
            None,
            &with_images(vec![ImageSlot {
                source: ImageSource::New {
                    data: b"CONTENU-IMAGE".to_vec(),
                    mime_type: "image/png".into(),
                    width: 600,
                    height: 600,
                },
                picture_type: 3,
                description: "Aジャケット".into(),
            }]),
        )
        .unwrap();

        // Relecture par NOTRE décodeur, puis réécriture.
        let again = rewrite(
            Some(&base),
            &with_images(vec![kept(b"CONTENU-IMAGE", 0)]),
        )
        .unwrap();

        assert_eq!(payloads(&reparse(&again)), vec![b"CONTENU-IMAGE".to_vec()]);
    }
}
