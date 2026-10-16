//! Denon et Marantz : protocole texte sur le port 23, une seule connexion admise par l'ampli.
//! Commandes ASCII terminées par CR ; l'ampli annonce lui-même chaque changement.
//! Référence : « AVR-X2000/E400 PROTOCOL » (Denon), commun aux modèles Denon et Marantz.

use std::time::Duration;

use super::model::{Amp, Brand, Caps, Choice, Codec, Command, NavKey, NetLine, Range, Status, Transport, Zone, ZoneState};

pub const PORT: u16 = 23;
/// L'ampli traite les commandes une à une : à la file, il en perd.
const GAP: Duration = Duration::from_millis(60);

/// Entrées courantes, pour un ampli qui ne décrit pas les siennes (SSFUN).
const DEFAULT_INPUTS: &[(&str, &str)] = &[
    ("BD", "Blu-ray"),
    ("DVD", "DVD"),
    ("SAT/CBL", "CBL/SAT"),
    ("GAME", "Game"),
    ("MPLAY", "Media Player"),
    ("TV", "TV Audio"),
    ("AUX1", "AUX"),
    ("CD", "CD"),
    ("PHONO", "Phono"),
    ("TUNER", "Tuner"),
    ("NET", "NET"),
    ("BT", "Bluetooth"),
    ("USB/IPOD", "USB"),
];

/// Modes directs, puis touches de famille (l'ampli choisit le décodage).
const MODES: &[(&str, &str)] = &[
    ("STEREO", "Stereo"),
    ("DIRECT", "Direct"),
    ("PURE DIRECT", "Pure Direct"),
    ("MCH STEREO", "Multi Ch Stereo"),
    ("AUTO", "Auto"),
    ("MOVIE", "Movie"),
    ("MUSIC", "Music"),
    ("GAME", "Game"),
];

/// Entrées servies par l'écran réseau : flèches et lecture passent par NS.
const NET_INPUTS: &[&str] = &["NET", "SERVER", "IRADIO", "FAVORITES", "USB/IPOD", "USB", "IPD", "IRP", "FVP", "SPOTIFY", "PANDORA", "SIRIUSXM", "FLICKR"];

pub fn default_caps() -> Caps {
    Caps {
        inputs: DEFAULT_INPUTS.iter().map(|(id, name)| Choice { id: id.to_string(), name: name.to_string() }).collect(),
        modes: MODES.iter().map(|(id, name)| Choice { id: id.to_string(), name: name.to_string() }).collect(),
        volume_max: 98.0,
        volume_step: 0.5,
        // PSBAS / PSTRE : 44 à 56, 50 = 0 dB.
        tone: Some(Range { min: -6, max: 6, step: 1 }),
        zone2: false,
    }
}

pub fn frame(cmd: &str) -> Vec<u8> {
    format!("{cmd}\r").into_bytes()
}

/// « 80 » → 80 ; « 805 » → 80,5 ; « 99 » : minimum (--).
fn parse_volume(p: &str) -> Option<f32> {
    let p = p.trim();
    if !(2..=3).contains(&p.len()) || !p.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if p == "99" {
        return Some(0.0);
    }
    let whole: f32 = p.get(..2)?.parse().ok()?;
    Some(if p.len() == 3 { whole + 0.5 } else { whole })
}

fn encode_volume(v: f32, max: f32) -> String {
    let v = (v.clamp(0.0, max) * 2.0).round() / 2.0;
    let whole = v.trunc() as u32;
    if v.fract() > 0.0 { format!("{whole:02}5") } else { format!("{whole:02}") }
}

/// « DOLBY SURROUND » → « Dolby Surround ».
fn mode_name(code: &str) -> String {
    if let Some((_, name)) = MODES.iter().find(|(id, _)| *id == code) {
        return name.to_string();
    }
    code.split(' ')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Default)]
pub struct DenonCodec {
    /// Entrées renommées (SSFUN), dans l'ordre de l'ampli.
    renamed: Vec<Choice>,
    /// Entrées masquées dans le menu de l'ampli (SSSOD … DEL).
    hidden: Vec<String>,
}

