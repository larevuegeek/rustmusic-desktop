//! Chapitres d'un épisode : fichier JSON (Podcasting 2.0), liste du flux (Podlove) ou trames ID3 du MP3.

use std::io::Read;

use serde::{Deserialize, Serialize};

use crate::core::audio_metadata::tag_format::id3v2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    /// Secondes depuis le début de l'épisode.
    pub start: f64,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Rangés dans l'ordre, doublons de début retirés ; moins de deux chapitres n'en font pas.
pub fn tidy(mut list: Vec<Chapter>) -> Vec<Chapter> {
    list.retain(|c| c.start.is_finite() && c.start >= 0.0);
    list.sort_by(|a, b| a.start.total_cmp(&b.start));
    list.dedup_by(|b, a| (b.start - a.start).abs() < 0.5);
    if list.len() < 2 {
        list.clear();
    }
    list
}

/// `{"chapters": [{"startTime": 0, "title": "…", "url": "…", "toc": false}]}` ; `toc: false` = caché de la liste.
pub fn parse_json(body: &str) -> Vec<Chapter> {
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        chapters: Vec<Entry>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Entry {
        start_time: f64,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        toc: Option<bool>,
    }
    let Ok(file) = serde_json::from_str::<File>(body) else { return Vec::new() };
    tidy(
        file.chapters
            .into_iter()
            .filter(|e| e.toc != Some(false))
            .map(|e| Chapter { start: e.start_time, title: e.title.unwrap_or_default().trim().to_string(), url: web_url(e.url) })
            .collect(),
    )
}

/// `01:02:03.500`, `02:03`, `123.5` → secondes.
pub fn parse_time(raw: &str) -> Option<f64> {
    let mut total = 0.0;
    for part in raw.trim().split(':') {
        let v: f64 = part.trim().parse().ok()?;
        total = total * 60.0 + v;
    }
    (total >= 0.0).then_some(total)
}

/// Lien de chapitre : site web seulement.
pub fn web_url(url: Option<String>) -> Option<String> {
    url.map(|u| u.trim().to_string()).filter(|u| u.starts_with("https://") || u.starts_with("http://"))
}

/// Trames CHAP en tête de fichier (le tag ID3 y est). `None` : tag illisible ou incomplet.
pub fn from_reader(mut reader: impl Read) -> Option<Vec<Chapter>> {
    let mut head = [0u8; 10];
    reader.read_exact(&mut head).ok()?;
    if &head[0..3] != b"ID3" {
        return Some(Vec::new());
    }
    let size = head[6..10].iter().fold(0usize, |acc, b| (acc << 7) | (*b as usize & 0x7F));
    let mut blob = head.to_vec();
    blob.resize(10 + size, 0);
    reader.read_exact(&mut blob[10..]).ok()?;
    let tag = id3v2::parse(&blob).ok()?;
    Some(tidy(
        id3v2::chapters(&tag)
            .into_iter()
            .map(|(ms, title)| Chapter { start: ms as f64 / 1000.0, title, url: None })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_chapters_skip_hidden_ones_and_sort() {
        let body = r#"{"version":"1.2.0","chapters":[
            {"startTime": 120.5, "title": "Le sujet", "url": "https://ex.com"},
            {"startTime": 0, "title": "Intro"},
            {"startTime": 60, "title": "Pub", "toc": false},
            {"startTime": 300, "title": "Fin", "url": "javascript:alert(1)"}]}"#;
        let c = parse_json(body);
        assert_eq!(c.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(), ["Intro", "Le sujet", "Fin"]);
        assert_eq!(c[1].url.as_deref(), Some("https://ex.com"));
        assert_eq!(c[2].url, None);
        assert!(parse_json("pas du json").is_empty());
    }

    #[test]
    fn times_and_single_chapter() {
        assert_eq!(parse_time("01:02:03.500"), Some(3723.5));
        assert_eq!(parse_time("02:03"), Some(123.0));
        assert_eq!(parse_time("abc"), None);
        assert!(tidy(vec![Chapter { start: 0.0, title: "Seul".into(), url: None }]).is_empty());
    }
}
