//! Compositeurs d'une bibliothèque, tirés du tag `composer` : découpés, regroupés, reliés aux artistes connus.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use sqlx::SqlitePool;

use crate::helper::string::string::normalize_name;
use crate::mapper::library::track::track_list_item_view::TrackListView;
use crate::repository::library::artist_split_repository::{key, ArtistSplitRepository};
use crate::repository::library::composer_repository::ComposerRepository;
use crate::repository::library::library_track_repository::LibraryTrackRepository;
use crate::service::library::artist_split;

#[derive(Debug, Serialize)]
pub struct ComposerView {
    pub name: String,
    pub total_albums: usize,
    pub total_tracks: usize,
    pub total_duration: f64,
    pub thumbnail_path: Option<String>,
    /// Le même nom parmi les artistes de la bibliothèque : sa page existe.
    pub artist_id: Option<String>,
}

/// Mentions qui ne nomment personne.
const JUNK: &[&str] = &[
    "various", "various composers", "various artists", "unknown", "unknown composer", "[unknown]",
    "composer information unavailable", "n/a", "na", "-", "?", "inconnu", "divers", "compositeurs divers",
];

fn is_junk(name: &str) -> bool {
    JUNK.contains(&name.trim().to_lowercase().as_str())
}

/// Noms d'un tag : la décision des artistes multiples s'il y en a une (« Earth, Wind & Fire » entier), sinon coupé sur « , ; / ».
pub fn split(raw: &str, rules: &HashMap<String, Vec<String>>) -> Vec<String> {
    let raw = raw.trim();
    if raw.is_empty() || is_junk(raw) {
        return vec![];
    }
    if let Some(parts) = rules.get(&key(raw)) {
        return parts.clone();
    }
    let mut seen = HashSet::new();
    raw.split([',', ';', '/', '\0'])
        .map(str::trim)
        .filter(|n| !n.is_empty() && !is_junk(n))
        .filter(|n| seen.insert(normalize_name(n)))
        .map(str::to_string)
        .collect()
}

#[derive(Default)]
struct Tally {
    spellings: HashMap<String, usize>,
    tracks: usize,
    albums: HashSet<String>,
    duration: f64,
}

pub async fn list(pool: &SqlitePool, library_id: i64) -> Result<Vec<ComposerView>, sqlx::Error> {
    let rows = ComposerRepository::rows(pool, library_id).await?;
    if rows.is_empty() {
        return Ok(vec![]);
    }
    let artists: HashMap<String, String> = ComposerRepository::library_artists(pool, library_id).await?.into_iter().collect();
    let rules = artist_split::rules();
    let photos = photos(pool).await;

    let mut tally: HashMap<String, Tally> = HashMap::new();
    for (_, album, duration, raw) in &rows {
        for name in split(raw, &rules) {
            let t = tally.entry(normalize_name(&name)).or_default();
            *t.spellings.entry(name).or_default() += 1;
            t.tracks += 1;
            t.duration += duration.unwrap_or(0.0);
            if let Some(a) = album {
                t.albums.insert(a.clone());
            }
        }
    }
    Ok(tally
        .into_iter()
        .map(|(key, t)| {
            // L'écriture la plus fréquente ; à égalité, la première dans l'ordre alphabétique.
            let name = t.spellings.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).map(|(n, _)| n).unwrap_or_default();
            ComposerView {
                name,
                total_albums: t.albums.len(),
                total_tracks: t.tracks,
                total_duration: t.duration,
                thumbnail_path: photos.get(&key).cloned(),
                artist_id: artists.get(&key).cloned(),
            }
        })
        .collect())
}

/// Photos par nom normalisé : celle de l'artiste du même nom d'abord, sinon celle trouvée pour le compositeur.
async fn photos(pool: &SqlitePool) -> HashMap<String, String> {
    let mut map: HashMap<String, String> =
        ComposerRepository::images(pool).await.unwrap_or_default().into_iter().filter(|(_, p)| !p.is_empty()).collect();
    map.extend(artist_split::photos(pool).await);
    map
}

