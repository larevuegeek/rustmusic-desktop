//! Adresse de flux collée : listes .m3u/.pls suivies, en-têtes ICY lus pour décrire la station.

use std::time::Duration;

use reqwest::header::{HeaderMap, CONTENT_TYPE};

use super::directory::{with_quality, RadioResult};
use crate::core::podcast::fetch::{client, normalize_url};

/// Une liste de lecture tient en quelques lignes ; au-delà, c'est le flux lui-même.
const MAX_PLAYLIST: usize = 64 * 1024;

enum Kind {
    Audio,
    Playlist,
    Html,
    Unknown,
}

fn kind(content_type: &str, url: &str) -> Kind {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    if content_type.contains("mpegurl") || content_type.contains("scpls") || [".m3u", ".m3u8", ".pls"].iter().any(|e| path.ends_with(e)) {
        Kind::Playlist
    } else if content_type.starts_with("audio/") || content_type == "application/ogg" {
        Kind::Audio
    } else if content_type.starts_with("text/html") {
        Kind::Html
    } else {
        Kind::Unknown
    }
}

/// Décrit la station derrière `raw` ; connue de l'annuaire, elle en prend nom, logo et genres.
pub async fn resolve(raw: &str) -> Result<RadioResult, String> {
    let found = probe(raw).await?;
    // L'annuaire garde l'adresse telle qu'on la lui a donnée : http ou https.
    let other = match found.stream_url.strip_prefix("https://") {
        Some(rest) => format!("http://{rest}"),
        None => found.stream_url.replacen("http://", "https://", 1),
    };
    for url in [found.stream_url.as_str(), other.as_str()] {
        if let Some(known) = super::directory::by_url(url).await {
            return Ok(RadioResult { stream_url: found.stream_url.clone(), ..known });
        }
    }
    Ok(found)
}

