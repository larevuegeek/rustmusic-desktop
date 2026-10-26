//! Lecture d'un flux RSS de podcast : la chaîne et ses épisodes audio.
//! Les balises iTunes (`itunes:`) complètent le RSS de base, très inégal d'un éditeur à l'autre.

use std::collections::HashSet;

use chrono::{DateTime, Utc};

use crate::core::podcast::chapters::{self, Chapter};
use crate::helper::xml;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Feed {
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub link: Option<String>,
    pub language: Option<String>,
    pub episodes: Vec<FeedEpisode>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeedEpisode {
    pub guid: String,
    pub title: String,
    /// HTML du flux, non nettoyé.
    pub description: Option<String>,
    /// Texte brut court, pour les listes.
    pub summary: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    /// Secondes.
    pub duration: Option<i64>,
    pub audio_url: String,
    pub audio_type: Option<String>,
    pub audio_size: Option<i64>,
    pub image_url: Option<String>,
    pub season: Option<i64>,
    pub episode: Option<i64>,
    /// Fichier JSON des chapitres (`<podcast:chapters>`).
    pub chapters_url: Option<String>,
    /// Chapitres écrits dans le flux (`<psc:chapters>`).
    pub chapters: Vec<Chapter>,
}

/// Longueur du résumé des listes, en caractères.
const SUMMARY_CHARS: usize = 280;

pub fn parse(body: &str) -> Result<Feed, String> {
    let channel = xml::first(body, "channel").ok_or("pas un flux RSS : élément <channel> absent")?;
    // Sans les épisodes ni le bloc <image> : leurs <title> et <link> ne se mêlent pas à ceux de la chaîne.
    let channel_only = xml::without(channel, "item");
    let head = without_unprefixed(&channel_only, "image");

    let title = field(&head, None, "title").ok_or("flux sans titre")?;
    let mut seen = HashSet::new();
    let episodes = xml::elements(channel, "item")
        .into_iter()
        .filter_map(parse_item)
        .filter(|e| seen.insert(e.guid.clone()))
        .collect();

    Ok(Feed {
        title: plain_text(&title),
        author: field(&head, Some("itunes"), "author").or_else(|| field(&head, None, "author")).map(|a| plain_text(&a)),
        description: field(&head, None, "description").or_else(|| field(&head, Some("itunes"), "summary")),
        image_url: itunes_image(&channel_only).or_else(|| {
            unprefixed(&channel_only, "image").and_then(|inner| field(inner, None, "url"))
        }),
        // Site web seulement : un autre schéma ferait naviguer la fenêtre de l'appli.
        link: field(&head, None, "link").map(|l| l.trim().to_string()).filter(|l| {
            let l = l.to_ascii_lowercase();
            l.starts_with("https://") || l.starts_with("http://")
        }),
        language: field(&head, None, "language"),
        episodes,
    })
}

fn parse_item(item: &str) -> Option<FeedEpisode> {
    let (tag, audio_url) = enclosure(item)?;
    let audio_type = xml::attr(tag, "type").filter(|t| !t.is_empty());
    if audio_type.as_deref().is_some_and(|t| t.starts_with("video/")) || is_video_url(&audio_url) {
        return None;
    }
    let title = field(item, None, "title")
        .or_else(|| field(item, Some("itunes"), "title"))
        .map(|t| plain_text(&t))
        .unwrap_or_else(|| file_name(&audio_url));
    let description = field(item, Some("content"), "encoded")
        .or_else(|| field(item, None, "description"))
        .or_else(|| field(item, Some("itunes"), "summary"));
    let summary = description
        .clone()
        .or_else(|| field(item, Some("itunes"), "subtitle"))
        .map(|d| truncate(&plain_text(&d), SUMMARY_CHARS))
        .filter(|s| !s.is_empty());

    Some(FeedEpisode {
        guid: field(item, None, "guid").unwrap_or_else(|| audio_url.clone()),
        title,
        description,
        summary,
        published_at: field(item, None, "pubDate").and_then(|d| parse_date(&d)),
        duration: field(item, Some("itunes"), "duration").and_then(|d| parse_duration(&d)),
        audio_size: xml::attr(tag, "length").and_then(|l| l.trim().parse::<i64>().ok()).filter(|&l| l > 0),
        audio_type,
        audio_url,
        image_url: itunes_image(item),
        season: field(item, Some("itunes"), "season").and_then(|s| s.parse().ok()),
        episode: field(item, Some("itunes"), "episode").and_then(|s| s.parse().ok()),
        chapters_url: chapters::web_url(
            xml::tagged(item, "chapters").into_iter().filter(|(tag, _)| prefix_of(tag) == Some("podcast")).find_map(|(tag, _)| xml::attr(tag, "url")),
        ),
        chapters: psc_chapters(item),
    })
}

/// `<psc:chapter start="00:01:30.000" title="…" href="…"/>` (Podlove).
fn psc_chapters(item: &str) -> Vec<Chapter> {
    chapters::tidy(
        xml::tagged(item, "chapter")
            .into_iter()
            .filter(|(tag, _)| prefix_of(tag) == Some("psc"))
            .filter_map(|(tag, _)| {
                Some(Chapter {
                    start: chapters::parse_time(&xml::attr(tag, "start")?)?,
                    title: xml::attr(tag, "title").unwrap_or_default().trim().to_string(),
                    url: chapters::web_url(xml::attr(tag, "href")),
                })
            })
            .collect(),
    )
}

/// Le fichier de l'épisode : `<enclosure>`, à défaut `<media:content>` audio.
fn enclosure(item: &str) -> Option<(&str, String)> {
    let with_url = |(tag, _): (&'_ str, &'_ str)| xml::attr(tag, "url").map(|u| u.trim().to_string()).filter(|u| !u.is_empty());
    xml::tagged(item, "enclosure")
        .into_iter()
        .find_map(|t| with_url(t).map(|u| (t.0, u)))
        .or_else(|| {
            xml::tagged(item, "content")
                .into_iter()
                .filter(|(tag, _)| prefix_of(tag) == Some("media"))
                .filter(|(tag, _)| xml::attr(tag, "type").is_none_or(|t| t.starts_with("audio/")))
                .find_map(|t| with_url(t).map(|u| (t.0, u)))
        })
}

fn is_video_url(url: &str) -> bool {
    let path = url.split(['?', '#']).next().unwrap_or("").to_ascii_lowercase();
    [".m4v", ".mov", ".webm", ".mkv"].iter().any(|ext| path.ends_with(ext))
}

fn file_name(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or("");
    let name = path.rsplit('/').next().unwrap_or(path);
    urlencoding::decode(name).map(|n| n.into_owned()).unwrap_or_else(|_| name.to_string())
}

/// `<itunes:image href="…">`.
fn itunes_image(xml_text: &str) -> Option<String> {
    xml::tagged(xml_text, "image")
        .into_iter()
        .filter(|(tag, _)| prefix_of(tag) == Some("itunes"))
        .find_map(|(tag, _)| xml::attr(tag, "href"))
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
}

/// Texte du premier élément `local` portant ce préfixe (`None` : sans préfixe).
fn field(xml_text: &str, prefix: Option<&str>, local: &str) -> Option<String> {
    xml::tagged(xml_text, local)
        .into_iter()
        .find(|(tag, inner)| prefix_of(tag) == prefix && !inner.trim().is_empty())
        .map(|(_, inner)| xml::decode(inner).trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Contenu du premier élément `local` sans préfixe.
fn unprefixed<'a>(xml_text: &'a str, local: &str) -> Option<&'a str> {
    xml::tagged(xml_text, local).into_iter().find(|(tag, _)| prefix_of(tag).is_none()).map(|(_, inner)| inner)
}

/// Retire les éléments `local` sans préfixe, garde les autres (`<image>` part, `<itunes:image>` reste).
fn without_unprefixed(xml_text: &str, local: &str) -> String {
    let mut out = xml_text.to_string();
    for inner in xml::elements(xml_text, local).into_iter().rev() {
        if inner.is_empty() {
            continue;
        }
        let start = inner.as_ptr() as usize - xml_text.as_ptr() as usize;
        let open = xml_text[..start].rfind('<').unwrap_or(start);
        if prefix_of(&xml_text[open..start]).is_some() {
            continue;
        }
        let end = start + inner.len();
        let close = xml_text[end..].find('>').map_or(xml_text.len(), |i| end + i + 1);
        out.replace_range(open..close, "");
    }
    out
}

/// Préfixe d'une balise ouvrante : `itunes` pour `<itunes:image …>`.
fn prefix_of(tag: &str) -> Option<&str> {
    let name = tag.trim_start_matches('<').split(|c: char| c.is_whitespace() || c == '/' || c == '>').next().unwrap_or("");
    name.split_once(':').map(|(p, _)| p)
}

/// `3723`, `62:03`, `1:02:03` ou `3723.5` → secondes.
pub fn parse_duration(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    let mut total = 0.0_f64;
    for part in raw.split(':') {
        let v: f64 = part.trim().parse().ok()?;
        if v < 0.0 {
            return None;
        }
        total = total * 60.0 + v;
    }
    // Au-delà de deux jours, c'est une valeur fausse (millisecondes, octets…).
    (total > 0.0 && total < 172_800.0).then(|| total.round() as i64)
}

/// Date RSS (RFC 2822), souvent approximative : jour de semaine faux, ISO 8601 à la place.
pub fn parse_date(raw: &str) -> Option<DateTime<Utc>> {
    let raw = raw.trim();
    let utc = |d: DateTime<chrono::FixedOffset>| d.with_timezone(&Utc);
    DateTime::parse_from_rfc2822(raw)
        .map(utc)
        .or_else(|_| DateTime::parse_from_rfc3339(raw).map(utc))
        .ok()
        .or_else(|| {
            // « Mon, » retiré : un jour de semaine erroné fait tout rejeter.
            let (_, rest) = raw.split_once(',')?;
            let rest = rest.trim();
            DateTime::parse_from_rfc2822(rest).ok().map(utc)
        })
}

/// Texte brut d'un fragment HTML : balises retirées, entités décodées, blancs resserrés.
pub fn plain_text(html: &str) -> String {
    // Une balise de bloc sépare deux mots ; une balise en ligne (<b>, <a>) non.
    const BLOCKS: &[&str] = &["p", "br", "div", "li", "ul", "ol", "tr", "td", "h1", "h2", "h3", "h4", "h5", "h6", "blockquote"];
    let mut text = String::with_capacity(html.len());
    let mut tag: Option<String> = None;
    for c in html.chars() {
        match (&mut tag, c) {
            (None, '<') => tag = Some(String::new()),
            (Some(name), '>') => {
                let name = name.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
                if BLOCKS.contains(&name.to_ascii_lowercase().as_str()) {
                    text.push(' ');
                }
                tag = None;
            }
            (Some(name), _) => name.push(c),
            (None, _) => text.push(c),
        }
    }
    let decoded = decode_html_entities(&xml::decode(&text));
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Entités HTML courantes, absentes du XML de base.
fn decode_html_entities(s: &str) -> String {
    const ENTITIES: &[(&str, &str)] = &[
        ("&nbsp;", " "), ("&eacute;", "é"), ("&egrave;", "è"), ("&ecirc;", "ê"), ("&euml;", "ë"),
        ("&agrave;", "à"), ("&acirc;", "â"), ("&ccedil;", "ç"), ("&icirc;", "î"), ("&iuml;", "ï"),
        ("&ocirc;", "ô"), ("&ouml;", "ö"), ("&ucirc;", "û"), ("&ugrave;", "ù"), ("&uuml;", "ü"),
        ("&Eacute;", "É"), ("&Agrave;", "À"), ("&Ccedil;", "Ç"), ("&oelig;", "œ"), ("&szlig;", "ß"),
        ("&ntilde;", "ñ"), ("&rsquo;", "’"), ("&lsquo;", "‘"), ("&ldquo;", "“"), ("&rdquo;", "”"),
        ("&laquo;", "«"), ("&raquo;", "»"), ("&hellip;", "…"), ("&ndash;", "–"), ("&mdash;", "—"),
        ("&euro;", "€"), ("&copy;", "©"), ("&deg;", "°"),
    ];
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = s.to_string();
    for (entity, ch) in ENTITIES {
        if out.contains(entity) {
            out = out.replace(entity, ch);
        }
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        None => s.to_string(),
        Some((cut, _)) => format!("{}…", s[..cut].trim_end()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapters_from_the_feed() {
        let item = r#"<item><title>É</title><enclosure url="https://ex.com/e.mp3" type="audio/mpeg"/>
            <podcast:chapters url="https://ex.com/e.json" type="application/json+chapters"/>
            <psc:chapters version="1.2">
              <psc:chapter start="00:00:00.000" title="Intro"/>
              <psc:chapter start="00:12:30.5" title="Le &amp; sujet" href="https://ex.com/s"/>
            </psc:chapters></item>"#;
        let e = parse_item(item).unwrap();
        assert_eq!(e.chapters_url.as_deref(), Some("https://ex.com/e.json"));
        assert_eq!(e.chapters.len(), 2);
        assert_eq!((e.chapters[1].start, e.chapters[1].title.as_str()), (750.5, "Le & sujet"));
        assert_eq!(e.chapters[1].url.as_deref(), Some("https://ex.com/s"));
    }

    const FEED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd" xmlns:content="http://purl.org/rss/1.0/modules/content/" xmlns:atom="http://www.w3.org/2005/Atom">
<channel>
  <atom:link href="https://ex.com/feed.xml" rel="self" type="application/rss+xml"/>
  <image><url>https://ex.com/small.jpg</url><title>Logo du flux</title><link>https://ex.com/logo</link></image>
  <title>Les Pieds &amp; la Tête</title>
  <link>https://ex.com</link>
  <language>fr</language>
  <itunes:author>Radio Exemple</itunes:author>
  <itunes:image href="https://ex.com/big.jpg"/>
  <description><![CDATA[<p>Une émission <b>sonore</b>.</p>]]></description>
  <item>
    <title>Épisode 2 : l'&#233;t&#xE9;</title>
    <guid isPermaLink="false">ep-2</guid>
    <pubDate>Tue, 07 Oct 2025 06:00:00 +0200</pubDate>
    <itunes:duration>1:02:03</itunes:duration>
    <itunes:image href="https://ex.com/ep2.jpg"/>
    <itunes:season>1</itunes:season>
    <itunes:episode>2</itunes:episode>
    <description>Court résumé</description>
    <content:encoded><![CDATA[<p>Texte &eacute;tendu avec <a href="https://ex.com">un lien</a>.</p>]]></content:encoded>
    <enclosure url="https://cdn.ex.com/ep2.mp3?x=1&amp;y=2" length="12345678" type="audio/mpeg"/>
  </item>
  <item>
    <title>Sans guid</title>
    <pubDate>Mon, 06 Oct 2025 06:00:00 GMT</pubDate>
    <itunes:duration>3723</itunes:duration>
    <enclosure url="https://cdn.ex.com/ep1.mp3" length="0" type="audio/mpeg"/>
  </item>
  <item><title>Vidéo</title><guid>v</guid><enclosure url="https://cdn.ex.com/v.mp4" type="video/mp4"/></item>
  <item><title>Sans fichier</title><guid>n</guid></item>
  <item><title>Doublon</title><guid isPermaLink="false">ep-2</guid><enclosure url="https://cdn.ex.com/dup.mp3"/></item>
</channel>
</rss>"#;

    #[test]
    fn reads_the_channel_without_mixing_image_and_items() {
        let f = parse(FEED).unwrap();
        assert_eq!(f.title, "Les Pieds & la Tête");
        assert_eq!(f.link.as_deref(), Some("https://ex.com"));
        assert_eq!(f.author.as_deref(), Some("Radio Exemple"));
        assert_eq!(f.image_url.as_deref(), Some("https://ex.com/big.jpg"));
        assert_eq!(f.language.as_deref(), Some("fr"));
        assert_eq!(f.description.as_deref(), Some("<p>Une émission <b>sonore</b>.</p>"));
    }

    #[test]
    fn keeps_only_audio_episodes_once() {
        let f = parse(FEED).unwrap();
        let titles: Vec<_> = f.episodes.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["Épisode 2 : l'été", "Sans guid"]);
    }

    #[test]
    fn reads_episode_fields() {
        let f = parse(FEED).unwrap();
        let e = &f.episodes[0];
        assert_eq!(e.guid, "ep-2");
        assert_eq!(e.audio_url, "https://cdn.ex.com/ep2.mp3?x=1&y=2");
        assert_eq!(e.audio_size, Some(12_345_678));
        assert_eq!(e.audio_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(e.duration, Some(3723));
        assert_eq!(e.image_url.as_deref(), Some("https://ex.com/ep2.jpg"));
        assert_eq!((e.season, e.episode), (Some(1), Some(2)));
        assert_eq!(e.published_at.unwrap().to_rfc3339(), "2025-10-07T04:00:00+00:00");
        assert!(e.description.as_deref().unwrap().contains("<a href"));
        assert_eq!(e.summary.as_deref(), Some("Texte étendu avec un lien."));
    }

    #[test]
    fn falls_back_to_the_file_address_without_guid() {
        let f = parse(FEED).unwrap();
        assert_eq!(f.episodes[1].guid, "https://cdn.ex.com/ep1.mp3");
        assert_eq!(f.episodes[1].audio_size, None);
    }

    #[test]
    fn rejects_what_is_not_a_feed() {
        assert!(parse("<html><body>404</body></html>").is_err());
    }

    #[test]
    fn reads_every_duration_form() {
        assert_eq!(parse_duration("3723"), Some(3723));
        assert_eq!(parse_duration("62:03"), Some(3723));
        assert_eq!(parse_duration(" 1:02:03 "), Some(3723));
        assert_eq!(parse_duration("1830.6"), Some(1831));
        assert_eq!(parse_duration("0"), None);
        assert_eq!(parse_duration("abc"), None);
        assert_eq!(parse_duration("98765432"), None);
    }

    #[test]
    fn reads_approximate_dates() {
        let iso = |d: &str| parse_date(d).map(|d| d.to_rfc3339());
        assert_eq!(iso("Mon, 06 Oct 2025 06:00:00 GMT").as_deref(), Some("2025-10-06T06:00:00+00:00"));
        assert_eq!(iso("Fri, 06 Oct 2025 06:00:00 GMT").as_deref(), Some("2025-10-06T06:00:00+00:00"));
        assert_eq!(iso("2025-10-06T08:00:00+02:00").as_deref(), Some("2025-10-06T06:00:00+00:00"));
        assert_eq!(iso("hier"), None);
    }

    #[test]
    fn plain_text_strips_tags_and_entities() {
        assert_eq!(plain_text("<p>Bonjour&nbsp;<b>à</b> tous&hellip;</p><p>Fin<br/>!</p>"), "Bonjour à tous… Fin !");
        assert_eq!(plain_text("<b>Gras</b>ici"), "Grasici");
        assert_eq!(truncate("abcdef", 3), "abc…");
        assert_eq!(truncate("abc", 3), "abc");
    }
}
