//! Paroles d'un morceau demandées au serveur : synchronisées (extension OpenSubsonic `songLyrics`,
//! rendues en LRC) ou en simple texte (`getLyrics`, Subsonic classique).

use serde_json::Value;

use super::client::Server;

#[derive(Debug, Default, PartialEq)]
pub struct ServerLyrics {
    pub plain: Option<String>,
    pub synced: Option<String>,
}

/// `[mm:ss.cc]` d'une ligne LRC.
fn lrc_time(ms: i64) -> String {
    let ms = ms.max(0);
    format!("[{:02}:{:02}.{:02}]", ms / 60_000, (ms / 1000) % 60, (ms % 1000) / 10)
}

/// Les paroles structurées : la version synchronisée d'abord, sinon le texte.
fn parse_structured(resp: &Value) -> Option<ServerLyrics> {
    let list = resp["lyricsList"]["structuredLyrics"].as_array()?;
    let pick = list.iter().find(|l| l["synced"].as_bool() == Some(true)).or_else(|| list.first())?;
    let lines = pick["line"].as_array()?;
    let text: Vec<&str> = lines.iter().filter_map(|l| l["value"].as_str()).collect();
    if text.iter().all(|t| t.trim().is_empty()) {
        return None;
    }
    let plain = Some(text.join("\n"));
    if pick["synced"].as_bool() != Some(true) {
        return Some(ServerLyrics { plain, synced: None });
    }
    let offset = pick["offset"].as_i64().unwrap_or(0);
    let synced = lines
        .iter()
        .map(|l| format!("{}{}", lrc_time(l["start"].as_i64().unwrap_or(0) + offset), l["value"].as_str().unwrap_or("")))
        .collect::<Vec<_>>()
        .join("\n");
    Some(ServerLyrics { plain: None, synced: Some(synced) })
}

fn parse_plain(resp: &Value) -> Option<ServerLyrics> {
    let text = resp["lyrics"]["value"].as_str()?.trim();
    (!text.is_empty()).then(|| ServerLyrics { plain: Some(text.to_string()), synced: None })
}

impl Server {
    /// Paroles du morceau ; `structured` quand le serveur annonce l'extension `songLyrics`.
    pub async fn lyrics(&self, song_id: &str, structured: bool, artist: &str, title: &str) -> Result<Option<ServerLyrics>, String> {
        if structured {
            if let Some(found) = parse_structured(&self.call("getLyricsBySongId", &[("id", song_id)]).await?) {
                return Ok(Some(found));
            }
        }
        if artist.is_empty() || title.is_empty() {
            return Ok(None);
        }
        Ok(parse_plain(&self.call("getLyrics", &[("artist", artist), ("title", title)]).await?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn synced_lines_become_lrc() {
        let resp = json!({ "lyricsList": { "structuredLyrics": [
            { "synced": false, "line": [{ "value": "texte seul" }] },
            { "synced": true, "offset": -100, "line": [{ "start": 1100, "value": "Premier" }, { "start": 65_430, "value": "Second" }] }
        ] } });
        let l = parse_structured(&resp).unwrap();
        assert_eq!(l.synced.as_deref(), Some("[00:01.00]Premier\n[01:05.33]Second"));
        assert_eq!(l.plain, None);
    }

    #[test]
    fn unsynced_and_empty_lyrics() {
        let plain = json!({ "lyricsList": { "structuredLyrics": [{ "synced": false, "line": [{ "value": "a" }, { "value": "b" }] }] } });
        assert_eq!(parse_structured(&plain), Some(ServerLyrics { plain: Some("a\nb".into()), synced: None }));
        assert_eq!(parse_structured(&json!({ "lyricsList": {} })), None);
        assert_eq!(parse_plain(&json!({ "lyrics": { "value": "  " } })), None);
        assert_eq!(parse_plain(&json!({ "lyrics": { "value": "la la" } })).unwrap().plain.as_deref(), Some("la la"));
    }

    // Essai réel : paroles de la démo Navidrome sur quelques morceaux tirés au hasard.
    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_demo_lyrics() {
        use super::super::catalog::parse_song;
        use super::super::client::Auth;
        let (server, info) = Server::connect("demo.navidrome.org", Auth::from_password("demo", "demo")).await.unwrap();
        let structured = info.extensions.iter().any(|e| e == "songLyrics");
        let random = server.call("getRandomSongs", &[("size", "40")]).await.unwrap();
        let mut found = 0;
        for s in random["randomSongs"]["song"].as_array().unwrap().iter().filter_map(parse_song) {
            if let Some(l) = server.lyrics(&s.id, structured, s.artist.as_deref().unwrap_or(""), &s.title).await.unwrap() {
                found += 1;
                let text = l.synced.as_deref().or(l.plain.as_deref()).unwrap_or("");
                println!("{} — {} : {}", s.title, if l.synced.is_some() { "synchronisées" } else { "texte" }, text.lines().take(2).collect::<Vec<_>>().join(" / "));
            }
        }
        println!("songLyrics : {structured} ; {found} morceau(x) avec paroles sur 40");
    }
}
