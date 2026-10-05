use std::fs::read_dir;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use image::{DynamicImage, ImageReader};
use fast_image_resize::{IntoImageView, Resizer};
use fast_image_resize::images::Image;
use tauri::Emitter;

use std::sync::OnceLock;

use crate::repository::artist::artist_repository::ArtistRepository;
use crate::repository::library::library_album_repository::LibraryAlbumRepository;
use crate::repository::library::library_cache_repository::LibraryCacheRepository;

/// Ouvre un fichier image en détectant son format réel à partir des magic bytes
/// au lieu de se fier à l'extension. Plein d'encodeurs (Deezer, scrapers, etc.)
/// renvoient du PNG/WebP qu'on a stocké en `.jpg` — `image::open` fait
/// confiance à l'extension et plante avec "Illegal start bytes". Avec
/// `with_guessed_format`, on relit l'en-tête et on utilise le bon décodeur.
fn open_image_guessed(path: &std::path::Path) -> Result<DynamicImage, image::ImageError> {
    ImageReader::open(path)?
        .with_guessed_format()?
        .decode()
}

/// Côté des pochettes floutées : le navigateur les agrandit, le flou absorbe la perte.
const BLURRED_SIZE: u32 = 64;

/// Pochette saturée puis floutée une fois, en cache : sous WebKitGTK, un `blur()`
/// CSS sur une grande image se recalcule à chaque image. `flou` : écart-type à 64 px.
pub fn blurred_cover(source: &str, saturation: f32, flou: f32) -> Result<PathBuf, String> {
    let dossier = dirs::data_dir()
        .ok_or("dossier de données introuvable")?
        .join("com.larevuegeek.rustmusic")
        .join("covers")
        .join("flou");
    let cle = format!("{:x}", md5::compute(format!("{source}|{saturation:.2}|{flou:.2}")));
    let sortie = dossier.join(format!("{cle}.jpg"));
    if sortie.exists() {
        return Ok(sortie);
    }

    // La miniature 1x suffit à 64 px, et se décode bien plus vite que l'original.
    let source_norm = source.replace('\\', "/");
    let miniature = source_norm.contains("/full/").then(|| PathBuf::from(source_norm.replace("/full/", "/1x/")));
    let chemin = miniature.filter(|m| m.exists()).unwrap_or_else(|| PathBuf::from(source));
    let image = open_image_guessed(&chemin).map_err(|e| format!("ouverture de {}: {e}", chemin.display()))?;

    let mut petite = image.thumbnail(BLURRED_SIZE, BLURRED_SIZE).to_rgb8();
    saturate(&mut petite, saturation);
    let floue = image::imageops::blur(&petite, flou);

    std::fs::create_dir_all(&dossier).map_err(|e| format!("création de {}: {e}", dossier.display()))?;
    // Écrite à côté puis renommée : un affichage concurrent ne lit jamais une image à moitié écrite.
    let temporaire = dossier.join(format!("{cle}.{}.tmp", std::process::id()));
    floue
        .save_with_format(&temporaire, image::ImageFormat::Jpeg)
        .map_err(|e| format!("écriture de {}: {e}", temporaire.display()))?;
    std::fs::rename(&temporaire, &sortie).map_err(|e| format!("renommage de {}: {e}", sortie.display()))?;
    Ok(sortie)
}

/// Saturation façon CSS `saturate()` : chaque canal s'écarte de la luminance.
fn saturate(image: &mut image::RgbImage, facteur: f32) {
    for pixel in image.pixels_mut() {
        let [r, g, b] = pixel.0.map(f32::from);
        let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        pixel.0 = [r, g, b].map(|c| (luminance + (c - luminance) * facteur).round().clamp(0.0, 255.0) as u8);
    }
}

/// Pool de threads dédié à la génération de miniatures (50% des cores)
fn thumbnail_pool() -> &'static rayon::ThreadPool {
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        let num_threads = (num_cpus::get() / 2).max(1);
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap()
    })
}

