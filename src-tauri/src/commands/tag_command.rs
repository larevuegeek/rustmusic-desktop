//! Commandes Tauri d'édition des métadonnées.
//!
//! # Enchaînement
//! 1. Écriture dans le fichier (`audio_metadata::injector`), de façon atomique.
//! 2. Réanalyse du fichier par la **logique d'import existante**.
//!
//! Le second point mérite d'être expliqué, parce qu'il évite un piège. Après
//! avoir corrigé un tag, la base contient encore l'ancienne valeur. On pourrait
//! être tenté de mettre à jour la ligne `library_tracks` directement — c'est
//! instantané, mais faux dès qu'on touche à l'artiste ou à l'album : ce sont
//! des tables séparées, et renommer un artiste peut vouloir dire en créer un
//! nouveau, en retrouver un existant, ou laisser un orphelin derrière soi.
//!
//! Toute cette logique est déjà écrite et éprouvée dans
//! `save_track_to_library`. On la rejoue donc sur le seul fichier édité.
//! Elle est même déjà taillée pour ça : elle ne saute la réanalyse que si le
//! fichier est indexé **et** que sa date de modification n'a pas bougé — or
//! écrire un tag change cette date. Elle réanalyse donc d'elle-même, sans
//! créer de doublon (l'insertion du fichier est un upsert).

use base64::engine::general_purpose;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::core::audio_metadata::injector::edit::{
    FieldEdit, ImagePlan, ImageSlot, ImageSource, TagEdit,
};
use crate::core::audio_metadata::injector::image_prep;
use crate::core::audio_metadata::injector::injector;
use crate::repository::library::library_files_repository::LibraryFilesRepository;
use crate::service::library::library_service::{create_context, save_track_to_library};
use crate::state::AppState;
use crate::entity::audio::audio_tags::ImageType;
use crate::mapper::library::track::track_list_item_view::TrackListView;

/// Ce que le frontend envoie.
///
/// Convention, identique pour tous les champs : **absent** signifie « ne
/// touche pas », **chaîne vide** signifie « efface ». Sans elle, effacer un
/// tag depuis un formulaire serait impossible à exprimer.
#[derive(Debug, Deserialize)]
pub struct TagEditPayload {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub comment: Option<String>,
    pub composer: Option<String>,
    pub track_number: Option<String>,
    pub total_tracks: Option<String>,
    pub disc_number: Option<String>,
    pub total_discs: Option<String>,
    /// Liste **finale** des images voulues, ou `None` pour ne pas y toucher.
    /// Voir `ImagePlan` : décrire l'état visé couvre les cinq gestes de
    /// l'interface en une seule écriture.
    pub images: Option<Vec<ImageSlotPayload>>,
}

/// Une image de la liste finale, telle que l'interface la décrit.
///
/// `id` et `path` s'excluent : l'un désigne une image déjà dans le fichier,
/// l'autre une image à intégrer depuis le disque. `path` plutôt que les octets
/// eux-mêmes — faire transiter plusieurs mégaoctets encodés en base64 par le
/// pont IPC à chaque enregistrement serait absurde quand Rust peut lire le
/// fichier directement.
#[derive(Debug, Deserialize)]
pub struct ImageSlotPayload {
    pub id: Option<String>,
    pub path: Option<String>,
    /// Type ID3 : 3 = pochette avant, 4 = pochette arrière, 0 = autre…
    pub picture_type: u8,
    pub description: Option<String>,
}

impl TryFrom<ImageSlotPayload> for ImageSlot {
    type Error = String;

    fn try_from(p: ImageSlotPayload) -> Result<Self, Self::Error> {
        let description = p.description.unwrap_or_default();

        let source = match (p.id, p.path) {
            (Some(id), _) => ImageSource::Existing(id),
            (None, Some(path)) => {
                let bytes = std::fs::read(&path)
                    .map_err(|e| format!("Lecture de l'image {path} : {e}"))?;
                // La préparation n'a lieu qu'ici, à l'ajout ou au remplacement.
                // Une image conservée est recopiée telle quelle, ce qui évite
                // de la réencoder — et de la dégrader — à chaque correction de
                // titre. Voir `image_prep`.
                let prepared = image_prep::prepare(&bytes).map_err(|e| e.to_string())?;
                ImageSource::New {
                    data: prepared.data,
                    mime_type: prepared.mime_type,
                    width: prepared.width,
                    height: prepared.height,
                }
            }
            (None, None) => {
                return Err("Image sans origine : ni identifiant, ni chemin".into())
            }
        };

        Ok(ImageSlot {
            source,
            picture_type: p.picture_type,
            description,
        })
    }
}

