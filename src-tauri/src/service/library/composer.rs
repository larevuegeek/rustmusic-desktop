//! Compositeurs d'une bibliothèque, tirés du tag `composer` : découpés, regroupés, reliés aux artistes connus.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use sqlx::SqlitePool;

use crate::entity::search::search_result::SearchResult;
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
pub const JUNK: &[&str] = &[
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

/// Recherche globale : les compositeurs dont le nom contient `query`, chacun une fois (sa première bibliothèque).
pub async fn search(pool: &SqlitePool, query: &str, limit: usize, profil_id: i64, source: Option<i64>) -> Result<Vec<SearchResult>, sqlx::Error> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let rows = ComposerRepository::search_raw(pool, &format!("%{q}%"), profil_id, source).await?;
    let rules = artist_split::rules();
    let photos = photos(pool).await;
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (library_id, raw) in rows {
        for name in split(&raw, &rules) {
            let k = normalize_name(&name);
            if name.to_lowercase().contains(&q) && seen.insert(k.clone()) {
                out.push(SearchResult {
                    id: name.clone(),
                    result_type: "composer".into(),
                    title: name,
                    subtitle: None,
                    thumbnail_path: photos.get(&k).cloned(),
                    path: None,
                    library_id: Some(library_id),
                });
            }
        }
    }
    // Ceux qui commencent par la recherche d'abord.
    out.sort_by_key(|r| (!r.title.to_lowercase().starts_with(&q), r.title.to_lowercase()));
    out.truncate(limit);
    Ok(out)
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

    /// Bibliothèque 5 : quatre morceaux à compositeur, un sans ; la bibliothèque 6 ne compte pas.
    async fn library() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        for lib in [5, 6] {
            sqlx::query("INSERT INTO library (id, profil_id, name) VALUES (?, 1, ?)").bind(lib).bind(format!("L{lib}")).execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO artists (id, name, name_normalized, sort_name, image_url) VALUES ('hz', 'Hans Zimmer', 'hans zimmer', 'Hans Zimmer', '/p/hz.jpg')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library_artists (library_id, artist_id) VALUES (5, 'hz')").execute(&pool).await.unwrap();
        for a in ["a1", "a2"] {
            sqlx::query("INSERT INTO library_albums (id, library_id, artist_id, title, title_normalized) VALUES (?, 5, 'hz', ?, ?)")
                .bind(a).bind(a).bind(a).execute(&pool).await.unwrap();
        }
        let tracks: [(&str, i64, Option<&str>, Option<&str>, f64); 6] = [
            ("t1", 5, Some("a1"), Some("Hans Zimmer, James Newton Howard"), 100.0),
            ("t2", 5, Some("a2"), Some("hans zimmer"), 50.0),
            ("t3", 5, Some("a2"), Some("Various Composers"), 10.0),
            ("t4", 5, Some("a2"), Some("Lisa Gerrard"), 20.0),
            ("t5", 5, Some("a2"), None, 30.0),
            ("t6", 6, None, Some("Hans Zimmer"), 40.0),
        ];
        for (id, lib, album, composer, duration) in tracks {
            let tags = composer.map(|c| serde_json::json!({ "composer": c }).to_string());
            sqlx::query("INSERT INTO library_files (id, library_id, path, filename) VALUES (?, ?, ?, 'x')")
                .bind(format!("f{id}")).bind(lib).bind(format!("/m/{id}.flac")).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, title, title_normalized, duration, library_album_id, tags) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(id).bind(lib).bind(format!("f{id}")).bind(id).bind(id).bind(duration).bind(album).bind(tags)
                .execute(&pool).await.unwrap();
        }
        ComposerRepository::save_image(&pool, &normalize_name("James Newton Howard"), "/p/jnh.jpg").await.unwrap();
        ComposerRepository::save_image(&pool, &normalize_name("Lisa Gerrard"), "").await.unwrap();
        pool
    }

    #[tokio::test]
    async fn lists_the_composers_of_one_library() {
        let pool = library().await;
        let mut all = list(&pool, 5).await.unwrap();
        all.sort_by(|a, b| a.name.cmp(&b.name));
        let names: Vec<&str> = all.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Hans Zimmer", "James Newton Howard", "Lisa Gerrard"]);

        // Deux écritures, deux albums ; photo et page de l'artiste du même nom.
        let hz = &all[0];
        assert_eq!((hz.total_tracks, hz.total_albums, hz.total_duration), (2, 2, 150.0));
        assert_eq!((hz.thumbnail_path.as_deref(), hz.artist_id.as_deref()), (Some("/p/hz.jpg"), Some("hz")));
        // Photo de compositeur ; cherchée sans résultat = pas de photo.
        assert_eq!(all[1].thumbnail_path.as_deref(), Some("/p/jnh.jpg"));
        assert_eq!(all[2].thumbnail_path, None);
    }

    #[tokio::test]
    async fn finds_the_tracks_of_a_composer() {
        let pool = library().await;
        let ids = |t: Vec<TrackListView>| t.into_iter().map(|t| t.id).collect::<Vec<_>>();
        assert_eq!(ids(tracks(&pool, 5, "HANS ZIMMER").await.unwrap()), ["t1", "t2"]);
        assert_eq!(ids(tracks(&pool, 5, "James Newton Howard").await.unwrap()), ["t1"]);
        assert!(tracks(&pool, 5, "Various Composers").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn the_global_search_finds_each_composer_once() {
        let pool = library().await;
        let titles = |r: Vec<SearchResult>| r.into_iter().map(|r| (r.title, r.library_id)).collect::<Vec<_>>();
        // Deux bibliothèques, deux écritures : un seul résultat.
        assert_eq!(titles(search(&pool, "ZIM", 5, 1, None).await.unwrap()), [("Hans Zimmer".to_string(), Some(5))]);
        assert_eq!(titles(search(&pool, "howard", 5, 1, None).await.unwrap()), [("James Newton Howard".to_string(), Some(5))]);
        assert!(search(&pool, "various", 5, 1, None).await.unwrap().is_empty());
        assert!(search(&pool, "zim", 5, 2, None).await.unwrap().is_empty(), "autre profil");
    }

    #[tokio::test]
    async fn only_unsearched_composers_go_to_deezer() {
        let pool = library().await;
        // Hans Zimmer a une fiche d'artiste, James et Lisa ont déjà été cherchés.
        assert!(without_photo(&pool).await.unwrap().is_empty());
        ComposerRepository::reset_not_found(&pool).await.unwrap();
        assert_eq!(without_photo(&pool).await.unwrap(), ["Lisa Gerrard"]);
        assert_eq!(known_photo(&pool, "james newton howard").await.as_deref(), Some("/p/jnh.jpg"));
    }

    #[test]
    fn the_multiple_artists_decision_wins() {
        let r = rules(&[("Earth, Wind & Fire", &["Earth, Wind & Fire"]), ("Hans Zimmer & Lisa Gerrard", &["Hans Zimmer", "Lisa Gerrard"])]);
        assert_eq!(split("Earth, Wind & Fire", &r), ["Earth, Wind & Fire"]);
        assert_eq!(split("Hans Zimmer & Lisa Gerrard", &r), ["Hans Zimmer", "Lisa Gerrard"]);
    }
}
