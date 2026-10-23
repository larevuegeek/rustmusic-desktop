//! Catalogue d'un dossier musical : tous ses morceaux, par pages (`search3` à requête vide),
//! et à défaut album par album. Pochettes par `getCoverArt`.

use std::time::Duration;

use serde_json::Value;

use super::client::{id_of, Server};
use crate::core::podcast::fetch::client;

/// Morceaux demandés par page.
const PAGE: usize = 500;

/// Un morceau du serveur, avec ce qu'il faut pour l'indexer comme un fichier.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RemoteSong {
    pub id: String,
    pub album_id: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<i64>,
    pub genre: Option<String>,
    pub track: Option<u16>,
    pub disc: Option<u16>,
    pub duration: Option<f64>,
    pub bit_rate: Option<u32>,
    pub bit_depth: Option<u32>,
    pub sampling_rate: Option<u32>,
    pub channels: Option<u32>,
    pub suffix: Option<String>,
    pub size: Option<i64>,
    pub cover_art: Option<String>,
    /// Note de l'utilisateur, 1 à 5.
    pub user_rating: Option<f64>,
    /// Favori (étoile) de l'utilisateur.
    pub starred: bool,
    /// Chemin du fichier sur le serveur, pour le nom affiché.
    pub server_path: Option<String>,
}

impl RemoteSong {
    /// Empreinte des champs indexés : un morceau inchangé n'est pas réécrit.
    /// Sans favori ni note : ils sont alignés à part (`apply_server_state`).
    pub fn fingerprint(&self) -> String {
        let indexed = RemoteSong { starred: false, user_rating: None, ..self.clone() };
        format!("{:x}", md5::compute(format!("{indexed:?}")))
    }
}

/// Noms d'une liste OpenSubsonic (`artists`, `albumArtists`), séparés par « ; » comme un tag multiple.
fn names(v: Option<&Value>) -> Option<String> {
    let list: Vec<&str> = v?.as_array()?.iter().filter_map(|a| a.get("name")?.as_str()).filter(|n| !n.trim().is_empty()).collect();
    (!list.is_empty()).then(|| list.join("; "))
}

pub(crate) fn parse_song(s: &Value) -> Option<RemoteSong> {
    if s.get("isVideo").and_then(Value::as_bool) == Some(true) || s.get("isDir").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let text = |k: &str| s.get(k).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    let num = |k: &str| s.get(k).and_then(Value::as_f64).filter(|n| *n > 0.0);
    let id = id_of(s.get("id")?)?;
    // Plusieurs artistes (OpenSubsonic) : la liste, sinon le libellé.
    let artist = names(s.get("artists")).filter(|n| n.contains(';')).or_else(|| text("artist")).or_else(|| text("displayArtist"));
    let album_artist = text("displayAlbumArtist").or_else(|| names(s.get("albumArtists")));
    Some(RemoteSong {
        title: text("title").unwrap_or_else(|| id.clone()),
        album_id: s.get("albumId").and_then(id_of).or_else(|| s.get("parent").and_then(id_of)),
        artist,
        album: text("album"),
        album_artist,
        year: num("year").map(|y| y as i64),
        genre: text("genre"),
        track: num("track").map(|n| n as u16),
        disc: num("discNumber").map(|n| n as u16),
        duration: num("duration"),
        bit_rate: num("bitRate").map(|n| n as u32),
        bit_depth: num("bitDepth").map(|n| n as u32),
        sampling_rate: num("samplingRate").map(|n| n as u32),
        channels: num("channelCount").map(|n| n as u32),
        suffix: text("suffix").map(|x| x.to_ascii_lowercase()),
        size: num("size").map(|n| n as i64),
        cover_art: s.get("coverArt").and_then(id_of),
        user_rating: num("userRating"),
        starred: s.get("starred").is_some_and(|v| !v.is_null()),
        server_path: text("path"),
        id,
    })
}

fn songs_of(v: Option<&Value>) -> Vec<RemoteSong> {
    v.and_then(Value::as_array).map(|l| l.iter().filter_map(parse_song).collect()).unwrap_or_default()
}

impl Server {
    /// Tous les morceaux d'un dossier musical ; `progress` reçoit le nombre déjà listé.
    pub async fn all_songs(&self, folder_id: &str, mut progress: impl FnMut(usize)) -> Result<Vec<RemoteSong>, String> {
        let mut songs = Vec::new();
        loop {
            let offset = songs.len().to_string();
            let count = PAGE.to_string();
            let resp = self
                .call(
                    "search3",
                    &[("query", ""), ("artistCount", "0"), ("albumCount", "0"), ("songCount", &count), ("songOffset", &offset), ("musicFolderId", folder_id)],
                )
                .await?;
            let page = songs_of(resp.pointer("/searchResult3/song"));
            let n = page.len();
            songs.extend(page);
            progress(songs.len());
            if n < PAGE {
                break;
            }
        }
        if songs.is_empty() {
            // Serveur qui refuse la requête vide : album par album.
            songs = self.songs_by_album(folder_id, &mut progress).await?;
        }
        Ok(songs)
    }

