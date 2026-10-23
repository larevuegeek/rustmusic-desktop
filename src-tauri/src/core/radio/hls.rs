//! Radios en HLS : listes `.m3u8` (maître → variante, puis segments), et son extrait des segments,
//! MPEG-TS (démultiplexé ici) ou AAC/MP3 brut précédé d'une étiquette ID3.

/// Une variante d'une liste maître.
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub uri: String,
    pub bandwidth: u64,
    pub codecs: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub uri: String,
    /// Titre annoncé dans `#EXTINF` (rare), affiché comme un titre ICY.
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Media {
    pub target_duration: f64,
    /// Numéro du premier segment de la liste.
    pub sequence: u64,
    pub segments: Vec<Segment>,
    pub ended: bool,
    /// Fragments MP4 (`#EXT-X-MAP`) : autre conteneur, pas encore lu.
    pub fmp4: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Playlist {
    Master(Vec<Variant>),
    Media(Media),
}

fn join(base: &str, uri: &str) -> String {
    reqwest::Url::parse(base).and_then(|b| b.join(uri)).map(|u| u.to_string()).unwrap_or_else(|_| uri.to_string())
}

/// `CLÉ=valeur,CLÉ="valeur, avec virgule"` → valeur de `key`.
fn attribute(list: &str, key: &str) -> Option<String> {
    let mut rest = list;
    while !rest.is_empty() {
        let (k, after) = rest.split_once('=')?;
        let (value, next) = match after.strip_prefix('"') {
            Some(quoted) => {
                let end = quoted.find('"')?;
                (&quoted[..end], quoted[end + 1..].trim_start_matches(','))
            }
            None => after.split_once(',').unwrap_or((after, "")),
        };
        if k.trim().eq_ignore_ascii_case(key) {
            return Some(value.to_string());
        }
        rest = next;
    }
    None
}

pub fn parse(text: &str, base: &str) -> Result<Playlist, String> {
    if !text.trim_start().starts_with("#EXTM3U") {
        return Err("ce n'est pas une liste HLS".into());
    }
    let mut variants = Vec::new();
    let mut media = Media { target_duration: 6.0, sequence: 0, segments: Vec::new(), ended: false, fmp4: false };
    let mut pending_variant: Option<(u64, Option<String>)> = None;
    let mut pending_title: Option<String> = None;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(attrs) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            let bandwidth = attribute(attrs, "BANDWIDTH").and_then(|b| b.parse().ok()).unwrap_or(0);
            pending_variant = Some((bandwidth, attribute(attrs, "CODECS")));
        } else if let Some(v) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
            media.target_duration = v.trim().parse().unwrap_or(6.0);
        } else if let Some(v) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            media.sequence = v.trim().parse().unwrap_or(0);
        } else if line.starts_with("#EXT-X-ENDLIST") {
            media.ended = true;
        } else if line.starts_with("#EXT-X-MAP") {
            media.fmp4 = true;
        } else if let Some(v) = line.strip_prefix("#EXTINF:") {
            // `durée,titre` : un titre ordinaire, pas une suite de `clé=valeur`.
            pending_title = v.split_once(',').map(|(_, t)| t.trim()).filter(|t| !t.is_empty() && !t.contains('=')).map(str::to_string);
        } else if !line.starts_with('#') {
            match pending_variant.take() {
                Some((bandwidth, codecs)) => variants.push(Variant { uri: join(base, line), bandwidth, codecs }),
                None => media.segments.push(Segment { uri: join(base, line), title: pending_title.take() }),
            }
        }
    }
    Ok(if variants.is_empty() { Playlist::Media(media) } else { Playlist::Master(variants) })
}

/// La variante à écouter : AAC-LC ou MP3 de préférence (l'AAC+ n'est lu qu'étouffé), au plus haut débit.
pub fn pick(variants: &[Variant]) -> Option<&Variant> {
    let he_aac = |v: &&Variant| v.codecs.as_deref().is_some_and(|c| c.contains("mp4a.40.5") || c.contains("mp4a.40.29"));
    variants
        .iter()
        .filter(|v| !he_aac(v))
        .max_by_key(|v| v.bandwidth)
        .or_else(|| variants.iter().max_by_key(|v| v.bandwidth))
}

/// Format du flux d'après la variante (pour la fiche de la station).
pub fn codec_label(codecs: Option<&str>) -> Option<&'static str> {
    let c = codecs?;
    Some(if c.contains("mp4a.40.5") || c.contains("mp4a.40.29") {
        "AAC+"
    } else if c.contains("mp4a.40.34") || c.contains("mp4a.6b") {
        "MP3"
    } else if c.contains("mp4a") {
        "AAC"
    } else {
        return None;
    })
}

