//! Catalogue DLNA : dossiers d'un serveur, morceaux d'un dossier (récursif), flux et pochettes.

use std::collections::HashSet;
use std::time::Duration;

use serde::Serialize;

use super::client::Server;
use crate::core::network_player::control::parse_time;
use crate::core::podcast::fetch::client;
use crate::core::subsonic::catalog::RemoteSong;
use crate::core::subsonic::client::MusicFolder;
use crate::helper::xml;

const PAGE: u32 = 200;
/// Garde-fou contre une arborescence qui boucle.
const MAX_DEPTH: usize = 32;

/// Un dossier du serveur ; `children` : le nombre annoncé d'éléments, si le serveur le dit.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub children: Option<u32>,
}

/// Identifiant DLNA → segment de chemin `remote://` (un `/` y couperait le chemin).
pub(crate) fn encode_id(id: &str) -> String {
    id.replace('%', "%25").replace('/', "%2F")
}

pub(crate) fn decode_id(s: &str) -> String {
    s.replace("%2F", "/").replace("%25", "%")
}

fn parse_folders(didl: &str) -> Vec<Folder> {
    xml::tagged(didl, "container")
        .into_iter()
        .filter_map(|(tag, inner)| {
            Some(Folder {
                id: xml::attr(tag, "id")?,
                name: xml::text(inner, "title").unwrap_or_default(),
                children: xml::attr(tag, "childCount").and_then(|c| c.parse().ok()),
            })
        })
        .collect()
}

/// Extension du fichier : celle de l'adresse, sinon d'après le type annoncé.
fn suffix_of(url: &str, protocol: &str) -> Option<String> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let last = path.rsplit('/').next().unwrap_or("");
    if let Some((_, ext)) = last.rsplit_once('.') {
        if (1..=5).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Some(ext.to_ascii_lowercase());
        }
    }
    let mime = protocol.split(':').nth(2).unwrap_or("").to_ascii_lowercase();
    let ext = match mime.as_str() {
        "audio/flac" | "audio/x-flac" => "flac",
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/mp4" | "audio/x-m4a" | "audio/m4a" | "audio/aac" => "m4a",
        "audio/x-dsf" | "audio/dsf" | "audio/dsd" => "dsf",
        "audio/x-dff" | "audio/dff" => "dff",
        "audio/wav" | "audio/x-wav" | "audio/wave" => "wav",
        "audio/ogg" | "audio/x-ogg" => "ogg",
        "audio/aiff" | "audio/x-aiff" => "aiff",
        "audio/x-ms-wma" => "wma",
        _ => return None,
    };
    Some(ext.into())
}

/// Le fichier d'origine parmi les `<res>` : audio, et pas une conversion du serveur (`CI=1`).
fn pick_res(inner: &str) -> Option<(&str, String)> {
    let all: Vec<(&str, String)> = xml::tagged(inner, "res")
        .into_iter()
        .map(|(tag, url)| (tag, xml::decode(url).trim().to_string()))
        .filter(|(tag, url)| !url.is_empty() && xml::attr(tag, "protocolInfo").is_none_or(|p| p.contains("audio/") || p.contains("*:*")))
        .collect();
    let original = all.iter().position(|(tag, _)| !xml::attr(tag, "protocolInfo").unwrap_or_default().contains("DLNA.ORG_CI=1"));
    all.into_iter().nth(original.unwrap_or(0))
}

fn short_hash(s: &str) -> String {
    format!("{:x}", md5::compute(s))[..16].to_string()
}

