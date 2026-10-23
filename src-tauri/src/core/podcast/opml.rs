//! OPML : la liste d'abonnements que s'échangent les applis de podcasts.

use crate::helper::xml::{attr, escape};

/// Un abonnement : titre et adresse du flux.
#[derive(Debug, PartialEq)]
pub struct Outline {
    pub title: Option<String>,
    pub feed_url: String,
}

/// Tous les `<outline xmlUrl="…">`, quelle que soit leur profondeur (dossiers de catégories).
pub fn parse(xml: &str) -> Vec<Outline> {
    let mut out: Vec<Outline> = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<outline") {
        let tag_start = &rest[i..];
        let end = tag_start.find('>').map_or(tag_start.len(), |e| e + 1);
        let tag = &tag_start[..end];
        let url = attr(tag, "xmlUrl").or_else(|| attr(tag, "xmlurl")).map(|u| u.trim().to_string());
        if let Some(feed_url) = url.filter(|u| !u.is_empty()) {
            if !out.iter().any(|o| o.feed_url == feed_url) {
                let title = attr(tag, "title").or_else(|| attr(tag, "text")).filter(|t| !t.trim().is_empty());
                out.push(Outline { title, feed_url });
            }
        }
        rest = &tag_start[end..];
    }
    out
}

/// Le fichier OPML des abonnements : (titre, flux, site).
pub fn build(items: &[(String, String, Option<String>)]) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<opml version=\"2.0\">\n  <head><title>RustMusic</title></head>\n  <body>\n");
    for (title, feed, link) in items {
        let link = link.as_deref().map(|l| format!(" htmlUrl=\"{}\"", escape(l))).unwrap_or_default();
        xml.push_str(&format!(
            "    <outline type=\"rss\" text=\"{t}\" title=\"{t}\" xmlUrl=\"{u}\"{link} />\n",
            t = escape(title),
            u = escape(feed)
        ));
    }
    xml.push_str("  </body>\n</opml>\n");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nested_outlines_and_round_trips() {
        let xml = r#"<?xml version="1.0"?><opml version="1.0"><body>
            <outline text="Tech">
              <outline type="rss" text="Le Rendez-vous Tech" xmlUrl="https://feed.example/rdv?a=1&amp;b=2"/>
              <outline text="Sans titre" xmlUrl="https://feed.example/two"></outline>
            </outline>
            <outline title="Doublon" xmlUrl="https://feed.example/two"/>
            <outline text="Pas un flux"/>
          </body></opml>"#;
        let got = parse(xml);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].feed_url, "https://feed.example/rdv?a=1&b=2");
        assert_eq!(got[0].title.as_deref(), Some("Le Rendez-vous Tech"));
        let rebuilt = build(&[("A & B".into(), got[0].feed_url.clone(), Some("https://site".into()))]);
        assert_eq!(parse(&rebuilt), vec![Outline { title: Some("A & B".into()), feed_url: got[0].feed_url.clone() }]);
    }
}
