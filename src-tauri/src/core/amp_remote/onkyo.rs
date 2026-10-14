//! Onkyo, Pioneer, Integra : protocole eISCP (TCP 60128).
//!
//! Chaque message est un en-tête « ISCP » suivi de « !1CMDparam ». L'ampli annonce
//! de lui-même chaque changement (télécommande, façade) : l'état reste juste.

use super::model::{Amp, Brand, Caps, Choice, Codec, Command, NavKey, NetLine, Range, Status, Transport, Zone, ZoneState};
use crate::core::network_player::xml;

pub const PORT: u16 = 60128;
const HEADER: usize = 16;

/// Trame eISCP d'une commande (« PWR01 »).
pub fn frame(cmd: &str) -> Vec<u8> {
    frame_unit('1', cmd)
}

fn frame_unit(unit: char, cmd: &str) -> Vec<u8> {
    let data = format!("!{unit}{cmd}\r");
    let mut out = Vec::with_capacity(HEADER + data.len());
    out.extend_from_slice(b"ISCP");
    out.extend_from_slice(&(HEADER as u32).to_be_bytes());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&[1, 0, 0, 0]);
    out.extend_from_slice(data.as_bytes());
    out
}

/// Questions de recherche : Onkyo/Integra (« x ») et Pioneer (« p »).
pub fn discovery_queries() -> Vec<Vec<u8>> {
    vec![frame_unit('x', "ECNQSTN"), frame_unit('p', "ECNQSTN")]
}

/// Message utile d'une trame complète : « PWR01 ». `None` tant qu'elle est incomplète.
fn next_message(buf: &[u8]) -> Option<(usize, Option<String>)> {
    let start = buf.windows(4).position(|w| w == b"ISCP")?;
    if buf.len() < start + HEADER {
        return None;
    }
    let h = &buf[start..];
    let header = u32::from_be_bytes([h[4], h[5], h[6], h[7]]) as usize;
    let size = u32::from_be_bytes([h[8], h[9], h[10], h[11]]) as usize;
    let end = start + header.max(HEADER) + size;
    if buf.len() < end {
        return None;
    }
    let text = String::from_utf8_lossy(&buf[start + header.max(HEADER)..end]);
    let text = text.trim_end_matches(['\u{1a}', '\r', '\n', '\0']);
    // « !1PWR01 » : on retire le « ! » et le type d'appareil.
    let msg = text.strip_prefix('!').and_then(|t| t.get(1..)).map(str::to_string);
    Some((end, msg))
}

/// Réponse de recherche (« !1ECNTX-RZ900/60128/XX/0009B0AA7C6A ») vue de `ip`.
pub fn parse_discovery(data: &[u8], ip: &str) -> Option<Amp> {
    let (_, msg) = next_message(data)?;
    let body = msg?.strip_prefix("ECN")?.to_string();
    let mut parts = body.split('/');
    let model = parts.next()?.trim().to_string();
    let port = parts.next()?.trim().parse().unwrap_or(PORT);
    let _region = parts.next();
    let mac = parts.next()?.trim().trim_end_matches(|c: char| !c.is_ascii_alphanumeric()).to_string();
    Some(Amp { id: format!("onkyo:{mac}"), name: model.clone(), brand: Brand::Onkyo, model, ip: ip.to_string(), port })
}

/// Entrées courantes, pour un ampli qui ne décrit pas les siennes.
const DEFAULT_INPUTS: &[(&str, &str)] = &[
    ("10", "BD/DVD"),
    ("01", "CBL/SAT"),
    ("02", "GAME"),
    ("12", "TV"),
    ("03", "AUX"),
    ("23", "CD"),
    ("22", "PHONO"),
    ("24", "FM"),
    ("2B", "NET"),
    ("2E", "BLUETOOTH"),
];

