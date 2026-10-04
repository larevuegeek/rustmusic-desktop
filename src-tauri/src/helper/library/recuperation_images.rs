//! Récupération des pochettes et portraits sur Deezer : une seule tâche de chaque
//! sorte à la fois, annulable, et qui distingue « rien trouvé » d'une panne passagère.

use std::sync::atomic::{AtomicBool, Ordering};

use image::imageops::FilterType;

pub struct Tache {
    en_cours: AtomicBool,
    annuler: AtomicBool,
}

pub static POCHETTES: Tache = Tache::new();
pub static PORTRAITS: Tache = Tache::new();

impl Tache {
    const fn new() -> Self {
        Self { en_cours: AtomicBool::new(false), annuler: AtomicBool::new(false) }
    }

    /// Réserve la tâche ; `None` si elle tourne déjà (deux passes se marchaient dessus).
    pub fn demarrer(&'static self) -> Option<Jeton> {
        if self.en_cours.swap(true, Ordering::SeqCst) {
            return None;
        }
        self.annuler.store(false, Ordering::SeqCst);
        Some(Jeton(self))
    }

    /// Demande l'arrêt ; vrai si une passe était en cours.
    pub fn annuler(&self) -> bool {
        let actif = self.en_cours.load(Ordering::SeqCst);
        if actif {
            self.annuler.store(true, Ordering::SeqCst);
        }
        actif
    }
}

/// Tant qu'il vit, la tâche est occupée.
pub struct Jeton(&'static Tache);

impl Jeton {
    pub fn annule(&self) -> bool {
        self.0.annuler.load(Ordering::SeqCst)
    }

    /// Pause entre deux requêtes, interrompue par une annulation.
    pub async fn patienter(&self, ms: u64) {
        let mut reste = ms;
        while reste > 0 && !self.annule() {
            let pas = reste.min(100);
            tokio::time::sleep(std::time::Duration::from_millis(pas)).await;
            reste -= pas;
        }
    }
}

impl Drop for Jeton {
    fn drop(&mut self) {
        self.0.en_cours.store(false, Ordering::SeqCst);
    }
}

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
