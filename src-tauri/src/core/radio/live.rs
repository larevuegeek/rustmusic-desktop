//! Radio en direct : le flux arrive dans un tampon (différé jusqu'à 30 min), le décodeur le lit à son rythme.
//! En pause, le décodeur s'arrête de lire mais le flux continue d'arriver : la reprise repart d'où on était.

use std::collections::VecDeque;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use reqwest::header::CONTENT_TYPE;
use symphonia::core::io::MediaSource;

use crate::core::podcast::fetch::client;

/// Chemin donné au lecteur pour la radio en cours : il n'existe pas sur le disque.
pub const LIVE_PATH: &str = "rustmusic-radio://live";
/// Différé gardé derrière le direct.
const WINDOW: Duration = Duration::from_secs(30 * 60);
/// Sans aucun octet pendant ce temps, on se reconnecte (côté réseau) ou on abandonne (côté lecture).
const STALL: Duration = Duration::from_secs(20);
/// Tentatives de reconnexion de suite sans recevoir de son.
const MAX_RETRIES: u32 = 5;

#[derive(Default)]
struct Buffer {
    /// Position absolue du premier octet gardé.
    start: u64,
    data: VecDeque<u8>,
    /// Titres annoncés par la station, à la position où ils commencent.
    titles: VecDeque<(u64, String)>,
    error: Option<String>,
}

/// Différé : le temps passé en pause depuis le dernier retour au direct.
#[derive(Default)]
struct Delay {
    past: Duration,
    paused_at: Option<Instant>,
}

impl Buffer {
    fn end(&self) -> u64 {
        self.start + self.data.len() as u64
    }
}

pub struct Live {
    url: String,
    buffer: Mutex<Buffer>,
    changed: Condvar,
    cancelled: AtomicBool,
    /// Débit en kbit/s (annoncé, sinon mesuré) : sert à borner le tampon et à mesurer le retard.
    bitrate: AtomicU32,
    /// Format du flux pour le décodeur (« mp3 », « aac »…), connu à la première réponse.
    format: Mutex<Option<&'static str>>,
    /// Position où le prochain lecteur commencera (retour au direct).
    next_start: Mutex<Option<u64>>,
    /// Position du lecteur : le titre affiché est celui qu'on entend.
    read_pos: AtomicU64,
    delay: Mutex<Delay>,
}

impl Live {
    fn byte_rate(&self) -> u64 {
        u64::from(self.bitrate.load(Ordering::Relaxed).max(32)) * 1000 / 8
    }

    /// Octets gardés au plus : la fenêtre de différé au débit du flux, entre 4 et 80 Mo.
    fn capacity(&self) -> usize {
        (self.byte_rate() * WINDOW.as_secs()).clamp(4 << 20, 80 << 20) as usize
    }

    fn push(&self, audio: &[u8], titles: Vec<(usize, String)>) {
        let cap = self.capacity();
        let mut b = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let base = b.end();
        for (offset, title) in titles {
            b.titles.push_back((base + offset as u64, title));
        }
        b.data.extend(audio);
        let excess = b.data.len().saturating_sub(cap);
        if excess > 0 {
            b.data.drain(..excess);
            b.start += excess as u64;
            // Un titre sorti de la fenêtre ne sert plus, sauf le dernier : c'est peut-être celui qu'on entend.
            while b.titles.len() > 1 && b.titles[1].0 <= b.start {
                b.titles.pop_front();
            }
        }
        drop(b);
        self.changed.notify_all();
    }

    fn fail(&self, error: String) {
        self.buffer.lock().unwrap_or_else(|e| e.into_inner()).error = Some(error);
        self.changed.notify_all();
    }

    /// Le titre en cours à la position `pos`.
    fn title_at(&self, pos: u64) -> Option<String> {
        let b = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        b.titles.iter().rev().find(|(at, _)| *at <= pos).map(|(_, t)| t.clone())
    }
}

fn active() -> &'static Mutex<Option<Arc<Live>>> {
    static ACTIVE: OnceLock<Mutex<Option<Arc<Live>>>> = OnceLock::new();
    ACTIVE.get_or_init(Default::default)
}