/// Modes proposés : directs, puis touches de famille (l'ampli fait défiler la sienne).
const MODES: &[(&str, &str)] = &[
    ("00", "Stereo"),
    ("01", "Direct"),
    ("11", "Pure Audio"),
    ("0C", "All Ch Stereo"),
    ("MOVIE", "Movie/TV"),
    ("MUSIC", "Music"),
    ("GAME", "Game"),
];

/// Nom d'un mode d'écoute annoncé par l'ampli (LMD).
fn mode_name(code: &str) -> String {
    let name = match code {
        "00" => "Stereo",
        "01" => "Direct",
        "02" => "Surround",
        "03" => "Film",
        "04" => "THX",
        "05" => "Action",
        "06" => "Musical",
        "07" => "Mono Movie",
        "08" => "Orchestra",
        "09" => "Unplugged",
        "0A" => "Studio-Mix",
        "0B" => "TV Logic",
        "0C" => "All Ch Stereo",
        "0D" => "Theater-Dimensional",
        "0E" => "Enhanced",
        "0F" => "Mono",
        "11" => "Pure Audio",
        "12" => "Multiplex",
        "13" => "Full Mono",
        "14" => "Dolby Virtual",
        "15" => "DTS Surround Sensation",
        "16" => "Audyssey DSX",
        "17" => "DTS Virtual:X",
        "40" => "Straight Decode",
        "41" => "Dolby EX",
        "42" => "THX Cinema",
        "43" => "THX Surround EX",
        "44" => "THX Music",
        "45" => "THX Games",
        "80" => "Dolby Surround",
        "81" => "Dolby PLII Music",
        "82" => "DTS Neural:X",
        "83" => "DTS Neo:6 Music",
        "86" => "Dolby PLII Game",
        "87" => "Neural Surround",
        "FF" => "Auto Surround",
        _ => return format!("Mode {code}"),
    };
    name.to_string()
}

pub fn default_caps() -> Caps {
    Caps {
        inputs: DEFAULT_INPUTS.iter().map(|(id, name)| Choice { id: id.to_string(), name: name.to_string() }).collect(),
        modes: MODES.iter().map(|(id, name)| Choice { id: id.to_string(), name: name.to_string() }).collect(),
        volume_max: 80.0,
        volume_step: 1.0,
        tone: Some(Range { min: -10, max: 10, step: 2 }),
        zone2: false,
    }
}

/// Fiche de l'ampli (NRI) : entrées, zones, plage de volume et de tonalité.
fn apply_nri(xml_text: &str, caps: &mut Caps) {
    let inputs: Vec<Choice> = xml::tagged(xml_text, "selector")
        .into_iter()
        .filter(|(tag, _)| xml::attr(tag, "value").as_deref() == Some("1"))
        .filter_map(|(tag, _)| {
            let id = xml::attr(tag, "id")?.to_ascii_uppercase();
            // « 80 » : la source de la zone principale, proposée aux autres zones.
            (id != "80").then(|| Choice { id, name: xml::attr(tag, "name").unwrap_or_default() })
        })
        .collect();
    if !inputs.is_empty() {
        caps.inputs = inputs;
    }
    for (tag, _) in xml::tagged(xml_text, "zone") {
        let on = xml::attr(&tag, "value").as_deref() == Some("1");
        match xml::attr(tag, "id").as_deref() {
            Some("1") => {
                if let Some(max) = xml::attr(tag, "volmax").and_then(|v| v.parse::<f32>().ok()).filter(|v| *v > 0.0) {
                    caps.volume_max = max;
                }
                // « volstep=0 » : pas d'un demi-point.
                caps.volume_step = if xml::attr(tag, "volstep").as_deref() == Some("0") { 0.5 } else { 1.0 };
            }
            Some("2") => caps.zone2 = on,
            _ => {}
        }
    }
    let bass = xml::tagged(xml_text, "control").into_iter().find(|(tag, _)| xml::attr(tag, "id").as_deref() == Some("Bass"));
    caps.tone = bass.and_then(|(tag, _)| {
        if xml::attr(tag, "value").as_deref() != Some("1") {
            return None;
        }
        let n = |k| xml::attr(tag, k).and_then(|v| v.parse::<i32>().ok());
        Some(Range { min: n("min")?, max: n("max")?, step: n("step").unwrap_or(1).max(1) })
    });
}

