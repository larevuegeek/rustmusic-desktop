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
use tauri::{Emitter, Manager, State};

use crate::core::batch::runner as batch_runner;

use crate::core::audio_metadata::injector::edit::{
    FieldEdit, ImagePlan, ImageSlot, ImageSource, TagEdit,
};
use crate::core::audio_metadata::injector::image_prep;
use crate::core::audio_metadata::injector::injector;
use crate::repository::library::library_files_repository::LibraryFilesRepository;
use crate::service::library::library_service::{create_context, save_track_to_library};
use crate::service::library::move_service;
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
    /// Pochette commune à poser **sans toucher aux autres images**.
    ///
    /// S'exclut de `images` : l'un décrit la liste entière, l'autre n'en
    /// remplace qu'un élément. C'est la forme qu'utilise l'édition multiple,
    /// où l'on ignore ce que contiennent les autres fichiers.
    pub cover: Option<ImageSlotPayload>,
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
        let images = match (p.images, p.cover) {
            (None, None) => ImagePlan::Keep,
            (Some(slots), None) => ImagePlan::Replace(
                slots
                    .into_iter()
                    .map(ImageSlot::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            (None, Some(cover)) => ImagePlan::SetCover(ImageSlot::try_from(cover)?),
            // Les deux ensemble n'ont pas de sens : la liste finale contient
            // déjà sa pochette. Accepter reviendrait à deviner laquelle prime.
            (Some(_), Some(_)) => {
                return Err(
                    "Charge utile incohérente : liste d'images et pochette commune \
                     ne peuvent pas être envoyées ensemble."
                        .to_string(),
                )
            }
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
#[derive(Debug, Serialize, Deserialize, Default)]
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

    Ok(to_editable(file.tags))
}

/// Ne garde d'un jeu de tags que ce qui est éditable.
fn to_editable(t: crate::entity::audio::audio_tags::AudioTags) -> EditableTags {
    EditableTags {
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
    }
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

/// Un morceau tel que l'atelier a besoin de le connaître.
#[derive(Debug, Serialize)]
pub struct WorkshopTrack {
    pub path: String,
    pub tags: EditableTags,
    /// Vignette de la pochette en data URI, `None` si le fichier n'en a pas.
    pub cover: Option<String>,
    /// Faux quand le fichier n'a pas pu être lu.
    pub readable: bool,
    /// Le format sait-il recevoir une réécriture de ses tags ?
    ///
    /// Rendu ici plutôt que par `can_write_tags` appelé fichier par fichier :
    /// cent appels séparés à travers le pont IPC coûtaient bien plus que la
    /// vérification elle-même, qui ne lit que quatre octets.
    pub writable: bool,
}

/// Côté de la vignette, en pixels. De quoi reconnaître une pochette dans une
/// liste, pas de quoi l'examiner — la loupe sert à ça.
const WORKSHOP_THUMB_EDGE: u32 = 96;

/// Lit tout ce dont l'atelier a besoin, en **une seule passe par fichier**.
///
/// # Pourquoi une commande dédiée
/// L'atelier appelait `read_track_tags` par fichier. Or cette analyse extrait
/// **et encode en base64** toutes les images intégrées au passage — puis
/// l'atelier les jetait. Sur cent morceaux, c'était des dizaines de mégaoctets
/// décodés, encodés et abandonnés, plus cent allers-retours par le pont IPC.
///
/// Ici l'analyse a lieu une fois et sert aux deux : les champs éditables, et
/// une vignette réduite de la pochette.
#[tauri::command]
pub async fn read_workshop_tracks(paths: Vec<String>) -> Result<Vec<WorkshopTrack>, String> {
    tokio::task::spawn_blocking(move || {
        use rayon::prelude::*;

        // En parallèle : chaque fichier est indépendant, et l'essentiel du
        // temps part en lecture — sur un partage réseau, attendre un fichier
        // à la fois multiplie la latence par leur nombre. `par_iter` conserve
        // l'ordre de la liste, dont dépend la numérotation.
        paths
            .into_par_iter()
            .map(|path| {
                let analysed = crate::core::audio_analyser::audio_analyser::AudioAnalyser::
                    analyse_audio_file(&std::path::PathBuf::from(&path));

                let writable = injector::is_writable(std::path::Path::new(&path));

                let Ok(file) = analysed else {
                    // Un fichier illisible reste dans la liste : le faire
                    // disparaître laisserait croire qu'il a été traité.
                    return WorkshopTrack {
                        path,
                        tags: EditableTags::default(),
                        cover: None,
                        readable: false,
                        writable: false,
                    };
                };

                let cover = file
                    .tags
                    .attached_images
                    .iter()
                    // La pochette est celle typée comme telle ; à défaut, la
                    // première image donne un aperçu utilisable.
                    .find(|img| matches!(img.image_type, Some(ImageType::CoverFront)))
                    .or_else(|| file.tags.attached_images.first())
                    .and_then(|img| {
                        image_prep::thumbnail(&img.image_data, WORKSHOP_THUMB_EDGE)
                            .ok()
                            .map(|small| {
                                format!(
                                    "data:image/jpeg;base64,{}",
                                    general_purpose::STANDARD.encode(&small)
                                )
                            })
                    });

                WorkshopTrack {
                    path,
                    tags: to_editable(file.tags),
                    cover,
                    readable: true,
                    writable,
                }
            })
            .collect()
    })
    .await
    .map_err(|e| format!("Lecture de l'atelier interrompue : {e}"))
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

/// Une image téléchargée, prête à rejoindre la liste des médias.
#[derive(Debug, Serialize)]
pub struct DownloadedImageView {
    /// Chemin du fichier temporaire contenant les octets **d'origine**.
    ///
    /// C'est lui qu'on renverra dans `ImageSlotPayload.path` : l'écriture
    /// relit et prépare elle-même, exactement comme pour une image du disque.
    /// Faire transiter les octets par le pont IPC dans les deux sens n'aurait
    /// aucun intérêt.
    pub path: String,
    pub src: String,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub bytes: usize,
    pub original_bytes: usize,
    pub recompressed: bool,
}

/// Poids maximal accepté pour une image téléchargée.
///
/// Une pochette dépasse rarement 2 Mo. La borne est là pour qu'une URL
/// inattendue ne remplisse pas le disque temporaire.
const MAX_DOWNLOAD_BYTES: usize = 20 * 1024 * 1024;

/// Devine l'extension d'une image à ses premiers octets.
///
/// Le nom du fichier temporaire n'a aucune valeur fonctionnelle — la
/// préparation lit les octets, jamais l'extension — mais un `.jpg` qui
/// contient du PNG est le genre de détail qui fait perdre une heure plus tard.
fn image_extension(bytes: &[u8]) -> &'static str {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => "png",
        [0xFF, 0xD8, ..] => "jpg",
        [b'G', b'I', b'F', ..] => "gif",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "webp",
        _ => "img",
    }
}

/// Télécharge une image et l'apprête, pour aperçu avant validation.
///
/// Le pendant distant de `prepare_image`, pour la pochette que propose une
/// source en ligne. Comme lui, il montre le résultat réel : on ne valide pas à
/// l'aveugle une image qu'on n'a pas vue à côté de celle qu'elle remplace.
#[tauri::command]
pub async fn prepare_image_from_url(url: String) -> Result<DownloadedImageView, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Client HTTP : {e}"))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Téléchargement de l'image : {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Téléchargement de l'image : {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Lecture de l'image : {e}"))?
        .to_vec();

    if bytes.is_empty() {
        return Err("Image vide.".to_string());
    }
    if bytes.len() > MAX_DOWNLOAD_BYTES {
        return Err(format!(
            "Image trop lourde ({} Mo).",
            bytes.len() / (1024 * 1024)
        ));
    }

    // Redimensionner et réencoder est du calcul pur : sur la boucle asynchrone,
    // une pochette de 1000 px bloquerait tout le reste le temps du traitement.
    let prepared = tokio::task::spawn_blocking({
        let bytes = bytes.clone();
        move || image_prep::prepare(&bytes).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Préparation de l'image : {e}"))??;

    // Nom déterministe : redemander deux fois la même pochette réécrit le même
    // fichier au lieu d'en semer un nouveau à chaque clic.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&url, &mut hasher);
    let name = format!(
        "rustmusic-cover-{:016x}.{}",
        std::hash::Hasher::finish(&hasher),
        image_extension(&bytes)
    );
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, &bytes)
        .map_err(|e| format!("Écriture du fichier temporaire : {e}"))?;

    let encoded = general_purpose::STANDARD.encode(&prepared.data);
    Ok(DownloadedImageView {
        path: path.to_string_lossy().to_string(),
        src: format!("data:{};base64,{}", prepared.mime_type, encoded),
        mime_type: prepared.mime_type,
        width: prepared.width,
        height: prepared.height,
        bytes: prepared.data.len(),
        original_bytes: prepared.original_bytes,
        recompressed: prepared.recompressed,
    })
}

