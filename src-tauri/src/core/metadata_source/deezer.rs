//! Client de l'API publique Deezer, pour les **métadonnées**.
//!
//! Les pochettes passaient déjà par cette API (`library_command`) ; ce module
//! va chercher le reste : la liste des pistes d'un album, avec leurs titres,
//! positions et durées.
//!
//! # Ce que Deezer ne donnera pas
//! À dire dans l'interface plutôt qu'à laisser découvrir :
//! - **aucun compositeur**, aucun parolier ;
//! - le **genre au niveau de l'album** seulement, et très grossier ;
//! - pas de numéro de disque fiable sur les coffrets ;
//! - une couverture inégale hors variété internationale.
//!
//! Pour ces champs, c'est MusicBrainz qu'il faudra. Le module d'appariement,
//! lui, ne connaît aucune source : il travaille sur des structures neutres et
//! resservira tel quel.
//!
//! # Débit
//! L'API publique est documentée à cinquante requêtes par tranche de cinq
//! secondes et par adresse. Une recherche d'album suivie d'une lecture de
//! piste, c'est deux appels — mais corriger cinquante albums d'affilée
//! saturerait. D'où l'attente minimale entre deux requêtes.

use std::time::Duration;

use serde::Serialize;
use tokio::sync::Mutex;
use tokio::time::Instant;

/// Attente minimale entre deux requêtes.
///
/// Cinquante requêtes pour cinq secondes autorisent une toutes les cent
/// millisecondes. On prend une marge : le quota est partagé par adresse, et
/// rien ne dit qu'on est seul derrière.
const MIN_INTERVAL: Duration = Duration::from_millis(150);

/// Horodatage du dernier appel, pour tenir le débit.
static LAST_CALL: Mutex<Option<Instant>> = Mutex::const_new(None);

/// Un album trouvé par la recherche.
#[derive(Debug, Clone, Serialize)]
pub struct AlbumHit {
    pub id: i64,
    pub title: String,
    pub artist: String,
    /// Vignette, directement affichable.
    pub cover: Option<String>,
    /// Nombre de pistes annoncé — le premier indice qu'on tient le bon album.
    pub track_count: u32,
}

/// Le détail d'un album, avec ses pistes.
#[derive(Debug, Clone, Serialize)]
pub struct AlbumDetail {
    pub id: i64,
    pub title: String,
    pub artist: String,
    /// Année seule, extraite de la date de sortie.
    pub year: Option<String>,
    /// Genre principal, au niveau de l'album — Deezer n'en donne pas par piste.
    pub genre: Option<String>,
    pub cover: Option<String>,
    pub tracks: Vec<AlbumTrack>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlbumTrack {
    pub position: u16,
    pub disc: u16,
    pub title: String,
    pub artist: String,
    /// En secondes.
    pub duration: u32,
}

/// Une piste trouvée par la recherche.
///
/// Sert au cas **un seul fichier** : y chercher un album puis y retrouver sa
/// piste demanderait deux décisions là où le titre suffit.
#[derive(Debug, Clone, Serialize)]
pub struct TrackHit {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Vignette de l'album, directement affichable.
    pub cover: Option<String>,
    /// En secondes — le meilleur discriminant entre deux versions d'un titre.
    pub duration: u32,
}

/// Tout ce que Deezer sait d'une piste, prêt à remplir un formulaire.
///
/// Les champs absents sont `None` et non une chaîne vide : l'interface ne doit
/// jamais proposer d'effacer une valeur existante faute de mieux.
#[derive(Debug, Clone, Serialize)]
pub struct TrackValues {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: Option<String>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<u16>,
    pub total_tracks: Option<u32>,
    pub disc_number: Option<u16>,
    pub cover: Option<String>,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Client HTTP : {e}"))
}

/// Attend si nécessaire pour ne pas dépasser le débit autorisé.
async fn throttle() {
    let mut last = LAST_CALL.lock().await;
    if let Some(previous) = *last {
        let elapsed = previous.elapsed();
        if elapsed < MIN_INTERVAL {
            tokio::time::sleep(MIN_INTERVAL - elapsed).await;
        }
    }
    *last = Some(Instant::now());
}

async fn get_json(url: &str) -> Result<serde_json::Value, String> {
    throttle().await;

    let response = client()?
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Deezer injoignable : {e}"))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Réponse Deezer illisible : {e}"))?;

    // L'API répond 200 même en cas d'erreur, avec un objet `error` : sans ce
    // contrôle, un quota dépassé passerait pour un album vide.
    if let Some(error) = json.get("error") {
        if !error.is_null() {
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("erreur inconnue");
            return Err(format!("Deezer : {message}"));
        }
    }

    Ok(json)
}