/// « +2 », « 00 », « -A » : réglage de tonalité en dB.
fn parse_tone(s: &str) -> Option<i32> {
    let s = s.get(..2)?;
    if s == "00" {
        return Some(0);
    }
    let value = i32::from_str_radix(s.get(1..)?, 16).ok()?;
    match s.get(..1)? {
        "+" => Some(value),
        "-" => Some(-value),
        _ => None,
    }
}

fn encode_tone(v: i32) -> String {
    match v {
        0 => "00".into(),
        v if v > 0 => format!("+{:X}", v.min(15)),
        v => format!("-{:X}", (-v).min(15)),
    }
}

fn hex_volume(param: &str, caps: &Caps) -> Option<f32> {
    let raw = u32::from_str_radix(param, 16).ok()? as f32;
    Some(if caps.volume_step < 1.0 { raw / 2.0 } else { raw })
}

/// Entrées dont l'écran est la liste réseau (NET, USB, anciens « Music Server », « Internet Radio »).
const NET_INPUTS: &[&str] = &["2B", "29", "27", "28", "2A", "2C"];

/// Nature d'une ligne de liste (NLS), en mot lisible par l'interface.
fn line_kind(c: char) -> &'static str {
    match c {
        '0' => "playing",
        'A' => "artist",
        'B' => "album",
        'F' => "folder",
        'M' => "track",
        'P' | 'b' => "playlist",
        'S' => "search",
        'a' => "account",
        _ => "item",
    }
}

#[derive(Default)]
pub struct OnkyoCodec {
    /// Pochette reçue par morceaux (hexadécimal) : (type, données).
    jacket: Option<(char, String)>,
}