/// Photographie les tags d'un lot avant de le réécrire.
///
/// Rendue à part parce que les deux commandes de lot en ont besoin, et parce
/// que la lecture est bloquante : cinq cents fichiers sur un partage réseau
/// n'ont rien à faire sur la boucle asynchrone.
async fn snapshot_tags(
    pool: &sqlx::SqlitePool,
    library_id: Option<i64>,
    paths: &[String],
) -> Option<String> {
    let list = paths.to_vec();
    let snapshots = tokio::task::spawn_blocking(move || {
        use rayon::prelude::*;
        list.par_iter()
            .map(|path| {
                let blob = read_track_tags(path.clone())
                    .ok()
                    .and_then(|tags| serde_json::to_string(&tags).ok());
                (path.clone(), blob)
            })
            .collect::<Vec<_>>()
    })
    .await
    .ok()?;

    let map: std::collections::HashMap<String, Option<String>> = snapshots.into_iter().collect();
    move_service::journal_tags(pool, library_id, paths, |path| {
        map.get(path).cloned().flatten()
    })
    .await
    .ok()
}

/// Réécrit dans un fichier les tags que le journal a conservés.
///
/// Chaque champ est explicitement posé — `Set` s'il avait une valeur, `Clear`
/// s'il était vide. Omettre les champs vides voudrait dire « ne touche pas »,
/// et l'annulation laisserait en place ce que le lot avait ajouté.
///
/// Les images ne sont pas restaurées : le journal ne les conserve pas. Une
/// pochette pèse plusieurs centaines de kilooctets, et cinquante lots de cinq
/// cents fichiers en feraient une base plus lourde que la bibliothèque. C'est
/// dit à l'utilisateur plutôt que promis à tort.
pub async fn restore_tags(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    path: &str,
    blob: &str,
) -> Result<(), String> {
    let tags: EditableTags =
        serde_json::from_str(blob).map_err(|e| format!("Journal illisible : {e}"))?;

    let field = |value: Option<String>| match value {
        Some(v) => FieldEdit::Set(v),
        None => FieldEdit::Clear,
    };
    let number = |value: Option<u16>| match value {
        Some(v) => FieldEdit::Set(v),
        None => FieldEdit::Clear,
    };

    let edit = TagEdit {
        title: field(tags.title),
        artist: field(tags.artist),
        album: field(tags.album),
        album_artist: field(tags.album_artist),
        year: field(tags.year),
        genre: field(tags.genre),
        comment: field(tags.comment),
        composer: field(tags.composer),
        track_number: number(tags.track_number),
        total_tracks: number(tags.total_tracks),
        disc_number: number(tags.disc_number),
        total_discs: number(tags.total_discs),
        images: ImagePlan::Keep,
    };

    let target = path.to_string();
    tokio::task::spawn_blocking(move || {
        injector::apply(std::path::Path::new(&target), &edit).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Restauration interrompue : {e}"))??;

    // La base décrit encore l'état corrigé : on rejoue l'import, comme après
    // n'importe quelle écriture.
    if let Ok(Some(file)) = LibraryFilesRepository::find_by_path_any(pool, path).await {
        let ctx = create_context(app.clone(), pool);
        let _ = save_track_to_library(&ctx, file.library_id, None, path.to_string()).await;
    }
    Ok(())
}

/// Applique le **même** jeu de modifications à plusieurs fichiers.
///
/// Rend la main tout de suite avec l'identifiant du lot : le traitement se
/// poursuit en fond et rend compte par les événements `batch-progress` puis
/// `batch-done`. Attendre la fin bloquerait l'interface pendant des minutes
/// sur une bibliothèque réseau.
///
/// L'annulation passe par `cancel_batch` avec cet identifiant.
#[tauri::command]
pub async fn write_tags_batch(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
    edit: TagEditPayload,
) -> Result<String, String> {
    // La préparation a lieu **une seule fois** pour tout le lot. La refaire par
    // fichier redécoderait et réencoderait la même image cinq cents fois.
    let tag_edit = prepare_shared_edit(edit).await?;

    // L'instantané est pris **avant** que rien ne soit écrit : c'est lui qui
    // rend le lot annulable, et le prendre après n'aurait aucun sens.
    let _ = snapshot_tags(&state.pool, None, &paths).await;

    let job_id = uuid::Uuid::new_v4().to_string();
    let cancel = state.batch.start(&job_id);
    let pool = state.pool.clone();

    let id = job_id.clone();
    let app_task = app.clone();
    tauri::async_runtime::spawn(async move {
        let app_progress = app_task.clone();
        let total = paths.len();

        let report = batch_runner::run(
            id.clone(),
            cancel,
            paths,
            batch_runner::DEFAULT_CONCURRENCY,
            move |path| {
                let edit = tag_edit.clone();
                let pool = pool.clone();
                let app = app_task.clone();
                async move { write_and_resync(app, pool, path, edit).await }
            },
            |progress| {
                let _ = app_progress.emit("batch-progress", progress);
            },
        )
        .await;

        log::info!(
            "🏷  Lot terminé : {}/{} réussis, {} échecs{}",
            report.succeeded,
            total,
            report.failures.len(),
            if report.cancelled { ", annulé" } else { "" }
        );

        let _ = app_progress.emit("batch-done", &report);
        app_progress.state::<AppState>().batch.finish(&id);
    });

    Ok(job_id)
}

/// Un fichier et les modifications qui lui sont propres.
#[derive(Debug, Deserialize)]
pub struct TagEditItem {
    pub path: String,
    pub edit: TagEditPayload,
}

/// Applique à chaque fichier des modifications **différentes**.
///
/// Complète `write_tags_batch`, qui applique le même jeu à tous. Certaines
/// corrections ne peuvent pas être communes : numéroter les pistes d'un album
/// de 1 à N donne par définition une valeur par fichier.
///
/// Le coût à connaître : chaque élément prépare ses propres images. C'est sans
/// conséquence pour une numérotation, qui n'en comporte pas, et acceptable pour
/// un album d'une douzaine de titres — mais ce n'est pas la commande à choisir
/// pour poser une pochette sur cinq cents fichiers. `write_tags_batch` la
/// prépare une seule fois.
#[tauri::command]
pub async fn write_tags_each(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    items: Vec<TagEditItem>,
) -> Result<String, String> {
    let paths: Vec<String> = items.iter().map(|item| item.path.clone()).collect();

    let prepared: std::collections::HashMap<String, TagEdit> =
        tokio::task::spawn_blocking(move || {
            items
                .into_iter()
                .map(|item| TagEdit::try_from(item.edit).map(|edit| (item.path, edit)))
                .collect::<Result<std::collections::HashMap<_, _>, String>>()
        })
        .await
        .map_err(|e| format!("Préparation du lot interrompue : {e}"))??;

    let _ = snapshot_tags(&state.pool, None, &paths).await;

    let job_id = uuid::Uuid::new_v4().to_string();
    let cancel = state.batch.start(&job_id);
    let pool = state.pool.clone();
    let prepared = std::sync::Arc::new(prepared);

    let id = job_id.clone();
    let app_task = app.clone();
    tauri::async_runtime::spawn(async move {
        let app_progress = app_task.clone();
        let total = paths.len();

        let report = batch_runner::run(
            id.clone(),
            cancel,
            paths,
            batch_runner::DEFAULT_CONCURRENCY,
            move |path| {
                let prepared = prepared.clone();
                let pool = pool.clone();
                let app = app_task.clone();
                async move {
                    // Un chemin sans modification associée ne peut venir que
                    // d'une incohérence de l'appelant : on le signale plutôt
                    // que de le passer sous silence.
                    let edit = prepared
                        .get(&path)
                        .cloned()
                        .ok_or_else(|| "Aucune modification pour ce fichier".to_string())?;
                    write_and_resync(app, pool, path, edit).await
                }
            },
            |progress| {
                let _ = app_progress.emit("batch-progress", progress);
            },
        )
        .await;

        log::info!(
            "🏷  Lot individualisé terminé : {}/{} réussis, {} échecs{}",
            report.succeeded,
            total,
            report.failures.len(),
            if report.cancelled { ", annulé" } else { "" }
        );

        let _ = app_progress.emit("batch-done", &report);
        app_progress.state::<AppState>().batch.finish(&id);
    });

    Ok(job_id)
}

/// Convertit la charge utile une fois pour tout le lot.
///
/// Refuse une image « déjà présente » : cette référence désigne le contenu
/// d'**un** fichier précis et n'a aucun sens appliquée à cinquante autres.
/// Mieux vaut refuser que d'écrire un résultat imprévisible.
async fn prepare_shared_edit(edit: TagEditPayload) -> Result<TagEdit, String> {
    tokio::task::spawn_blocking(move || {
        // Une image « déjà présente » est désignée par son contenu dans UN
        // fichier : la référence ne veut rien dire appliquée aux quarante-neuf
        // autres. Vrai pour la liste comme pour la pochette commune.
        let references_existing = edit
            .images
            .iter()
            .flatten()
            .chain(edit.cover.iter())
            .any(|slot| slot.id.is_some());

        if references_existing {
            return Err(
                "Sur plusieurs fichiers, seules de nouvelles images peuvent être \
                 appliquées : une image déjà présente appartient à un seul fichier."
                    .to_string(),
            );
        }

        TagEdit::try_from(edit)
    })
    .await
    .map_err(|e| format!("Préparation du lot interrompue : {e}"))?
}

/// Écrit un fichier puis remet la base en accord avec lui.
///
/// Les deux vont ensemble : une écriture réussie dont la resynchronisation
/// échoue laisse la bibliothèque en désaccord avec le disque, ce qui compte
/// comme un échec pour l'utilisateur.
async fn write_and_resync(
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    path: String,
    edit: TagEdit,
) -> Result<(), String> {
    let target = path.clone();
    tokio::task::spawn_blocking(move || {
        injector::apply(std::path::Path::new(&target), &edit).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Tâche d'écriture interrompue : {e}"))??;

    let library_file = LibraryFilesRepository::find_by_path_any(&pool, &path)
        .await
        .map_err(|e| format!("Lecture du fichier en base : {e}"))?
        .ok_or_else(|| format!("Fichier absent de la bibliothèque : {path}"))?;

    let ctx = create_context(app, &pool);
    save_track_to_library(&ctx, library_file.library_id, None, path)
        .await
        .map_err(|e| format!("Resynchronisation de la bibliothèque : {e}"))?;

    Ok(())
}

/// Prépare puis écrit les tags, hors du runtime async.
///
/// Isolée parce que le traitement par lot en a besoin autant que l'édition
/// d'un fichier : les deux doivent tenir le même contrat — préparation des
/// images et écriture sur un fil bloquant, échec avant toute écriture si une
/// image est illisible.
pub async fn write_tags_blocking(path: String, edit: TagEditPayload) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        // La conversion prépare les images ajoutées : elle échoue AVANT toute
        // écriture si l'une d'elles est illisible.
        let tag_edit = TagEdit::try_from(edit)?;
        injector::apply(std::path::Path::new(&path), &tag_edit).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Tâche d'écriture interrompue : {e}"))?
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
    // ─── 1. Écriture dans le fichier ───
    // En cas d'échec on s'arrête ici : la base ne doit jamais décrire un
    // contenu que le fichier ne porte pas.
    //
    // Le tout part sur un fil bloquant. Préparer une image la décode et la
    // réencode, et l'écriture recopie le fichier entier — 2,6 s mesurées pour
    // un FLAC de 34 Mo sur un partage réseau. Laisser ça sur un worker du
    // runtime async l'immobilise d'autant, et un traitement par lot de
    // plusieurs centaines de fichiers gèlerait l'application.
    write_tags_blocking(path.clone(), edit).await?;

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
