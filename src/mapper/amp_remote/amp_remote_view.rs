//! La télécommande d'ampli : ce qu'affiche son panneau, bâti depuis l'état publié par la session.

use crate::app::music_app::MusicApp;
use crate::core::amp_remote::model::{Brand, Caps, Choice, Command, NavKey, NetLine, Zone, ZoneState};
use crate::models::amp_remote::*;
use crate::models::app::Msg;
use karbon_ui::intl::t;
use karbon_ui::ui::runtime::WheelEvent;

/// Distance au bord d'une télécommande collée.
pub(crate) const DOCK_EDGE: f32 = 20.;
/// Lâchée à moins de 48 unités d'un bord, la télécommande s'y colle ; ailleurs, elle reste
/// où on la pose.
pub(crate) const SNAP_ZONE: f32 = 48.;
/// Largeurs de la languette et du panneau.
pub(crate) const HANDLE_WIDTH: f32 = 40.;
pub(crate) const PANEL_WIDTH: f32 = 340.;
/// Hauteur par défaut, sous la barre de titre ; bornes : la barre de titre en haut, de
/// quoi montrer la languette au-dessus du lecteur en bas.
const DOCK_TOP: f32 = 84.;
const TOP_MIN: f32 = 56.;
const BOTTOM_ROOM: f32 = 290.;
/// Place laissée sous le panneau (lecteur).
const PANEL_ROOM: f32 = 146.;

/// Hauteur gardée dans la fenêtre.
pub(crate) fn clamp_top(top: f32, height: f32) -> f32 {
    top.clamp(TOP_MIN, (height - BOTTOM_ROOM).max(TOP_MIN))
}

/// Réglage « right:0.105:20 » (côté, hauteur relative à la fenêtre, distance au bord) ;
/// par défaut à droite, collée, sous la barre de titre.
pub(crate) fn parse_dock(value: Option<&str>, height: f32) -> AmpDock {
    let mut parts = value.unwrap_or("").split(':');
    let left = parts.next() == Some("left");
    let number = |p: Option<&str>| p.and_then(|v| v.parse::<f32>().ok()).filter(|v| v.is_finite());
    let top = number(parts.next()).map_or(DOCK_TOP, |r| r * height);
    let inset = number(parts.next()).map_or(DOCK_EDGE, |i| i.max(0.));
    AmpDock { left, inset, top: clamp_top(top, height) }
}

/// La position à mémoriser : la hauteur relative suit les changements de taille de la fenêtre.
pub(crate) fn dock_setting(dock: AmpDock, height: f32) -> String {
    format!("{}:{:.4}:{:.0}", if dock.left { "left" } else { "right" }, dock.top / height.max(1.), dock.inset)
}

/// Icône d'une entrée d'après son nom, tel que l'ampli le donne.
pub(crate) fn input_icon(name: &str) -> &'static str {
    let n = name.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    let starts = |words: &[&str]| words.iter().any(|w| n.starts_with(w));
    if has(&["bd", "dvd", "bluray", "blu-ray"]) {
        "material-symbols:movie-outline-rounded"
    } else if starts(&["tv"]) {
        "material-symbols:tv-outline-rounded"
    } else if has(&["cbl", "sat", "cable"]) {
        "material-symbols:satellite-alt-outline-rounded"
    } else if has(&["strm", "stream", "media"]) {
        "material-symbols:cast-rounded"
    } else if has(&["game"]) {
        "material-symbols:sports-esports-outline-rounded"
    } else if starts(&["pc"]) {
        "material-symbols:computer-outline-rounded"
    } else if starts(&["cd"]) {
        "material-symbols:album-outline-rounded"
    } else if has(&["phono"]) {
        "material-symbols:radio-button-checked-outline"
    } else if starts(&["fm", "am", "tuner", "dab"]) {
        "material-symbols:radio-outline-rounded"
    } else if starts(&["net"]) {
        "material-symbols:lan-outline-rounded"
    } else if has(&["bluetooth"]) {
        "material-symbols:bluetooth-rounded"
    } else if has(&["usb"]) {
        "material-symbols:usb-rounded"
    } else if has(&["aux"]) {
        "material-symbols:cable-rounded"
    } else if has(&["hdmi", "video"]) {
        "material-symbols:settings-input-hdmi-outline-rounded"
    } else {
        "material-symbols:input-rounded"
    }
}