/// Flux direct, ou première adresse d'une liste, décrit par ses en-têtes.
async fn probe(raw: &str) -> Result<RadioResult, String> {
    let mut url = normalize_url(raw).map_err(|_| "adresse de radio invalide".to_string())?;
    for _ in 0..4 {
        let mut resp = client()
            .get(&url)
            .header("Icy-MetaData", "1")
            .timeout(Duration::from_secs(12))
            .send()
            .await
            .map_err(|e| format!("radio injoignable : {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("radio refusée : HTTP {}", resp.status().as_u16()));
        }
        let content_type = resp.headers().get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_ascii_lowercase();
        match kind(&content_type, resp.url().as_str()) {
            Kind::Audio => return Ok(describe(&url, resp.headers(), &content_type)),
            Kind::Html => return Err("cette adresse est une page web, pas un flux audio".into()),
            k => {
                let limit = if matches!(k, Kind::Playlist) { MAX_PLAYLIST } else { 4096 };
                let mut body = Vec::new();
                while body.len() < limit {
                    match resp.chunk().await {
                        Ok(Some(c)) => body.extend_from_slice(&c),
                        _ => break,
                    }
                }
                let text = String::from_utf8_lossy(&body[..body.len().min(limit)]);
                if text.contains("#EXT-X-") {
                    return Ok(describe_hls(&url, &text, resp.headers()));
                }
                match first_url(&text) {
                    Some(next) => url = next,
                    // Type inconnu sans adresse dedans : c'est le son lui-même.
                    None if matches!(k, Kind::Unknown) => return Ok(describe(&url, resp.headers(), &content_type)),
                    None => return Err("liste de lecture vide".into()),
                }
            }
        }
    }
    Err("trop de listes imbriquées".into())
}

/// Première adresse d'une liste .m3u (lignes) ou .pls (`File1=`).
fn first_url(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .map(|l| match l.split_once('=') {
            Some((k, v)) if k.trim().to_ascii_lowercase().starts_with("file") => v.trim(),
            _ => l,
        })
        .find(|l| l.starts_with("http://") || l.starts_with("https://"))
        .map(str::to_string)
}

fn codec_of(content_type: &str) -> Option<String> {
    let c = match content_type.split(';').next().unwrap_or("").trim() {
        "audio/mpeg" | "audio/mp3" => "MP3",
        "audio/aacp" => "AAC+",
        "audio/aac" | "audio/x-aac" => "AAC",
        "audio/ogg" | "application/ogg" => "OGG",
        "audio/opus" => "OPUS",
        "audio/flac" | "audio/x-flac" => "FLAC",
        _ => return None,
    };
    Some(c.into())
}

/// Station HLS : nom d'après l'hôte, format et débit d'après la meilleure variante.
fn describe_hls(url: &str, text: &str, headers: &HeaderMap) -> RadioResult {
    let mut r = describe(url, headers, "");
    if let Ok(super::hls::Playlist::Master(variants)) = super::hls::parse(text, url) {
        if let Some(v) = super::hls::pick(&variants) {
            r.codec = super::hls::codec_label(v.codecs.as_deref()).map(str::to_string);
            r.bitrate = (v.bandwidth > 0).then_some((v.bandwidth / 1000) as u32);
        }
    }
    with_quality(r)
}

fn describe(url: &str, headers: &HeaderMap, content_type: &str) -> RadioResult {
    let icy = |k: &str| headers.get(k).and_then(|v| v.to_str().ok()).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    let host = reqwest::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_else(|| url.to_string());
    let tags = icy("icy-genre").map(|g| super::directory::clean_tags(&g.replace('/', ","))).unwrap_or_default();
    // « 128 » ou « 128,128 » selon les serveurs.
    let bitrate = icy("icy-br").and_then(|b| b.split(',').next()?.trim().parse().ok()).filter(|&b: &u32| b > 0);
    // Certains serveurs annoncent le nom du fichier (« fip-midfi.mp3 ») : l'hôte vaut mieux.
    let name = icy("icy-name").filter(|n| {
        let n = n.to_ascii_lowercase();
        ![".mp3", ".aac", ".ogg", ".opus", ".m4a", ".flac"].iter().any(|e| n.ends_with(e))
    });
    with_quality(RadioResult {
        variants: Vec::new(),
        uuid: None,
        name: name.unwrap_or(host),
        stream_url: url.to_string(),
        homepage: icy("icy-url").filter(|h| h.starts_with("http")),
        favicon: None,
        country_code: None,
        tags,
        codec: codec_of(content_type),
        bitrate,
        reduced_quality: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn reads_m3u_and_pls() {
        assert_eq!(first_url("#EXTM3U\n#EXTINF:-1,FIP\nhttps://icecast.radiofrance.fr/fip-midfi.mp3\n").as_deref(), Some("https://icecast.radiofrance.fr/fip-midfi.mp3"));
        assert_eq!(first_url("[playlist]\nNumberOfEntries=1\nFile1=http://stream.x/live\nTitle1=X\n").as_deref(), Some("http://stream.x/live"));
        assert_eq!(first_url("rien ici"), None);
    }

    #[test]
    fn recognises_playlists_by_type_or_extension() {
        assert!(matches!(kind("audio/x-mpegurl", "http://x/s"), Kind::Playlist));
        assert!(matches!(kind("application/octet-stream", "http://x/radio.pls?t=1"), Kind::Playlist));
        assert!(matches!(kind("audio/mpeg", "http://x/live"), Kind::Audio));
        assert!(matches!(kind("text/html; charset=utf-8", "http://x/"), Kind::Html));
    }

    #[test]
    fn describes_from_icy_headers() {
        let mut h = HeaderMap::new();
        h.insert("icy-name", HeaderValue::from_static("Radio Test"));
        h.insert("icy-genre", HeaderValue::from_static("Jazz; Soul"));
        h.insert("icy-br", HeaderValue::from_static("128,128"));
        h.insert("icy-url", HeaderValue::from_static("https://radio.test"));
        let r = describe("http://s.radio.test/live", &h, "audio/aacp");
        assert_eq!(r.name, "Radio Test");
        assert_eq!(r.tags, vec!["jazz", "soul"]);
        assert_eq!(r.bitrate, Some(128));
        assert_eq!(r.codec.as_deref(), Some("AAC+"));
        assert!(r.reduced_quality);
        assert_eq!(describe("http://s.radio.test/live", &HeaderMap::new(), "audio/mpeg").name, "s.radio.test");
    }

    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_stream() {
        let r = resolve(&std::env::var("RADIO").unwrap_or_else(|_| "https://icecast.radiofrance.fr/fip-midfi.mp3".into())).await.unwrap();
        println!("{r:#?}");
    }
}
