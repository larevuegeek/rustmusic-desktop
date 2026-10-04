//! Récupération des pochettes et portraits sur Deezer : une seule tâche de chaque
//! sorte à la fois, annulable, et qui distingue « rien trouvé » d'une panne passagère.

use image::imageops::FilterType;

use crate::helper::tache::Tache;

pub static POCHETTES: Tache = Tache::new();
pub static PORTRAITS: Tache = Tache::new();

/// Image « par défaut » de Deezer (pas de vraie photo) : `/artist//…` ou le md5 vide.
pub fn est_image_par_defaut(url: &str) -> bool {
    url.contains("/artist//") || url.contains("/artist/d41d8cd98f00b204e9800998ecf8427e/")
}

/// Premier résultat d'une recherche Deezer. `Ok(None)` : rien trouvé, c'est définitif ;
/// `Err` : réseau, quota ou réponse illisible — on réessaiera plus tard.
pub async fn chercher(client: &reqwest::Client, url: &str, champs: &[&str]) -> Result<Option<String>, String> {
    let reponse = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !reponse.status().is_success() {
        return Err(format!("HTTP {}", reponse.status()));
    }
    let json: serde_json::Value = reponse.json().await.map_err(|e| e.to_string())?;
    if let Some(erreur) = json.get("error") {
        return Err(format!("Deezer : {erreur}"));
    }
    let premier = &json["data"][0];
    Ok(champs
        .iter()
        .find_map(|c| premier[*c].as_str())
        .filter(|u| !est_image_par_defaut(u))
        .map(str::to_string))
}

/// Octets d'une image distante ; `None` si le téléchargement échoue.
pub async fn telecharger(client: &reqwest::Client, url: &str) -> Option<Vec<u8>> {
    let reponse = client.get(url).send().await.ok()?;
    if !reponse.status().is_success() {
        return None;
    }
    reponse.bytes().await.ok().map(|b| b.to_vec())
}

/// Côté le plus long gardé pour une image en pleine taille.
const COTE_MAX: u32 = 1500;

/// Réduit et réencode une image lourde (pochette embarquée de plusieurs Mo) ;
/// `None` si elle est déjà raisonnable ou si le gain est nul.
pub fn alleger(octets: &[u8]) -> Option<Vec<u8>> {
    if octets.len() < 600_000 {
        return None;
    }
    let image = image::load_from_memory(octets).ok()?;
    let image = if image.width().max(image.height()) > COTE_MAX {
        image.resize(COTE_MAX, COTE_MAX, FilterType::Lanczos3)
    } else {
        image
    };
    let mut sortie = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut sortie, 88)
        .encode_image(&image.to_rgb8())
        .ok()?;
    (sortie.len() < octets.len()).then_some(sortie)
}