/// Les sources d'image (lecteurs, TV, consoles…), rangées à part du son et de la radio.
pub(crate) fn is_video(name: &str) -> bool {
    let n = name.to_lowercase();
    ["bd", "dvd", "bluray", "blu-ray", "cbl", "sat", "strm", "stream", "game", "hdmi", "video"].iter().any(|w| n.contains(w))
        || ["tv", "pc"].iter().any(|w| n.starts_with(w))
}

/// NET, USB : l'ampli montre la liste de ses services.
pub(crate) fn is_network(name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with("net") || n.starts_with("usb")
}

/// Icône d'une ligne de la liste réseau : sa nature, ou le logo d'un service connu.
pub(crate) fn line_icon(line: &NetLine) -> &'static str {
    if line.kind != "item" {
        return match line.kind.as_str() {
            "playing" => "material-symbols:graphic-eq-rounded",
            "artist" => "material-symbols:person-outline-rounded",
            "album" => "material-symbols:album-outline-rounded",
            "folder" => "material-symbols:folder-outline-rounded",
            "track" => "material-symbols:music-note-rounded",
            "playlist" => "material-symbols:queue-music-rounded",
            "search" => "material-symbols:search-rounded",
            "account" => "material-symbols:account-circle-outline",
            _ => "material-symbols:chevron-right-rounded",
        };
    }
    let n = line.text.to_lowercase();
    const SERVICES: [(&str, &str); 12] = [
        ("spotify", "simple-icons:spotify"),
        ("deezer", "simple-icons:deezer"),
        ("tunein", "simple-icons:tunein"),
        ("pandora", "simple-icons:pandora"),
        ("dlna", "simple-icons:dlna"),
        ("airplay", "simple-icons:airplayaudio"),
        ("tidal", "simple-icons:tidal"),
        ("amazon", "simple-icons:amazonmusic"),
        ("iheart", "simple-icons:iheartradio"),
        ("soundcloud", "simple-icons:soundcloud"),
        ("home media", "material-symbols:home-storage-outline-rounded"),
        ("usb", "material-symbols:usb-rounded"),
    ];
    match SERVICES.iter().find(|(prefix, _)| n.starts_with(prefix)) {
        Some((_, icon)) => icon,
        None if n.contains("favo") => "material-symbols:star-outline-rounded",
        None => "material-symbols:chevron-right-rounded",
    }
}

/// Volume affiché : au dixième quand l'ampli règle au demi-point.
pub(crate) fn format_volume(v: Option<f32>, step: f32) -> String {
    match v {
        None => "—".into(),
        Some(v) if step < 1. => format!("{v:.1}"),
        Some(v) => format!("{}", v.round() as i64),
    }
}

/// Segments allumés d'un caractère (a haut, b, c, d bas, e, f, g milieu) ; une case vide
/// pour tout autre caractère.
pub(crate) fn segments_of(c: char) -> [bool; 7] {
    let bits: u8 = match c {
        '0' => 0b0111111,
        '1' => 0b0000110,
        '2' => 0b1011011,
        '3' => 0b1001111,
        '4' => 0b1100110,
        '5' => 0b1101101,
        '6' => 0b1111101,
        '7' => 0b0000111,
        '8' => 0b1111111,
        '9' => 0b1101111,
        '-' => 0b1000000,
        _ => 0,
    };
    std::array::from_fn(|i| bits & (1 << i) != 0)
}

/// Le volume en chiffres 7 segments, calé à droite sur autant de cases que le maximum en
/// demande (cases vides devant) ; « -- » sans volume connu.
pub(crate) fn digits(v: Option<f32>, max: f32, step: f32) -> Vec<AmpDigit> {
    let cells = (max.floor() as i64).to_string().len();
    let text = match v {
        None => "-".repeat(cells),
        Some(_) => format_volume(v, step),
    };
    let (int, dec) = text.split_once('.').map_or((text.as_str(), None), |(i, d)| (i, Some(d)));
    let mut out: Vec<AmpDigit> = std::iter::repeat_n(' ', cells.saturating_sub(int.chars().count()))
        .chain(int.chars())
        .map(|c| AmpDigit { segments: segments_of(c), dot: false })
        .collect();
    if let Some(dec) = dec {
        if let Some(last) = out.last_mut() {
            last.dot = true;
        }
        out.extend(dec.chars().map(|c| AmpDigit { segments: segments_of(c), dot: false }));
    } else if step < 1. {
        // Demi-point attendu : une case de décimale éteinte garde la largeur.
        out.push(AmpDigit { segments: [false; 7], dot: false });
    }
    out
}