pub fn thumbnail_saver(
    covers_dir: &PathBuf,
    image_data: &Vec<u8>,
    force: bool
) -> Result<String, String> {

    // Les covers albums vont dans covers/albums/
    let albums_dir = covers_dir.join("albums");

    create_thumbnail_dirs(&albums_dir)?;

    let hash: String = format!("{:x}", md5::compute(&image_data));

    let cover_path: PathBuf = albums_dir.join("full").join(format!("{}.jpg", &hash));

    if cover_path.exists() && !force {
        return Ok(cover_path.to_string_lossy().to_string());
    }

    let shrunk = super::image_fetch::shrink_image(image_data);
    std::fs::write(&cover_path, shrunk.as_deref().unwrap_or(image_data))
        .map_err(|e| e.to_string())?;

    Ok(cover_path.to_string_lossy().to_string())

}

pub fn create_thumbnail_dirs(
    covers_dir: &PathBuf
) -> Result<(), String> {

    // 1. Créer le dossier principal si besoin
    std::fs::create_dir_all(&covers_dir)
        .map_err(|e| e.to_string())?;

    //2. Création des 3 dossiers de chemin Full/1x/2x :
    std::fs::create_dir_all(covers_dir.join("full"))
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn generate_thumbnail(
    src_image: &DynamicImage,
    size: u32,
    output_path: &PathBuf
) -> Result<(), String> {

    let src_image_rgba = DynamicImage::ImageRgba8(src_image.to_rgba8());

    let (w, h) = (src_image_rgba.width(), src_image_rgba.height());
    let ratio = f64::min(size as f64 / w as f64, size as f64 / h as f64);

    let dst_width = (w as f64 * ratio) as u32;
    let dst_height = (h as f64 * ratio) as u32;
    let pixel_type = src_image_rgba.pixel_type().unwrap();

    // Créer le conteneur destination
    let mut dst_image = Image::new(dst_width, dst_height, pixel_type);

    // Redimensionner
    let mut resizer = Resizer::new();
    resizer.resize(&src_image_rgba, &mut dst_image, None)
        .map_err(|e| format!("Erreur resize: {}", e))?;

    let result = image::RgbaImage::from_raw(dst_width, dst_height, dst_image.buffer().to_vec()).unwrap();
    DynamicImage::ImageRgba8(result).save(output_path)
        .map_err(|e| format!("Erreur save resize: {}", e))?;
    
    Ok(())
}

/// Résout le chemin d'un thumbnail.
/// Si la miniature (1x/2x) n'existe pas : retourne le full immédiatement
/// et lance la génération en background pour les prochains affichages.
pub fn resolve_thumbnail(path: &str) -> Option<String> {

    let path_buf = PathBuf::from(path);

    // Déjà existant → rien à faire
    if path_buf.exists() {
        return Some(path.to_string());
    }

    // Déterminer la taille et le chemin full correspondant
    let path_str = path.replace('\\', "/");
    let (size, full_path) = if path_str.contains("/1x/") {
        (250u32, path_str.replace("/1x/", "/full/"))
    } else if path_str.contains("/2x/") {
        (500u32, path_str.replace("/2x/", "/full/"))
    } else {
        return None;
    };

    let full_buf = PathBuf::from(&full_path);
    if !full_buf.exists() {
        return None;
    }

    // Lancer la génération en background via le pool limité (50% des cores)
    let thumb_path = path_buf.clone();
    let full_clone = full_buf.clone();
    thumbnail_pool().spawn(move || {
        if let Some(parent) = thumb_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match open_image_guessed(&full_clone) {
            Ok(img) => {
                if let Err(e) = generate_thumbnail(&img, size, &thumb_path) {
                    log::warn!("[Thumbnail] Erreur génération: {}", e);
                }
            }
            Err(e) => {
                log::warn!("[Thumbnail] Impossible d'ouvrir {:?}: {}", full_clone, e);
            }
        }
    });

    // Retourner le full immédiatement
    Some(full_path)
}

/// Sauvegarde une image artiste dans full/ + génère les miniatures 1x/2x en background
pub fn save_artist_image(
    artists_dir: &PathBuf,
    filename: &str,
    image_data: &[u8],
) -> Result<String, String> {
    create_thumbnail_dirs(artists_dir)?;

    let full_path = artists_dir.join("full").join(filename);

    let shrunk = super::image_fetch::shrink_image(image_data);
    std::fs::write(&full_path, shrunk.as_deref().unwrap_or(image_data))
        .map_err(|e| format!("Erreur écriture image artiste: {}", e))?;

    // Générer les miniatures en background via le pool partagé
    let dir_1x = artists_dir.join("1x").join(filename);
    let dir_2x = artists_dir.join("2x").join(filename);
    let full_clone = full_path.clone();
    thumbnail_pool().spawn(move || {
        match open_image_guessed(&full_clone) {
            Ok(img) => {
                let _ = generate_thumbnail(&img, 250, &dir_1x);
                let _ = generate_thumbnail(&img, 500, &dir_2x);
            }
            Err(e) => {
                log::warn!("[Thumbnail] Impossible d'ouvrir {:?}: {}", full_clone, e);
            }
        }
    });

    Ok(full_path.to_string_lossy().to_string())
}

/// Migration unifiée des miniatures (covers ou artistes)
/// `migration_type` : "covers" ou "artists"
///
/// Covers : cherche dans covers/*.jpg + covers/full/*.jpg → covers/albums/full/
/// Artists : cherche dans covers/artists/*.jpg → covers/artists/full/
pub async fn migrate_old_thumbnails(
    app: &tauri::AppHandle,
    covers_dir: &PathBuf,
    db: &sqlx::SqlitePool,
    migration_type: &str,
) -> Result<u32, String> {

    let label = format!("[Migration {}]", migration_type);

    // Déterminer les sources et la destination selon le type
    let (source_dirs, dest_dir) = match migration_type {
        "covers" => {
            // Chercher dans covers/ (plat) et covers/full/ (ancien format)
            let mut sources: Vec<PathBuf> = Vec::new();
            sources.push(covers_dir.clone());              // covers/*.jpg
            let old_full = covers_dir.join("full");
            if old_full.exists() {
                sources.push(old_full);                     // covers/full/*.jpg
            }
            (sources, covers_dir.join("albums"))            // → covers/albums/full/
        }
        "artists" => {
            let artists_dir = covers_dir.join("artists");
            (vec![artists_dir.clone()], artists_dir)        // → covers/artists/full/
        }
        _ => return Ok(0),
    };

    log::info!("{} Début — destination: {:?}", label, dest_dir);

    // 1. Collecter tous les fichiers des dossiers sources
    let mut entries: Vec<PathBuf> = Vec::new();
    for source in &source_dirs {
        if !source.exists() { continue; }
        if let Ok(dir_entries) = read_dir(source) {
            for entry in dir_entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    entries.push(path);
                }
            }
        }
    }

    log::info!("{} {} fichiers trouvés à migrer", label, entries.len());

    if entries.is_empty() {
        log::info!("{} Rien à migrer", label);
        return Ok(0);
    }

    // 2. Créer les sous-dossiers full/1x/2x dans la destination
    create_thumbnail_dirs(&dest_dir)?;

    let total = entries.len();
    let mut migrated: u32 = 0;

    // ── PASS 1 : Déplacement fichiers + mise à jour DB ──
    log::info!("{} Pass 1 : déplacement + DB pour {} fichiers", label, total);
    let mut tx = db.begin().await.map_err(|e| format!("Erreur transaction: {}", e))?;
    let mut migrated_files: Vec<(PathBuf, String)> = Vec::new();

    let progress = AtomicUsize::new(0);
    let total_thumb = entries.len();

    for (i, file_path) in entries.iter().enumerate() {
        let file_name = match file_path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        let new_path = dest_dir.join("full").join(&file_name);

        // Eviter de déplacer vers soi-même
        if file_path == &new_path { continue; }

        if let Err(e) = std::fs::rename(&file_path, &new_path) {
            log::warn!("{} Impossible de déplacer {}: {}", label, file_name, e);
            continue;
        }

        let old_path_str = file_path.to_string_lossy().to_string();
        let new_path_str = new_path.to_string_lossy().to_string();

        // Mise à jour DB selon le type
        match migration_type {
            "covers" => {
                LibraryAlbumRepository::update_library_album_cover(&mut *tx, &new_path_str, &old_path_str)
                    .await
                    .map_err(|e| format!("Erreur DB library_albums pour {}: {}", file_name, e))?;

                LibraryCacheRepository::update_library_cache_thumbnail_path(&mut *tx, &new_path_str, &old_path_str)
                    .await
                    .map_err(|e| format!("Erreur DB library_cache pour {}: {}", file_name, e))?;
            }
            "artists" => {
                ArtistRepository::update_image_url_by_path(&mut *tx, &new_path_str, &old_path_str)
                    .await
                    .map_err(|e| format!("Erreur DB artists pour {}: {}", file_name, e))?;
            }
            _ => {}
        }

        migrated_files.push((new_path, file_name.clone()));
        migrated += 1;


        let current = progress.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = app.emit("migration-progress", serde_json::json!({
            "current": current,
            "total": total_thumb,
            "percent": (current * 100) / total_thumb,
            "file_name": file_name,
        }));

        log::debug!("{} [{}/{}] Déplacé: {}", label, i + 1, total, file_name);
    }

    tx.commit().await.map_err(|e| format!("Erreur commit {}: {}", migration_type, e))?;
    log::info!("{} Pass 1 terminée — {} fichiers déplacés", label, migrated);

    log::info!("{} Terminé — {} fichiers migrés sur {}", label, migrated, total);
    Ok(migrated)
}