impl OnkyoCodec {
    fn apply(&mut self, msg: &str, status: &mut Status) {
        let (cmd, param) = (msg.get(..3).unwrap_or(""), msg.get(3..).unwrap_or(""));
        let on = param == "01";
        let Status { caps, state, cover } = status;
        let net = &mut state.net;
        match cmd {
            // Titre de l'écran réseau : service, type d'écran, curseur, nombre d'éléments, titre.
            "NLT" if param.len() >= 22 => {
                let ui = param.as_bytes()[2];
                net.playback = ui == b'2';
                if matches!(ui, b'0' | b'1' | b'5') {
                    // Découpe sûre : un octet non ASCII ne doit pas faire tomber la session.
                    net.count = param.get(8..12).and_then(|c| u32::from_str_radix(c, 16).ok()).unwrap_or(0);
                    net.title = param.get(22..).unwrap_or_default().to_string();
                }
            }
            // Liste : « C0P » curseur (P : nouvelle page), « U3-Spotify » une ligne.
            "NLS" => {
                let mut chars = param.chars();
                match (chars.next(), chars.next(), chars.next()) {
                    (Some('C'), Some(line), Some(update)) => {
                        if update == 'P' {
                            net.lines.clear();
                        }
                        net.cursor = line.to_digit(10).map(|d| d as u8);
                    }
                    (Some('U' | 'A'), Some(line), Some(kind)) => {
                        if let Some(i) = line.to_digit(10) {
                            let i = i as usize;
                            if net.lines.len() <= i {
                                net.lines.resize(i + 1, NetLine::default());
                            }
                            net.lines[i] = NetLine { text: chars.as_str().to_string(), kind: line_kind(kind).to_string() };
                        }
                    }
                    _ => {}
                }
            }
            "NTI" => net.track = param.to_string(),
            "NAT" => net.artist = param.to_string(),
            "NAL" => net.album = param.to_string(),
            "NTM" => {
                net.time = if param.starts_with("--") { String::new() } else { param.replace('/', " / ") };
            }
            "NST" => {
                net.status = match param.chars().next() {
                    Some('P') => "play",
                    Some('p') => "pause",
                    _ => "stop",
                }
                .to_string();
            }
            // Pochette : « 1 0 <hex> » début, « 1 1 » suite, « 1 2 » fin ; « n- » : aucune.
            "NJA" => {
                let mut chars = param.chars();
                match (chars.next(), chars.next()) {
                    (Some(t @ ('0' | '1')), Some(part)) => {
                        let data = chars.as_str();
                        if part == '0' || self.jacket.is_none() {
                            self.jacket = Some((t, String::new()));
                        }
                        if let Some((_, hex)) = self.jacket.as_mut() {
                            hex.push_str(data);
                        }
                        if part == '2' {
                            if let Some((t, hex)) = self.jacket.take() {
                                let bytes: Vec<u8> = (0..hex.len() / 2).filter_map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()).collect();
                                let mime = if t == '0' { "image/bmp" } else { "image/jpeg" };
                                *cover = (!bytes.is_empty()).then(|| (mime.to_string(), bytes));
                                net.cover_rev = net.cover_rev.wrapping_add(1);
                            }
                        }
                    }
                    (Some('n'), _) => {
                        self.jacket = None;
                        if cover.take().is_some() {
                            net.cover_rev = net.cover_rev.wrapping_add(1);
                        }
                    }
                    _ => {}
                }
            }
            "PWR" => state.main.power = on,
            "MVL" => state.main.volume = hex_volume(param, caps),
            "AMT" => state.main.muted = on,
            "SLI" => state.main.input = Some(param.to_ascii_uppercase()),
            "LMD" => state.mode = (param != "N/A").then(|| Choice { id: param.to_string(), name: mode_name(param) }),
            "TFR" => {
                if let Some(b) = param.find('B') {
                    state.bass = param.get(b + 1..).and_then(parse_tone);
                }
                if let Some(t) = param.find('T') {
                    state.treble = param.get(t + 1..).and_then(parse_tone);
                }
            }
            "ZPW" | "ZVL" | "ZMT" | "SLZ" => {
                // Sans zone 2, l'ampli répond « N/A » à sa mise en marche.
                if param == "N/A" {
                    if cmd == "ZPW" {
                        caps.zone2 = false;
                        state.zone2 = None;
                    }
                    return;
                }
                caps.zone2 = true;
                let volume = hex_volume(param, caps);
                let z = state.zone2.get_or_insert_with(ZoneState::default);
                match cmd {
                    "ZPW" => z.power = on,
                    "ZVL" => z.volume = volume,
                    "ZMT" => z.muted = on,
                    _ => z.input = Some(param.to_ascii_uppercase()),
                }
            }
            "NRI" => apply_nri(param, caps),
            _ => {}
        }
    }
}

impl Codec for OnkyoCodec {
    fn hello(&self) -> Vec<Vec<u8>> {
        [
            "NRIQSTN", "PWRQSTN", "MVLQSTN", "AMTQSTN", "SLIQSTN", "LMDQSTN", "TFRQSTN", "ZPWQSTN", "ZVLQSTN", "ZMTQSTN", "SLZQSTN",
            "NLTQSTN", "NSTQSTN", "NTIQSTN", "NATQSTN", "NALQSTN", "NTMQSTN",
        ]
            .iter()
            .map(|c| frame(c))
            .collect()
    }

    fn keepalive(&self) -> Vec<u8> {
        frame("PWRQSTN")
    }