/// Traduit un champ numérique : vide = effacer, non numérique = ignorer.
///
/// Ignorer plutôt que rejeter : une saisie incomplète ne doit pas faire échouer
/// l'enregistrement des autres champs corrigés au même moment.
fn numeric_field(value: Option<String>) -> FieldEdit<u16> {
    match value {
        None => FieldEdit::Keep,
        Some(v) if v.trim().is_empty() => FieldEdit::Clear,
        Some(v) => match v.trim().parse::<u16>() {
            Ok(n) => FieldEdit::Set(n),
            Err(_) => FieldEdit::Keep,
        },
    }
}

impl TryFrom<TagEditPayload> for TagEdit {
    // `TryFrom` et non `From` : intégrer une image peut échouer (fichier
    // illisible, format non reconnu). Échouer à la conversion garantit qu'on
    // n'écrit rien du tout — mieux vaut refuser l'enregistrement entier que
    // d'écrire les textes en laissant tomber l'image en silence.
    type Error = String;

    fn try_from(p: TagEditPayload) -> Result<Self, Self::Error> {
        let images = match p.images {
            None => ImagePlan::Keep,
            Some(slots) => ImagePlan::Replace(
                slots
                    .into_iter()
                    .map(ImageSlot::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        };

        Ok(TagEdit {
            title: FieldEdit::from_optional(p.title),
            artist: FieldEdit::from_optional(p.artist),
            album: FieldEdit::from_optional(p.album),
            album_artist: FieldEdit::from_optional(p.album_artist),
            year: FieldEdit::from_optional(p.year),
            genre: FieldEdit::from_optional(p.genre),
            comment: FieldEdit::from_optional(p.comment),
            composer: FieldEdit::from_optional(p.composer),
            track_number: numeric_field(p.track_number),
            total_tracks: numeric_field(p.total_tracks),
            disc_number: numeric_field(p.disc_number),
            total_discs: numeric_field(p.total_discs),
            images,
        })
    }
}

/// Les tags tels qu'ils sont **dans le fichier**, limités aux champs éditables.
///
/// Ne pas confondre avec la vue de la bibliothèque : celle-ci est une
/// projection de la base, qui ne stocke qu'une partie des tags (ni commentaire,
/// ni compositeur, ni totaux). Un éditeur qui s'appuierait dessus afficherait
/// des champs vides pour tout le reste, et les effacerait à l'enregistrement.
#[derive(Debug, Serialize, Default)]
pub struct EditableTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub comment: Option<String>,
    pub composer: Option<String>,
    pub track_number: Option<u16>,
    pub total_tracks: Option<u16>,
    pub disc_number: Option<u16>,
    pub total_discs: Option<u16>,
}

/// Relit les tags éditables directement depuis le fichier.
#[tauri::command]
pub fn read_track_tags(path: String) -> Result<EditableTags, String> {
    let file = crate::core::audio_analyser::audio_analyser::AudioAnalyser::analyse_audio_file(
        &std::path::PathBuf::from(&path),
    )
    .map_err(|e| e.to_string())?;
    let t = file.tags;

    Ok(EditableTags {
        title: t.title,
        artist: t.artist,
        album: t.album,
        album_artist: t.album_artist,
        year: t.year.map(|y| y.to_string()),
        genre: t.genre,
        comment: t.comment,
        composer: t.composer,
        track_number: t.track_number,
        total_tracks: t.total_tracks,
        disc_number: t.disc_number,
        total_discs: t.total_discs,
    })
}

/// Une image intégrée au fichier.
///
/// # Ce que « pochette » veut dire ici
/// L'ID3v2 n'a pas de notion de « première image » : chaque image porte un
/// **type** parmi 21 (pochette avant, pochette arrière, livret, photo
/// d'artiste…), et la pochette est celle de type **3**. Une image n'est donc
/// pas la pochette parce qu'elle arrive en premier, mais parce qu'elle est
/// typée ainsi — et il ne doit y en avoir qu'une.
///
/// L'ordre, lui, est simplement l'ordre des images dans le fichier. Il n'existe
/// aucun champ de position : réordonner signifie déplacer les données.
#[derive(Debug, Serialize)]
pub struct EmbeddedImage {
    /// Identifiant tiré du **contenu** de l'image. C'est par lui que
    /// l'interface la redésigne lors d'une réécriture : une position se
    /// décalerait au premier réordonnancement, un contenu non.
    pub id: String,
    /// Position dans le fichier, à partir de 0. C'est l'ordre d'affichage.
    pub index: usize,
    /// Type d'image, tel que nommé par notre parser (`CoverFront`, `Artist`…).
    pub kind: String,
    /// Le même type, sous sa forme numérique ID3 — c'est celle qu'il faut
    /// renvoyer à l'écriture, et la seule que la spécification définit.
    pub picture_type: u8,
    /// Vrai pour la pochette avant — celle que l'interface met en avant.
    pub is_cover: bool,
    pub mime_type: String,
    pub description: Option<String>,
    /// Taille des données décodées, pour affichage.
    pub bytes: usize,
    /// Data URI, directement affichable par l'interface.
    pub src: String,
}

/// Liste les images intégrées au fichier, dans leur ordre d'apparition.
#[tauri::command]
pub fn read_track_images(path: String) -> Result<Vec<EmbeddedImage>, String> {
    let file = crate::core::audio_analyser::audio_analyser::AudioAnalyser::analyse_audio_file(
        &std::path::PathBuf::from(&path),
    )
    .map_err(|e| e.to_string())?;

    Ok(file
        .tags
        .attached_images
        .into_iter()
        .enumerate()
        .map(|(index, img)| {
            let image_type = img.image_type.unwrap_or(ImageType::Other);
            // Les variantes de `ImageType` sont déclarées dans l'ordre exact de
            // la spécification ID3 : leur rang EST le code du type d'image.
            let picture_type = image_type.clone() as u8;
            let kind = format!("{image_type:?}");
            // La taille utile est celle des octets décodés ; `src` est une data
            // URI en base64, donc environ un tiers plus lourde.
            let bytes = img.image_data.len();
            EmbeddedImage {
                id: image_prep::image_id(&img.image_data),
                index,
                is_cover: picture_type == 3,
                kind,
                picture_type,
                mime_type: img.mime_type,
                description: img.description.filter(|d| !d.trim().is_empty()),
                bytes,
                src: img.image_src,
            }
        })
        .collect())
}

/// Une image choisie sur le disque, préparée et prête à être montrée.
#[derive(Debug, Serialize)]
pub struct PreparedImageView {
    /// Data URI de l'image **telle qu'elle sera intégrée**, pas de l'original.
    pub src: String,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    /// Poids après préparation.
    pub bytes: usize,
    pub original_bytes: usize,
    /// Vrai si l'image a été redimensionnée ou recompressée.
    pub recompressed: bool,
}

/// Prépare une image choisie par l'utilisateur, pour aperçu avant validation.
///
/// L'aperçu montre le résultat réel : `image_prep::prepare` est déterministe,
/// donc l'image affichée ici est exactement celle qui sera écrite dans le
/// fichier à l'enregistrement. Sans cet aller-retour, l'utilisateur validerait
/// à l'aveugle une recompression qu'il ne peut pas défaire.
#[tauri::command]
pub fn prepare_image(path: String) -> Result<PreparedImageView, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("Lecture de l'image : {e}"))?;
    let prepared = image_prep::prepare(&bytes).map_err(|e| e.to_string())?;

    let encoded = general_purpose::STANDARD.encode(&prepared.data);
    Ok(PreparedImageView {
        src: format!("data:{};base64,{}", prepared.mime_type, encoded),
        mime_type: prepared.mime_type,
        width: prepared.width,
        height: prepared.height,
        bytes: prepared.data.len(),
        original_bytes: prepared.original_bytes,
        recompressed: prepared.recompressed,
    })
}