pub fn current() -> Option<Arc<Live>> {
    active().lock().unwrap_or_else(|e| e.into_inner()).clone()
}

pub fn is_live_path(path: &Path) -> bool {
    path.as_os_str() == LIVE_PATH
}

/// Coupe la radio en cours (réseau compris).
pub fn stop() {
    if let Some(live) = active().lock().unwrap_or_else(|e| e.into_inner()).take() {
        live.cancelled.store(true, Ordering::Relaxed);
        live.changed.notify_all();
    }
}

/// Ouvre le flux `url` (la radio précédente est coupée) ; rend une fois les premiers octets arrivés.
pub async fn start(url: &str, bitrate: Option<u32>) -> Result<Arc<Live>, String> {
    stop();
    let live = Arc::new(Live {
        url: url.to_string(),
        buffer: Mutex::new(Buffer::default()),
        changed: Condvar::new(),
        cancelled: AtomicBool::new(false),
        bitrate: AtomicU32::new(bitrate.unwrap_or(0)),
        format: Mutex::new(None),
        next_start: Mutex::new(None),
        read_pos: AtomicU64::new(0),
        delay: Mutex::new(Delay::default()),
    });
    *active().lock().unwrap_or_else(|e| e.into_inner()) = Some(live.clone());
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let task = live.clone();
    tauri::async_runtime::spawn(async move { run(task, ready_tx).await });
    match tokio::time::timeout(STALL, ready_rx).await {
        Ok(Ok(Ok(()))) => Ok(live),
        Ok(Ok(Err(e))) => {
            stop();
            Err(e)
        }
        _ => {
            stop();
            Err("la radio ne répond pas".into())
        }
    }
}

/// Le prochain lecteur repart du direct (deux secondes avant, le temps de remplir la sortie).
pub fn rewind_to_live() {
    if let Some(live) = current() {
        let b = live.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let start = b.end().saturating_sub(live.byte_rate() * 2).max(b.start);
        drop(b);
        *live.next_start.lock().unwrap_or_else(|e| e.into_inner()) = Some(start);
        *live.delay.lock().unwrap_or_else(|e| e.into_inner()) = Delay::default();
    }
}

/// Pause ou reprise de la radio en cours, d'où qu'elle vienne (bouton, clavier, touches média).
pub fn set_paused(paused: bool) {
    let Some(live) = current() else { return };
    let mut d = live.delay.lock().unwrap_or_else(|e| e.into_inner());
    match (paused, d.paused_at) {
        (true, None) => d.paused_at = Some(Instant::now()),
        (false, Some(at)) => {
            d.past += at.elapsed();
            d.paused_at = None;
        }
        _ => {}
    }
}

/// État pour l'interface : titre entendu, débit, retard dû aux pauses (la rafale de connexion, c'est le direct).
pub struct Status {
    pub title: Option<String>,
    pub bitrate: u32,
    pub behind_secs: f64,
    pub paused: bool,
}

pub fn status() -> Option<Status> {
    let live = current()?;
    let pos = live.read_pos.load(Ordering::Relaxed);
    let d = live.delay.lock().unwrap_or_else(|e| e.into_inner());
    let behind = (d.past + d.paused_at.map_or(Duration::ZERO, |at| at.elapsed())).min(WINDOW);
    Some(Status {
        title: live.title_at(pos),
        bitrate: live.bitrate.load(Ordering::Relaxed),
        behind_secs: behind.as_secs_f64(),
        paused: d.paused_at.is_some(),
    })
}

/// Format du flux pour le décodeur.
pub fn format_hint() -> Option<&'static str> {
    *current()?.format.lock().unwrap_or_else(|e| e.into_inner())
}