    fn encode(&self, cmd: &Command, status: &Status) -> Vec<Vec<u8>> {
        let caps = &status.caps;
        // Les flèches pilotent la liste réseau sur NET/USB, le menu à l'écran sinon.
        let on_net = status.state.main.input.as_deref().is_some_and(|i| NET_INPUTS.contains(&i));
        let z = |zone: &Zone, main: &str, z2: &str| if *zone == Zone::Main { main.to_string() } else { z2.to_string() };
        let flag = |on: bool| if on { "01" } else { "00" };
        let text = match cmd {
            Command::Power { zone, on } => format!("{}{}", z(zone, "PWR", "ZPW"), flag(*on)),
            Command::Volume { zone, value } => {
                let v = value.clamp(0.0, caps.volume_max);
                let raw = if caps.volume_step < 1.0 { (v * 2.0).round() } else { v.round() };
                format!("{}{:02X}", z(zone, "MVL", "ZVL"), raw as u32)
            }
            Command::VolumeStep { zone, up } => format!("{}{}", z(zone, "MVL", "ZVL"), if *up { "UP" } else { "DOWN" }),
            Command::Mute { zone, on } => format!("{}{}", z(zone, "AMT", "ZMT"), flag(*on)),
            Command::Input { zone, id } => format!("{}{}", z(zone, "SLI", "SLZ"), id.to_ascii_uppercase()),
            Command::Mode { id } => format!("LMD{id}"),
            Command::Bass { value } => format!("TFRB{}", encode_tone(*value)),
            Command::Treble { value } => format!("TFRT{}", encode_tone(*value)),
            Command::Nav { key } => {
                let (net, osd) = match key {
                    NavKey::Up => ("UP", "UP"),
                    NavKey::Down => ("DOWN", "DOWN"),
                    NavKey::Left => ("LEFT", "LEFT"),
                    NavKey::Right => ("RIGHT", "RIGHT"),
                    NavKey::Enter => ("SELECT", "ENTER"),
                    NavKey::Back => ("RETURN", "EXIT"),
                    NavKey::Home => ("TOP", "HOME"),
                    NavKey::Menu => ("MENU", "MENU"),
                };
                if on_net { format!("NTC{net}") } else { format!("OSD{osd}") }
            }
            Command::NetSelect { line } => format!("NLSL{}", line.min(&9)),
            Command::Transport { action } => format!(
                "NTC{}",
                match action {
                    // P/P est ignoré par certaines sources (DLNA) : ordre explicite.
                    Transport::PlayPause => if status.state.net.status == "play" { "PAUSE" } else { "PLAY" },
                    Transport::Stop => "STOP",
                    Transport::Next => "TRUP",
                    Transport::Previous => "TRDN",
                    Transport::Repeat => "REPEAT",
                    Transport::Shuffle => "RANDOM",
                }
            ),
        };
        // La tonalité n'est pas toujours renvoyée après réglage : on la redemande.
        let mut out = vec![frame(&text)];
        if matches!(cmd, Command::Bass { .. } | Command::Treble { .. }) {
            out.push(frame("TFRQSTN"));
        }
        out
    }

