//! Annuaire Radio Browser (libre, sans clé) : recherche et stations populaires par pays et genre.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::podcast::fetch::client;

/// Une station, telle que l'annuaire (ou une adresse collée) la décrit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RadioResult {
    /// Identifiant Radio Browser ; `None` pour une adresse collée.
    pub uuid: Option<String>,
    pub name: String,
    pub stream_url: String,
    pub homepage: Option<String>,
    pub favicon: Option<String>,
    pub country_code: Option<String>,
    pub tags: Vec<String>,
    pub codec: Option<String>,
    pub bitrate: Option<u32>,
    /// AAC+ (HE-AAC) : le décodeur n'en lit que la base, le son est étouffé.
    pub reduced_quality: bool,
}

/// Miroirs connus, si la liste officielle ne répond pas.
const FALLBACK_SERVERS: &[&str] = &["de1.api.radio-browser.info", "de2.api.radio-browser.info", "fi1.api.radio-browser.info"];
const LIMIT: &str = "60";

/// Les miroirs de l'API ; la liste officielle d'abord, gardée pour la session.
async fn servers() -> Vec<String> {
    static SERVERS: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let known = SERVERS.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if !known.is_empty() {
        return known;
    }
    let listed: Vec<String> = async {
        let v: Value = client()
            .get("https://all.api.radio-browser.info/json/servers")
            .timeout(Duration::from_secs(8))
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;
        Some(v.as_array()?.iter().filter_map(|s| s.get("name")?.as_str().map(str::to_string)).collect())
    }
    .await
    .unwrap_or_default();
    let mut list: Vec<String> = if listed.is_empty() { FALLBACK_SERVERS.iter().map(|s| s.to_string()).collect() } else { listed };
    list.sort();
    list.dedup();
    // Départ différent d'une session à l'autre : la charge se répartit entre miroirs.
    let start = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as usize).unwrap_or(0) % list.len().max(1);
    list.rotate_left(start);
    *SERVERS.lock().unwrap_or_else(|e| e.into_inner()) = list.clone();
    list
}

/// Requête sur un miroir ; le suivant prend le relais s'il ne répond pas.
async fn api(path: &str, params: &[(&str, &str)]) -> Result<Value, String> {
    let mut last = String::from("annuaire des radios injoignable");
    for server in servers().await.iter().take(3) {
        let url = reqwest::Url::parse_with_params(&format!("https://{server}{path}"), params).map_err(|e| e.to_string())?;
        match client().get(url).timeout(Duration::from_secs(15)).send().await.and_then(|r| r.error_for_status()) {
            Ok(resp) => return resp.json().await.map_err(|e| format!("réponse de l'annuaire illisible : {e}")),
            Err(e) => last = format!("annuaire des radios injoignable : {e}"),
        }
    }
    Err(last)
}

fn country_code(country: &str) -> Option<String> {
    (country.len() == 2 && country.chars().all(|c| c.is_ascii_alphabetic())).then(|| country.to_ascii_uppercase())
}

/// Recherche par nom ; `country` et `tag` restreignent si fournis.
pub async fn search(term: &str, country: Option<&str>, tag: Option<&str>) -> Result<Vec<RadioResult>, String> {
    let term = term.trim();
    if term.is_empty() {
        return Ok(Vec::new());
    }
    let cc = country.and_then(country_code);
    let mut params = vec![("name", term), ("order", "clickcount"), ("reverse", "true"), ("hidebroken", "true"), ("limit", LIMIT)];
    if let Some(cc) = cc.as_deref() {
        params.push(("countrycode", cc));
    }
    if let Some(tag) = tag.filter(|t| !t.is_empty()) {
        params.push(("tag", tag));
    }
    Ok(parse_stations(&api("/json/stations/search", &params).await?))
}

type TopCache = HashMap<(String, Option<String>), (Instant, Vec<RadioResult>)>;

/// Les plus écoutées d'un pays, d'un genre ; gardées 6 h.
pub async fn top(country: &str, tag: Option<&str>) -> Result<Vec<RadioResult>, String> {
    static CACHE: Mutex<Option<TopCache>> = Mutex::new(None);
    let cc = country_code(country).unwrap_or_else(|| "US".into());
    let tag = tag.filter(|t| !t.is_empty()).map(str::to_string);
    let key = (cc.clone(), tag.clone());
    if let Some((at, list)) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).get(&key) {
        if at.elapsed() < Duration::from_secs(6 * 3600) {
            return Ok(list.clone());
        }
    }
    let mut params = vec![("countrycode", cc.as_str()), ("order", "clickcount"), ("reverse", "true"), ("hidebroken", "true"), ("limit", LIMIT)];
    if let Some(t) = tag.as_deref() {
        params.push(("tag", t));
    }
    let list = parse_stations(&api("/json/stations/search", &params).await?);
    CACHE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(key, (Instant::now(), list.clone()));
    Ok(list)
}

/// La station de l'annuaire qui diffuse à cette adresse, s'il la connaît.
pub async fn by_url(url: &str) -> Option<RadioResult> {
    let json = api("/json/stations/byurl", &[("url", url)]).await.ok()?;
    // Plusieurs fiches pour un même flux : la plus écoutée porte le bon nom.
    let mut list = json.as_array()?.clone();
    list.sort_by_key(|s| std::cmp::Reverse(s.get("clickcount").and_then(Value::as_i64).unwrap_or(0)));
    parse_stations(&Value::Array(list)).into_iter().next()
}