/// Source pour le décodeur, si `path` désigne la radio en cours.
pub fn reader(path: &Path) -> Option<LiveReader> {
    if !is_live_path(path) {
        return None;
    }
    let live = current()?;
    let pos = live.next_start.lock().unwrap_or_else(|e| e.into_inner()).take().unwrap_or_else(|| {
        // Première écoute : depuis le début reçu (quelques secondes, de quoi démarrer vite).
        live.buffer.lock().unwrap_or_else(|e| e.into_inner()).start
    });
    live.read_pos.store(pos, Ordering::Relaxed);
    Some(LiveReader { live, pos })
}

fn format_of(content_type: &str, url: &str) -> &'static str {
    let ct = content_type.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    match ct.as_str() {
        "audio/aac" | "audio/aacp" | "audio/x-aac" => "aac",
        "audio/ogg" | "application/ogg" | "audio/opus" => "ogg",
        "audio/flac" | "audio/x-flac" => "flac",
        "audio/mpeg" | "audio/mp3" => "mp3",
        _ if path.ends_with(".aac") || path.ends_with(".aacp") => "aac",
        _ if path.ends_with(".ogg") || path.ends_with(".opus") => "ogg",
        _ => "mp3",
    }
}

/// Fil réseau : reçoit le flux, en retire les titres, se reconnecte après une coupure.
async fn run(live: Arc<Live>, ready: tokio::sync::oneshot::Sender<Result<(), String>>) {
    let mut ready = Some(ready);
    let mut retries = 0u32;
    let started = Instant::now();
    let mut received: u64 = 0;
    let mut last_error = String::new();
    while !live.cancelled.load(Ordering::Relaxed) && retries <= MAX_RETRIES {
        if retries > 0 {
            tokio::time::sleep(Duration::from_secs(u64::from(retries).min(5))).await;
            log::info!("📻 Reconnexion à la radio ({retries}/{MAX_RETRIES})");
        }
        retries += 1;
        let resp = client()
            .get(&live.url)
            .header("Icy-MetaData", "1")
            // Le flux ne finit pas : pas de délai global, seulement par morceau reçu.
            .timeout(Duration::from_secs(365 * 24 * 3600))
            .send()
            .await
            .and_then(|r| r.error_for_status());
        let mut resp = match resp {
            Ok(r) => r,
            Err(e) => {
                last_error = format!("radio injoignable : {e}");
                continue;
            }
        };
        let header = |k: &str| resp.headers().get(k).and_then(|v| v.to_str().ok()).map(str::to_string);
        let content_type = header(CONTENT_TYPE.as_str()).unwrap_or_default();
        if content_type.starts_with("text/html") {
            last_error = "cette adresse est une page web, pas un flux audio".into();
            break;
        }
        // Liste HLS : relue au fil de l'eau, segment par segment.
        let path = resp.url().path().to_ascii_lowercase();
        if content_type.to_ascii_lowercase().contains("mpegurl") || path.ends_with(".m3u8") {
            drop(resp);
            return run_hls(live, ready).await;
        }
        *live.format.lock().unwrap_or_else(|e| e.into_inner()) = Some(format_of(&content_type, resp.url().as_str()));
        if let Some(br) = header("icy-br").and_then(|b| b.split(',').next()?.trim().parse::<u32>().ok()).filter(|&b| b > 0) {
            live.bitrate.store(br, Ordering::Relaxed);
        }
        let mut demux = IcyDemux::new(header("icy-metaint").and_then(|m| m.trim().parse().ok()));

        loop {
            if live.cancelled.load(Ordering::Relaxed) {
                return;
            }
            let chunk = match tokio::time::timeout(STALL, resp.chunk()).await {
                Ok(Ok(Some(c))) => c,
                Ok(Ok(None)) => {
                    last_error = "la radio a coupé le flux".into();
                    break;
                }
                Ok(Err(e)) => {
                    last_error = format!("flux interrompu : {e}");
                    break;
                }
                Err(_) => {
                    last_error = "la radio n'envoie plus rien".into();
                    break;
                }
            };
            let (audio, titles) = demux.feed(&chunk);
            if audio.is_empty() {
                continue;
            }
            retries = 0;
            received += audio.len() as u64;
            // Débit non annoncé : mesuré sur les premières secondes.
            let secs = started.elapsed().as_secs_f64();
            if live.bitrate.load(Ordering::Relaxed) == 0 && secs > 8.0 {
                live.bitrate.store(((received as f64 * 8.0 / 1000.0) / secs) as u32, Ordering::Relaxed);
            }
            live.push(&audio, titles);
            if let Some(tx) = ready.take() {
                let _ = tx.send(Ok(()));
            }
        }
    }
    if live.cancelled.load(Ordering::Relaxed) {
        return;
    }
    log::warn!("📻 Radio perdue : {last_error}");
    match ready.take() {
        Some(tx) => {
            let _ = tx.send(Err(last_error));
        }
        None => live.fail(last_error),
    }
}

