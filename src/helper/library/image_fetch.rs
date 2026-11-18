//! Récupération des pochettes et portraits sur Deezer : une seule tâche de chaque
//! sorte à la fois, annulable, et qui distingue « rien trouvé » d'une panne passagère.

use image::imageops::FilterType;

use crate::helper::task::Task;

pub static COVER_FETCH: Task = Task::new();
pub static PORTRAIT_FETCH: Task = Task::new();

/// Image « par défaut » de Deezer (pas de vraie photo) : `/artist//…` ou le md5 vide.
pub fn is_default_image(url: &str) -> bool {
    url.contains("/artist//") || url.contains("/artist/d41d8cd98f00b204e9800998ecf8427e/")
}

/// Premier résultat d'une recherche Deezer. `Ok(None)` : rien trouvé, c'est définitif ;
/// `Err` : réseau, quota ou réponse illisible — on réessaiera plus tard.
pub async fn search_first(client: &reqwest::Client, url: &str, fields: &[&str]) -> Result<Option<String>, String> {
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    if let Some(error) = json.get("error") {
        return Err(format!("Deezer : {error}"));
    }
    let first = &json["data"][0];
    Ok(fields
        .iter()
        .find_map(|f| first[*f].as_str())
        .filter(|u| !is_default_image(u))
        .map(str::to_string))
}

/// Octets d'une image distante ; `None` si le téléchargement échoue.
pub async fn download(client: &reqwest::Client, url: &str) -> Option<Vec<u8>> {
    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.bytes().await.ok().map(|b| b.to_vec())
}

/// Côté le plus long gardé pour une image en pleine taille.
const MAX_SIDE: u32 = 1500;
const MAX_PIXELS: u64 = 40_000_000;
static DECODE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Réduit et réencode une image lourde (pochette embarquée de plusieurs Mo) ;
/// `None` si elle est déjà raisonnable ou si le gain est nul.
pub fn shrink_image(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 600_000 {
        return None;
    }
    // Dimensions lues dans l'en-tête : un scan géant reste tel quel au lieu d'occuper des centaines de Mo.
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().ok()?;
    let (w, h) = reader.into_dimensions().ok()?;
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return None;
    }
    // Une image décodée à la fois : le scan en analyse huit en parallèle.
    let _guard = DECODE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let image = image::load_from_memory(bytes).ok()?;
    let image = if image.width().max(image.height()) > MAX_SIDE {
        image.resize(MAX_SIDE, MAX_SIDE, FilterType::Lanczos3)
    } else {
        image
    };
    let mut output = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 88)
        .encode_image(&image.to_rgb8())
        .ok()?;
    (output.len() < bytes.len()).then_some(output)
}