/// Le fichier accepte-t-il une réécriture de ses tags ?
///
/// Permet à l'interface de ne pas proposer « Modifier les tags » sur un
/// fichier qu'on ne saurait pas réenregistrer.
#[tauri::command]
pub fn can_write_tags(path: String) -> bool {
    injector::is_writable(std::path::Path::new(&path))
}

/// Écrit les tags puis resynchronise la bibliothèque.
///
/// Retourne la vue du morceau remise à jour, pour que l'interface se rafraîchisse
/// sans recharger toute la bibliothèque.
#[tauri::command]
pub async fn write_track_tags(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
    edit: TagEditPayload,
) -> Result<TrackListView, String> {
    let file_path = std::path::PathBuf::from(&path);
    // La conversion prépare les images ajoutées : elle échoue AVANT toute
    // écriture si l'une d'elles est illisible.
    let tag_edit = TagEdit::try_from(edit)?;

    // ─── 1. Écriture dans le fichier ───
    // En cas d'échec on s'arrête ici : la base ne doit jamais décrire un
    // contenu que le fichier ne porte pas.
    injector::apply(&file_path, &tag_edit).map_err(|e| e.to_string())?;

    // ─── 2. Retrouver la bibliothèque d'origine ───
    // On repart du chemin, seule information dont dispose l'appelant.
    let library_file = LibraryFilesRepository::find_by_path_any(&state.pool, &path)
        .await
        .map_err(|e| format!("Lecture du fichier en base : {e}"))?
        .ok_or_else(|| format!("Fichier absent de la bibliothèque : {path}"))?;

    // ─── 3. Rejouer l'import sur ce seul fichier ───
    // `library_dir_id` reste à `None` : l'insertion est un upsert qui renvoie
    // la ligne existante sans l'écraser, donc le rattachement au dossier
    // d'origine est préservé.
    let ctx = create_context(app, &state.pool);
    let view = save_track_to_library(&ctx, library_file.library_id, None, path.clone())
        .await
        .map_err(|e| format!("Resynchronisation de la bibliothèque : {e}"))?;

    log::info!("🏷  Tags écrits et bibliothèque resynchronisée : {path}");
    Ok(view)
}