impl DenonCodec {
    fn refresh_inputs(&self, caps: &mut Caps) {
        let inputs: Vec<Choice> = self.renamed.iter().filter(|c| !self.hidden.contains(&c.id)).cloned().collect();
        if !inputs.is_empty() {
            caps.inputs = inputs;
        }
    }

    fn apply(&mut self, msg: &str, status: &mut Status) {
        let Status { caps, state, .. } = status;
        let (cmd, param) = (msg.get(..2).unwrap_or(""), msg.get(2..).unwrap_or(""));
        match cmd {
            "PW" if param == "STANDBY" => {
                state.main.power = false;
                if let Some(z) = state.zone2.as_mut() {
                    z.power = false;
                }
            }
            "ZM" => match param {
                "ON" => state.main.power = true,
                "OFF" => state.main.power = false,
                _ => {}
            },
            "MV" => {
                if let Some(max) = param.strip_prefix("MAX") {
                    if let Some(m) = parse_volume(max) {
                        caps.volume_max = m;
                    }
                } else if let Some(v) = parse_volume(param) {
                    state.main.volume = Some(v);
                }
            }
            "MU" => state.main.muted = param == "ON",
            "SI" if !param.is_empty() => state.main.input = Some(param.to_string()),
            "MS" if !param.is_empty() && !param.starts_with("QUICK") => {
                state.mode = Some(Choice { id: param.to_string(), name: mode_name(param) });
            }
            "PS" => {
                let tone = |v: &str| v.trim().parse::<i32>().ok().filter(|n| (0..=99).contains(n)).map(|n| n - 50);
                if let Some(v) = param.strip_prefix("BAS") {
                    state.bass = tone(v).or(state.bass);
                } else if let Some(v) = param.strip_prefix("TRE") {
                    state.treble = tone(v).or(state.treble);
                }
            }
            "Z2" => self.apply_zone2(param, caps, state.zone2.get_or_insert_with(ZoneState::default)),
            "SS" => {
                // « SSFUNCD CD » : nom donné à l'entrée ; « SSSODDVD DEL » : entrée masquée.
                if let Some(rest) = param.strip_prefix("FUN") {
                    if let Some((id, name)) = rest.split_once(' ') {
                        let id = id.to_string();
                        self.renamed.retain(|c| c.id != id);
                        self.renamed.push(Choice { id, name: name.trim().to_string() });
                        self.refresh_inputs(caps);
                    }
                } else if let Some(rest) = param.strip_prefix("SOD") {
                    if let Some((id, usage)) = rest.split_once(' ') {
                        self.hidden.retain(|h| h != id);
                        if usage.trim() == "DEL" {
                            self.hidden.push(id.to_string());
                        }
                        self.refresh_inputs(caps);
                    }
                }
            }
            "NS" => apply_screen(param, &mut state.net),
            _ => {}
        }
    }

    fn apply_zone2(&mut self, param: &str, caps: &mut Caps, z: &mut ZoneState) {
        caps.zone2 = true;
        if let Some(m) = param.strip_prefix("MU") {
            z.muted = m == "ON";
        } else if param == "ON" || param == "OFF" {
            z.power = param == "ON";
        } else if let Some(v) = parse_volume(param) {
            z.volume = Some(v);
        } else if !["CV", "SLP", "PS", "QUICK", "HPF", "SMART", "FAVORITE", "STBY"].iter().any(|p| param.starts_with(p)) && !param.is_empty() {
            z.input = Some(param.to_string());
        }
    }
}