/// Un morceau du DIDL ; `folder` : le chemin des dossiers parcourus, `parent` : le dossier qui le contient.
pub(crate) fn parse_item(tag: &str, inner: &str, folder: &str, parent: &str) -> Option<RemoteSong> {
    if !xml::text(inner, "class").is_some_and(|c| c.starts_with("object.item.audioItem")) {
        return None;
    }
    let id = xml::attr(tag, "id")?;
    let (res_tag, url) = pick_res(inner)?;
    let res = |k: &str| xml::attr(res_tag, k);
    let num = |v: Option<String>| v.and_then(|s| s.trim().parse::<f64>().ok()).filter(|n| *n > 0.0);
    let text = |k: &str| xml::text(inner, k).filter(|v| !v.is_empty());
    // `upnp:artist` sans rôle (ou interprète) ; l'artiste de l'album porte `role="AlbumArtist"`.
    let artists = xml::tagged(inner, "artist");
    let with_role = |want: &dyn Fn(Option<String>) -> bool| -> Option<String> {
        let names: Vec<String> = artists.iter().filter(|(t, _)| want(xml::attr(t, "role"))).map(|(_, v)| xml::decode(v).trim().to_string()).filter(|v| !v.is_empty()).collect();
        (!names.is_empty()).then(|| names.join("; "))
    };
    let artist = with_role(&|r| r.is_none_or(|r| r.eq_ignore_ascii_case("Performer"))).or_else(|| text("creator"));
    let album_artist = with_role(&|r| r.is_some_and(|r| r.eq_ignore_ascii_case("AlbumArtist"))).or_else(|| text("albumArtist"));
    let album = text("album");
    let title = text("title").unwrap_or_else(|| id.clone());
    let suffix = suffix_of(&url, &res("protocolInfo").unwrap_or_default());
    // Album : son nom et son artiste ; à défaut, le dossier.
    let album_id = match &album {
        Some(a) => short_hash(&format!("{}\u{1}{a}", album_artist.as_deref().or(artist.as_deref()).unwrap_or(""))),
        None => short_hash(parent),
    };
    Some(RemoteSong {
        album_id: Some(album_id),
        artist,
        album_artist,
        album,
        year: text("date").and_then(|d| d.get(..4)?.parse().ok()),
        genre: text("genre"),
        track: num(text("originalTrackNumber")).map(|n| n as u16),
        disc: num(text("originalDiscNumber")).map(|n| n as u16),
        duration: res("duration").and_then(|d| parse_time(&d)).filter(|d| *d > 0.0),
        // UPnP : débit en octets par seconde.
        bit_rate: num(res("bitrate")).map(|b| (b * 8.0 / 1000.0) as u32),
        bit_depth: num(res("bitsPerSample")).map(|b| b as u32),
        sampling_rate: num(res("sampleFrequency")).map(|r| r as u32),
        channels: num(res("nrAudioChannels")).map(|c| c as u32),
        size: num(res("size")).map(|s| s as i64),
        cover_art: text("albumArtURI"),
        user_rating: None,
        starred: false,
        server_path: Some(format!("{folder}/{title}{}", suffix.as_deref().map(|s| format!(".{s}")).unwrap_or_default())),
        suffix,
        title,
        id: encode_id(&id),
    })
}

impl Server {
    /// Les sous-dossiers de `id` (`0` : la racine).
    pub async fn folders(&self, id: &str) -> Result<Vec<Folder>, String> {
        let mut out = Vec::new();
        let mut start = 0;
        loop {
            let (didl, returned, total) = self.browse(id, true, start, PAGE).await?;
            out.extend(parse_folders(&didl));
            start += returned;
            if returned == 0 || (total > 0 && start >= total) {
                break;
            }
        }
        Ok(out)
    }

    /// Le nom d'un dossier choisi, pour sa bibliothèque.
    pub async fn folder(&self, id: &str) -> Result<MusicFolder, String> {
        let (didl, _, _) = self.browse(id, false, 0, 1).await?;
        let name = xml::tagged(&didl, "container").into_iter().find_map(|(_, inner)| xml::text(inner, "title")).filter(|n| !n.is_empty());
        Ok(MusicFolder { id: id.to_string(), name: name.unwrap_or_else(|| "DLNA".into()) })
    }

    /// Tous les morceaux sous un dossier, sous-dossiers compris ; `progress` reçoit le nombre trouvé.
    pub async fn all_songs(&self, folder_id: &str, mut progress: impl FnMut(usize)) -> Result<Vec<RemoteSong>, String> {
        let root = self.folder(folder_id).await.map(|f| f.name).unwrap_or_default();
        let mut stack = vec![(folder_id.to_string(), root, 0usize)];
        let mut visited = HashSet::new();
        let mut seen = HashSet::new();
        let mut songs = Vec::new();
        while let Some((id, path, depth)) = stack.pop() {
            if depth > MAX_DEPTH || !visited.insert(id.clone()) {
                continue;
            }
            let mut start = 0;
            loop {
                let (didl, returned, total) = self.browse(&id, true, start, PAGE).await?;
                for f in parse_folders(&didl).into_iter().rev() {
                    stack.push((f.id, format!("{path}/{}", f.name), depth + 1));
                }
                for (tag, inner) in xml::tagged(&didl, "item") {
                    if let Some(mut song) = parse_item(tag, inner, &path, &id) {
                        if seen.insert(song.id.clone()) {
                            song.cover_art = song.cover_art.map(|u| self.rebase(&u));
                            songs.push(song);
                        }
                    }
                }
                start += returned;
                if returned == 0 || (total > 0 && start >= total) {
                    break;
                }
            }
            progress(songs.len());
        }
        Ok(songs)
    }

    /// L'adresse du fichier, redemandée au serveur (elle peut changer d'un démarrage à l'autre).
    pub async fn stream_url(&self, song_id: &str) -> Result<reqwest::Url, String> {
        let (didl, _, _) = self.browse(&decode_id(song_id), false, 0, 1).await?;
        let (_, inner) = xml::tagged(&didl, "item").into_iter().next().ok_or("morceau introuvable sur le serveur")?;
        let (_, url) = pick_res(inner).ok_or("le serveur ne donne pas de fichier pour ce morceau")?;
        reqwest::Url::parse(&self.rebase(&url)).map_err(|e| e.to_string())
    }

