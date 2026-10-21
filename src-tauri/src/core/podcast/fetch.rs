//! Téléchargement des flux : poli (validateurs HTTP), borné en taille et en temps.

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use reqwest::StatusCode;

/// Au-delà, ce n'est plus un flux : des archives de plusieurs milliers d'épisodes font ~20 Mo.
const MAX_BYTES: usize = 40 * 1024 * 1024;

pub struct Fetched {
    pub body: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

pub enum Outcome {
    /// Flux inchangé depuis la dernière visite.
    NotModified,
    Fresh(Fetched),
}

/// Client partagé : un seul pool de connexions pour flux, annuaire et pochettes.
pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(concat!("RustMusic/", env!("CARGO_PKG_VERSION"), " (+https://rustmusic.dev)"))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(45))
            .build()
            .unwrap_or_default()
    })
}

pub async fn fetch(url: &str, etag: Option<&str>, last_modified: Option<&str>) -> Result<Outcome, String> {
    let mut req = client().get(url);
    if let Some(e) = etag {
        req = req.header(IF_NONE_MATCH, e);
    }
    if let Some(l) = last_modified {
        req = req.header(IF_MODIFIED_SINCE, l);
    }
    let mut resp = req.send().await.map_err(|e| format!("flux injoignable : {e}"))?;
    if resp.status() == StatusCode::NOT_MODIFIED {
        return Ok(Outcome::NotModified);
    }
    if !resp.status().is_success() {
        return Err(format!("flux refusé : HTTP {}", resp.status().as_u16()));
    }
    if resp.content_length().is_some_and(|l| l as usize > MAX_BYTES) {
        return Err("flux trop volumineux".into());
    }
    let header = |name: reqwest::header::HeaderName| resp.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
    let (etag, last_modified) = (header(ETAG), header(LAST_MODIFIED));

    let mut bytes = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("flux interrompu : {e}"))? {
        if bytes.len() + chunk.len() > MAX_BYTES {
            return Err("flux trop volumineux".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(Outcome::Fresh(Fetched { body: decode_body(bytes), etag, last_modified }))
}

/// UTF-8 le plus souvent ; sinon Latin-1, l'autre encodage des vieux flux.
fn decode_body(bytes: Vec<u8>) -> String {
    let text = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => e.into_bytes().iter().map(|&b| b as char).collect(),
    };
    text.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(text)
}

/// Adresse saisie ou collée : schémas des applis de podcasts (`feed://`, `itpc://`…) ramenés à HTTPS.
pub fn normalize_url(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    // `feed:https://…` : l'adresse réelle suit le préfixe.
    let raw = raw.strip_prefix("feed:").filter(|r| r.starts_with("http")).unwrap_or(raw);
    let lower = raw.to_ascii_lowercase();
    let url = if lower.starts_with("http://") || lower.starts_with("https://") {
        raw.to_string()
    } else if let Some(rest) = ["feed://", "itpc://", "pcast://", "podcast://"].iter().find_map(|s| {
        lower.starts_with(s).then(|| &raw[s.len()..])
    }) {
        let rest = rest.trim_start_matches("http://").trim_start_matches("https://");
        format!("https://{rest}")
    } else if !raw.contains("://") && raw.contains('.') && !raw.contains(' ') {
        format!("https://{raw}")
    } else {
        return Err("adresse de flux invalide".into());
    };
    reqwest::Url::parse(&url).map_err(|_| "adresse de flux invalide".to_string())?;
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_podcast_app_schemes() {
        assert_eq!(normalize_url(" https://ex.com/feed ").unwrap(), "https://ex.com/feed");
        assert_eq!(normalize_url("http://ex.com/feed").unwrap(), "http://ex.com/feed");
        assert_eq!(normalize_url("feed://ex.com/rss").unwrap(), "https://ex.com/rss");
        assert_eq!(normalize_url("itpc://ex.com/rss").unwrap(), "https://ex.com/rss");
        assert_eq!(normalize_url("feed:https://ex.com/rss").unwrap(), "https://ex.com/rss");
        assert_eq!(normalize_url("ex.com/rss.xml").unwrap(), "https://ex.com/rss.xml");
        assert!(normalize_url("pas une adresse").is_err());
        assert!(normalize_url("ftp://ex.com/rss").is_err());
    }

    /// Vrais flux et vrai annuaire : `cargo test --lib real_feeds -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_feeds() {
        let top = super::super::directory::top("fr", None).await.unwrap();
        let history = super::super::directory::top("fr", Some(1487)).await.unwrap();
        println!("Top FR : {} ({:?}…), Histoire : {} ({:?}…)", top.len(), top.iter().take(3).map(|r| &r.title).collect::<Vec<_>>(), history.len(), history.first().map(|r| &r.title));
        let found = super::super::directory::search("binge audio", "FR").await.unwrap();
        println!("Apple : {} résultats, 1er : {:?}", found.len(), found.first().map(|r| (&r.title, &r.feed_url)));
        let mut urls: Vec<String> = found.iter().take(3).map(|r| r.feed_url.clone()).collect();
        urls.extend(super::super::directory::search("the daily", "US").await.unwrap().into_iter().take(1).map(|r| r.feed_url));
        urls.extend(super::super::directory::search("podcast science", "FR").await.unwrap().into_iter().take(2).map(|r| r.feed_url));
        for url in urls {
            match fetch(&url, None, None).await {
                Ok(Outcome::Fresh(f)) => match super::super::feed::parse(&f.body) {
                    Ok(feed) => {
                        let e = feed.episodes.first();
                        println!(
                            "OK {url}\n   « {} » — {} épisodes, image {}, etag {}, 1er : {:?} / {:?} / {:?} s",
                            feed.title, feed.episodes.len(), feed.image_url.is_some(), f.etag.is_some(),
                            e.map(|e| &e.title), e.and_then(|e| e.published_at), e.and_then(|e| e.duration)
                        );
                        if let Some(et) = f.etag.as_deref() {
                            let again = fetch(&url, Some(et), f.last_modified.as_deref()).await;
                            println!("   revisite : {}", if matches!(again, Ok(Outcome::NotModified)) { "304 inchangé" } else { "retéléchargé" });
                        }
                    }
                    Err(e) => println!("ILLISIBLE {url} : {e}"),
                },
                Ok(Outcome::NotModified) => println!("304 ? {url}"),
                Err(e) => println!("ÉCHEC {url} : {e}"),
            }
        }
    }

    #[test]
    fn decodes_latin1_and_bom() {
        assert_eq!(decode_body(b"\xef\xbb\xbf<rss>".to_vec()), "<rss>");
        assert_eq!(decode_body(b"caf\xe9".to_vec()), "café");
    }
}