async fn fetch_text(url: &str) -> Result<String, String> {
    let resp = client().get(url).timeout(Duration::from_secs(10)).send().await.and_then(|r| r.error_for_status());
    resp.map_err(|e| format!("liste HLS injoignable : {e}"))?.text().await.map_err(|e| format!("liste HLS illisible : {e}"))
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let resp = client().get(url).timeout(STALL).send().await.and_then(|r| r.error_for_status());
    Ok(resp.map_err(|e| format!("segment HLS injoignable : {e}"))?.bytes().await.map_err(|e| format!("segment HLS interrompu : {e}"))?.to_vec())
}

/// Radio en HLS : la liste est relue au fil de l'eau, le son de chaque nouveau segment rejoint le tampon.
async fn run_hls(live: Arc<Live>, mut ready: Option<tokio::sync::oneshot::Sender<Result<(), String>>>) {
    let mut media_url = live.url.clone();
    let mut last_seq: Option<u64> = None;
    let mut extractor = super::hls::Extractor::default();
    let (started, mut received) = (Instant::now(), 0u64);
    let (mut failures, mut last_error) = (0u32, String::new());
    while !live.cancelled.load(Ordering::Relaxed) && failures <= MAX_RETRIES {
        let playlist = match fetch_text(&media_url).await.and_then(|t| super::hls::parse(&t, &media_url)) {
            Ok(p) => p,
            Err(e) => {
                last_error = e;
                failures += 1;
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        let media = match playlist {
            super::hls::Playlist::Master(variants) => {
                let Some(v) = super::hls::pick(&variants) else {
                    last_error = "liste HLS sans variante".into();
                    break;
                };
                if live.bitrate.load(Ordering::Relaxed) == 0 && v.bandwidth > 0 {
                    live.bitrate.store((v.bandwidth / 1000) as u32, Ordering::Relaxed);
                }
                media_url = v.uri.clone();
                continue;
            }
            super::hls::Playlist::Media(m) => m,
        };
        if media.fmp4 {
            last_error = "HLS en fragments MP4 : pas encore pris en charge".into();
            break;
        }
        // Première lecture : les trois derniers segments, de quoi démarrer près du direct.
        let first_new = last_seq.map_or_else(|| (media.sequence + media.segments.len() as u64).saturating_sub(3), |s| s + 1);
        for (i, seg) in media.segments.iter().enumerate() {
            let seq = media.sequence + i as u64;
            if seq < first_new {
                continue;
            }
            if live.cancelled.load(Ordering::Relaxed) {
                return;
            }
            let bytes = match fetch_bytes(&seg.uri).await {
                Ok(b) => b,
                Err(e) => {
                    last_error = e;
                    failures += 1;
                    break;
                }
            };
            last_seq = Some(seq);
            failures = 0;
            let (audio, format) = extractor.extract(&bytes);
            if let Some(f) = format {
                live.format.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert(f);
            }
            if audio.is_empty() {
                continue;
            }
            received += audio.len() as u64;
            let secs = started.elapsed().as_secs_f64();
            if live.bitrate.load(Ordering::Relaxed) == 0 && secs > 8.0 {
                live.bitrate.store(((received as f64 * 8.0 / 1000.0) / secs) as u32, Ordering::Relaxed);
            }
            live.push(&audio, seg.title.clone().map(|t| vec![(0, t)]).unwrap_or_default());
            if let Some(tx) = ready.take() {
                let _ = tx.send(Ok(()));
            }
        }
        if media.ended && last_seq.is_some_and(|s| s + 1 >= media.sequence + media.segments.len() as u64) {
            last_error = "la radio a terminé sa diffusion".into();
            break;
        }
        tokio::time::sleep(Duration::from_secs_f64((media.target_duration / 2.0).clamp(1.0, 10.0))).await;
    }
    if live.cancelled.load(Ordering::Relaxed) {
        return;
    }
    log::warn!("📻 Radio HLS perdue : {last_error}");
    match ready.take() {
        Some(tx) => {
            let _ = tx.send(Err(last_error));
        }
        None => live.fail(last_error),
    }
}

/// Retire du flux les métadonnées ICY intercalées tous les `metaint` octets de son.
struct IcyDemux {
    metaint: Option<usize>,
    until_meta: usize,
    meta_left: Option<usize>,
    meta: Vec<u8>,
}

impl IcyDemux {
    fn new(metaint: Option<usize>) -> Self {
        let metaint = metaint.filter(|&m| m > 0);
        Self { metaint, until_meta: metaint.unwrap_or(0), meta_left: None, meta: Vec::new() }
    }

    /// Son et titres (avec leur position dans le son rendu) d'un morceau reçu.
    fn feed(&mut self, mut chunk: &[u8]) -> (Vec<u8>, Vec<(usize, String)>) {
        let Some(metaint) = self.metaint else { return (chunk.to_vec(), Vec::new()) };
        let mut audio = Vec::with_capacity(chunk.len());
        let mut titles = Vec::new();
        while !chunk.is_empty() {
            match self.meta_left {
                None if self.until_meta > 0 => {
                    let n = self.until_meta.min(chunk.len());
                    audio.extend_from_slice(&chunk[..n]);
                    self.until_meta -= n;
                    chunk = &chunk[n..];
                }
                None => {
                    self.meta_left = Some(chunk[0] as usize * 16);
                    self.meta.clear();
                    chunk = &chunk[1..];
                }
                Some(left) => {
                    let n = left.min(chunk.len());
                    self.meta.extend_from_slice(&chunk[..n]);
                    chunk = &chunk[n..];
                    if left == n {
                        self.meta_left = None;
                        self.until_meta = metaint;
                        if let Some(t) = stream_title(&self.meta) {
                            titles.push((audio.len(), t));
                        }
                    } else {
                        self.meta_left = Some(left - n);
                    }
                }
            }
        }
        (audio, titles)
    }
}

/// `StreamTitle='Artiste - Titre';` ; UTF-8, sinon Latin-1 (vieux serveurs).
fn stream_title(meta: &[u8]) -> Option<String> {
    let text = match std::str::from_utf8(meta) {
        Ok(s) => s.to_string(),
        Err(_) => meta.iter().map(|&b| b as char).collect(),
    };
    let start = text.find("StreamTitle='")? + "StreamTitle='".len();
    let rest = &text[start..];
    let end = rest.find("';").unwrap_or_else(|| rest.trim_end_matches('\0').trim_end_matches('\'').len());
    let title = rest[..end].trim();
    (!title.is_empty() && title != "-").then(|| title.to_string())
}

/// Lecture du tampon pour le décodeur : attend le son qui arrive, saute en avant si la pause a dépassé la fenêtre.
pub struct LiveReader {
    live: Arc<Live>,
    pos: u64,
}

impl Read for LiveReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let live = &self.live;
        let mut b = live.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let deadline = Instant::now() + STALL;
        while self.pos >= b.end() && b.error.is_none() && !live.cancelled.load(Ordering::Relaxed) {
            let wait = deadline.saturating_duration_since(Instant::now());
            if wait.is_zero() {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "la radio n'envoie plus rien"));
            }
            b = live.changed.wait_timeout(b, wait).unwrap_or_else(|e| e.into_inner()).0;
        }
        if live.cancelled.load(Ordering::Relaxed) {
            return Ok(0);
        }
        if self.pos >= b.end() {
            return Err(io::Error::other(b.error.clone().unwrap_or_default()));
        }
        // Pause plus longue que la fenêtre : on reprend au plus ancien gardé.
        self.pos = self.pos.max(b.start);
        let from = (self.pos - b.start) as usize;
        let (a, c) = b.data.as_slices();
        let src = if from < a.len() { &a[from..] } else { &c[from - a.len()..] };
        let n = buf.len().min(src.len());
        buf[..n].copy_from_slice(&src[..n]);
        self.pos += n as u64;
        live.read_pos.store(self.pos, Ordering::Relaxed);
        Ok(n)
    }
}

