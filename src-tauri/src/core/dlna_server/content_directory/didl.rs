//! Builder for DIDL-Lite XML responses.
//!
//! DIDL-Lite is the XML schema DLNA uses inside ContentDirectory `Browse`
//! responses to describe items (containers = folders, items = tracks).
//!
//! Reference : UPnP ContentDirectory:1 specification, Annex B.

use std::fmt::Write;

use crate::core::dlna_server::content_directory::object_id::encode_path;
use crate::core::dlna_server::library::provider::{DlnaAlbum, DlnaArtist, DlnaLibrary, DlnaTrack};
use crate::core::dlna_server::xml::{xml_attr, xml_text};

/// Quatrième champ du protocolInfo : saut par octets (OP=01) et pause par
/// arrêt de la connexion (FLAGS). Sans lui, l'ampli désactive pause et avance.
pub const DLNA_FEATURES: &str = "DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000";

/// Builds a DIDL-Lite XML document one item at a time.
pub struct DidlBuilder {
    inner: String,
    /// `http://192.168.1.10:8200` — used to build absolute media URLs.
    server_base_url: String,
}

impl DidlBuilder {
    pub fn new(server_base_url: String) -> Self {
        let mut inner = String::with_capacity(2048);
        inner.push_str(
            r#"<DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dlna="urn:schemas-dlna-org:metadata-1-0/">"#,
        );
        Self {
            inner,
            server_base_url,
        }
    }

    /// Append a generic `<container>` for a browsable folder
    /// (e.g. root, "Artists", "Albums").
    pub fn add_container(
        &mut self,
        id: &str,
        parent_id: &str,
        title: &str,
        child_count: u32,
        upnp_class: &str,
    ) {
        let _ = write!(
            self.inner,
            "<container id=\"{}\" parentID=\"{}\" childCount=\"{}\" restricted=\"1\">\
             <dc:title>{}</dc:title>\
             <upnp:class>{}</upnp:class>\
             </container>",
            xml_attr(id),
            xml_attr(parent_id),
            child_count,
            xml_text(title),
            upnp_class
        );
    }

    /// Append a `<container>` representing a library root.
    /// Each library exposes its own Artists/Albums/Folders children, so the
    /// container is browsable. We expose the artist count as `childCount`
    /// (not strictly accurate — there are also Albums/Folders subcontainers
    /// — but most amps only use it as a non-zero hint).
    pub fn add_library(&mut self, library: &DlnaLibrary, parent_id: &str) {
        let id = format!("0/lib/{}", library.id);
        // 3 fixed subcontainers (Artists, Albums, Folders)
        let _ = write!(
            self.inner,
            "<container id=\"{}\" parentID=\"{}\" childCount=\"3\" restricted=\"1\">\
             <dc:title>{}</dc:title>\
             <upnp:class>object.container.storageFolder</upnp:class>\
             </container>",
            xml_attr(&id),
            xml_attr(parent_id),
            xml_text(&library.name)
        );
    }

    /// Append a `<container>` representing an artist folder.
    pub fn add_artist(&mut self, library_id: i64, artist: &DlnaArtist, parent_id: &str) {
        let id = format!("0/lib/{}/artists/{}", library_id, artist.id);
        let _ = write!(
            self.inner,
            "<container id=\"{}\" parentID=\"{}\" childCount=\"{}\" restricted=\"1\">\
             <dc:title>{}</dc:title>\
             <upnp:class>object.container.person.musicArtist</upnp:class>\
             </container>",
            xml_attr(&id),
            xml_attr(parent_id),
            artist.album_count,
            xml_text(&artist.name)
        );
    }

    /// Append a `<container>` representing a filesystem folder.
    /// `path` is the absolute filesystem path; it's base64-encoded into
    /// the DLNA object ID.
    ///
    /// `childCount` is set to 1 (we don't know the real count without
    /// listing — and many amps refuse to browse a container annoncé
    /// avec childCount=0).
    pub fn add_folder(&mut self, library_id: i64, name: &str, path: &str, parent_id: &str) {
        let id = format!("0/lib/{}/folders/{}", library_id, encode_path(path));
        let _ = write!(
            self.inner,
            "<container id=\"{}\" parentID=\"{}\" childCount=\"1\" restricted=\"1\">\
             <dc:title>{}</dc:title>\
             <upnp:class>object.container.storageFolder</upnp:class>\
             </container>",
            xml_attr(&id),
            xml_attr(parent_id),
            xml_text(name)
        );
    }