/// La photo d'un compositeur, si on l'a déjà ; `Some("")` : cherchée sans résultat.
pub async fn known_photo(pool: &SqlitePool, name: &str) -> Option<String> {
    let k = normalize_name(name);
    if let Some(p) = artist_split::photos(pool).await.remove(&k) {
        return Some(p);
    }
    ComposerRepository::images(pool).await.ok()?.into_iter().find(|(n, _)| *n == k).map(|(_, p)| p)
}

/// Compositeurs de toutes les bibliothèques à chercher sur Deezer : ni photo cherchée, ni fiche d'artiste.
pub async fn without_photo(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    let mut skip: HashSet<String> = ComposerRepository::images(pool).await?.into_iter().map(|(k, _)| k).collect();
    skip.extend(ArtistSplitRepository::known_names(pool).await?);
    let rules = artist_split::rules();
    let mut out = Vec::new();
    for raw in ComposerRepository::all_raw(pool).await? {
        for name in split(&raw, &rules) {
            if skip.insert(normalize_name(&name)) {
                out.push(name);
            }
        }
    }
    out.sort_by_key(|n| n.to_lowercase());
    Ok(out)
}

/// Enregistre le résultat d'une recherche : chemin de la photo, ou "" si introuvable.
pub async fn save_photo(pool: &SqlitePool, name: &str, path: &str) -> Result<(), sqlx::Error> {
    ComposerRepository::save_image(pool, &normalize_name(name), path).await
}

/// Nom du fichier de sa photo, stable d'une passe à l'autre.
pub fn photo_file(name: &str) -> String {
    format!("composer_{:x}.jpg", md5::compute(normalize_name(name)))
}

/// Les morceaux d'un compositeur, toutes écritures de son nom confondues.
pub async fn tracks(pool: &SqlitePool, library_id: i64, name: &str) -> Result<Vec<TrackListView>, sqlx::Error> {
    let wanted = normalize_name(name);
    let rules = artist_split::rules();
    let ids: Vec<String> = ComposerRepository::rows(pool, library_id)
        .await?
        .into_iter()
        .filter(|(_, _, _, raw)| split(raw, &rules).iter().any(|n| normalize_name(n) == wanted))
        .map(|(id, ..)| id)
        .collect();
    let mut tracks = LibraryTrackRepository::find_views_by_ids(pool, &ids).await?;
    tracks.sort_by(|a, b| {
        a.album.cmp(&b.album).then(a.disc_number.cmp(&b.disc_number)).then(a.track_number.cmp(&b.track_number))
    });
    Ok(tracks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs.iter().map(|(n, p)| (key(n), p.iter().map(|s| s.to_string()).collect())).collect()
    }

    #[test]
    fn splits_lists_and_drops_junk() {
        let k = rules(&[]);
        assert_eq!(split("Hans Zimmer, James Newton Howard", &k), ["Hans Zimmer", "James Newton Howard"]);
        assert_eq!(split("Scott Weiland,Robert DeLeo;Dean DeLeo / Eric Kretz", &k).len(), 4);
        assert!(split("Various Composers", &k).is_empty());
        assert_eq!(split("Mozart, Unknown, mozart", &k), ["Mozart"]);
        // « & » ne coupe pas : souvent un duo d'auteurs qu'on cherche ensemble.
        assert_eq!(split("Lennon & McCartney", &k), ["Lennon & McCartney"]);
    }

    #[test]
    fn the_multiple_artists_decision_wins() {
        let r = rules(&[("Earth, Wind & Fire", &["Earth, Wind & Fire"]), ("Hans Zimmer & Lisa Gerrard", &["Hans Zimmer", "Lisa Gerrard"])]);
        assert_eq!(split("Earth, Wind & Fire", &r), ["Earth, Wind & Fire"]);
        assert_eq!(split("Hans Zimmer & Lisa Gerrard", &r), ["Hans Zimmer", "Lisa Gerrard"]);
    }
}