/// « +2 dB », « −2 dB », « 0 dB ».
pub(crate) fn decibels(v: i32) -> String {
    let sign = match v.signum() {
        1 => "+",
        -1 => "−",
        _ => "",
    };
    format!("{sign}{} dB", v.abs())
}

/// Molette sur l'afficheur : un pas de volume ; l'événement est consommé (le panneau ne
/// défile pas).
pub fn wheel_volume() -> impl Fn(WheelEvent) -> Option<Msg> + 'static {
    |e| (e.delta_y != 0.).then(|| Msg::AmpRemote(AmpRemoteMsg::Wheel(e.delta_y < 0.)))
}

/// Molette sur le pavé rond : un cran vers le haut ou le bas déplace dans la liste.
pub fn wheel_nav() -> impl Fn(WheelEvent) -> Option<Msg> + 'static {
    |e| {
        let key = if e.delta_y < 0. {
            NavKey::Up
        } else if e.delta_y > 0. {
            NavKey::Down
        } else {
            return None;
        };
        Some(Msg::AmpRemote(AmpRemoteMsg::NavWheel(key)))
    }
}

fn same_id(a: Option<&str>, b: &str) -> bool {
    a.is_some_and(|a| a.eq_ignore_ascii_case(b))
}

fn input_name(caps: &Caps, id: Option<&str>) -> String {
    caps.inputs.iter().find(|i| same_id(id, &i.id)).map(|i| i.name.clone()).unwrap_or_default()
}

fn send(cmd: Command) -> Msg {
    Msg::AmpRemote(AmpRemoteMsg::Send(cmd))
}

fn choices(list: &[&Choice], current: Option<&str>, icons: bool, cmd: impl Fn(&str) -> Command) -> Vec<AmpChoice> {
    list.iter()
        .map(|c| AmpChoice {
            id: c.id.clone(),
            name: c.name.clone(),
            icon: if icons { input_icon(&c.name) } else { "" },
            selected: same_id(current, &c.id),
            msg: send(cmd(&c.id)),
        })
        .collect()
}

impl MusicApp {
    /// Où se tient la télécommande : en cours de déplacement, sinon celle des réglages.
    pub(crate) fn amp_dock(&self) -> AmpDock {
        let (width, height) = self.viewport.get();
        let mut dock = self.views.amp_remote.dock.unwrap_or_else(|| parse_dock(self.data.setting("amp_remote_dock"), height));
        // Fenêtre rétrécie : la télécommande reste dedans.
        dock.inset = dock.inset.min((width - self.amp_span()).max(0.));
        dock
    }

    /// Largeur de la télécommande : la languette, plus le panneau s'il est sorti.
    pub(crate) fn amp_span(&self) -> f32 {
        HANDLE_WIDTH + if self.data.flag("amp_remote_open") || self.export_amp.is_some_and(|p| !p.contains("closed")) { PANEL_WIDTH } else { 0. }
    }

    /// Volume d'une zone, avec le glissement en cours.
    fn amp_zone(&self, zone: Zone, z: &ZoneState, caps: &Caps) -> AmpZoneView {
        let max = if caps.volume_max > 0. { caps.volume_max } else { 100. };
        let drag = self.views.amp_remote.drag.filter(|d| d.0 == zone).map(|d| d.1);
        let volume = drag.or(z.volume).unwrap_or(0.);
        let label = if z.power {
            format!("{} · {}", input_name(caps, z.input.as_deref()), format_volume(Some(volume), caps.volume_step))
        } else {
            t("amp_remote.standby")
        };
        AmpZoneView { zone, power: z.power, muted: z.muted, volume, max, step: caps.volume_step.max(0.1), fill: (volume / max).clamp(0., 1.), label }
    }

