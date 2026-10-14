//! Client MusicBrainz, pour les **métadonnées**, et Cover Art Archive pour les pochettes.
//!
//! Ce que Deezer ne donne pas : compositeurs, numéros de disque fiables, éditions
//! distinctes (pays, support), et une bonne couverture du classique.
//!
//! # Règles d'usage
//! Gratuit, sans clé, mais **une requête par seconde** au plus et un
//! User-Agent qui identifie l'application : au-delà, le serveur répond 503.
//!
//! # Ce que MusicBrainz ne donnera pas
//! - pas toujours de pochette (Cover Art Archive est alimentée à la main) ;
//! - des genres seulement quand la communauté en a voté.

use std::time::Duration;

use serde_json::Value;
use tokio::sync::Mutex;
use tokio::time::Instant;

use super::model::{AlbumDetail, AlbumHit, AlbumTrack, TrackHit, TrackValues};

const API: &str = "https://musicbrainz.org/ws/2";
const COVERS: &str = "https://coverartarchive.org/release";
/// Une requête par seconde, avec une marge.
const MIN_INTERVAL: Duration = Duration::from_millis(1100);

static LAST_CALL: Mutex<Option<Instant>> = Mutex::const_new(None);

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent(format!("RustMusic/{} ( https://rustmusic.dev )", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("Client HTTP : {e}"))
}

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

/// `path` : la ressource et sa requête, sans `fmt`.
async fn get_json(path: &str) -> Result<Value, String> {
    throttle().await;
    let sep = if path.contains('?') { '&' } else { '?' };
    let response = client()?
        .get(format!("{API}/{path}{sep}fmt=json"))
        .send()
        .await
        .map_err(|e| format!("MusicBrainz injoignable : {e}"))?;
    let status = response.status();
    if status.as_u16() == 503 {
        return Err("MusicBrainz est surchargé : réessayez dans un instant.".into());
    }
    let json: Value = response.json().await.map_err(|e| format!("Réponse MusicBrainz illisible : {e}"))?;
    if !status.is_success() {
        let message = json["error"].as_str().unwrap_or("erreur inconnue");
        return Err(format!("MusicBrainz : {message}"));
    }
    Ok(json)
}