    /// La pochette : `id` est son adresse (`upnp:albumArtURI`).
    pub async fn cover_art(&self, id: &str) -> Option<Vec<u8>> {
        let resp = client().get(self.rebase(id)).timeout(Duration::from_secs(20)).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        resp.bytes().await.ok().map(|b| b.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIDL: &str = r#"<DIDL-Lite xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/" xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/">
<container id="64$1$0" parentID="64$1" restricted="1" childCount="12"><dc:title>Queen - 1975 - A Night at the Opera</dc:title><upnp:class>object.container.storageFolder</upnp:class></container>
<item id="64$1$3" parentID="64$1" restricted="1"><dc:title>Bohemian Rhapsody</dc:title><upnp:class>object.item.audioItem.musicTrack</upnp:class>
<dc:creator>Queen</dc:creator><upnp:artist>Queen</upnp:artist><upnp:artist role="AlbumArtist">Queen</upnp:artist><upnp:album>A Night at the Opera</upnp:album>
<upnp:genre>Rock</upnp:genre><upnp:originalTrackNumber>11</upnp:originalTrackNumber><dc:date>1975-11-21</dc:date>
<upnp:albumArtURI dlna:profileID="JPEG_TN" xmlns:dlna="urn:schemas-dlna-org:metadata-1-0/">http://172.21.0.2:8201/AlbumArt/7-22.jpg</upnp:albumArtURI>
<res size="62914560" duration="0:05:55.000" bitrate="211680" sampleFrequency="96000" bitsPerSample="24" nrAudioChannels="2" protocolInfo="http-get:*:audio/x-flac:*">http://172.21.0.2:8201/MediaItems/22.flac</res></item>
<item id="64$1$4"><dc:title>Photo</dc:title><upnp:class>object.item.imageItem.photo</upnp:class><res protocolInfo="http-get:*:image/jpeg:*">http://x/1.jpg</res></item>
</DIDL-Lite>"#;

    #[test]
    fn reads_folders_and_tracks() {
        let folders = parse_folders(DIDL);
        assert_eq!(folders, vec![Folder { id: "64$1$0".into(), name: "Queen - 1975 - A Night at the Opera".into(), children: Some(12) }]);
        let items: Vec<RemoteSong> = xml::tagged(DIDL, "item").into_iter().filter_map(|(t, i)| parse_item(t, i, "Musique/FLAC", "64$1")).collect();
        assert_eq!(items.len(), 1, "la photo est écartée");
        let s = &items[0];
        assert_eq!((s.id.as_str(), s.title.as_str(), s.suffix.as_deref()), ("64$1$3", "Bohemian Rhapsody", Some("flac")));
        assert_eq!((s.artist.as_deref(), s.album_artist.as_deref(), s.album.as_deref()), (Some("Queen"), Some("Queen"), Some("A Night at the Opera")));
        assert_eq!((s.track, s.year, s.genre.as_deref()), (Some(11), Some(1975), Some("Rock")));
        assert_eq!((s.bit_rate, s.bit_depth, s.sampling_rate, s.channels), (Some(1693), Some(24), Some(96000), Some(2)));
        assert_eq!(s.duration, Some(355.0));
        assert_eq!(s.server_path.as_deref(), Some("Musique/FLAC/Bohemian Rhapsody.flac"));
        assert_eq!(s.cover_art.as_deref(), Some("http://172.21.0.2:8201/AlbumArt/7-22.jpg"));
    }

    #[test]
    fn ids_survive_the_path() {
        let id = "0/music/50%/a";
        assert!(!encode_id(id).contains('/'));
        assert_eq!(decode_id(&encode_id(id)), id);
        assert_eq!(suffix_of("http://h/stream?id=4", "http-get:*:audio/x-dsf:*").as_deref(), Some("dsf"));
    }

    // Essai réel : le MiniDLNA d'essai de David (`J:\docker\minidlna`), s'il tourne.
    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_minidlna() {
        let (server, info) = Server::connect("localhost:8201").await.unwrap();
        println!("{info:?}");
        let root = server.folders("0").await.unwrap();
        println!("racine : {root:?}");
        let browse = root.iter().find(|f| f.id == "64").expect("Browse Folders");
        let first = server.folders(&browse.id).await.unwrap().remove(0);
        let sub = server.folders(&first.id).await.unwrap();
        let target = sub.first().map(|f| f.id.clone()).unwrap_or(first.id.clone());
        let songs = server.all_songs(&target, |_| {}).await.unwrap();
        let s = &songs[0];
        println!("{} morceaux ; {} — {:?} {:?} {:?}", songs.len(), s.title, s.artist, s.suffix, s.server_path);
        let url = server.stream_url(&s.id).await.unwrap();
        let resp = client().get(url.clone()).header("Range", "bytes=0-1023").send().await.unwrap();
        println!("{url} → HTTP {}", resp.status());
        assert!(resp.status().is_success());
    }
}
