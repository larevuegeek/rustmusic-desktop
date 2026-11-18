//! Préparation d'une image avant intégration dans un fichier audio.
//!
//! # La règle, et pourquoi elle est prudente
//! Une pochette de 3000×3000 en PNG pèse facilement plusieurs mégaoctets. Dans
//! un MP3 de 5 Mo c'est absurde, et surtout le bloc de tags est relu à chaque
//! scan et à chaque ouverture du fichier : des pochettes énormes ralentissent
//! toute la bibliothèque.
//!
//! Mais recompresser est **irréversible**. Quelqu'un qui a délibérément intégré
//! un scan haute résolution ne s'attend pas à ce qu'un éditeur de tags le
//! dégrade dans son dos. On n'agit donc que **si l'image dépasse un seuil**, et
//! l'appelant reçoit de quoi l'annoncer à l'utilisateur avant d'enregistrer.
//!
//! # Le piège de la perte de génération
//! Recompresser à chaque enregistrement ferait réencoder le JPEG à chaque
//! correction de titre, chaque passe abîmant un peu plus l'image. C'est
//! pourquoi cette préparation n'intervient **qu'à l'ajout ou au remplacement**
//! d'une image — jamais lors d'une simple édition de texte, où l'encodeur
//! recopie les frames d'image telles quelles.

use std::io::Cursor;

use image::{DynamicImage, ImageFormat, ImageReader};

use crate::core::audio_metadata::injector::injector::InjectError;

/// Au-delà de cette dimension (côté le plus long), on redimensionne.
const MAX_SOURCE_EDGE: u32 = 1500;
/// Au-delà de ce poids, on recompresse même si les dimensions sont modestes.
const MAX_SOURCE_BYTES: usize = 1024 * 1024;
/// Dimension cible du côté le plus long après redimensionnement.
const TARGET_EDGE: u32 = 1200;
/// Qualité JPEG de sortie. 88 est le point où l'on cesse de voir la différence
/// sur une pochette tout en divisant le poids par cinq ou plus.
const JPEG_QUALITY: u8 = 88;

/// Désigne une image **par son contenu**, pour que l'interface puisse la
/// référencer sans ambiguïté lors d'une réécriture.
///
/// # Pourquoi pas simplement sa position
/// La liste montrée à l'utilisateur vient du parser sémantique, la réécriture
/// travaille sur les frames `APIC` brutes. Ces deux parcours peuvent diverger —
/// il suffit qu'une image mal formée soit ignorée d'un côté et pas de l'autre —
/// et un décalage d'indice ferait alors supprimer ou déplacer **la mauvaise
/// image**, silencieusement. Un identifiant tiré du contenu ne peut pas dériver.
///
/// FNV-1a 64 bits : pas de dépendance, déterministe, et amplement suffisant
/// pour distinguer les quelques images d'un fichier. Il ne quitte jamais un
/// aller-retour lecture → édition → écriture, donc sa stabilité entre versions
/// n'entre pas en jeu.
pub fn image_id(data: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Réduit une image à une vignette, pour l'afficher en liste.
///
/// # Pourquoi ne pas envoyer l'original
/// Une pochette pèse couramment 400 Ko, et son encodage en base64 un tiers de
/// plus. Pour cent morceaux affichés côte à côte, cela ferait cinquante
/// mégaoctets à faire transiter puis à garder en mémoire dans l'interface —
/// pour des vignettes de trente pixels de côté.
///
/// Toujours en JPEG : une vignette n'a pas besoin de canal alpha, et le PNG y
/// serait plus lourd sans rien apporter.
pub fn thumbnail(bytes: &[u8], max_edge: u32) -> Result<Vec<u8>, InjectError> {
    let img = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| InjectError::Malformed(format!("format d'image illisible : {e}")))?
        .decode()
        .map_err(|e| InjectError::Malformed(format!("image invalide : {e}")))?;

    // `resize` conserve le rapport et ne fait que réduire : une pochette déjà
    // minuscule n'est pas agrandie, ce qui la rendrait floue pour rien.
    let small = img.resize(max_edge, max_edge, image::imageops::FilterType::Triangle);
    encode_jpeg(&small)
}

/// Une image prête à être intégrée, avec de quoi expliquer ce qui a été fait.
#[derive(Debug, Clone)]
pub struct PreparedImage {
    pub data: Vec<u8>,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    /// Poids du fichier d'origine, pour afficher « 3,2 Mo → 180 Ko ».
    pub original_bytes: usize,
    /// Faux quand l'image a été intégrée telle quelle.
    pub recompressed: bool,
}