#[cfg(test)]
mod tests_flou {
    use super::*;

    #[test]
    fn saturation_matches_css() {
        let mut gris = image::RgbImage::from_pixel(1, 1, image::Rgb([90, 90, 90]));
        saturate(&mut gris, 2.4);
        assert_eq!(gris.get_pixel(0, 0).0, [90, 90, 90], "un gris reste gris");

        let mut rouge = image::RgbImage::from_pixel(1, 1, image::Rgb([180, 90, 90]));
        saturate(&mut rouge, 2.0);
        let [r, g, b] = rouge.get_pixel(0, 0).0;
        assert!(r > 180 && g < 90 && g == b, "plus saturé : {r},{g},{b}");
    }

    #[test]
    fn blurred_cover_is_small_and_cached() {
        let dossier = std::env::temp_dir().join(format!("rm-flou-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        // Damier contrasté : flouté, il ne doit plus rester de noir ni de blanc purs.
        let source = dossier.join("pochette.png");
        image::RgbImage::from_fn(300, 300, |x, y| {
            if (x / 30 + y / 30) % 2 == 0 { image::Rgb([0, 0, 0]) } else { image::Rgb([255, 255, 255]) }
        })
        .save(&source)
        .unwrap();

        let floue = blurred_cover(source.to_str().unwrap(), 1.0, 4.0).expect("pochette floue");
        let lue = image::open(&floue).unwrap().to_rgb8();
        assert_eq!(lue.dimensions(), (BLURRED_SIZE, BLURRED_SIZE));
        let (min, max) = lue.pixels().fold((255u8, 0u8), |(lo, hi), p| (lo.min(p.0[0]), hi.max(p.0[0])));
        // Les bords, moins entourés, restent un peu plus contrastés que le centre.
        assert!(min > 20 && max < 235, "flou insuffisant : {min}..{max}");

        let date = std::fs::metadata(&floue).unwrap().modified().unwrap();
        assert_eq!(blurred_cover(source.to_str().unwrap(), 1.0, 4.0).unwrap(), floue, "même clé, même fichier");
        assert_eq!(std::fs::metadata(&floue).unwrap().modified().unwrap(), date, "pas régénérée");

        let _ = std::fs::remove_file(&floue);
        let _ = std::fs::remove_dir_all(&dossier);
    }
}
