//! Annuaires de podcasts : une recherche par mots-clés, rendue en adresses de flux.
//! Apple pour commencer ; d'autres annuaires ouverts pourront s'ajouter à côté.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;

use super::fetch::client;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DirectoryResult {
    pub title: String,
    pub author: Option<String>,
    pub feed_url: String,
    pub image_url: Option<String>,
    pub genre: Option<String>,
    pub episode_count: Option<i64>,
    /// Date du dernier épisode (ISO 8601), pour le tri.
    pub released_at: Option<String>,
    /// Annuaire d'origine, affiché à côté du résultat.
    pub source: &'static str,
}

/// `country` : code pays à deux lettres, celui de la langue de l'appli.
pub async fn search(term: &str, country: &str) -> Result<Vec<DirectoryResult>, String> {
    let term = term.trim();
    if term.is_empty() {
        return Ok(Vec::new());
    }
    apple(term, country).await
}

fn country_code(country: &str) -> String {
    if country.len() == 2 && country.chars().all(|c| c.is_ascii_alphabetic()) {
        country.to_ascii_uppercase()
    } else {
        "US".to_string()
    }
}

type TopCache = HashMap<(String, Option<u32>), (Instant, Vec<DirectoryResult>)>;

/// Les plus écoutés du moment (classement Apple), par pays et catégorie Apple (`genre`).
/// Gardés 6 h : le classement bouge peu, et l'accueil des podcasts le redemande souvent.
pub async fn top(country: &str, genre: Option<u32>) -> Result<Vec<DirectoryResult>, String> {
    static CACHE: Mutex<Option<TopCache>> = Mutex::new(None);
    let key = (country_code(country), genre);
    if let Some((at, list)) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).get(&key) {
        if at.elapsed() < Duration::from_secs(6 * 3600) {
            return Ok(list.clone());
        }
    }
    let list = apple_top(&key.0, genre).await?;
    CACHE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(key, (Instant::now(), list.clone()));
    Ok(list)
}

async fn apple_top(country: &str, genre: Option<u32>) -> Result<Vec<DirectoryResult>, String> {
    // Le classement ne donne que des identifiants ; une seule recherche les rend en flux.
    let lower = country.to_ascii_lowercase();
    let chart = match genre {
        Some(g) => format!("https://itunes.apple.com/{lower}/rss/toppodcasts/limit=40/genre={g}/json"),
        None => format!("https://itunes.apple.com/{lower}/rss/toppodcasts/limit=40/json"),
    };
    let json: Value = client()
        .get(&chart)
        .send()
        .await
        .map_err(|e| format!("classement injoignable : {e}"))?
        .error_for_status()
        .map_err(|e| format!("classement refusé : {e}"))?
        .json()
        .await
        .map_err(|e| format!("classement illisible : {e}"))?;
    let ids = chart_ids(&json);
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let joined = ids.join(",");
    let url = reqwest::Url::parse_with_params(
        "https://itunes.apple.com/lookup",
        &[("id", joined.as_str()), ("entity", "podcast"), ("country", country)],
    )
    .map_err(|e| e.to_string())?;
    let found: Value = client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("annuaire injoignable : {e}"))?
        .json()
        .await
        .map_err(|e| format!("réponse de l'annuaire illisible : {e}"))?;
    // L'ordre du classement, pas celui de la réponse.
    let by_id: HashMap<String, DirectoryResult> = found
        .get("results")
        .and_then(Value::as_array)
        .map(|r| {
            r.iter()
                .filter_map(|x| Some((x.get("collectionId")?.as_i64()?.to_string(), parse_apple_one(x)?)))
                .collect()
        })
        .unwrap_or_default();
    Ok(dedup(ids.iter().filter_map(|id| by_id.get(id).cloned()).collect()))
}

/// Une émission n'apparaît qu'une fois : le classement en cite parfois deux fiches au même flux.
fn dedup(list: Vec<DirectoryResult>) -> Vec<DirectoryResult> {
    let mut seen = std::collections::HashSet::new();
    list.into_iter().filter(|r| seen.insert(r.feed_url.clone())).collect()
}

fn chart_ids(json: &Value) -> Vec<String> {
    json.pointer("/feed/entry")
        .and_then(Value::as_array)
        .map(|e| e.iter().filter_map(|x| x.pointer("/id/attributes/im:id")?.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

async fn apple(term: &str, country: &str) -> Result<Vec<DirectoryResult>, String> {
    let country = country_code(country);
    let url = reqwest::Url::parse_with_params(
        "https://itunes.apple.com/search",
        &[("media", "podcast"), ("entity", "podcast"), ("limit", "30"), ("term", term), ("country", country.as_str())],
    )
    .map_err(|e| e.to_string())?;
    let json: Value = client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("annuaire injoignable : {e}"))?
        .error_for_status()
        .map_err(|e| format!("annuaire indisponible : {e}"))?
        .json()
        .await
        .map_err(|e| format!("réponse de l'annuaire illisible : {e}"))?;
    Ok(parse_apple(&json))
}

fn parse_apple(json: &Value) -> Vec<DirectoryResult> {
    json.get("results")
        .and_then(Value::as_array)
        .map(|results| dedup(results.iter().filter_map(parse_apple_one).collect()))
        .unwrap_or_default()
}

/// Un résultat Apple ; écarté sans flux public (émission réservée à l'appli de l'éditeur).
fn parse_apple_one(r: &Value) -> Option<DirectoryResult> {
    let text = |k: &str| r.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    Some(DirectoryResult {
        title: text("collectionName")?,
        feed_url: text("feedUrl")?,
        author: text("artistName"),
        image_url: text("artworkUrl600").or_else(|| text("artworkUrl100")),
        genre: text("primaryGenreName"),
        episode_count: r.get("trackCount").and_then(Value::as_i64),
        released_at: text("releaseDate"),
        source: "apple",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_results_that_have_a_feed() {
        let json = serde_json::json!({ "resultCount": 2, "results": [
            { "collectionName": "Affaires sensibles", "artistName": "France Inter", "feedUrl": "https://radiofrance-podcast.net/podcast09/rss_13940.xml",
              "artworkUrl600": "https://img/600.jpg", "primaryGenreName": "Histoire", "trackCount": 312,
              "releaseDate": "2026-10-20T04:00:00Z" },
            { "collectionName": "Exclusivité sans flux", "artistName": "X" }
        ]});
        let r = parse_apple(&json);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].title, "Affaires sensibles");
        assert_eq!(r[0].episode_count, Some(312));
        assert_eq!(r[0].released_at.as_deref(), Some("2026-10-20T04:00:00Z"));
        assert_eq!(r[0].image_url.as_deref(), Some("https://img/600.jpg"));
        assert!(parse_apple(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn reads_chart_ids_in_order() {
        let json = serde_json::json!({ "feed": { "entry": [
            { "id": { "attributes": { "im:id": "12" } } },
            { "id": { "attributes": { "im:id": "7" } } },
            { "title": "sans id" }
        ]}});
        assert_eq!(chart_ids(&json), ["12", "7"]);
        let twice = serde_json::json!({ "results": [
            { "collectionName": "A", "feedUrl": "https://x/rss" },
            { "collectionName": "A bis", "feedUrl": "https://x/rss" }
        ]});
        assert_eq!(parse_apple(&twice).len(), 1);
        assert!(chart_ids(&serde_json::json!({})).is_empty());
    }
}