/// Cherche des albums.
pub async fn search_albums(query: &str, limit: u32) -> Result<Vec<AlbumHit>, String> {
    let encoded = urlencoding::encode(query);
    let url = format!("https://api.deezer.com/search/album?q={encoded}&limit={limit}");
    let json = get_json(&url).await?;

    Ok(json["data"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    Some(AlbumHit {
                        id: item["id"].as_i64()?,
                        title: item["title"].as_str()?.to_string(),
                        artist: item["artist"]["name"].as_str().unwrap_or("").to_string(),
                        cover: item["cover_medium"].as_str().map(str::to_string),
                        track_count: item["nb_tracks"].as_u64().unwrap_or(0) as u32,
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Cherche des pistes.
pub async fn search_tracks(query: &str, limit: u32) -> Result<Vec<TrackHit>, String> {
    let encoded = urlencoding::encode(query);
    let url = format!("https://api.deezer.com/search/track?q={encoded}&limit={limit}");
    let json = get_json(&url).await?;

    Ok(json["data"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    Some(TrackHit {
                        id: item["id"].as_i64()?,
                        title: item["title"].as_str()?.to_string(),
                        artist: item["artist"]["name"].as_str().unwrap_or("").to_string(),
                        album: item["album"]["title"].as_str().unwrap_or("").to_string(),
                        cover: item["album"]["cover_medium"].as_str().map(str::to_string),
                        duration: item["duration"].as_u64().unwrap_or(0) as u32,
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Récupère tout ce qui est connu d'une piste, album compris.
///
/// Deux requêtes, et c'est incompressible : la fiche de la piste donne son
/// rang et sa date, mais ni le genre, ni le nombre total de pistes, ni
/// l'artiste de l'album — qui sont des propriétés de l'album. Les demander en
/// une fois côté interface la ferait attendre deux allers-retours au lieu d'un.
pub async fn track_values(track_id: i64) -> Result<TrackValues, String> {
    let track = get_json(&format!("https://api.deezer.com/track/{track_id}")).await?;

    let album_id = track["album"]["id"].as_i64();
    // L'album est un complément : son absence ne doit pas priver du titre et
    // de l'artiste, qui sont déjà là.
    let album = match album_id {
        Some(id) => album_detail(id).await.ok(),
        None => None,
    };

    let year = track["release_date"]
        .as_str()
        .and_then(|d| d.get(0..4))
        .map(str::to_string)
        .or_else(|| album.as_ref().and_then(|a| a.year.clone()));

    Ok(TrackValues {
        title: track["title"].as_str().unwrap_or("").to_string(),
        artist: track["artist"]["name"].as_str().unwrap_or("").to_string(),
        album: track["album"]["title"].as_str().unwrap_or("").to_string(),
        album_artist: album.as_ref().map(|a| a.artist.clone()),
        year,
        genre: album.as_ref().and_then(|a| a.genre.clone()),
        track_number: track["track_position"].as_u64().map(|n| n as u16),
        total_tracks: album.as_ref().map(|a| a.tracks.len() as u32),
        disc_number: track["disk_number"].as_u64().map(|n| n as u16),
        // La grande version : cette pochette n'est pas seulement affichée, elle
        // peut être intégrée au fichier. Une vignette de 250 px y serait
        // définitive et illisible sur n'importe quel écran.
        cover: track["album"]["cover_xl"]
            .as_str()
            .or_else(|| track["album"]["cover_big"].as_str())
            .or_else(|| track["album"]["cover_medium"].as_str())
            .map(str::to_string)
            .or_else(|| album.as_ref().and_then(|a| a.cover.clone())),
    })
}

/// Récupère un album et sa liste de pistes.
pub async fn album_detail(album_id: i64) -> Result<AlbumDetail, String> {
    let json = get_json(&format!("https://api.deezer.com/album/{album_id}")).await?;

    let tracks = json["tracks"]["data"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    Some(AlbumTrack {
                        // Certaines réponses omettent la position : le rang
                        // dans la liste est alors le meilleur substitut.
                        position: item["track_position"]
                            .as_u64()
                            .unwrap_or(index as u64 + 1) as u16,
                        disc: item["disk_number"].as_u64().unwrap_or(1) as u16,
                        title: item["title"].as_str()?.to_string(),
                        artist: item["artist"]["name"].as_str().unwrap_or("").to_string(),
                        duration: item["duration"].as_u64().unwrap_or(0) as u32,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(AlbumDetail {
        id: album_id,
        title: json["title"].as_str().unwrap_or("").to_string(),
        artist: json["artist"]["name"].as_str().unwrap_or("").to_string(),
        year: json["release_date"]
            .as_str()
            .and_then(|d| d.get(0..4))
            .map(str::to_string),
        genre: json["genres"]["data"]
            .as_array()
            .and_then(|g| g.first())
            .and_then(|g| g["name"].as_str())
            .map(str::to_string),
        // La grande version : cette pochette n'est pas seulement affichée, elle
        // peut être intégrée aux fichiers. Une vignette de 250 px y serait
        // définitive et illisible sur n'importe quel écran.
        cover: json["cover_xl"]
            .as_str()
            .or_else(|| json["cover_big"].as_str())
            .or_else(|| json["cover_medium"].as_str())
            .map(str::to_string),
        tracks,
    })
}