/// Signale une écoute à l'annuaire (classement des stations) ; sans conséquence si ça échoue.
pub async fn count_click(uuid: &str) {
    let _ = api(&format!("/json/url/{uuid}"), &[]).await;
}

/// Genres seulement : l'annuaire y mêle format, débit et mots creux (« aac », « 128k », « radio »).
pub(crate) fn clean_tags(raw: &str) -> Vec<String> {
    const NOISE: &[&str] = &["aac", "aac+", "aacp", "mp3", "ogg", "opus", "flac", "hls", "radio", "webradio", "internet radio", "fm", "am", "stream", "live", "online"];
    raw.split([',', ';'])
        .map(|x| x.trim().to_lowercase())
        .filter(|x| !x.is_empty() && !NOISE.contains(&x.as_str()))
        .filter(|x| !(x.ends_with('k') || x.ends_with("kbps")) || !x.starts_with(|c: char| c.is_ascii_digit()))
        .take(6)
        .collect()
}

fn is_reduced(codec: Option<&str>) -> bool {
    codec.is_some_and(|c| {
        let c = c.to_ascii_lowercase();
        c.contains("aac+") || c.contains("aacp") || c.contains("he-aac")
    })
}

/// Complète `reduced_quality` d'après le format.
pub(crate) fn with_quality(mut r: RadioResult) -> RadioResult {
    r.reduced_quality = is_reduced(r.codec.as_deref());
    r
}

/// Stations de la réponse, sans HLS (pas encore lu) ni doublons.
fn parse_stations(json: &Value) -> Vec<RadioResult> {
    let Some(list) = json.as_array() else { return Vec::new() };
    let text = |s: &Value, k: &str| s.get(k).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    let stations = list.iter().filter_map(|s| {
        if s.get("hls").and_then(Value::as_i64) == Some(1) {
            return None;
        }
        let url = text(s, "url_resolved").or_else(|| text(s, "url"))?;
        if !url.starts_with("http") {
            return None;
        }
        let tags = text(s, "tags").map(|t| clean_tags(&t)).unwrap_or_default();
        let bitrate = s.get("bitrate").and_then(Value::as_u64).filter(|&b| b > 0).map(|b| b as u32);
        Some(with_quality(RadioResult {
            uuid: text(s, "stationuuid"),
            name: text(s, "name")?,
            stream_url: url,
            homepage: text(s, "homepage").filter(|h| h.starts_with("http")),
            favicon: text(s, "favicon").filter(|f| f.starts_with("http")),
            country_code: text(s, "countrycode"),
            tags,
            codec: text(s, "codec"),
            bitrate,
            reduced_quality: false,
        }))
    });
    prefer_best(stations.collect())
}

/// Une station listée plusieurs fois (formats, débits) : la meilleure version, à la place de la première.
fn prefer_best(list: Vec<RadioResult>) -> Vec<RadioResult> {
    let key = |r: &RadioResult| (r.name.trim().to_lowercase(), r.country_code.clone());
    let score = |r: &RadioResult| (!r.reduced_quality, r.bitrate.unwrap_or(0));
    let mut best: HashMap<(String, Option<String>), usize> = HashMap::new();
    let mut out: Vec<RadioResult> = Vec::new();
    for r in list {
        match best.get(&key(&r)) {
            Some(&i) => {
                if score(&r) > score(&out[i]) {
                    out[i] = r;
                }
            }
            None => {
                best.insert(key(&r), out.len());
                out.push(r);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_best_version_of_a_station() {
        let json = serde_json::json!([
            { "stationuuid": "a", "name": "FIP", "url_resolved": "https://x/fip-hifi.aac", "countrycode": "FR", "codec": "AAC+", "bitrate": 64, "tags": "jazz,Eclectic" },
            { "stationuuid": "b", "name": "Radio HLS", "url": "https://x/live.m3u8", "hls": 1 },
            { "stationuuid": "c", "name": "fip ", "url_resolved": "https://x/fip-midfi.mp3", "countrycode": "FR", "codec": "MP3", "bitrate": 128 },
            { "stationuuid": "d", "name": "Sans flux" }
        ]);
        let r = parse_stations(&json);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].uuid.as_deref(), Some("c"));
        assert!(!r[0].reduced_quality);
    }

    #[test]
    fn flags_he_aac_as_reduced() {
        let json = serde_json::json!([{ "name": "Lo-fi", "url": "http://x/s", "codec": "AAC+", "tags": "Lofi, chill ,", "bitrate": 0 }]);
        let r = parse_stations(&json);
        assert!(r[0].reduced_quality);
        assert_eq!(r[0].tags, vec!["lofi", "chill"]);
        assert_eq!(clean_tags("AAC, News,128k, radio;Talk,192kbps"), vec!["news", "talk"]);
        assert_eq!(r[0].bitrate, None);
    }

    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_directory() {
        let top = top("FR", None).await.unwrap();
        let jazz = search("jazz", Some("FR"), None).await.unwrap();
        println!("Top FR : {} ({:?}…), jazz : {}", top.len(), top.iter().take(5).map(|r| &r.name).collect::<Vec<_>>(), jazz.len());
        assert!(!top.is_empty());
    }
}