/// Écran réseau : « E0 » titre, « E1 »… lignes (octet d'état puis texte), texte arrêté au premier NUL.
fn apply_screen(param: &str, net: &mut super::model::Net) {
    let mut chars = param.chars();
    if chars.next() != Some('E') {
        return;
    }
    let Some(n) = chars.next().and_then(|c| c.to_digit(10)) else { return };
    let rest = chars.as_str();
    let cut = |s: &str| s.split('\0').next().unwrap_or_default().trim().to_string();
    if n == 0 {
        net.title = cut(rest);
        net.playback = net.title.starts_with("Now Playing");
        if net.playback {
            net.lines.clear();
            net.cursor = None;
        }
        return;
    }
    let i = n as usize - 1;
    // Lignes 1 à 6 : un octet d'état (bit 1 lisible, bit 2 dossier, bit 4 curseur) précède le texte.
    let (flags, text) = if (1..=6).contains(&n) {
        let mut c = rest.chars();
        let f = c.next().map(|c| c as u32).unwrap_or(0);
        (f, cut(c.as_str()))
    } else {
        (0, cut(rest))
    };
    if net.playback {
        match n {
            1 => net.track = text,
            2 => net.artist = text,
            4 => net.album = text,
            5 => net.time = text.split(' ').next().unwrap_or_default().to_string(),
            _ => {}
        }
        return;
    }
    if net.lines.len() <= i {
        net.lines.resize(i + 1, NetLine::default());
    }
    let kind = if flags & 0x02 != 0 { "folder" } else if flags & 0x01 != 0 { "track" } else { "item" };
    net.lines[i] = NetLine { text, kind: kind.to_string() };
    if flags & 0x08 != 0 {
        net.cursor = Some(i as u8);
    }
}

impl Codec for DenonCodec {
    fn hello(&self) -> Vec<Vec<u8>> {
        ["PW?", "ZM?", "MV?", "MU?", "SI?", "MS?", "PSBAS ?", "PSTRE ?", "Z2?", "Z2MU?", "SSFUN ?", "SSSOD ?", "NSE"]
            .iter()
            .map(|c| frame(c))
            .collect()
    }

    fn keepalive(&self) -> Vec<u8> {
        frame("PW?")
    }

    fn gap(&self) -> Duration {
        GAP
    }

    fn encode(&self, cmd: &Command, status: &Status) -> Vec<Vec<u8>> {
        let caps = &status.caps;
        let net = &status.state.net;
        let on_net = status.state.main.input.as_deref().is_some_and(|i| NET_INPUTS.contains(&i));
        let z = |zone: &Zone, main: &str, z2: &str| if *zone == Zone::Main { main.to_string() } else { z2.to_string() };
        let mut out: Vec<String> = match cmd {
            Command::Power { zone, on } => vec![format!("{}{}", z(zone, "ZM", "Z2"), if *on { "ON" } else { "OFF" })],
            Command::Volume { zone, value } => {
                // Zone 2 : deux chiffres seulement.
                let v = match zone {
                    Zone::Main => encode_volume(*value, caps.volume_max),
                    Zone::Zone2 => format!("{:02}", value.clamp(0.0, 98.0).round() as u32),
                };
                vec![format!("{}{v}", z(zone, "MV", "Z2"))]
            }
            Command::VolumeStep { zone, up } => vec![format!("{}{}", z(zone, "MV", "Z2"), if *up { "UP" } else { "DOWN" })],
            Command::Mute { zone, on } => vec![format!("{}{}", z(zone, "MU", "Z2MU"), if *on { "ON" } else { "OFF" })],
            Command::Input { zone, id } => vec![format!("{}{id}", z(zone, "SI", "Z2"))],
            Command::Mode { id } => vec![format!("MS{id}")],
            // Sans « TONE CTRL ON », l'ampli ignore graves et aigus.
            Command::Bass { value } => vec!["PSTONE CTRL ON".into(), format!("PSBAS {:02}", (50 + value).clamp(0, 99))],
            Command::Treble { value } => vec!["PSTONE CTRL ON".into(), format!("PSTRE {:02}", (50 + value).clamp(0, 99))],
            Command::Nav { key } if on_net => {
                let ns = match key {
                    NavKey::Up => "NS90",
                    NavKey::Down => "NS91",
                    NavKey::Left | NavKey::Back | NavKey::Home => "NS92",
                    NavKey::Right => "NS93",
                    NavKey::Enter => "NS94",
                    NavKey::Menu => "MNMEN ON",
                };
                vec![ns.into(), "NSE".into()]
            }
            Command::Nav { key } => vec![match key {
                NavKey::Up => "MNCUP",
                NavKey::Down => "MNCDN",
                NavKey::Left => "MNCLT",
                NavKey::Right => "MNCRT",
                NavKey::Enter => "MNENT",
                NavKey::Back => "MNRTN",
                NavKey::Home => "MNMEN OFF",
                NavKey::Menu => "MNMEN ON",
            }
            .into()],
            // Pas de choix direct d'une ligne : le curseur y descend, puis validation.
            Command::NetSelect { line } => {
                let from = net.cursor.unwrap_or(0) as i32;
                let moves = *line as i32 - from;
                let step = if moves > 0 { "NS91" } else { "NS90" };
                let mut v: Vec<String> = (0..moves.abs()).map(|_| step.to_string()).collect();
                v.extend(["NS94".to_string(), "NSE".to_string()]);
                v
            }
            Command::Transport { action } => vec![
                match action {
                    Transport::PlayPause => if net.status == "play" { "NS9B" } else { "NS9A" },
                    Transport::Stop => "NS9C",
                    Transport::Next => "NS9D",
                    Transport::Previous => "NS9E",
                    Transport::Repeat => "NSRPT",
                    Transport::Shuffle => "NSRND",
                }
                .into(),
                "NSE".into(),
            ],
        };
        // Tonalité pas toujours renvoyée après réglage : on la redemande.
        if matches!(cmd, Command::Bass { .. } | Command::Treble { .. }) {
            out.extend(["PSBAS ?".to_string(), "PSTRE ?".to_string()]);
        }
        out.iter().map(|c| frame(c)).collect()
    }