    /// Append a `<container>` representing an album folder.
    pub fn add_album(&mut self, library_id: i64, album: &DlnaAlbum, parent_id: &str) {
        let id = format!("0/lib/{}/albums/{}", library_id, album.id);
        let creator = album.artist.as_deref().unwrap_or("Unknown");
        let _ = write!(
            self.inner,
            "<container id=\"{}\" parentID=\"{}\" childCount=\"{}\" restricted=\"1\">\
             <dc:title>{}</dc:title>\
             <upnp:class>object.container.album.musicAlbum</upnp:class>\
             <dc:creator>{}</dc:creator>\
             <upnp:artist>{}</upnp:artist>",
            xml_attr(&id),
            xml_attr(parent_id),
            album.track_count,
            xml_text(&album.title),
            xml_text(creator),
            xml_text(creator)
        );
        if let Some(year) = album.year {
            let _ = write!(self.inner, "<dc:date>{}-01-01</dc:date>", year);
        }
        if album.has_cover {
            self.write_album_art_uri("album", &album.id);
        }
        self.inner.push_str("</container>");
    }

    /// Append an `<item>` representing a streamable audio track.
    /// `parent_id` is the album-or-folder id this track lives under
    /// (e.g. `"0/albums/<album-id>"`).
    pub fn add_track(&mut self, track: &DlnaTrack, parent_id: &str) {
        let id = format!("{}/tracks/{}", parent_id, track.id);
        let url = format!("{}/media/{}", self.server_base_url, track.id);
        let creator = track.artist.as_deref().unwrap_or("Unknown");

        let _ = write!(
            self.inner,
            "<item id=\"{}\" parentID=\"{}\" restricted=\"1\">\
             <dc:title>{}</dc:title>\
             <upnp:class>object.item.audioItem.musicTrack</upnp:class>\
             <dc:creator>{}</dc:creator>\
             <upnp:artist>{}</upnp:artist>",
            xml_attr(&id),
            xml_attr(parent_id),
            xml_text(&track.title),
            xml_text(creator),
            xml_text(creator)
        );

        if let Some(album) = &track.album {
            let _ = write!(self.inner, "<upnp:album>{}</upnp:album>", xml_text(album));
        }
        if let Some(track_num) = track.track_number {
            let _ = write!(
                self.inner,
                "<upnp:originalTrackNumber>{}</upnp:originalTrackNumber>",
                track_num
            );
        }

        // <res> : the streamable resource. Order of attributes matters for
        // some strict clients (e.g. some old Sonos firmwares) — keep
        // protocolInfo first.
        let _ = write!(
            self.inner,
            "<res protocolInfo=\"http-get:*:{}:{}\"",
            xml_attr(&track.mime_type),
            DLNA_FEATURES
        );
        if let Some(size) = track.file_size {
            let _ = write!(self.inner, " size=\"{}\"", size);
        }
        if let Some(dur) = track.duration_seconds {
            let _ = write!(self.inner, " duration=\"{}\"", format_duration(dur));
        }
        if let Some(br_kbps) = track.bitrate {
            // DIDL `bitrate` attribute is in BYTES per second per spec.
            // Our internal bitrate is kbps → bytes/s = kbps * 1000 / 8 = kbps * 125.
            let _ = write!(self.inner, " bitrate=\"{}\"", br_kbps as u64 * 125);
        }
        if let Some(sr) = track.sample_rate {
            let _ = write!(self.inner, " sampleFrequency=\"{}\"", sr);
        }
        if let Some(bps) = track.bits_per_sample {
            let _ = write!(self.inner, " bitsPerSample=\"{}\"", bps);
        }
        if let Some(ch) = track.channels {
            let _ = write!(self.inner, " nrAudioChannels=\"{}\"", ch);
        }
        let _ = write!(self.inner, ">{}</res>", xml_text(&url));

        if track.has_cover {
            self.write_album_art_uri("track", &track.id);
        }

        self.inner.push_str("</item>");
    }

    /// Finalize and return the full DIDL-Lite XML document.
    pub fn build(mut self) -> String {
        self.inner.push_str("</DIDL-Lite>");
        self.inner
    }

    /// Append a `<upnp:albumArtURI>` pointing to the server's `/cover/<kind>:<id>`
    /// endpoint. Includes the DLNA `profileID` hint so amps know to fetch it
    /// as a JPEG thumbnail.
    fn write_album_art_uri(&mut self, kind: &str, id: &str) {
        let url = format!("{}/cover/{}:{}", self.server_base_url, kind, id);
        let _ = write!(
            self.inner,
            "<upnp:albumArtURI dlna:profileID=\"JPEG_TN\">{}</upnp:albumArtURI>",
            xml_text(&url)
        );
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────

/// Format a duration in seconds as `H:MM:SS.000` (DIDL `duration` format).
fn format_duration(seconds: u32) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    format!("{}:{:02}:{:02}.000", h, m, s)
}