/// Son extrait des segments, d'un segment à l'autre (l'état du démultiplexeur se garde).
#[derive(Default)]
pub struct Extractor {
    pmt_pid: Option<u16>,
    audio_pid: Option<u16>,
    format: Option<&'static str>,
    pes: Vec<u8>,
}

/// Taille d'une étiquette ID3v2 en tête, 0 sans étiquette.
fn id3_len(data: &[u8]) -> usize {
    if data.len() < 10 || &data[..3] != b"ID3" {
        return 0;
    }
    let size = data[6..10].iter().fold(0usize, |acc, b| (acc << 7) | (*b as usize & 0x7f));
    10 + size + if data[5] & 0x10 != 0 { 10 } else { 0 }
}

/// AAC (ADTS) ou MP3, d'après la synchro de la première trame.
fn sniff(audio: &[u8]) -> Option<&'static str> {
    let (a, b) = (*audio.first()?, *audio.get(1)?);
    if a != 0xFF || b & 0xE0 != 0xE0 {
        return None;
    }
    // ADTS : couche « 00 » ; MP3 : couche 1 à 3.
    Some(if b & 0x06 == 0 { "aac" } else { "mp3" })
}

impl Extractor {
    /// Son du segment, et son format une fois reconnu.
    pub fn extract(&mut self, data: &[u8]) -> (Vec<u8>, Option<&'static str>) {
        let ts = data.first() == Some(&0x47) && (data.len() < 189 || data[188] == 0x47);
        let audio = if ts {
            let mut out = self.demux(data);
            out.extend(self.flush());
            out
        } else {
            data[id3_len(data).min(data.len())..].to_vec()
        };
        if self.format.is_none() {
            self.format = sniff(&audio);
        }
        (audio, self.format)
    }

    fn demux(&mut self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut i = 0;
        while i + 188 <= data.len() {
            if data[i] != 0x47 {
                i += 1;
                continue;
            }
            let p = &data[i..i + 188];
            i += 188;
            let start = p[1] & 0x40 != 0;
            let pid = (u16::from(p[1] & 0x1f) << 8) | u16::from(p[2]);
            let mut off = 4;
            match (p[3] >> 4) & 0x3 {
                1 => {}
                3 => off += 1 + p[4] as usize,
                _ => continue,
            }
            if off >= 188 {
                continue;
            }
            let payload = &p[off..];
            if pid == 0 {
                self.pat(payload, start);
            } else if Some(pid) == self.pmt_pid {
                self.pmt(payload, start);
            } else if Some(pid) == self.audio_pid {
                if start {
                    out.extend(self.flush());
                }
                self.pes.extend_from_slice(payload);
            }
        }
        out
    }

    /// Le paquet PES en cours, sans son en-tête.
    fn flush(&mut self) -> Vec<u8> {
        let p = std::mem::take(&mut self.pes);
        if p.len() < 9 || p[..3] != [0, 0, 1] {
            return Vec::new();
        }
        let header = 9 + p[8] as usize;
        p.get(header..).map(<[u8]>::to_vec).unwrap_or_default()
    }

    /// Section de table : après le pointeur, jusqu'au CRC exclu.
    fn section(payload: &[u8], start: bool) -> Option<&[u8]> {
        let d = if start { payload.get(1 + *payload.first()? as usize..)? } else { payload };
        let len = ((usize::from(*d.get(1)? & 0x0f)) << 8) | usize::from(*d.get(2)?);
        d.get(..(3 + len).min(d.len()).checked_sub(4)?)
    }

    fn pat(&mut self, payload: &[u8], start: bool) {
        let Some(d) = Self::section(payload, start) else { return };
        let mut j = 8;
        while j + 4 <= d.len() {
            let program = u16::from_be_bytes([d[j], d[j + 1]]);
            if program != 0 {
                self.pmt_pid = Some((u16::from(d[j + 2] & 0x1f) << 8) | u16::from(d[j + 3]));
                return;
            }
            j += 4;
        }
    }