    pub fn amp_remote_view(&self) -> Option<AmpRemoteView> {
        let r = &self.views.amp_remote;
        let preview = self.export_amp;
        let view = r.view.as_ref().filter(|_| self.data.flag("amp_remote") || preview.is_some())?;
        let (amp, caps, state) = (&view.amp, &view.status.caps, &view.status.state);
        let on = state.connected && state.main.power;
        let dock = self.amp_dock();
        let main = self.amp_zone(Zone::Main, &state.main, caps);
        let input = input_name(caps, state.main.input.as_deref());
        let mode = state.mode.as_ref().map(|m| m.name.clone()).unwrap_or_default();
        let mode_key = mode.to_lowercase().replace('é', "e");
        let code = if input.is_empty() { "—".to_string() } else { input.to_uppercase() };
        let code_len = code.chars().count();
        // En veille, l'afficheur s'éteint : les cases restent, sans segment allumé.
        let mut digits = digits(Some(main.volume), main.max, caps.volume_step);
        if !on {
            digits.iter_mut().for_each(|d| *d = AmpDigit { segments: [false; 7], dot: false });
        }
        let brand = match amp.brand {
            Brand::Onkyo => "Onkyo",
            Brand::Denon => "Denon",
            Brand::Yamaha => "Yamaha",
        };
        let status = match (state.connected, on) {
            (false, _) => t("amp_remote.reconnecting"),
            (true, false) => t("amp_remote.standby"),
            (true, true) => format!("{} · {}", if !amp.model.is_empty() && amp.model != amp.name { &amp.model } else { brand }, amp.ip),
        };

        let (video, audio): (Vec<&Choice>, Vec<&Choice>) = caps.inputs.iter().partition(|c| is_video(&c.name));
        let mut families: Vec<(String, Vec<AmpChoice>)> = [("amp_remote.group_video", video), ("amp_remote.group_audio", audio)]
            .into_iter()
            .filter(|(_, items)| !items.is_empty())
            .map(|(key, items)| (t(key), choices(&items, state.main.input.as_deref(), true, |id| Command::Input { zone: Zone::Main, id: id.into() })))
            .collect();
        if families.len() == 1 {
            families[0].0.clear();
        }
        let modes = choices(&caps.modes.iter().collect::<Vec<_>>(), state.mode.as_ref().map(|m| m.id.as_str()), false, |id| Command::Mode { id: id.into() });

        let mut tone = Vec::new();
        if let Some(range) = caps.tone {
            let rows = [("amp_remote.bass", state.bass, true), ("amp_remote.treble", state.treble, false)];
            for (key, value, bass) in rows {
                let Some(v) = value else { continue };
                let cmd = move |value: i32| if bass { Command::Bass { value } } else { Command::Treble { value } };
                tone.push(AmpToneRow {
                    label: t(key),
                    value: decibels(v),
                    zero: v == 0,
                    minus: (v > range.min).then(|| send(cmd((v - range.step).max(range.min)))),
                    plus: (v < range.max).then(|| send(cmd((v + range.step).min(range.max)))),
                    first: tone.is_empty(),
                });
            }
        }

        let zone2 = state.zone2.as_ref().filter(|_| caps.zone2).map(|z| self.amp_zone(Zone::Zone2, z, caps));
        let zone2_inputs = match &state.zone2 {
            Some(z) if z.power => choices(&caps.inputs.iter().collect::<Vec<_>>(), z.input.as_deref(), false, |id| Command::Input { zone: Zone::Zone2, id: id.into() }),
            _ => Vec::new(),
        };

        let net = &state.net;
        let net_lines: Vec<AmpNetLine> = net
            .lines
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.text.is_empty())
            .map(|(i, l)| AmpNetLine {
                text: l.text.clone(),
                icon: line_icon(l),
                selected: net.cursor == Some(i as u8),
                folder: l.kind == "folder" || l.kind == "item",
                msg: send(Command::NetSelect { line: i as u8 }),
            })
            .collect();