    fn feed(&mut self, buf: &[u8], status: &mut Status) -> usize {
        let mut used = 0;
        while let Some((end, msg)) = next_message(&buf[used..]) {
            if let Some(msg) = msg {
                self.apply(&msg, status);
            }
            used += end;
        }
        used
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status() -> Status {
        Status { caps: default_caps(), state: Default::default(), cover: None }
    }

    /// Trame telle que l'ampli l'envoie (fin « \x1a\r\n »).
    fn reply(text: &str) -> Vec<u8> {
        let data = format!("!1{text}\u{1a}\r\n");
        let mut out = b"ISCP".to_vec();
        out.extend_from_slice(&16u32.to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(&[1, 0, 0, 0]);
        out.extend_from_slice(data.as_bytes());
        out
    }

    #[test]
    fn frames_a_command() {
        let f = frame("PWRQSTN");
        assert_eq!(&f[..4], b"ISCP");
        assert_eq!(u32::from_be_bytes(f[8..12].try_into().unwrap()), 10);
        assert_eq!(&f[16..], b"!1PWRQSTN\r");
    }

    #[test]
    fn reads_the_real_rz900_status() {
        // Réponses réelles d'un TX-RZ900, arrivées en vrac et coupées au milieu.
        let mut wire = Vec::new();
        for r in ["PWR01", "MVL2A", "AMT00", "SLI11", "LMD80", "TFRB+2T00", "ZPW00", "ZVLN/A", "SLZ12"] {
            wire.extend(reply(r));
        }
        let mut s = status();
        let mut codec = OnkyoCodec::default();
        let used = codec.feed(&wire[..50], &mut s);
        assert!(used < 50);
        let rest = [&wire[used..50], &wire[50..]].concat();
        assert_eq!(codec.feed(&rest, &mut s), rest.len());

        assert!(s.state.main.power);
        assert_eq!(s.state.main.volume, Some(42.0));
        assert!(!s.state.main.muted);
        assert_eq!(s.state.main.input.as_deref(), Some("11"));
        assert_eq!(s.state.mode, Some(Choice { id: "80".into(), name: "Dolby Surround".into() }));
        assert_eq!((s.state.bass, s.state.treble), (Some(2), Some(0)));
        assert!(s.caps.zone2);
        let z2 = s.state.zone2.unwrap();
        assert!(!z2.power);
        assert_eq!(z2.input.as_deref(), Some("12"));
    }

    #[test]
    fn reads_capabilities_from_nri() {
        let nri = r#"<?xml version="1.0"?><response status="ok"><device id="TX-RZ900">
            <zonelist count="2"><zone id="1" value="1" name="Main" volmax="100" volstep="1" /><zone id="2" value="1" name="Zone2" volmax="100" volstep="1" /></zonelist>
            <selectorlist count="4"><selector id="10" value="1" name="BD/DVD" zone="07" /><selector id="2b" value="1" name="NET" zone="07" />
            <selector id="06" value="0" name="Hidden" zone="07" /><selector id="80" value="1" name="Source" zone="06" /></selectorlist>
            <controllist><control id="Bass" value="1" zone="3" min="-10" max="10" step="2" /></controllist></device></response>"#;
        let mut s = status();
        OnkyoCodec::default().feed(&reply(&format!("NRI{nri}")), &mut s);
        assert_eq!(s.caps.inputs, vec![Choice { id: "10".into(), name: "BD/DVD".into() }, Choice { id: "2B".into(), name: "NET".into() }]);
        assert_eq!(s.caps.volume_max, 100.0);
        assert!(s.caps.zone2);
        assert_eq!(s.caps.tone, Some(Range { min: -10, max: 10, step: 2 }));
    }

    #[test]
    fn survives_non_ascii_bytes() {
        // Caractères multi-octets aux bornes du découpage : pas de panique.
        let mut s = status();
        OnkyoCodec::default().apply("NLT0001000éé00000000000Titre", &mut s);
        OnkyoCodec::default().apply("NLT00é", &mut s);
        assert_eq!(parse_tone("é"), None);
    }

    #[test]
    fn encodes_commands() {
        let mut st = status();
        let text = |c: Command, st: &Status| String::from_utf8(OnkyoCodec::default().encode(&c, st)[0][16..].to_vec()).unwrap();
        assert_eq!(text(Command::Power { zone: Zone::Main, on: true }, &st), "!1PWR01\r");
        assert_eq!(text(Command::Power { zone: Zone::Zone2, on: false }, &st), "!1ZPW00\r");
        assert_eq!(text(Command::Volume { zone: Zone::Main, value: 42.0 }, &st), "!1MVL2A\r");
        // Plafonné au maximum de l'ampli.
        assert_eq!(text(Command::Volume { zone: Zone::Main, value: 500.0 }, &st), "!1MVL50\r");
        assert_eq!(text(Command::VolumeStep { zone: Zone::Zone2, up: false }, &st), "!1ZVLDOWN\r");
        assert_eq!(text(Command::Input { zone: Zone::Main, id: "2b".into() }, &st), "!1SLI2B\r");
        assert_eq!(text(Command::Mode { id: "MUSIC".into() }, &st), "!1LMDMUSIC\r");
        assert_eq!(text(Command::Bass { value: -4 }, &st), "!1TFRB-4\r");
        assert_eq!(text(Command::Treble { value: 10 }, &st), "!1TFRT+A\r");
        assert_eq!(text(Command::Treble { value: 0 }, &st), "!1TFRT00\r");

        // Flèches : menu à l'écran hors réseau, liste réseau sur NET.
        assert_eq!(text(Command::Nav { key: NavKey::Enter }, &st), "!1OSDENTER\r");
        st.state.main.input = Some("2B".into());
        assert_eq!(text(Command::Nav { key: NavKey::Enter }, &st), "!1NTCSELECT\r");
        assert_eq!(text(Command::Nav { key: NavKey::Back }, &st), "!1NTCRETURN\r");
        assert_eq!(text(Command::NetSelect { line: 5 }, &st), "!1NLSL5\r");
        assert_eq!(text(Command::Transport { action: Transport::PlayPause }, &st), "!1NTCPLAY\r");
        st.state.net.status = "play".into();
        assert_eq!(text(Command::Transport { action: Transport::PlayPause }, &st), "!1NTCPAUSE\r");
    }

    #[test]
    fn reads_the_real_net_screen() {
        // Écran NET réel d'un TX-RZ900, puis entrée dans DLNA.
        let mut wire = Vec::new();
        for r in [
            "NLTF300000000090011FFFF00NET", "NLSC0P", "NLSU0-TuneIn", "NLSU1-Pandora", "NLSU2-Spotify",
            "NSTSxx", "NTM--:--:--/--:--:--", "NTI",
        ] {
            wire.extend(reply(r));
        }
        let mut s = status();
        let mut codec = OnkyoCodec::default();
        codec.feed(&wire, &mut s);
        let net = &s.state.net;
        assert_eq!((net.title.as_str(), net.count, net.cursor), ("NET", 9, Some(0)));
        assert_eq!(net.lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["TuneIn", "Pandora", "Spotify"]);
        assert_eq!((net.status.as_str(), net.time.as_str()), ("stop", ""));

        let mut wire = Vec::new();
        for r in ["NLT0001000000010000FF0000DLNA", "NLSC0P", "NLSU0FMusique", "NTM01:23/04:56", "NSTPxx"] {
            wire.extend(reply(r));
        }
        codec.feed(&wire, &mut s);
        let net = &s.state.net;
        assert_eq!((net.title.as_str(), net.count), ("DLNA", 1));
        assert_eq!(net.lines, vec![NetLine { text: "Musique".into(), kind: "folder".into() }]);
        assert_eq!((net.status.as_str(), net.time.as_str()), ("play", "01:23 / 04:56"));
    }

    #[test]
    fn assembles_the_cover_from_chunks() {
        let mut s = status();
        let mut codec = OnkyoCodec::default();
        let wire = [reply("NJA10FFD8"), reply("NJA11FFE0"), reply("NJA12FFD9")].concat();
        codec.feed(&wire, &mut s);
        assert_eq!(s.cover, Some(("image/jpeg".to_string(), vec![0xFF, 0xD8, 0xFF, 0xE0, 0xFF, 0xD9])));
        assert_eq!(s.state.net.cover_rev, 1);
        codec.feed(&reply("NJAn-"), &mut s);
        assert_eq!((s.cover.is_none(), s.state.net.cover_rev), (true, 2));
    }

    #[test]
    fn reads_a_discovery_reply() {
        let amp = parse_discovery(&reply("ECNTX-RZ900/60128/XX/0009B0AA7C6A\u{19}"), "10.0.0.220").unwrap();
        assert_eq!(amp.id, "onkyo:0009B0AA7C6A");
        assert_eq!((amp.model.as_str(), amp.port), ("TX-RZ900", 60128));
    }
}
