//! Yamaha (MusicCast) : API « Yamaha Extended Control » (YXC), HTTP/JSON sur le port 80.
//! Pas d'annonce sans abonnement : l'état est relu à intervalle court.
//! Référence : « Yamaha Extended Control API Specification (Basic) ».

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::mpsc::UnboundedReceiver;

use super::model::{Amp, Brand, Caps, Choice, Command, NavKey, Net, NetLine, Range, Status, Transport, Zone, ZoneState};

pub const PORT: u16 = 80;
const BASE: &str = "/YamahaExtendedControl/v1";
const POLL: Duration = Duration::from_millis(1500);
/// Lignes lues par page de liste (8 au plus, d'après l'API).
const PAGE: u32 = 8;

/// Ce que l'API ne redit pas à chaque lecture : entrées réseau, noms, page de liste.
#[derive(Debug, Default, Clone)]
pub struct Ctx {
    loaded: bool,
    /// Entrées servies par « netusb » (lecture, listes).
    net_inputs: Vec<String>,
    program_names: Vec<(String, String)>,
    /// Début de la page de liste affichée, et curseur tenu ici (l'ampli n'en a pas).
    page: u32,
    cursor: u8,
    /// Attributs des lignes affichées (dossier, lisible).
    attrs: Vec<u64>,
    cover_url: String,
}

fn ok(v: &Value) -> bool {
    v["response_code"].as_i64() == Some(0)
}

fn names(list: &Value) -> Vec<(String, String)> {
    list.as_array()
        .map(|a| a.iter().filter_map(|e| Some((e["id"].as_str()?.to_string(), e["text"].as_str()?.to_string()))).collect())
        .unwrap_or_default()
}

fn name_of(names: &[(String, String)], id: &str) -> String {
    names.iter().find(|(i, _)| i == id).map(|(_, t)| t.clone()).unwrap_or_else(|| id.to_string())
}