    fn pmt(&mut self, payload: &[u8], start: bool) {
        let Some(d) = Self::section(payload, start) else { return };
        if d.len() < 12 {
            return;
        }
        let mut j = 12 + ((usize::from(d[10] & 0x0f) << 8) | usize::from(d[11]));
        while j + 5 <= d.len() {
            let kind = d[j];
            let pid = (u16::from(d[j + 1] & 0x1f) << 8) | u16::from(d[j + 2]);
            // 0x0F : AAC (ADTS) ; 0x03 / 0x04 : MP3.
            if self.audio_pid.is_none() && matches!(kind, 0x0F | 0x03 | 0x04) {
                self.audio_pid = Some(pid);
                self.format = Some(if kind == 0x0F { "aac" } else { "mp3" });
            }
            j += 5 + ((usize::from(d[j + 3] & 0x0f) << 8) | usize::from(d[j + 4]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_then_media_playlists() {
        let master = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=64000,CODECS=\"mp4a.40.5\"\nlow.m3u8\n\
                      #EXT-X-STREAM-INF:BANDWIDTH=192000,CODECS=\"mp4a.40.2\"\nhifi/live.m3u8\n\
                      #EXT-X-STREAM-INF:BANDWIDTH=320000,CODECS=\"mp4a.40.29\"\nhe.m3u8\n";
        let Ok(Playlist::Master(v)) = parse(master, "https://radio.example/fip/master.m3u8?id=1") else { panic!() };
        let best = pick(&v).unwrap();
        assert_eq!(best.uri, "https://radio.example/fip/hifi/live.m3u8");
        assert_eq!(codec_label(best.codecs.as_deref()), Some("AAC"));

        let media = "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-MEDIA-SEQUENCE:812\n#EXTINF:4.0,\nseg812.aac\n#EXTINF:4.0,Daft Punk - One More Time\nseg813.aac\n";
        let Ok(Playlist::Media(m)) = parse(media, "https://radio.example/fip/hifi/live.m3u8") else { panic!() };
        assert_eq!((m.target_duration, m.sequence, m.segments.len(), m.fmp4, m.ended), (4.0, 812, 2, false, false));
        assert_eq!(m.segments[1].uri, "https://radio.example/fip/hifi/seg813.aac");
        assert_eq!(m.segments[1].title.as_deref(), Some("Daft Punk - One More Time"));
        assert!(parse("<html>", "https://x").is_err());
    }

    #[test]
    fn packed_audio_loses_its_id3_tag() {
        let mut seg = b"ID3\x04\x00\x00\x00\x00\x00\x05PRIV!".to_vec();
        seg.extend_from_slice(&[0xFF, 0xF1, 0x50, 0x80, 1, 2, 3]);
        let (audio, format) = Extractor::default().extract(&seg);
        assert_eq!(audio, vec![0xFF, 0xF1, 0x50, 0x80, 1, 2, 3]);
        assert_eq!(format, Some("aac"));
    }

    /// Paquet TS de 188 octets : en-tête, puis charge complétée par du bourrage.
    fn packet(pid: u16, start: bool, payload: &[u8]) -> Vec<u8> {
        let mut p = vec![0x47, (if start { 0x40 } else { 0 }) | ((pid >> 8) as u8 & 0x1f), pid as u8, 0x10];
        p.extend_from_slice(payload);
        p.resize(188, 0xFF);
        p
    }

    #[test]
    fn mpeg_ts_is_demultiplexed() {
        // PAT : programme 1 → PMT en 0x1000 ; PMT : flux AAC (0x0F) en 0x0101.
        let pat = [0, 0x00, 0xB0, 0x0D, 0, 1, 0xC1, 0, 0, 0, 1, 0xF0, 0x00, 0, 0, 0, 0];
        let pmt = [0, 0x02, 0xB0, 0x12, 0, 1, 0xC1, 0, 0, 0xE1, 0x01, 0xF0, 0x00, 0x0F, 0xE1, 0x01, 0xF0, 0x00, 0, 0, 0, 0];
        // PES : en-tête de 9 octets + 5 de PTS, puis l'ADTS.
        let mut pes = vec![0, 0, 1, 0xC0, 0, 0, 0x80, 0x80, 5, 0x21, 0, 1, 0, 1];
        pes.extend_from_slice(&[0xFF, 0xF1, 0x50, 0x80, 9, 9]);
        let mut seg = packet(0, true, &pat);
        seg.extend(packet(0x1000, true, &pmt));
        seg.extend(packet(0x0101, true, &pes));
        let (audio, format) = Extractor::default().extract(&seg);
        assert_eq!(format, Some("aac"));
        assert_eq!(&audio[..6], &[0xFF, 0xF1, 0x50, 0x80, 9, 9]);
    }
}