    async fn songs_by_album(&self, folder_id: &str, progress: &mut impl FnMut(usize)) -> Result<Vec<RemoteSong>, String> {
        let mut album_ids = Vec::new();
        loop {
            let offset = album_ids.len().to_string();
            let resp = self
                .call("getAlbumList2", &[("type", "alphabeticalByName"), ("size", "500"), ("offset", &offset), ("musicFolderId", folder_id)])
                .await?;
            let page: Vec<String> = resp
                .pointer("/albumList2/album")
                .and_then(Value::as_array)
                .map(|l| l.iter().filter_map(|a| a.get("id").and_then(id_of)).collect())
                .unwrap_or_default();
            let n = page.len();
            album_ids.extend(page);
            if n < 500 {
                break;
            }
        }
        let mut songs = Vec::new();
        for id in album_ids {
            let resp = self.call("getAlbum", &[("id", &id)]).await?;
            songs.extend(songs_of(resp.pointer("/album/song")));
            progress(songs.len());
        }
        Ok(songs)
    }

    /// Pochette `id` (≈ `size` px), en octets ; `None` si le serveur n'en a pas.
    pub async fn cover_art(&self, id: &str, size: u32) -> Option<Vec<u8>> {
        let url = self.url("getCoverArt", &[("id", id), ("size", &size.to_string())]).ok()?;
        let resp = client().get(url).timeout(Duration::from_secs(20)).send().await.ok()?;
        let is_image = resp.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).is_some_and(|c| c.starts_with("image/"));
        if !resp.status().is_success() || !is_image {
            return None;
        }
        resp.bytes().await.ok().map(|b| b.to_vec()).filter(|b| !b.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_navidrome_song() {
        let json = serde_json::json!({
            "id": "abc", "parent": "dir1", "albumId": "al9", "title": "Get Lucky", "album": "Random Access Memories",
            "artist": "Daft Punk feat. Pharrell Williams", "artists": [{ "id": "1", "name": "Daft Punk" }, { "id": "2", "name": "Pharrell Williams" }],
            "displayAlbumArtist": "Daft Punk", "year": 2013, "genre": "Disco", "track": 8, "discNumber": 1, "duration": 369,
            "bitRate": 1016, "bitDepth": 24, "samplingRate": 88200, "channelCount": 2, "suffix": "FLAC", "size": 46_000_000,
            "coverArt": "al-al9", "userRating": 5, "path": "Daft Punk/RAM/08 Get Lucky.flac"
        });
        let s = parse_song(&json).unwrap();
        assert_eq!(s.id, "abc");
        assert_eq!(s.album_id.as_deref(), Some("al9"));
        assert_eq!(s.artist.as_deref(), Some("Daft Punk; Pharrell Williams"));
        assert_eq!(s.album_artist.as_deref(), Some("Daft Punk"));
        assert_eq!((s.track, s.disc, s.bit_depth, s.sampling_rate), (Some(8), Some(1), Some(24), Some(88200)));
        assert_eq!(s.suffix.as_deref(), Some("flac"));
        assert_eq!(s.user_rating, Some(5.0));
        assert!(parse_song(&serde_json::json!({ "id": "v", "isVideo": true })).is_none());
    }

    #[test]
    fn fingerprint_follows_the_tags() {
        let a = RemoteSong { id: "1".into(), title: "A".into(), ..Default::default() };
        let mut b = a.clone();
        assert_eq!(a.fingerprint(), b.fingerprint());
        b.title = "B".into();
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_demo_catalog() {
        use super::super::client::Auth;
        let (server, _) = Server::connect("demo.navidrome.org", Auth::from_password("demo", "demo")).await.unwrap();
        let folder = server.music_folders().await.unwrap().remove(0);
        let songs = server.all_songs(&folder.id, |n| println!("… {n}")).await.unwrap();
        println!("{} morceaux ; premier : {:?}", songs.len(), songs.first());
        let cover = songs.iter().find_map(|s| s.cover_art.clone()).unwrap();
        let bytes = server.cover_art(&cover, 600).await.unwrap();
        println!("pochette : {} octets", bytes.len());
        assert!(!songs.is_empty());
    }
}