/// Capacités lues dans `getFeatures` et `getNameText`.
pub fn read_features(features: &Value, name_text: &Value, caps: &mut Caps, ctx: &mut Ctx) {
    let inputs = names(&name_text["input_list"]);
    ctx.program_names = names(&name_text["sound_program_list"]);
    ctx.net_inputs = features["system"]["input_list"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|i| i["play_info_type"] == "netusb")
                .filter_map(|i| i["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let zones = features["zone"].as_array().cloned().unwrap_or_default();
    caps.zone2 = zones.iter().any(|z| z["id"] == "zone2");
    let Some(main) = zones.iter().find(|z| z["id"] == "main") else { return };
    let ids = |key: &str| -> Vec<String> {
        main[key].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default()
    };
    caps.inputs = ids("input_list").into_iter().map(|id| Choice { name: name_of(&inputs, &id), id }).collect();
    caps.modes = ids("sound_program_list").into_iter().map(|id| Choice { name: name_of(&ctx.program_names, &id), id }).collect();
    let range = |id: &str| main["range_step"].as_array().and_then(|a| a.iter().find(|r| r["id"] == id).cloned());
    if let Some(v) = range("volume") {
        caps.volume_max = v["max"].as_f64().unwrap_or(100.0) as f32;
        caps.volume_step = v["step"].as_f64().unwrap_or(1.0) as f32;
    }
    let has_tone = main["func_list"].as_array().is_some_and(|f| f.iter().any(|x| x == "tone_control"));
    caps.tone = range("tone_control").filter(|_| has_tone).map(|t| Range {
        min: t["min"].as_i64().unwrap_or(-12) as i32,
        max: t["max"].as_i64().unwrap_or(12) as i32,
        step: t["step"].as_i64().unwrap_or(1).max(1) as i32,
    });
}

/// État d'une zone (`getStatus`) ; rend mode et tonalité pour la zone principale.
pub fn read_status(v: &Value, zone: &mut ZoneState) -> (Option<String>, Option<i32>, Option<i32>) {
    zone.power = v["power"] == "on";
    zone.volume = v["volume"].as_f64().map(|x| x as f32);
    zone.muted = v["mute"].as_bool().unwrap_or(false);
    zone.input = v["input"].as_str().map(str::to_string);
    let tone = |k: &str| v["tone_control"][k].as_i64().map(|x| x as i32);
    (v["sound_program"].as_str().map(str::to_string), tone("bass"), tone("treble"))
}

/// Lecture en cours (`netusb/getPlayInfo`).
pub fn read_play_info(v: &Value, net: &mut Net) {
    let s = |k: &str| v[k].as_str().unwrap_or_default().to_string();
    net.track = s("track");
    net.artist = s("artist");
    net.album = s("album");
    net.status = match v["playback"].as_str() {
        Some("play") => "play",
        Some("pause") => "pause",
        _ => "stop",
    }
    .to_string();
    let mmss = |t: i64| format!("{:02}:{:02}", t / 60, t % 60);
    net.time = match (v["play_time"].as_i64(), v["total_time"].as_i64()) {
        (Some(p), Some(t)) if t > 0 => format!("{} / {}", mmss(p), mmss(t)),
        (Some(p), _) if p > 0 => mmss(p),
        _ => String::new(),
    };
}

/// Page de liste (`netusb/getListInfo`). Attributs : bit 1 dossier, bit 2 lisible, bit 5 en lecture.
pub fn read_list(v: &Value, net: &mut Net, ctx: &mut Ctx) {
    net.title = v["menu_name"].as_str().unwrap_or_default().to_string();
    net.count = v["max_line"].as_u64().unwrap_or(0) as u32;
    let items = v["list_info"].as_array().cloned().unwrap_or_default();
    ctx.attrs = items.iter().map(|i| i["attribute"].as_u64().unwrap_or(0)).collect();
    net.lines = items
        .iter()
        .map(|i| {
            let a = i["attribute"].as_u64().unwrap_or(0);
            let kind = if a & 0x10 != 0 {
                "playing"
            } else if a & 0x01 != 0 && a & 0x02 == 0 {
                "folder"
            } else if a & 0x02 != 0 {
                "track"
            } else {
                "item"
            };
            NetLine { text: i["text"].as_str().unwrap_or_default().to_string(), kind: kind.into() }
        })
        .collect();
    if !net.lines.is_empty() {
        ctx.cursor = ctx.cursor.min(net.lines.len() as u8 - 1);
    }
    net.cursor = (!net.lines.is_empty()).then_some(ctx.cursor);
    // Une liste s'affiche dès qu'elle a des lignes ; sinon l'écran de lecture.
    net.playback = net.lines.is_empty();
}

fn on_net(status: &Status, ctx: &Ctx) -> Option<String> {
    status.state.main.input.clone().filter(|i| ctx.net_inputs.contains(i))
}

/// Requêtes (chemins sous `/v1`) d'une commande ; le curseur de liste est tenu dans `ctx`.
pub fn encode(cmd: &Command, status: &Status, ctx: &mut Ctx) -> Vec<String> {
    let z = |zone: &Zone| if *zone == Zone::Main { "main" } else { "zone2" };
    let list = |ctx: &Ctx, kind: &str| format!("netusb/setListControl?list_id=main&type={kind}&index={}&zone=main", ctx.page + ctx.cursor as u32);
    // Une ligne : dossier ouvert, morceau joué.
    let open = |ctx: &Ctx| {
        let a = ctx.attrs.get(ctx.cursor as usize).copied().unwrap_or(1);
        list(ctx, if a & 0x01 != 0 && a & 0x02 == 0 { "select" } else { "play" })
    };
    let net = on_net(status, ctx);
    match cmd {
        Command::Power { zone, on } => vec![format!("{}/setPower?power={}", z(zone), if *on { "on" } else { "standby" })],
        Command::Volume { zone, value } => vec![format!("{}/setVolume?volume={}", z(zone), value.clamp(0.0, status.caps.volume_max).round() as i64)],
        Command::VolumeStep { zone, up } => vec![format!("{}/setVolume?volume={}", z(zone), if *up { "up" } else { "down" })],
        Command::Mute { zone, on } => vec![format!("{}/setMute?enable={on}", z(zone))],
        Command::Input { zone, id } => vec![format!("{}/setInput?input={id}", z(zone))],
        Command::Mode { id } => vec![format!("main/setSoundProgram?program={id}")],
        Command::Bass { value } => vec![format!("main/setToneControl?mode=manual&bass={value}")],
        Command::Treble { value } => vec![format!("main/setToneControl?mode=manual&treble={value}")],
        Command::Nav { key } if net.is_some() => {
            let count = status.state.net.count;
            let shown = status.state.net.lines.len() as u8;
            match key {
                NavKey::Up => {
                    if ctx.cursor > 0 {
                        ctx.cursor -= 1;
                    } else if ctx.page > 0 {
                        ctx.page = ctx.page.saturating_sub(PAGE);
                        ctx.cursor = PAGE as u8 - 1;
                    }
                    vec![]
                }
                NavKey::Down => {
                    if ctx.cursor + 1 < shown {
                        ctx.cursor += 1;
                    } else if ctx.page + PAGE < count {
                        ctx.page += PAGE;
                        ctx.cursor = 0;
                    }
                    vec![]
                }
                NavKey::Enter | NavKey::Right => {
                    let r = open(ctx);
                    (ctx.page, ctx.cursor) = (0, 0);
                    vec![r]
                }
                NavKey::Left | NavKey::Back | NavKey::Home | NavKey::Menu => {
                    (ctx.page, ctx.cursor) = (0, 0);
                    vec![list(ctx, "return")]
                }
            }
        }
        Command::Nav { key } => vec![format!(
            "main/controlCursor?cursor={}",
            match key {
                NavKey::Up => "up",
                NavKey::Down => "down",
                NavKey::Left => "left",
                NavKey::Right => "right",
                NavKey::Enter => "select",
                NavKey::Back => "return",
                NavKey::Home => "home",
                NavKey::Menu => "setup",
            }
        )],
        Command::NetSelect { line } => {
            ctx.cursor = *line;
            let r = open(ctx);
            (ctx.page, ctx.cursor) = (0, 0);
            vec![r]
        }
        Command::Transport { action } => vec![match action {
            Transport::PlayPause => format!("netusb/setPlayback?playback={}", if status.state.net.status == "play" { "pause" } else { "play" }),
            Transport::Stop => "netusb/setPlayback?playback=stop".into(),
            Transport::Next => "netusb/setPlayback?playback=next".into(),
            Transport::Previous => "netusb/setPlayback?playback=previous".into(),
            Transport::Repeat => "netusb/toggleRepeat".into(),
            Transport::Shuffle => "netusb/toggleShuffle".into(),
        }],
    }
}

/// Ampli Yamaha trouvé par la recherche UPnP.
pub fn amp_from_upnp(id: &str, name: &str, manufacturer: &str, model: &str, ip: &str) -> Option<Amp> {
    manufacturer.to_lowercase().contains("yamaha").then(|| Amp {
        id: format!("yamaha:{}", id.trim_start_matches("uuid:")),
        name: name.to_string(),
        brand: Brand::Yamaha,
        model: model.to_string(),
        ip: ip.to_string(),
        port: PORT,
    })
}

pub fn default_caps() -> Caps {
    Caps { inputs: vec![], modes: vec![], volume_max: 100.0, volume_step: 1.0, tone: None, zone2: false }
}

fn lock(status: &Mutex<Status>) -> MutexGuard<'_, Status> {
    status.lock().unwrap_or_else(|e| e.into_inner())
}

struct Client {
    http: reqwest::Client,
    base: String,
    origin: String,
}

impl Client {
    async fn get(&self, path: &str) -> Option<Value> {
        let v: Value = self.http.get(format!("{}/{path}", self.base)).send().await.ok()?.json().await.ok()?;
        if !ok(&v) {
            log::debug!("🎛️  Yamaha {path} : code {}", v["response_code"]);
        }
        ok(&v).then_some(v)
    }
}

/// Relit tout ce qui s'affiche ; `false` si l'ampli ne répond pas.
async fn poll(c: &Client, ctx: &mut Ctx, status: &Mutex<Status>) -> bool {
    if !ctx.loaded {
        let (Some(f), Some(n)) = (c.get("system/getFeatures").await, c.get("system/getNameText").await) else { return false };
        read_features(&f, &n, &mut lock(status).caps, ctx);
        ctx.loaded = true;
    }
    let Some(main) = c.get("main/getStatus").await else { return false };
    let zone2 = if lock(status).caps.zone2 { c.get("zone2/getStatus").await } else { None };
    {
        let mut st = lock(status);
        let (program, bass, treble) = read_status(&main, &mut st.state.main);
        st.state.mode = program.map(|id| Choice { name: name_of(&ctx.program_names, &id), id });
        (st.state.bass, st.state.treble) = (bass, treble);
        if let Some(max) = main["max_volume"].as_f64() {
            st.caps.volume_max = max as f32;
        }
        if let Some(z2) = &zone2 {
            read_status(z2, st.state.zone2.get_or_insert_with(ZoneState::default));
        }
    }
    let input = on_net(&lock(status), ctx);
    if let Some(input) = input {
        let play = c.get("netusb/getPlayInfo").await;
        let list = c.get(&format!("netusb/getListInfo?input={input}&index={}&size={PAGE}&lang=en", ctx.page)).await;
        let art = play.as_ref().and_then(|p| p["albumart_url"].as_str()).unwrap_or_default().to_string();
        {
            let mut st = lock(status);
            if let Some(p) = &play {
                read_play_info(p, &mut st.state.net);
            }
            if let Some(l) = &list {
                read_list(l, &mut st.state.net, ctx);
            }
        }
        if art != ctx.cover_url {
            ctx.cover_url = art.clone();
            let cover = if art.is_empty() { None } else { fetch_image(c, &art).await };
            let mut st = lock(status);
            st.cover = cover;
            st.state.net.cover_rev = st.state.net.cover_rev.wrapping_add(1);
        }
    }
    true
}

async fn fetch_image(c: &Client, url: &str) -> Option<(String, Vec<u8>)> {
    let url = if url.starts_with("http") { url.to_string() } else { format!("{}{url}", c.origin) };
    let resp = c.http.get(url).send().await.ok()?;
    let mime = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("image/jpeg").to_string();
    let bytes = resp.bytes().await.ok()?.to_vec();
    (!bytes.is_empty()).then_some((mime, bytes))
}

/// Interroge l'ampli et envoie les commandes ; `changed(cover)` après chaque changement visible.
pub async fn run(amp: Amp, status: Arc<Mutex<Status>>, mut rx: UnboundedReceiver<Command>, changed: impl Fn(bool) + Send) {
    let origin = format!("http://{}:{}", amp.ip, amp.port);
    let c = Client {
        http: reqwest::Client::builder().timeout(Duration::from_secs(3)).build().unwrap_or_default(),
        base: format!("{origin}{BASE}"),
        origin,
    };
    let mut ctx = Ctx::default();
    let mut tick = tokio::time::interval(POLL);
    loop {
        tokio::select! {
            _ = tick.tick() => {}
            cmd = rx.recv() => {
                let Some(cmd) = cmd else { return };
                let paths = { let st = lock(&status); encode(&cmd, &st, &mut ctx) };
                for p in paths {
                    c.get(&p).await;
                }
            }
        }
        let before = lock(&status).state.clone();
        let connected = poll(&c, &mut ctx, &status).await;
        if !connected {
            // Reconnexion : capacités relues, l'ampli a pu changer.
            ctx.loaded = false;
        }
        let after = {
            let mut st = lock(&status);
            st.state.connected = connected;
            st.state.clone()
        };
        if after != before {
            changed(after.net.cover_rev != before.net.cover_rev);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn features() -> Value {
        json!({"response_code":0,
            "system":{"input_list":[{"id":"hdmi1","play_info_type":"none"},{"id":"server","play_info_type":"netusb"},{"id":"net_radio","play_info_type":"netusb"}]},
            "zone":[
                {"id":"main","func_list":["power","volume","mute","sound_program","tone_control"],
                 "input_list":["hdmi1","server","net_radio"],"sound_program_list":["straight","2ch_stereo"],
                 "range_step":[{"id":"volume","min":0,"max":161,"step":1},{"id":"tone_control","min":-12,"max":12,"step":1}]},
                {"id":"zone2","func_list":["power","volume"],"input_list":["server"]}]})
    }

    fn name_text() -> Value {
        json!({"response_code":0,"input_list":[{"id":"hdmi1","text":"Télé"},{"id":"server","text":"Server"}],
               "sound_program_list":[{"id":"straight","text":"Straight"},{"id":"2ch_stereo","text":"2ch Stereo"}]})
    }

    fn ready() -> (Status, Ctx) {
        let (mut st, mut ctx) = (Status { caps: default_caps(), state: Default::default(), cover: None }, Ctx::default());
        read_features(&features(), &name_text(), &mut st.caps, &mut ctx);
        (st, ctx)
    }

    #[test]
    fn reads_features_and_status() {
        let (mut st, ctx) = ready();
        let inputs: Vec<_> = st.caps.inputs.iter().map(|i| (i.id.as_str(), i.name.as_str())).collect();
        assert_eq!(inputs, vec![("hdmi1", "Télé"), ("server", "Server"), ("net_radio", "net_radio")]);
        assert_eq!(st.caps.modes[1].name, "2ch Stereo");
        assert_eq!((st.caps.volume_max, st.caps.zone2), (161.0, true));
        assert_eq!(st.caps.tone, Some(Range { min: -12, max: 12, step: 1 }));
        assert_eq!(ctx.net_inputs, vec!["server", "net_radio"]);

        let s = json!({"response_code":0,"power":"on","volume":72,"mute":false,"input":"server","sound_program":"straight","tone_control":{"mode":"manual","bass":2,"treble":-4}});
        let (program, bass, treble) = read_status(&s, &mut st.state.main);
        assert!(st.state.main.power);
        assert_eq!((st.state.main.volume, st.state.main.input.as_deref()), (Some(72.0), Some("server")));
        assert_eq!((program.as_deref(), bass, treble), (Some("straight"), Some(2), Some(-4)));
    }

    #[test]
    fn reads_playback_and_lists() {
        let (mut st, mut ctx) = ready();
        read_play_info(&json!({"response_code":0,"playback":"pause","artist":"Muse","album":"Drones","track":"Mercy","play_time":65,"total_time":230}), &mut st.state.net);
        assert_eq!((st.state.net.status.as_str(), st.state.net.time.as_str()), ("pause", "01:05 / 03:50"));
        read_list(&json!({"response_code":0,"menu_name":"Server","max_line":12,"list_info":[{"text":"Musique","attribute":1},{"text":"Mercy","attribute":2}]}), &mut st.state.net, &mut ctx);
        assert_eq!((st.state.net.title.as_str(), st.state.net.count), ("Server", 12));
        assert_eq!((st.state.net.lines[0].kind.as_str(), st.state.net.lines[1].kind.as_str()), ("folder", "track"));
        assert_eq!(st.state.net.cursor, Some(0));
    }

    #[test]
    fn encodes_commands() {
        let (mut st, mut ctx) = ready();
        assert_eq!(encode(&Command::VolumeStep { zone: Zone::Main, up: true }, &st, &mut ctx), vec!["main/setVolume?volume=up"]);
        assert_eq!(encode(&Command::Power { zone: Zone::Zone2, on: false }, &st, &mut ctx), vec!["zone2/setPower?power=standby"]);
        assert_eq!(encode(&Command::Mute { zone: Zone::Main, on: true }, &st, &mut ctx), vec!["main/setMute?enable=true"]);
        assert_eq!(encode(&Command::Bass { value: 3 }, &st, &mut ctx), vec!["main/setToneControl?mode=manual&bass=3"]);
        assert_eq!(encode(&Command::Nav { key: NavKey::Up }, &st, &mut ctx), vec!["main/controlCursor?cursor=up"]);

        // Sur une entrée réseau, la liste se parcourt ici puis s'ouvre à la bonne ligne.
        st.state.main.input = Some("server".into());
        read_list(&json!({"response_code":0,"max_line":2,"list_info":[{"text":"Musique","attribute":1},{"text":"Mercy","attribute":2}]}), &mut st.state.net, &mut ctx);
        assert!(encode(&Command::Nav { key: NavKey::Down }, &st, &mut ctx).is_empty());
        assert_eq!(encode(&Command::Nav { key: NavKey::Enter }, &st, &mut ctx), vec!["netusb/setListControl?list_id=main&type=play&index=1&zone=main"]);
        assert_eq!(encode(&Command::NetSelect { line: 0 }, &st, &mut ctx), vec!["netusb/setListControl?list_id=main&type=select&index=0&zone=main"]);
        st.state.net.status = "play".into();
        assert_eq!(encode(&Command::Transport { action: Transport::PlayPause }, &st, &mut ctx), vec!["netusb/setPlayback?playback=pause"]);
    }

    #[tokio::test]
    async fn talks_to_a_fake_yamaha() {
        use axum::{extract::Request, routing::get, Router};
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen = calls.clone();
        let app = Router::new().fallback(get(move |req: Request| {
            let seen = seen.clone();
            async move {
                let path = req.uri().path_and_query().map(|p| p.to_string()).unwrap_or_default();
                seen.lock().unwrap().push(path.clone());
                let body = if path.ends_with("system/getFeatures") {
                    features()
                } else if path.ends_with("system/getNameText") {
                    name_text()
                } else if path.ends_with("main/getStatus") {
                    json!({"response_code":0,"power":"on","volume":40,"mute":false,"input":"hdmi1","sound_program":"straight"})
                } else {
                    json!({"response_code":0})
                };
                axum::Json(body)
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let amp = Amp { id: "yamaha:t".into(), name: "Banc".into(), brand: Brand::Yamaha, model: "RX-V6A".into(), ip: addr.ip().to_string(), port: addr.port() };
        let status = Arc::new(Mutex::new(Status { caps: default_caps(), state: Default::default(), cover: None }));
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(run(amp, status.clone(), rx, |_| {}));

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while lock(&status).state.main.volume != Some(40.0) {
            assert!(tokio::time::Instant::now() < deadline, "état jamais lu");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(lock(&status).state.connected);
        assert_eq!(lock(&status).caps.inputs.len(), 3);
        tx.send(Command::VolumeStep { zone: Zone::Main, up: false }).unwrap();
        while !calls.lock().unwrap().iter().any(|c| c.ends_with("main/setVolume?volume=down")) {
            assert!(tokio::time::Instant::now() < deadline, "commande jamais reçue");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        task.abort();
    }
}
