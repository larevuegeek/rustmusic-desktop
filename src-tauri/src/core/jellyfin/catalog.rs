//! Catalogue Jellyfin : morceaux d'une bibliothèque (par pages), pochettes, flux, paroles, favoris et écoutes.

use std::time::Duration;

use serde_json::{json, Value};

use super::client::Server;
use crate::core::podcast::fetch::client;
use crate::core::remote::RemotePlaylist;
use crate::core::subsonic::catalog::RemoteSong;
use crate::core::subsonic::lyrics::ServerLyrics;

const PAGE: usize = 500;
const FIELDS: &str = "MediaSources,Path,Genres,ProductionYear,ParentIndexNumber,IndexNumber,UserData,AlbumArtists,Artists";
/// Les durées Jellyfin sont en « ticks » de 100 ns.
const TICKS_PER_SECOND: f64 = 10_000_000.0;

pub(crate) fn parse_song(s: &Value) -> Option<RemoteSong> {
    let text = |k: &str| s.get(k).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    let id = text("Id")?;
    let source = s.pointer("/MediaSources/0");
    let audio = source
        .and_then(|m| m.get("MediaStreams"))
        .and_then(Value::as_array)
        .and_then(|l| l.iter().find(|st| st.get("Type").and_then(Value::as_str) == Some("Audio")));
    let num = |v: Option<&Value>| v.and_then(Value::as_f64).filter(|n| *n > 0.0);
    let artists: Vec<&str> = s.get("Artists").and_then(Value::as_array).map(|l| l.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    // La pochette de l'album, sinon celle du morceau.
    let cover_art = if s.get("AlbumPrimaryImageTag").is_some_and(|t| !t.is_null()) {
        text("AlbumId")
    } else if s.pointer("/ImageTags/Primary").is_some() {
        Some(id.clone())
    } else {
        None
    };
    Some(RemoteSong {
        title: text("Name").unwrap_or_else(|| id.clone()),
        album_id: text("AlbumId"),
        artist: (!artists.is_empty()).then(|| artists.join("; ")),
        album: text("Album"),
        album_artist: text("AlbumArtist"),
        year: num(s.get("ProductionYear")).map(|y| y as i64),
        genre: s.get("Genres").and_then(Value::as_array).and_then(|g| g.first()).and_then(Value::as_str).map(str::to_string),
        track: num(s.get("IndexNumber")).map(|n| n as u16),
        disc: num(s.get("ParentIndexNumber")).map(|n| n as u16),
        duration: num(s.get("RunTimeTicks")).map(|t| t / TICKS_PER_SECOND),
        bit_rate: num(source.and_then(|m| m.get("Bitrate"))).map(|b| (b / 1000.0) as u32),
        bit_depth: num(audio.and_then(|a| a.get("BitDepth"))).map(|b| b as u32),
        sampling_rate: num(audio.and_then(|a| a.get("SampleRate"))).map(|r| r as u32),
        channels: num(audio.and_then(|a| a.get("Channels"))).map(|c| c as u32),
        suffix: source.and_then(|m| m.get("Container")).and_then(Value::as_str).or_else(|| s.get("Container").and_then(Value::as_str)).map(|c| c.to_ascii_lowercase()),
        size: num(source.and_then(|m| m.get("Size"))).map(|n| n as i64),
        cover_art,
        // Jellyfin n'a pas de note de 1 à 5 par utilisateur.
        user_rating: None,
        starred: s.pointer("/UserData/IsFavorite").and_then(Value::as_bool).unwrap_or(false),
        server_path: text("Path"),
        id,
    })
}

/// Paroles Jellyfin (`/Audio/{id}/Lyrics`) : lignes horodatées en ticks.
fn parse_lyrics(v: &Value) -> Option<ServerLyrics> {
    let lines = v.get("Lyrics")?.as_array()?;
    let text: Vec<&str> = lines.iter().filter_map(|l| l.get("Text").and_then(Value::as_str)).collect();
    if text.iter().all(|t| t.trim().is_empty()) {
        return None;
    }
    let synced = v.pointer("/Metadata/IsSynced").and_then(Value::as_bool).unwrap_or(false);
    if !synced {
        return Some(ServerLyrics { plain: Some(text.join("\n")), synced: None });
    }
    let lrc = lines
        .iter()
        .map(|l| {
            let ms = l.get("Start").and_then(Value::as_i64).unwrap_or(0) / 10_000;
            format!("[{:02}:{:02}.{:02}]{}", ms / 60_000, (ms / 1000) % 60, (ms % 1000) / 10, l.get("Text").and_then(Value::as_str).unwrap_or(""))
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(ServerLyrics { plain: None, synced: Some(lrc) })
}

impl Server {
    /// Tous les morceaux d'une bibliothèque ; `progress` reçoit le nombre déjà listé.
    pub async fn all_songs(&self, folder_id: &str, mut progress: impl FnMut(usize)) -> Result<Vec<RemoteSong>, String> {
        let mut songs = Vec::new();
        loop {
            let start = songs.len().to_string();
            let limit = PAGE.to_string();
            let page = self
                .get(
                    &format!("/Users/{}/Items", self.user_id),
                    &[("ParentId", folder_id), ("IncludeItemTypes", "Audio"), ("Recursive", "true"), ("Fields", FIELDS), ("StartIndex", &start), ("Limit", &limit), ("SortBy", "SortName")],
                )
                .await?;
            let items = page["Items"].as_array().cloned().unwrap_or_default();
            let got = items.len();
            songs.extend(items.iter().filter_map(parse_song));
            progress(songs.len());
            let total = page["TotalRecordCount"].as_u64().unwrap_or(0) as usize;
            if got < PAGE || songs.len() >= total {
                break;
            }
        }
        Ok(songs)
    }

    /// La pochette (`size` pixels de côté au plus) ; `None` si le serveur n'en a pas.
    pub async fn cover_art(&self, id: &str, size: u32) -> Option<Vec<u8>> {
        let url = self.url(&format!("/Items/{id}/Images/Primary"), &[("maxWidth", &size.to_string()), ("quality", "90")]).ok()?;
        let resp = client().get(url).timeout(Duration::from_secs(20)).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        resp.bytes().await.ok().map(|b| b.to_vec())
    }

    /// Adresse du flux : le fichier d'origine, ou transcodé (format, débit en kbit/s).
    pub fn stream_url(&self, song_id: &str, transcode: Option<(&str, i64)>) -> Result<reqwest::Url, String> {
        match transcode {
            None => self.url(&format!("/Audio/{song_id}/stream"), &[("static", "true")]),
            Some((format, kbps)) => {
                let bitrate = (kbps * 1000).to_string();
                self.url(&format!("/Audio/{song_id}/stream.{format}"), &[("audioCodec", format), ("audioBitRate", &bitrate), ("static", "false")])
            }
        }
    }

    /// Paroles du morceau, s'il en a.
    pub async fn lyrics(&self, song_id: &str) -> Result<Option<ServerLyrics>, String> {
        match self.get(&format!("/Audio/{song_id}/Lyrics"), &[]).await {
            Ok(v) => Ok(parse_lyrics(&v)),
            Err(e) if e.starts_with("élément introuvable") => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Les playlists musicales de l'utilisateur, avec leurs morceaux.
    pub async fn playlists(&self) -> Result<Vec<RemotePlaylist>, String> {
        let list = self
            .get(&format!("/Users/{}/Items", self.user_id), &[("IncludeItemTypes", "Playlist"), ("Recursive", "true"), ("Fields", "Overview")])
            .await?;
        let mut out = Vec::new();
        for p in list["Items"].as_array().into_iter().flatten() {
            // Les playlists de vidéos ne nous concernent pas.
            if p.get("MediaType").and_then(Value::as_str).is_some_and(|m| !m.eq_ignore_ascii_case("Audio")) {
                continue;
            }
            let Some(id) = p.get("Id").and_then(Value::as_str) else { continue };
            let items = self.get(&format!("/Playlists/{id}/Items"), &[("UserId", &self.user_id), ("Fields", FIELDS)]).await?;
            let text = |k: &str| p.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
            out.push(RemotePlaylist {
                name: text("Name").unwrap_or_else(|| id.to_string()),
                comment: text("Overview"),
                songs: items["Items"].as_array().into_iter().flatten().filter_map(parse_song).collect(),
                id: id.to_string(),
            });
        }
        Ok(out)
    }

    /// Crée une playlist musicale vide ; rend son identifiant.
    pub async fn create_playlist(&self, name: &str) -> Result<String, String> {
        let body = json!({ "Name": name, "UserId": self.user_id, "MediaType": "Audio", "Ids": [] });
        let created = self.send(reqwest::Method::POST, "/Playlists", &[], Some(body)).await?;
        created.get("Id").and_then(Value::as_str).map(str::to_string).ok_or_else(|| "le serveur n'a pas rendu la playlist créée".into())
    }

    /// Renomme la playlist et/ou remplace tout son contenu (Jellyfin 10.9 et plus).
    pub async fn update_playlist(&self, id: &str, name: Option<&str>, song_ids: Option<&[String]>) -> Result<(), String> {
        let mut body = json!({});
        if let Some(n) = name {
            body["Name"] = json!(n);
        }
        if let Some(ids) = song_ids {
            body["Ids"] = json!(ids);
        }
        self.send(reqwest::Method::POST, &format!("/Playlists/{id}"), &[], Some(body)).await.map(|_| ())
    }

    pub async fn delete_item(&self, id: &str) -> Result<(), String> {
        self.send(reqwest::Method::DELETE, &format!("/Items/{id}"), &[], None).await.map(|_| ())
    }

    pub async fn set_favorite(&self, song_id: &str, on: bool) -> Result<(), String> {
        let method = if on { reqwest::Method::POST } else { reqwest::Method::DELETE };
        self.send(method, &format!("/Users/{}/FavoriteItems/{song_id}", self.user_id), &[], None).await.map(|_| ())
    }

    /// Écoute comptée, à l'instant donné (millisecondes).
    pub async fn mark_played(&self, song_id: &str, at_ms: i64) -> Result<(), String> {
        let at = chrono::DateTime::from_timestamp_millis(at_ms).unwrap_or_else(chrono::Utc::now).format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        self.send(reqwest::Method::POST, &format!("/Users/{}/PlayedItems/{song_id}", self.user_id), &[("datePlayed", &at)], None).await.map(|_| ())
    }

    /// « En cours de lecture », pour les autres applis du serveur.
    pub async fn now_playing(&self, song_id: &str) -> Result<(), String> {
        self.send(reqwest::Method::POST, "/Sessions/Playing", &[], Some(json!({ "ItemId": song_id, "CanSeek": true }))).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_song() {
        let item = json!({
            "Id": "a79", "Name": "Bend", "Album": "You Can't Stop Da Funk", "AlbumId": "f21", "AlbumArtist": "Binaerpilot",
            "Artists": ["Binaerpilot", "Invité"], "ProductionYear": 2006, "Genres": ["Electronic"], "IndexNumber": 1,
            "RunTimeTicks": 2_675_461_220_i64, "UserData": { "IsFavorite": true }, "AlbumPrimaryImageTag": "95b", "Path": "/media/Music/bend.mp3",
            "MediaSources": [{ "Container": "flac", "Size": 6_429_605, "Bitrate": 1_011_000,
                "MediaStreams": [{ "Type": "Audio", "Codec": "flac", "BitDepth": 24, "SampleRate": 96000, "Channels": 2 }] }]
        });
        let s = parse_song(&item).unwrap();
        assert_eq!((s.id.as_str(), s.title.as_str(), s.album_id.as_deref(), s.cover_art.as_deref()), ("a79", "Bend", Some("f21"), Some("f21")));
        assert_eq!(s.artist.as_deref(), Some("Binaerpilot; Invité"));
        assert_eq!((s.track, s.disc, s.year), (Some(1), None, Some(2006)));
        assert_eq!((s.bit_rate, s.bit_depth, s.sampling_rate, s.channels), (Some(1011), Some(24), Some(96000), Some(2)));
        assert_eq!(s.suffix.as_deref(), Some("flac"));
        assert!((s.duration.unwrap() - 267.546122).abs() < 0.001);
        assert!(s.starred && s.user_rating.is_none());
    }

    #[test]
    fn synced_lyrics_become_lrc() {
        let v = json!({ "Metadata": { "IsSynced": true }, "Lyrics": [{ "Text": "Premier", "Start": 11_000_000 }, { "Text": "Second", "Start": 654_300_000 }] });
        assert_eq!(parse_lyrics(&v).unwrap().synced.as_deref(), Some("[00:01.10]Premier\n[01:05.43]Second"));
        let plain = json!({ "Metadata": { "IsSynced": false }, "Lyrics": [{ "Text": "a" }, { "Text": "b" }] });
        assert_eq!(parse_lyrics(&plain).unwrap().plain.as_deref(), Some("a\nb"));
    }

    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_demo_catalog() {
        let (server, _) = Server::connect("demo.jellyfin.org/stable", "demo", Some(""), None).await.unwrap();
        let folder = server.music_folders().await.unwrap().remove(0);
        let songs = server.all_songs(&folder.id, |_| {}).await.unwrap();
        let s = &songs[0];
        println!("{} morceaux ; premier : {} — {:?} ({:?}, {:?} kbit/s)", songs.len(), s.title, s.artist, s.suffix, s.bit_rate);
        assert!(!songs.is_empty());
        if let Some(id) = s.cover_art.as_deref() {
            assert!(server.cover_art(id, 300).await.is_some_and(|b| b.len() > 100));
        }
        let url = server.stream_url(&s.id, None).unwrap();
        let resp = client().get(url).header("Range", "bytes=0-1023").send().await.unwrap();
        println!("flux : HTTP {} {:?}", resp.status(), resp.headers().get("content-type"));
        assert!(resp.status().is_success());
    }
}