impl Seek for LiveReader {
    /// Seulement dans ce qui est gardé : le décodeur relit parfois l'en-tête qu'il vient de lire.
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let b = self.live.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let target = match to {
            SeekFrom::Start(n) => Some(n),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
            SeekFrom::End(_) => None,
        }
        .filter(|&t| t >= b.start && t <= b.end())
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "flux en direct"))?;
        self.pos = target;
        Ok(target)
    }
}

impl MediaSource for LiveReader {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta_block(text: &str) -> Vec<u8> {
        let mut m = text.as_bytes().to_vec();
        m.resize(text.len().div_ceil(16) * 16, 0);
        let mut out = vec![(m.len() / 16) as u8];
        out.extend(m);
        out
    }

    #[test]
    fn strips_icy_metadata_across_chunks() {
        let mut stream = vec![1u8; 8];
        stream.extend(meta_block("StreamTitle='Daft Punk - Da Funk';"));
        stream.extend(vec![2u8; 8]);
        stream.push(0); // bloc vide
        stream.extend(vec![3u8; 4]);
        let mut d = IcyDemux::new(Some(8));
        let (mut audio, mut titles) = (Vec::new(), Vec::new());
        // Morceaux de taille quelconque : 5 octets à la fois.
        for c in stream.chunks(5) {
            let base = audio.len();
            let (a, t) = d.feed(c);
            titles.extend(t.into_iter().map(|(o, s)| (base + o, s)));
            audio.extend(a);
        }
        assert_eq!(audio, [vec![1u8; 8], vec![2u8; 8], vec![3u8; 4]].concat());
        assert_eq!(titles, vec![(8, "Daft Punk - Da Funk".to_string())]);
    }