    fn feed(&mut self, buf: &[u8], status: &mut Status) -> usize {
        let mut used = 0;
        while let Some(end) = buf[used..].iter().position(|&b| b == b'\r') {
            let msg = String::from_utf8_lossy(&buf[used..used + end]).to_string();
            let msg = msg.trim_start_matches('\n');
            if !msg.is_empty() {
                self.apply(msg, status);
            }
            used += end + 1;
        }
        used
    }
}

/// Ampli Denon/Marantz trouvé par la recherche UPnP ; les enceintes HEOS n'ont pas de port 23.
pub fn amp_from_upnp(id: &str, name: &str, manufacturer: &str, model: &str, ip: &str) -> Option<Amp> {
    let maker = manufacturer.to_lowercase();
    if !(maker.contains("denon") || maker.contains("marantz")) {
        return None;
    }
    let m = model.to_lowercase();
    if m.starts_with("heos") || m.contains("home ") || m.contains("bar") {
        return None;
    }
    Some(Amp {
        id: format!("denon:{}", id.trim_start_matches("uuid:")),
        name: name.to_string(),
        brand: Brand::Denon,
        model: model.to_string(),
        ip: ip.to_string(),
        port: PORT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status() -> Status {
        Status { caps: default_caps(), state: Default::default(), cover: None }
    }

    fn feed(codec: &mut DenonCodec, st: &mut Status, lines: &[&str]) {
        let bytes: Vec<u8> = lines.iter().flat_map(|l| frame(l)).collect();
        assert_eq!(codec.feed(&bytes, st), bytes.len());
    }

    fn sent(frames: Vec<Vec<u8>>) -> Vec<String> {
        frames.into_iter().map(|f| String::from_utf8(f).unwrap().trim_end_matches('\r').to_string()).collect()
    }

    #[test]
    fn reads_status_events() {
        let (mut c, mut st) = (DenonCodec::default(), status());
        feed(&mut c, &mut st, &["ZMON", "MV455", "MVMAX 805", "MUOFF", "SINET", "MSDOLBY SURROUND", "PSBAS 53", "PSTRE 47", "Z2ON", "Z2CD", "Z235", "Z2MUON"]);
        assert!(st.state.main.power);
        assert_eq!(st.state.main.volume, Some(45.5));
        assert_eq!(st.caps.volume_max, 80.5);
        assert_eq!(st.state.main.input.as_deref(), Some("NET"));
        assert_eq!(st.state.mode.as_ref().map(|m| m.name.as_str()), Some("Dolby Surround"));
        assert_eq!((st.state.bass, st.state.treble), (Some(3), Some(-3)));
        let z2 = st.state.zone2.clone().unwrap();
        assert!(st.caps.zone2 && z2.power && z2.muted);
        assert_eq!((z2.input.as_deref(), z2.volume), (Some("CD"), Some(35.0)));

        feed(&mut c, &mut st, &["PWSTANDBY", "MV99"]);
        assert!(!st.state.main.power && !st.state.zone2.unwrap().power);
        assert_eq!(st.state.main.volume, Some(0.0));
    }

    #[test]
    fn reads_renamed_and_hidden_inputs() {
        let (mut c, mut st) = (DenonCodec::default(), status());
        feed(&mut c, &mut st, &["SSFUNCD Platine CD", "SSFUNDVD DVD", "SSFUNNET Online Music", "SSSODDVD DEL", "SSSODCD USE"]);
        let names: Vec<_> = st.caps.inputs.iter().map(|i| (i.id.as_str(), i.name.as_str())).collect();
        assert_eq!(names, vec![("CD", "Platine CD"), ("NET", "Online Music")]);
    }

    #[test]
    fn reads_the_network_screen() {
        let (mut c, mut st) = (DenonCodec::default(), status());
        feed(&mut c, &mut st, &["NSE0Favorites\0??", "NSE1\u{2}Radio\0??", "NSE2\u{9}Morceau\0", "NSE3\u{0}\0"]);
        assert_eq!(st.state.net.title, "Favorites");
        assert_eq!(st.state.net.lines[0], NetLine { text: "Radio".into(), kind: "folder".into() });
        assert_eq!(st.state.net.lines[1].kind, "track");
        assert_eq!(st.state.net.cursor, Some(1));

        feed(&mut c, &mut st, &["NSE0Now Playing Server\0", "NSE1\u{1}Come Away With Me\0", "NSE2\u{1}Norah Jones\0", "NSE5\u{1}00:11 100%\0"]);
        assert!(st.state.net.playback);
        assert_eq!((st.state.net.track.as_str(), st.state.net.artist.as_str(), st.state.net.time.as_str()), ("Come Away With Me", "Norah Jones", "00:11"));
    }

    #[test]
    fn encodes_commands() {
        let c = DenonCodec::default();
        let mut st = status();
        assert_eq!(sent(c.encode(&Command::Volume { zone: Zone::Main, value: 45.5 }, &st)), vec!["MV455"]);
        assert_eq!(sent(c.encode(&Command::Volume { zone: Zone::Main, value: 7.0 }, &st)), vec!["MV07"]);
        assert_eq!(sent(c.encode(&Command::Volume { zone: Zone::Zone2, value: 40.0 }, &st)), vec!["Z240"]);
        assert_eq!(sent(c.encode(&Command::Power { zone: Zone::Main, on: false }, &st)), vec!["ZMOFF"]);
        assert_eq!(sent(c.encode(&Command::Mute { zone: Zone::Zone2, on: true }, &st)), vec!["Z2MUON"]);
        assert_eq!(sent(c.encode(&Command::Input { zone: Zone::Main, id: "SAT/CBL".into() }, &st)), vec!["SISAT/CBL"]);
        assert_eq!(sent(c.encode(&Command::Bass { value: -2 }, &st)), vec!["PSTONE CTRL ON", "PSBAS 48", "PSBAS ?", "PSTRE ?"]);
        assert_eq!(sent(c.encode(&Command::Nav { key: NavKey::Up }, &st)), vec!["MNCUP"]);

        st.state.main.input = Some("NET".into());
        st.state.net.cursor = Some(1);
        assert_eq!(sent(c.encode(&Command::Nav { key: NavKey::Up }, &st)), vec!["NS90", "NSE"]);
        assert_eq!(sent(c.encode(&Command::NetSelect { line: 3 }, &st)), vec!["NS91", "NS91", "NS94", "NSE"]);
        st.state.net.status = "play".into();
        assert_eq!(sent(c.encode(&Command::Transport { action: Transport::PlayPause }, &st)), vec!["NS9B", "NSE"]);
    }

    #[test]
    fn keeps_amps_and_drops_speakers() {
        assert!(amp_from_upnp("uuid:1", "Salon", "Denon", "AVR-X2700H", "10.0.0.5").is_some());
        assert_eq!(amp_from_upnp("uuid:2", "Salon", "Marantz", "SR6015", "10.0.0.6").unwrap().port, 23);
        assert!(amp_from_upnp("uuid:3", "Cuisine", "Denon", "HEOS 1", "10.0.0.7").is_none());
        assert!(amp_from_upnp("uuid:4", "Salon", "Sonos, Inc.", "One", "10.0.0.8").is_none());
    }
}