/// Chaque mot doit figurer dans l'un des champs : « floyd wall » trouve « The Wall »
/// de Pink Floyd, dans n'importe quel ordre.
fn lucene(query: &str, fields: &[&str]) -> String {
    query
        .split_whitespace()
        .map(escape)
        .filter(|w| !w.is_empty())
        .map(|w| format!("({})", fields.iter().map(|f| format!("{f}:{w}")).collect::<Vec<_>>().join(" OR ")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

/// Neutralise la syntaxe Lucene ; « AND », « OR », « NOT » deviennent de simples mots.
fn escape(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    for c in word.to_lowercase().chars() {
        if "+-&|!(){}[]^\"~*?:\\/".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// « Artiste feat. Autre » tel que MusicBrainz le crédite.
fn credit(value: &Value) -> String {
    value
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .map(|p| format!("{}{}", p["name"].as_str().unwrap_or(""), p["joinphrase"].as_str().unwrap_or("")))
                .collect::<String>()
        })
        .unwrap_or_default()
}

fn year(date: &Value) -> Option<String> {
    date.as_str().and_then(|d| d.get(0..4)).map(str::to_string)
}

/// Millisecondes → secondes arrondies.
fn seconds(ms: &Value) -> u32 {
    ms.as_u64().map(|ms| ((ms + 500) / 1000) as u32).unwrap_or(0)
}

/// « 1979 · GB · 2×CD » : ce qui distingue deux éditions d'un même album.
fn edition(release: &Value) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(y) = year(&release["date"]) {
        parts.push(y);
    }
    if let Some(country) = release["country"].as_str() {
        parts.push(country.to_string());
    }
    if let Some(media) = release["media"].as_array().filter(|m| !m.is_empty()) {
        let format = media[0]["format"].as_str().unwrap_or("?");
        let same = media.iter().all(|m| m["format"].as_str().unwrap_or("?") == format);
        parts.push(match (media.len(), same) {
            (1, _) => format.to_string(),
            (n, true) => format!("{n}×{format}"),
            (n, false) => format!("{n} supports"),
        });
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn cover_thumb(release_id: &str) -> String {
    format!("{COVERS}/{release_id}/front-250")
}

/// Cherche des albums (éditions).
pub async fn search_albums(query: &str, limit: u32) -> Result<Vec<AlbumHit>, String> {
    let q = urlencoding::encode(&lucene(query, &["release", "artist"])).into_owned();
    let json = get_json(&format!("release?query={q}&limit={limit}")).await?;
    Ok(json["releases"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|r| {
                    let id = r["id"].as_str()?.to_string();
                    Some(AlbumHit {
                        cover: Some(cover_thumb(&id)),
                        title: r["title"].as_str()?.to_string(),
                        artist: credit(&r["artist-credit"]),
                        track_count: r["track-count"].as_u64().unwrap_or(0) as u32,
                        detail: edition(r),
                        id,
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Cherche des pistes ; chacune est rattachée à l'une de ses éditions, l'album officiel de préférence.
pub async fn search_tracks(query: &str, limit: u32) -> Result<Vec<TrackHit>, String> {
    let q = urlencoding::encode(&lucene(query, &["recording", "artist"])).into_owned();
    let json = get_json(&format!("recording?query={q}&limit={limit}")).await?;
    Ok(json["recordings"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|rec| {
                    let releases = rec["releases"].as_array()?;
                    let official_album = |r: &&Value| {
                        r["status"].as_str() == Some("Official") && r["release-group"]["primary-type"].as_str() == Some("Album")
                    };
                    let release = releases.iter().find(official_album).or_else(|| releases.first())?;
                    let release_id = release["id"].as_str()?;
                    Some(TrackHit {
                        id: format!("{}@{release_id}", rec["id"].as_str()?),
                        title: rec["title"].as_str()?.to_string(),
                        artist: credit(&rec["artist-credit"]),
                        album: release["title"].as_str().unwrap_or("").to_string(),
                        cover: Some(cover_thumb(release_id)),
                        duration: seconds(&rec["length"]),
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Compositeurs d'un enregistrement, par l'œuvre qu'il interprète.
fn composers(recording: &Value) -> Option<String> {
    let mut names: Vec<String> = Vec::new();
    for rel in recording["relations"].as_array().into_iter().flatten() {
        if rel["target-type"].as_str() != Some("work") {
            continue;
        }
        for wr in rel["work"]["relations"].as_array().into_iter().flatten() {
            if matches!(wr["type"].as_str(), Some("composer" | "writer")) {
                if let Some(name) = wr["artist"]["name"].as_str() {
                    if !names.iter().any(|n| n == name) {
                        names.push(name.to_string());
                    }
                }
            }
        }
    }
    (!names.is_empty()).then(|| names.join(", "))
}

/// Genre le plus voté, la majuscule en tête (« hip hop » → « Hip hop »).
fn top_genre(genres: &Value) -> Option<String> {
    let best = genres.as_array()?.iter().max_by_key(|g| g["count"].as_u64().unwrap_or(0))?;
    let name = best["name"].as_str()?;
    let mut chars = name.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect())
}

/// Une édition et ses pistes, compositeurs compris.
pub async fn album_detail(release_id: &str) -> Result<AlbumDetail, String> {
    let inc = "recordings+artist-credits+genres+release-groups+recording-level-rels+work-rels+work-level-rels+artist-rels";
    let r = get_json(&format!("release/{}?inc={inc}", urlencoding::encode(release_id))).await?;
    let album_artist = credit(&r["artist-credit"]);

    let mut tracks = Vec::new();
    let mut key = 0u16;
    for medium in r["media"].as_array().into_iter().flatten() {
        let disc = medium["position"].as_u64().unwrap_or(1) as u16;
        for (i, t) in medium["tracks"].as_array().into_iter().flatten().enumerate() {
            key += 1;
            let recording = &t["recording"];
            let artist = credit(&t["artist-credit"]);
            tracks.push(AlbumTrack {
                key,
                position: t["position"].as_u64().unwrap_or(i as u64 + 1) as u16,
                disc,
                title: t["title"].as_str().or_else(|| recording["title"].as_str()).unwrap_or("").to_string(),
                artist: if artist.is_empty() { album_artist.clone() } else { artist },
                duration: match seconds(&t["length"]) {
                    0 => seconds(&recording["length"]),
                    d => d,
                },
                composer: composers(recording),
                id: recording["id"].as_str().unwrap_or("").to_string(),
            });
        }
    }

    // Les genres sont souvent votés sur l'album plutôt que sur l'édition.
    let mut genre = top_genre(&r["genres"]);
    if genre.is_none() {
        if let Some(rg) = r["release-group"]["id"].as_str() {
            if let Ok(group) = get_json(&format!("release-group/{rg}?inc=genres")).await {
                genre = top_genre(&group["genres"]);
            }
        }
    }

    Ok(AlbumDetail {
        id: release_id.to_string(),
        title: r["title"].as_str().unwrap_or("").to_string(),
        artist: album_artist,
        year: year(&r["date"]).or_else(|| year(&r["release-group"]["first-release-date"])),
        genre,
        // Pochette proposée seulement si l'archive en a une ; 1200 px, intégrable au fichier.
        cover: (r["cover-art-archive"]["front"].as_bool() == Some(true)).then(|| format!("{COVERS}/{release_id}/front-1200")),
        tracks,
    })
}

/// Tout ce qui est connu d'une piste : `id` vaut « enregistrement@édition ».
pub async fn track_values(id: &str) -> Result<TrackValues, String> {
    let (recording, release) = id.split_once('@').ok_or_else(|| format!("Identifiant MusicBrainz invalide : {id}"))?;
    let album = album_detail(release).await?;
    let track = album
        .tracks
        .iter()
        .find(|t| t.id == recording)
        .ok_or("Piste introuvable dans cette édition")?;
    // Total de pistes du disque de la piste, comme l'écrivent les lecteurs.
    let on_disc = album.tracks.iter().filter(|t| t.disc == track.disc).count() as u32;
    Ok(TrackValues {
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: album.title.clone(),
        album_artist: Some(album.artist.clone()),
        year: album.year.clone(),
        genre: album.genre.clone(),
        track_number: Some(track.position),
        total_tracks: Some(on_disc),
        disc_number: Some(track.disc),
        composer: track.composer.clone(),
        cover: album.cover.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_query_on_every_word() {
        assert_eq!(lucene("Pink Floyd", &["release", "artist"]), "(release:pink OR artist:pink) AND (release:floyd OR artist:floyd)");
        // Opérateurs et caractères spéciaux neutralisés.
        assert_eq!(lucene("AC/DC AND", &["artist"]), "(artist:ac\\/dc) AND (artist:and)");
        assert_eq!(lucene("   ", &["artist"]), "");
    }

    #[test]
    fn reads_a_release_with_two_discs_and_composers() {
        let r: Value = serde_json::json!({
            "media": [
                { "position": 1, "format": "CD", "tracks": [
                    { "position": 1, "title": "Intro", "length": 61400,
                      "recording": { "id": "r1", "relations": [{ "target-type": "work", "work": { "relations": [
                          { "type": "composer", "artist": { "name": "Ravel" } },
                          { "type": "lyricist", "artist": { "name": "X" } } ] } }] } } ] },
                { "position": 2, "format": "CD", "tracks": [
                    { "position": 1, "title": "Suite", "recording": { "id": "r2", "length": 120000 } } ] }
            ],
            "country": "FR", "date": "1999-03-01"
        });
        assert_eq!(edition(&r).as_deref(), Some("1999 · FR · 2×CD"));
        assert_eq!(composers(&r["media"][0]["tracks"][0]["recording"]).as_deref(), Some("Ravel"));
        assert_eq!(composers(&r["media"][1]["tracks"][0]["recording"]), None);
        assert_eq!(seconds(&serde_json::json!(61400)), 61);
        assert_eq!(top_genre(&serde_json::json!([{ "name": "rock", "count": 2 }, { "name": "hip hop", "count": 5 }])).as_deref(), Some("Hip hop"));
        assert_eq!(credit(&serde_json::json!([{ "name": "A", "joinphrase": " feat. " }, { "name": "B" }])), "A feat. B");
    }

    /// Interroge le vrai service : `cargo test musicbrainz -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn live_lookup() {
        let hits = search_albums("radiohead ok computer", 5).await.unwrap();
        assert!(!hits.is_empty());
        let album = album_detail(&hits[0].id).await.unwrap();
        assert!(album.tracks.len() >= 10, "{album:?}");
        let tracks = search_tracks("radiohead paranoid android", 3).await.unwrap();
        let values = track_values(&tracks[0].id).await.unwrap();
        assert_eq!(values.title.to_lowercase(), "paranoid android");
    }
}