        Some(AmpRemoteView {
            open: preview.map_or(self.data.flag("amp_remote_open"), |p| !p.contains("closed")),
            slide: r.slid || preview.is_some_and(|p| p.ends_with("slide")),
            dock,
            panel_max: (self.viewport.get().1 - dock.top - PANEL_ROOM).max(160.),
            panning: r.pan_from.is_some(),
            mini: self.data.flag("amp_remote_mini") || preview == Some("mini"),
            compact: self.data.flag("amp_remote_mini") || preview == Some("mini") || !state.connected,
            connected: state.connected,
            on,
            name: amp.name.clone(),
            status,
            led: if !state.connected { "warn" } else if on { "on" } else { "off" },
            holding: r.holding,
            hint: r.hint,
            unreachable: t("amp_remote.unreachable").replace("{name}", &amp.name),
            lights: vec![
                ("STEREO", ["stereo", "direct", "pure"].iter().any(|w| mode_key.contains(w))),
                ("DOLBY", mode_key.contains("dolby")),
                ("DTS", mode_key.contains("dts")),
                ("NET", is_network(&input) || input.to_lowercase().starts_with("bluetooth")),
                ("ZONE2", state.zone2.as_ref().is_some_and(|z| z.power)),
                ("MUTING", state.main.muted),
            ],
            code_size: if code_len <= 6 { 28. } else if code_len <= 9 { 21. } else { 16. },
            code,
            mini_line: if !on {
                amp.name.to_uppercase()
            } else if state.main.muted {
                "MUTING".into()
            } else {
                mode.to_uppercase()
            },
            mode: if mode.is_empty() { "—".into() } else { mode.to_uppercase() },
            lit: !state.main.muted,
            digits,
            volume_tag: if state.main.muted { "MUTING".into() } else { format!("VOL / {}", format_volume(Some(main.max), caps.volume_step)) },
            input_icon: input_icon(&input),
            input_name: if input.is_empty() { "—".into() } else { input.clone() },
            main,
            tab: r.tab,
            families,
            modes,
            tone,
            zone2,
            zone2_inputs,
            on_net: is_network(&input),
            net_title: if net.title.is_empty() { input.clone() } else { net.title.clone() },
            net_count: if net.count > 0 { t("amp_remote.items").replace("{n}", &net.count.to_string()) } else { String::new() },
            net_loading: net_lines.is_empty(),
            net_lines,
            playing: !net.track.is_empty(),
            track: net.track.clone(),
            track_line: [net.artist.as_str(), net.album.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · "),
            time: net.time.clone(),
            paused: net.status != "play",
            cover: r.cover.clone(),
            go_net: caps.inputs.iter().find(|i| i.name.to_lowercase().starts_with("net")).map(|i| send(Command::Input { zone: Zone::Main, id: i.id.clone() })),
            wheel: self.data.setting("amp_remote_pad") != Some("keys"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_are_right_aligned_with_blank_cells_and_keep_the_half_step() {
        let lit = |d: &[AmpDigit]| d.iter().map(|d| d.segments.iter().filter(|s| **s).count()).collect::<Vec<_>>();
        assert_eq!(lit(&digits(Some(36.), 100., 1.)), [0, 5, 6]);
        assert_eq!(lit(&digits(Some(42.), 80., 1.)), [4, 5]);
        let half = digits(Some(35.5), 98., 0.5);
        assert_eq!(lit(&half), [5, 5, 5]);
        assert!(half[1].dot);
        assert_eq!(lit(&digits(None, 80., 1.)), [1, 1]);
    }

    #[test]
    fn segments_draw_each_digit() {
        assert_eq!(segments_of('8'), [true; 7]);
        assert_eq!(segments_of('1'), [false, true, true, false, false, false, false]);
        assert_eq!(segments_of(' '), [false; 7]);
    }

    #[test]
    fn inputs_are_sorted_into_picture_and_sound() {
        assert!(is_video("BD/DVD") && is_video("TV") && is_video("GAME"));
        assert!(!is_video("CD") && !is_video("NET") && !is_video("PHONO"));
        assert_eq!(input_icon("NET"), "material-symbols:lan-outline-rounded");
        assert_eq!(input_icon("FM"), "material-symbols:radio-outline-rounded");
    }

    #[test]
    fn the_dock_setting_keeps_side_and_relative_height() {
        let dock = parse_dock(Some("left:0.25"), 1000.);
        assert_eq!(dock, AmpDock { left: true, inset: DOCK_EDGE, top: 250. });
        assert_eq!(dock_setting(dock, 1000.), "left:0.2500:20");
        // Posée loin du bord : la distance est gardée.
        let free = parse_dock(Some("right:0.3:412"), 1000.);
        assert_eq!((free.left, free.inset, free.top), (false, 412., 300.));
        assert_eq!(parse_dock(None, 900.), AmpDock { left: false, inset: DOCK_EDGE, top: 84. });
        // Trop bas pour la fenêtre : remontée au-dessus du lecteur.
        assert_eq!(parse_dock(Some("right:0.95"), 800.).top, 800. - 290.);
    }

    #[test]
    fn decibels_use_a_true_minus_sign() {
        assert_eq!(decibels(2), "+2 dB");
        assert_eq!(decibels(-4), "−4 dB");
        assert_eq!(decibels(0), "0 dB");
    }
}