    #[test]
    fn reads_titles() {
        assert_eq!(stream_title(b"StreamTitle='FIP - Jazz';StreamUrl='';\0\0").as_deref(), Some("FIP - Jazz"));
        assert_eq!(stream_title(b"StreamTitle='';\0"), None);
        assert_eq!(stream_title(&[b"StreamTitle='Caf".as_slice(), &[0xE9], b"';"].concat()).as_deref(), Some("Café"));
    }

    fn live_with(data: &[u8], bitrate: u32) -> Arc<Live> {
        let live = Arc::new(Live {
            url: String::new(),
            buffer: Mutex::new(Buffer::default()),
            changed: Condvar::new(),
            cancelled: AtomicBool::new(false),
            bitrate: AtomicU32::new(bitrate),
            format: Mutex::new(Some("mp3")),
            next_start: Mutex::new(None),
            read_pos: AtomicU64::new(0),
            delay: Mutex::new(Delay::default()),
        });
        live.push(data, vec![(0, "Un".into()), (4, "Deux".into())]);
        live
    }

    #[test]
    fn reader_follows_titles_and_jumps_ahead_of_the_window() {
        let live = live_with(&[0, 1, 2, 3, 4, 5, 6, 7], 128);
        let mut r = LiveReader { live: live.clone(), pos: 0 };
        let mut buf = [0u8; 3];
        assert_eq!(r.read(&mut buf).unwrap(), 3);
        assert_eq!(live.title_at(live.read_pos.load(Ordering::Relaxed)).as_deref(), Some("Un"));
        assert_eq!(r.read(&mut buf).unwrap(), 3);
        assert_eq!(buf, [3, 4, 5]);
        assert_eq!(live.title_at(live.read_pos.load(Ordering::Relaxed)).as_deref(), Some("Deux"));
        // La fenêtre a avancé pendant une pause : lecture au plus ancien gardé.
        {
            let mut b = live.buffer.lock().unwrap();
            b.data.drain(..7);
            b.start = 7;
        }
        assert_eq!(r.read(&mut buf).unwrap(), 1);
        assert_eq!(buf[0], 7);
    }

