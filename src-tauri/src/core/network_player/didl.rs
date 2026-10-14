//! Métadonnées DIDL-Lite envoyées avec chaque piste : ce qu'affiche le lecteur.

use super::control::format_time;
use super::media::DLNA_FLAGS;
use super::xml::escape;

/// Ce qu'on dit d'une piste au lecteur.
#[derive(Debug, Clone, Default)]
pub struct TrackMeta {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub cover_url: Option<String>,
    pub duration: Option<f64>,
    pub size: Option<u64>,
    pub sample_rate: Option<u32>,
    pub bits: Option<u32>,
    pub channels: Option<u32>,
}

/// Document DIDL-Lite d'une piste servie à `url` au type `mime`.
pub fn item(meta: &TrackMeta, url: &str, mime: &str) -> String {
    let mut out = String::from(
        "<DIDL-Lite xmlns=\"urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/\" \
xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
xmlns:upnp=\"urn:schemas-upnp-org:metadata-1-0/upnp/\" \
xmlns:dlna=\"urn:schemas-dlna-org:metadata-1-0/\">\
<item id=\"0\" parentID=\"-1\" restricted=\"1\">",
    );
    out.push_str(&format!("<dc:title>{}</dc:title>", escape(&meta.title)));
    if let Some(artist) = &meta.artist {
        out.push_str(&format!("<dc:creator>{0}</dc:creator><upnp:artist>{0}</upnp:artist>", escape(artist)));
    }
    if let Some(album_artist) = &meta.album_artist {
        out.push_str(&format!("<upnp:artist role=\"AlbumArtist\">{}</upnp:artist>", escape(album_artist)));
    }
    if let Some(album) = &meta.album {
        out.push_str(&format!("<upnp:album>{}</upnp:album>", escape(album)));
    }
    if let Some(cover) = &meta.cover_url {
        out.push_str(&format!("<upnp:albumArtURI>{}</upnp:albumArtURI>", escape(cover)));
    }
    out.push_str("<upnp:class>object.item.audioItem.musicTrack</upnp:class>");

    let mut res = format!("<res protocolInfo=\"http-get:*:{}:{}\"", escape(mime), DLNA_FLAGS);
    if let Some(d) = meta.duration.filter(|d| *d > 0.0) {
        res.push_str(&format!(" duration=\"{}.000\"", format_time(d)));
    }
    let attrs = [
        ("size", meta.size.map(|v| v.to_string())),
        ("sampleFrequency", meta.sample_rate.map(|v| v.to_string())),
        ("bitsPerSample", meta.bits.filter(|b| *b > 0).map(|v| v.to_string())),
        ("nrAudioChannels", meta.channels.map(|v| v.to_string())),
    ];
    for (name, value) in attrs {
        if let Some(v) = value {
            res.push_str(&format!(" {name}=\"{v}\""));
        }
    }
    res.push_str(&format!(">{}</res>", escape(url)));
    out.push_str(&res);
    out.push_str("</item></DIDL-Lite>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::network_player::xml;

    #[test]
    fn describes_a_track_readably() {
        let meta = TrackMeta {
            title: "Rock & Roll".into(),
            artist: Some("Led Zeppelin".into()),
            album: Some("IV".into()),
            cover_url: Some("http://10.0.0.121:8201/art/ab".into()),
            duration: Some(220.0),
            sample_rate: Some(96000),
            bits: Some(24),
            channels: Some(2),
            ..Default::default()
        };
        let d = item(&meta, "http://10.0.0.121:8201/stream/ab.flac", "audio/flac");
        assert_eq!(xml::text(&d, "title").as_deref(), Some("Rock & Roll"));
        assert_eq!(xml::text(&d, "albumArtURI").as_deref(), Some("http://10.0.0.121:8201/art/ab"));
        assert!(d.contains("protocolInfo=\"http-get:*:audio/flac:DLNA.ORG_OP=01"));
        assert!(d.contains("duration=\"0:03:40.000\""));
        assert!(d.contains("sampleFrequency=\"96000\" bitsPerSample=\"24\" nrAudioChannels=\"2\""));
        assert_eq!(xml::text(&d, "res").as_deref(), Some("http://10.0.0.121:8201/stream/ab.flac"));
    }
}