/// Prépare les octets d'une image choisie par l'utilisateur.
///
/// Le format est déduit des **octets magiques**, pas de l'extension : une image
/// nommée `.jpg` mais réellement en PNG est un cas courant (les pochettes
/// récupérées en ligne le sont souvent), et se fier au nom mènerait à un
/// fichier dont le type MIME déclaré ment.
pub fn prepare(bytes: &[u8]) -> Result<PreparedImage, InjectError> {
    let original_bytes = bytes.len();

    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| InjectError::Malformed(format!("format d'image illisible : {e}")))?;

    let source_format = reader.format();
    let img = reader
        .decode()
        .map_err(|e| InjectError::Malformed(format!("image invalide : {e}")))?;

    let (w, h) = (img.width(), img.height());
    let too_large = w.max(h) > MAX_SOURCE_EDGE || original_bytes > MAX_SOURCE_BYTES;

    // Sous le seuil : on intègre l'original tel quel. Beaucoup de pochettes
    // font déjà 600×600 pour 80 Ko — les toucher n'apporterait rien et ferait
    // perdre de la qualité pour rien.
    if !too_large {
        return Ok(PreparedImage {
            data: bytes.to_vec(),
            mime_type: mime_for(source_format),
            width: w,
            height: h,
            original_bytes,
            recompressed: false,
        });
    }

    let resized = if w.max(h) > TARGET_EDGE {
        img.resize(TARGET_EDGE, TARGET_EDGE, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    // Un PNG transparent converti en JPEG perdrait son canal alpha et son fond
    // deviendrait noir. C'est rarissime sur une pochette, mais le dégât serait
    // visible et définitif : on reste alors en PNG, redimensionné seulement.
    let keep_png = resized.color().has_alpha();

    let (data, mime_type) = if keep_png {
        (encode_png(&resized)?, "image/png".to_string())
    } else {
        (encode_jpeg(&resized)?, "image/jpeg".to_string())
    };

    Ok(PreparedImage {
        width: resized.width(),
        height: resized.height(),
        data,
        mime_type,
        original_bytes,
        recompressed: true,
    })
}

fn encode_jpeg(img: &DynamicImage) -> Result<Vec<u8>, InjectError> {
    let mut out = Vec::new();
    // `to_rgb8` écarte l'éventuel canal alpha : le JPEG n'en a pas.
    let rgb = img.to_rgb8();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(&DynamicImage::ImageRgb8(rgb))
        .map_err(|e| InjectError::Malformed(format!("encodage JPEG : {e}")))?;
    Ok(out)
}

fn encode_png(img: &DynamicImage) -> Result<Vec<u8>, InjectError> {
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|e| InjectError::Malformed(format!("encodage PNG : {e}")))?;
    Ok(out)
}

fn mime_for(format: Option<ImageFormat>) -> String {
    match format {
        Some(ImageFormat::Png) => "image/png",
        Some(ImageFormat::Gif) => "image/gif",
        Some(ImageFormat::WebP) => "image/webp",
        Some(ImageFormat::Bmp) => "image/bmp",
        // JPEG par défaut : c'est ce que portent l'écrasante majorité des
        // pochettes, et un type MIME inconnu ferait échouer certains lecteurs.
        _ => "image/jpeg",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fabrique une image unie encodée en PNG.
    fn png(width: u32, height: u32, alpha: bool) -> Vec<u8> {
        let img = if alpha {
            DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
                width,
                height,
                image::Rgba([10, 200, 120, 128]),
            ))
        } else {
            DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                width,
                height,
                image::Rgb([10, 200, 120]),
            ))
        };
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn leaves_a_reasonable_cover_untouched() {
        let source = png(600, 600, false);
        let prepared = prepare(&source).unwrap();

        assert!(!prepared.recompressed, "une petite pochette a été dégradée");
        assert_eq!(prepared.data, source, "les octets d'origine ont changé");
        assert_eq!((prepared.width, prepared.height), (600, 600));
        assert_eq!(prepared.mime_type, "image/png");
    }

    #[test]
    fn shrinks_an_oversized_cover_to_jpeg() {
        let source = png(3000, 3000, false);
        let prepared = prepare(&source).unwrap();

        assert!(prepared.recompressed);
        assert_eq!(prepared.width.max(prepared.height), TARGET_EDGE);
        assert_eq!(prepared.mime_type, "image/jpeg");
        assert!(
            prepared.data.len() < prepared.original_bytes,
            "la recompression a alourdi l'image"
        );
    }

    #[test]
    fn keeps_png_when_the_image_has_transparency() {
        // Convertir en JPEG rendrait le fond noir : dégât visible et définitif.
        let source = png(2000, 2000, true);
        let prepared = prepare(&source).unwrap();

        assert!(prepared.recompressed);
        assert_eq!(prepared.mime_type, "image/png");
        assert_eq!(prepared.width.max(prepared.height), TARGET_EDGE);
    }

    #[test]
    fn preserves_the_aspect_ratio() {
        let prepared = prepare(&png(3000, 1500, false)).unwrap();
        assert_eq!(prepared.width, TARGET_EDGE);
        assert_eq!(prepared.height, TARGET_EDGE / 2);
    }

    #[test]
    fn detects_the_format_from_the_bytes_not_the_name() {
        // Un PNG que rien n'annonce comme tel doit être reconnu quand même.
        let prepared = prepare(&png(400, 400, false)).unwrap();
        assert_eq!(prepared.mime_type, "image/png");
    }

    #[test]
    fn rejects_data_that_is_not_an_image() {
        assert!(prepare(b"ceci n'est pas une image").is_err());
    }

    #[test]
    fn the_image_id_depends_only_on_the_content() {
        let a = png(64, 64, false);
        let b = png(64, 65, false);

        assert_eq!(image_id(&a), image_id(&a.clone()), "identifiant instable");
        assert_ne!(image_id(&a), image_id(&b), "deux images ont le même identifiant");
        assert_eq!(image_id(&a).len(), 16);
    }
}