    #[test]
    fn reader_waits_for_data_then_stops_when_cancelled() {
        let live = live_with(&[9], 128);
        let mut r = LiveReader { live: live.clone(), pos: 1 };
        let pusher = live.clone();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            pusher.push(&[10, 11], Vec::new());
        });
        let mut buf = [0u8; 4];
        assert_eq!(r.read(&mut buf).unwrap(), 2);
        assert_eq!(&buf[..2], &[10, 11]);
        t.join().unwrap();
        live.cancelled.store(true, Ordering::Relaxed);
        assert_eq!(r.read(&mut buf).unwrap(), 0);
    }

    /// Vraie radio : connexion, décodage de quelques secondes, titre en cours.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "réseau"]
    async fn real_live() {
        use symphonia::core::codecs::audio::AudioDecoderOptions;
        use symphonia::core::formats::probe::Hint;
        use symphonia::core::io::MediaSourceStream;
        let url = std::env::var("RADIO").unwrap_or_else(|_| "http://icecast.radiofrance.fr/fip-hifi.aac".into());
        start(&url, None).await.expect("radio");
        let reader = reader(Path::new(LIVE_PATH)).unwrap();
        let ext = format_hint().unwrap();
        let decoded = tokio::task::spawn_blocking(move || {
            let mut hint = Hint::new();
            hint.with_extension(ext);
            let mss = MediaSourceStream::new(Box::new(reader), Default::default());
            let mut format = symphonia::default::get_probe().probe(&hint, mss, Default::default(), Default::default()).unwrap();
            let track = format.tracks()[0].clone();
            let params = track.codec_params.as_ref().unwrap().audio().unwrap().clone();
            let mut dec = crate::core::audio_player::codecs::codec_registry().make_audio_decoder(&params, &AudioDecoderOptions::default()).unwrap();
            let (mut frames, rate) = (0usize, params.sample_rate.unwrap());
            while frames < rate as usize * 4 {
                let p = format.next_packet().unwrap().unwrap();
                frames += dec.decode(&p).map(|b| b.frames()).unwrap_or(0);
            }
            (frames, rate, params.codec)
        })
        .await
        .unwrap();
        let s = status().unwrap();
        println!("{ext} {:?} : {} images à {} Hz, {} kbit/s, titre {:?}", decoded.2, decoded.0, decoded.1, s.bitrate, s.title);
        stop();
        assert!(decoded.0 > 0);
    }

    #[test]
    fn delay_comes_from_pauses_and_resets_on_going_live() {
        let live = live_with(&[1, 2, 3], 128);
        *active().lock().unwrap() = Some(live);
        assert_eq!(status().unwrap().behind_secs, 0.0);
        set_paused(true);
        std::thread::sleep(Duration::from_millis(60));
        let s = status().unwrap();
        assert!(s.paused && s.behind_secs >= 0.05);
        set_paused(false);
        let after = status().unwrap();
        assert!(!after.paused && after.behind_secs >= 0.05, "le retard reste après la reprise");
        rewind_to_live();
        assert_eq!(status().unwrap().behind_secs, 0.0);
        stop();
    }

    #[test]
    fn guesses_the_decoder_format() {
        assert_eq!(format_of("audio/aacp", "http://x/s"), "aac");
        assert_eq!(format_of("application/octet-stream", "http://x/fip-hifi.aac?id=1"), "aac");
        assert_eq!(format_of("audio/mpeg", "http://x/s"), "mp3");
        assert_eq!(format_of("", "http://x/live"), "mp3");
    }
}
