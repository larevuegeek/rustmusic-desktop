//! Cache des épisodes, et lecture pendant le téléchargement ([`GrowingFile`], via un `.part`).

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::Duration;

use reqwest::header::RANGE;
use reqwest::StatusCode;
use symphonia::core::io::MediaSource;
use tokio::io::AsyncWriteExt;

use super::fetch::client;

/// Sans nouvel octet pendant ce temps, la lecture abandonne plutôt que de rester figée.
const STALL_TIMEOUT: Duration = Duration::from_secs(30);
/// Lecture loin devant le téléchargement (fin du fichier, saut) : le morceau est demandé à part,
/// par blocs de cette taille, au lieu d'attendre que tout arrive.
const BLOCK: u64 = 256 * 1024;
/// En deçà de cette avance, attendre le téléchargement coûte moins qu'une requête de plus.
const NEAR: u64 = 1024 * 1024;
/// Fin d'un fichier de taille estimée, lue d'avance pour ses tags : réputée vide.
const TAIL: u64 = 256 * 1024;
/// Blocs gardés en mémoire (~12 Mo).
const MAX_BLOCKS: usize = 48;

/// Dossier des épisodes téléchargés.
pub fn episodes_dir() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_default();
    dir.push("com.larevuegeek.rustmusic");
    dir.push("podcasts");
    dir
}

/// Noms courts : le chemin complet doit tenir sous la limite de 260 caractères de Windows.
const MAX_FOLDER: usize = 60;
const MAX_NAME: usize = 110;

/// Chemin de l'épisode dans le cache : `<émission>/<date> <titre>[ (suffixe)].<ext>`.
/// L'extension suit le fichier distant (le décodeur s'en sert) ; `suffix` départage deux homonymes.
pub fn episode_path(
    podcast_title: &str,
    published: Option<&str>,
    title: &str,
    audio_url: &str,
    audio_type: Option<&str>,
    suffix: Option<&str>,
) -> PathBuf {
    let short = |s: String, max: usize| -> String {
        let s: String = s.chars().take(max).collect();
        s.trim_end_matches([' ', '.']).to_string()
    };
    let folder = short(crate::core::tag_pattern::sanitize::component(podcast_title), MAX_FOLDER);
    let folder = if folder.is_empty() { "Podcast".to_string() } else { folder };
    let date = published.and_then(|d| d.get(..10)).map(|d| format!("{d} ")).unwrap_or_default();
    let mut name = short(crate::core::tag_pattern::sanitize::component(&format!("{date}{title}")), MAX_NAME);
    if let Some(s) = suffix {
        name = format!("{name} ({s})");
    }
    episodes_dir().join(folder).join(format!("{name}.{}", extension(audio_url, audio_type)))
}

/// Un chemin du cache des podcasts : ni récents, ni lecture de tags.
pub fn is_episode_path(path: &Path) -> bool {
    path.starts_with(episodes_dir())
}

fn extension(url: &str, mime: Option<&str>) -> &'static str {
    const KNOWN: &[&str] = &["mp3", "m4a", "aac", "ogg", "oga", "opus", "flac", "wav", "mp4"];
    let path = url.split(['?', '#']).next().unwrap_or("").to_ascii_lowercase();
    if let Some(ext) = path.rsplit('/').next().and_then(|n| n.rsplit_once('.')).map(|(_, e)| e) {
        if let Some(k) = KNOWN.iter().find(|k| **k == ext) {
            return k;
        }
    }
    match mime.unwrap_or("").to_ascii_lowercase().as_str() {
        "audio/mp4" | "audio/x-m4a" | "audio/m4a" => "m4a",
        "audio/aac" | "audio/aacp" => "aac",
        "audio/ogg" | "application/ogg" => "ogg",
        "audio/opus" => "opus",
        "audio/flac" | "audio/x-flac" => "flac",
        "audio/wav" | "audio/x-wav" => "wav",
        _ => "mp3",
    }
}

#[derive(Default)]
struct Progress {
    written: u64,
    total: Option<u64>,
    done: bool,
    error: Option<String>,
}

/// Un téléchargement en cours, partagé entre l'écrivain et les lecteurs.
pub struct Download {
    path: PathBuf,
    part: PathBuf,
    /// Adresse finale (après redirections) : l'adresse d'origine peut servir une autre version (pubs insérées).
    url: Mutex<Option<String>>,
    /// Abandon demandé (podcast quitté) : le `.part` est effacé.
    cancelled: AtomicBool,
    /// Faux dès qu'un serveur ignore les requêtes partielles : on attend alors le téléchargement.
    ranges: AtomicBool,
    blocks: Mutex<HashMap<u64, Arc<Vec<u8>>>>,
    /// Durée annoncée par le flux : à défaut d'en-tête, le décodeur ne la connaît pas.
    pub duration_hint: Option<f64>,
    /// Taille estimée quand le serveur ne l'annonce pas (0 = inconnue) : le décodeur en a besoin pour sauter.
    size_hint: AtomicU64,
    progress: Mutex<Progress>,
    changed: Condvar,
}

impl Download {
    /// (octets reçus, taille annoncée).
    pub fn progress(&self) -> (u64, Option<u64>) {
        let p = self.progress.lock().unwrap_or_else(|e| e.into_inner());
        (p.written, p.total)
    }

    pub fn set_size_hint(&self, bytes: u64) {
        self.size_hint.store(bytes, Ordering::Relaxed);
    }

    /// Taille annoncée, sinon estimée.
    fn expected_len(&self) -> Option<u64> {
        self.progress().1.or(Some(self.size_hint.load(Ordering::Relaxed)).filter(|&n| n > 0))
    }

    /// (reçus, taille annoncée, fini).
    fn snapshot(&self) -> (u64, Option<u64>, bool) {
        let p = self.progress.lock().unwrap_or_else(|e| e.into_inner());
        (p.written, p.total, p.done)
    }

    /// Bloc `index` demandé à part (`Range`) ; `None` si le serveur ne le permet pas.
    /// Appelé depuis le fil du décodeur, jamais depuis une tâche asynchrone.
    fn block(&self, index: u64) -> io::Result<Option<Arc<Vec<u8>>>> {
        if !self.ranges.load(Ordering::Relaxed) {
            return Ok(None);
        }
        if let Some(b) = self.cached(index) {
            return Ok(Some(b));
        }
        tauri::async_runtime::block_on(self.fetch_block(index))
    }

    fn cached(&self, index: u64) -> Option<Arc<Vec<u8>>> {
        self.blocks.lock().unwrap_or_else(|e| e.into_inner()).get(&index).cloned()
    }

    async fn fetch_block(&self, index: u64) -> io::Result<Option<Arc<Vec<u8>>>> {
        let url = self.url.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let (Some(url), Some(total)) = (url, self.progress().1) else { return Ok(None) };
        let start = index * BLOCK;
        let resp = match client()
            .get(&url)
            .header(RANGE, format!("bytes={start}-{}", (start + BLOCK).min(total) - 1))
            .timeout(Duration::from_secs(20))
            .send()
            .await
        {
            Ok(r) => r,
            // Serveur débordé : la suite viendra par le téléchargement.
            Err(e) => {
                log::info!("[podcasts] requête partielle sans réponse ({e}) : lecture au fil du téléchargement");
                return Ok(None);
            }
        };
        // Même fichier que le téléchargement : réponse partielle, et même taille totale.
        let same_file = resp.status() == StatusCode::PARTIAL_CONTENT
            && resp
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.rsplit('/').next())
                .and_then(|v| v.parse::<u64>().ok())
                == Some(total);
        if !same_file {
            log::info!("[podcasts] requêtes partielles inutilisables : lecture au fil du téléchargement");
            self.ranges.store(false, Ordering::Relaxed);
            return Ok(None);
        }
        let bytes = Arc::new(resp.bytes().await.map_err(io::Error::other)?.to_vec());
        let mut blocks = self.blocks.lock().unwrap_or_else(|e| e.into_inner());
        if blocks.len() >= MAX_BLOCKS {
            blocks.clear();
        }
        blocks.insert(index, bytes.clone());
        Ok(Some(bytes))
    }

    fn update(&self, f: impl FnOnce(&mut Progress)) {
        f(&mut self.progress.lock().unwrap_or_else(|e| e.into_inner()));
        self.changed.notify_all();
    }

    /// Attend `min_bytes` reçus, la fin ou une erreur.
    pub fn wait_for(&self, min_bytes: u64, timeout: Duration) -> Result<(), String> {
        let guard = self.progress.lock().unwrap_or_else(|e| e.into_inner());
        let (p, _) = self
            .changed
            .wait_timeout_while(guard, timeout, |p| p.written < min_bytes && !p.done && p.error.is_none())
            .unwrap_or_else(|e| e.into_inner());
        match &p.error {
            Some(e) => Err(e.clone()),
            None if p.written >= min_bytes || p.done => Ok(()),
            None => Err("téléchargement trop lent".into()),
        }
    }
}

fn active() -> &'static Mutex<HashMap<PathBuf, Arc<Download>>> {
    static ACTIVE: OnceLock<Mutex<HashMap<PathBuf, Arc<Download>>>> = OnceLock::new();
    ACTIVE.get_or_init(Default::default)
}

/// Le téléchargement en cours vers `path`, s'il y en a un.
pub fn get(path: &Path) -> Option<Arc<Download>> {
    active().lock().unwrap_or_else(|e| e.into_inner()).get(path).cloned()
}

/// Arrête le téléchargement en cours vers `path`, s'il y en a un.
pub fn cancel(path: &Path) {
    if let Some(dl) = get(path) {
        dl.cancelled.store(true, Ordering::Relaxed);
    }
}

/// Lance le téléchargement de `url` vers `path` (ou rend celui déjà en cours).
/// `on_done` reçoit le résultat une fois le fichier en place.
pub fn start(
    url: &str,
    path: PathBuf,
    duration_hint: Option<f64>,
    on_done: impl FnOnce(Result<PathBuf, String>) + Send + 'static,
) -> Arc<Download> {
    let mut map = active().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(d) = map.get(&path) {
        return d.clone();
    }
    let mut part = path.clone().into_os_string();
    part.push(".part");
    let dl = Arc::new(Download {
        path: path.clone(),
        part: PathBuf::from(part),
        url: Mutex::new(None),
        cancelled: AtomicBool::new(false),
        ranges: AtomicBool::new(true),
        blocks: Mutex::new(HashMap::new()),
        duration_hint,
        size_hint: AtomicU64::new(0),
        progress: Mutex::new(Progress::default()),
        changed: Condvar::new(),
    });
    map.insert(path.clone(), dl.clone());
    drop(map);

    let (task, url) = (dl.clone(), url.to_string());
    tauri::async_runtime::spawn(async move {
        let result = fetch_into(&task, &url).await;
        if let Err(e) = &result {
            if task.cancelled.load(Ordering::Relaxed) {
                log::debug!("[téléchargement] annulé : {url}");
            } else {
                log::warn!("[téléchargement] interrompu ({url}) : {e}");
            }
            let _ = std::fs::remove_file(&task.part);
            task.update(|p| p.error = Some(e.clone()));
        }
        active().lock().unwrap_or_else(|e| e.into_inner()).remove(&task.path);
        on_done(result.map(|_| task.path.clone()));
    });
    dl
}

async fn fetch_into(dl: &Arc<Download>, url: &str) -> Result<(), String> {
    if let Some(dir) = dl.part.parent() {
        tokio::fs::create_dir_all(dir).await.map_err(|e| e.to_string())?;
    }
    // Pas de délai global : un épisode de deux heures peut mettre longtemps à venir.
    let mut resp = client()
        .get(url)
        .timeout(Duration::from_secs(24 * 3600))
        .send()
        .await
        .map_err(|e| format!("fichier injoignable : {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("fichier refusé : HTTP {}", resp.status().as_u16()));
    }
    // Un serveur Subsonic répond aux erreurs en JSON ou XML, avec un statut 200.
    let kind = resp.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_ascii_lowercase();
    if kind.contains("json") || kind.contains("xml") || kind.starts_with("text/html") {
        let body = resp.text().await.unwrap_or_default();
        let message = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.pointer("/subsonic-response/error/message").and_then(|m| m.as_str()).map(str::to_string));
        return Err(message.unwrap_or_else(|| "le serveur n'a pas envoyé de fichier audio".into()));
    }
    let total = resp.content_length();
    dl.update(|p| p.total = total);
    *dl.url.lock().unwrap_or_else(|e| e.into_inner()) = Some(resp.url().to_string());
    // Fin du fichier lue dès l'analyse (ID3v1/APE) : demandée avant le gros téléchargement.
    if let Some(total) = total.filter(|&t| t > NEAR) {
        let _ = tokio::time::timeout(Duration::from_secs(3), dl.fetch_block((total - 1) / BLOCK)).await;
    }
    let mut file = tokio::fs::File::create(&dl.part).await.map_err(|e| e.to_string())?;
    loop {
        if dl.cancelled.load(Ordering::Relaxed) {
            return Err("téléchargement annulé".into());
        }
        let chunk = tokio::time::timeout(STALL_TIMEOUT, resp.chunk())
            .await
            .map_err(|_| "téléchargement figé".to_string())?
            .map_err(|e| format!("téléchargement interrompu : {e}"))?;
        let Some(chunk) = chunk else { break };
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        let n = chunk.len() as u64;
        dl.update(|p| p.written += n);
    }
    // Flux transcodé coupé net par le serveur : trop court pour le cache.
    let hint = dl.size_hint.load(Ordering::Relaxed);
    let written = dl.progress().0;
    if dl.progress().1.is_none() && hint > 0 && written < hint * 9 / 10 {
        return Err(format!("flux interrompu par le serveur ({} Ko sur ~{} Ko)", written / 1024, hint / 1024));
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    tokio::fs::rename(&dl.part, &dl.path).await.map_err(|e| e.to_string())?;
    dl.update(|p| {
        p.done = true;
        p.total = Some(p.written);
    });
    Ok(())
}

/// Source pour le décodeur : le fichier qui arrive, ou `None` s'il n'est pas en téléchargement.
pub fn reader(path: &Path) -> Option<io::Result<GrowingFile>> {
    let dl = get(path)?;
    Some(GrowingFile::open(dl))
}

/// Fichier lu pendant qu'il s'écrit : une lecture au-delà de ce qui est arrivé attend la suite.
pub struct GrowingFile {
    dl: Arc<Download>,
    file: File,
    pos: u64,
}

impl GrowingFile {
    fn open(dl: Arc<Download>) -> io::Result<Self> {
        // Fini entre-temps : le `.part` est devenu le fichier définitif.
        let file = File::open(&dl.part).or_else(|_| File::open(&dl.path))?;
        Ok(Self { dl, file, pos: 0 })
    }

    /// Octets lisibles à partir de `pos`, en attendant s'il le faut ; 0 à la fin réelle.
    fn available(&self) -> io::Result<u64> {
        let mut p = self.dl.progress.lock().unwrap_or_else(|e| e.into_inner());
        // Abandon après STALL_TIMEOUT sans aucun octet nouveau, pas sans atteindre `pos`.
        while p.written <= self.pos && !p.done && p.error.is_none() {
            let before = p.written;
            let (next, timeout) = self
                .dl
                .changed
                .wait_timeout_while(p, STALL_TIMEOUT, |p| p.written == before && !p.done && p.error.is_none())
                .unwrap_or_else(|e| e.into_inner());
            p = next;
            if timeout.timed_out() && p.written == before {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "épisode en attente de données"));
            }
        }
        if let Some(e) = &p.error {
            return Err(io::Error::other(e.clone()));
        }
        Ok(p.written.saturating_sub(self.pos))
    }
}

impl Read for GrowingFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        // Loin devant le téléchargement : ce morceau seul, sans attendre le reste.
        let (written, total, done) = self.dl.snapshot();
        if !done && self.pos >= written + NEAR {
            if total.is_some_and(|t| self.pos >= t) {
                return Ok(0);
            }
            // Taille seulement estimée : la fin n'est lue que pour ses tags, on ne l'attend pas.
            let hint = self.dl.size_hint.load(Ordering::Relaxed);
            if total.is_none() && hint > 0 && self.pos + TAIL >= hint {
                let n = buf.len().min(hint.saturating_sub(self.pos) as usize);
                buf[..n].fill(0);
                self.pos += n as u64;
                return Ok(n);
            }
            if let Some(block) = self.dl.block(self.pos / BLOCK)? {
                let off = (self.pos % BLOCK) as usize;
                if off >= block.len() {
                    return Ok(0);
                }
                let n = buf.len().min(block.len() - off);
                buf[..n].copy_from_slice(&block[off..off + n]);
                self.pos += n as u64;
                return Ok(n);
            }
        }
        let ready = self.available()?;
        if ready == 0 {
            return Ok(0);
        }
        let want = buf.len().min(ready.min(usize::MAX as u64) as usize);
        self.file.seek(SeekFrom::Start(self.pos))?;
        let n = self.file.read(&mut buf[..want])?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for GrowingFile {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let end = || self.dl.expected_len().unwrap_or_else(|| self.dl.progress().0);
        let pos = match to {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(d) => self.pos as i64 + d,
            SeekFrom::End(d) => end() as i64 + d,
        };
        if pos < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "position négative"));
        }
        self.pos = pos as u64;
        Ok(self.pos)
    }
}

impl MediaSource for GrowingFile {
    fn is_seekable(&self) -> bool {
        true
    }

    /// Taille annoncée : le décodeur en déduit la durée d'un MP3 sans index.
    fn byte_len(&self) -> Option<u64> {
        self.dl.expected_len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn fake(dir: &Path, total: Option<u64>) -> Arc<Download> {
        let path = dir.join("1.mp3");
        let dl = Arc::new(Download {
            path: path.clone(),
            part: dir.join("1.mp3.part"),
            url: Mutex::new(None),
            cancelled: AtomicBool::new(false),
            ranges: AtomicBool::new(false),
            blocks: Mutex::new(HashMap::new()),
            duration_hint: None,
            size_hint: AtomicU64::new(0),
            progress: Mutex::new(Progress { total, ..Default::default() }),
            changed: Condvar::new(),
        });
        File::create(&dl.part).unwrap();
        dl
    }

    fn append(dl: &Download, bytes: &[u8]) {
        let mut f = std::fs::OpenOptions::new().append(true).open(&dl.part).unwrap();
        f.write_all(bytes).unwrap();
        dl.update(|p| p.written += bytes.len() as u64);
    }

    #[test]
    fn reads_while_the_file_is_still_arriving() {
        let dir = tempfile_dir("arrive");
        let dl = fake(&dir, Some(6));
        append(&dl, b"abc");
        let mut r = GrowingFile::open(dl.clone()).unwrap();
        let writer = {
            let dl = dl.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                append(&dl, b"def");
                dl.update(|p| p.done = true);
            })
        };
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        writer.join().unwrap();
        assert_eq!(out, b"abcdef");
        assert_eq!(r.byte_len(), Some(6));
    }

    #[test]
    fn seeks_ahead_and_waits_for_the_bytes() {
        let dir = tempfile_dir("saut");
        let dl = fake(&dir, Some(8));
        append(&dl, b"0123");
        let mut r = GrowingFile::open(dl.clone()).unwrap();
        assert_eq!(r.seek(SeekFrom::End(-2)).unwrap(), 6);
        let dl2 = dl.clone();
        let w = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(80));
            append(&dl2, b"4567");
        });
        let mut two = [0u8; 2];
        r.read_exact(&mut two).unwrap();
        w.join().unwrap();
        assert_eq!(&two, b"67");
    }

    #[test]
    fn a_failed_download_ends_the_read_with_an_error() {
        let dir = tempfile_dir("echec");
        let dl = fake(&dir, None);
        let mut r = GrowingFile::open(dl.clone()).unwrap();
        dl.update(|p| p.error = Some("coupé".into()));
        let mut buf = [0u8; 4];
        assert!(r.read(&mut buf).is_err());
    }

    #[test]
    fn names_episodes_readably_and_picks_an_extension() {
        let p = episode_path("100% HONDELATTE", Some("2026-10-21 06:00:00"), "Les disparus : l'enquête/1", "https://cdn.ex.com/ep.M4A?t=1", None, None);
        assert_eq!(p.parent().unwrap().file_name().unwrap(), "100% HONDELATTE");
        assert_eq!(p.file_name().unwrap(), "2026-10-21 Les disparus - l'enquête - 1.m4a");
        assert!(is_episode_path(&p));
        assert!(!is_episode_path(Path::new("C:/Musique/a.mp3")));
        let twin = episode_path("A", None, "Titre", "https://x/y", Some("audio/mpeg"), Some("42"));
        assert_eq!(twin.file_name().unwrap(), "Titre (42).mp3");
        let long = episode_path(&"é".repeat(300), None, &"x".repeat(400), "https://x/y.mp3", None, None);
        assert!(long.parent().unwrap().file_name().unwrap().to_string_lossy().chars().count() <= MAX_FOLDER);
        assert!(long.file_stem().unwrap().to_string_lossy().chars().count() <= MAX_NAME);
    }

    /// Vrai épisode décodé pendant qu'il arrive, avec un saut loin devant :
    /// `cargo test --lib real_episode -- --ignored --nocapture`.
    #[test]
    #[ignore = "réseau"]
    fn real_episode_plays_while_downloading() {
        use symphonia::core::codecs::audio::AudioDecoderOptions;
        use symphonia::core::formats::probe::Hint;
        use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo, TrackType};
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::units::Time;

        let rt = tokio::runtime::Runtime::new().unwrap();
        let feed = rt.block_on(async {
            let url = std::env::var("FLUX").unwrap_or_else(|_| "https://feed.ausha.co/k11jwfkmvn27".into());
            match super::super::fetch::fetch(&url, None, None).await.unwrap() {
                super::super::fetch::Outcome::Fresh(f) => super::super::feed::parse(&f.body).unwrap(),
                _ => unreachable!(),
            }
        });
        let ep = &feed.episodes[0];
        let dir = tempfile_dir("reel");
        let path = dir.join(format!("1.{}", extension(&ep.audio_url, ep.audio_type.as_deref())));
        let _ = std::fs::remove_file(&path);
        let t0 = std::time::Instant::now();
        let dl = start(&ep.audio_url, path.clone(), ep.duration.map(|d| d as f64), |_| {});
        dl.wait_for(384 * 1024, Duration::from_secs(30)).unwrap();
        println!("prêt à lire en {:?} ({:?} octets annoncés)", t0.elapsed(), dl.progress().1);

        let source = reader(&path).expect("en téléchargement").unwrap();
        let mut hint = Hint::new();
        hint.with_extension(path.extension().unwrap().to_str().unwrap());
        let mss = MediaSourceStream::new(Box::new(source), Default::default());
        let mut format = symphonia::default::get_probe().probe(&hint, mss, FormatOptions::default(), MetadataOptions::default()).unwrap();
        println!("analyse finie à {:?} ({} Ko reçus sur {:?})", t0.elapsed(), dl.progress().0 / 1024, dl.progress().1.map(|t| t / 1024));
        let track = format.default_track(TrackType::Audio).unwrap().clone();
        let params = track.codec_params.as_ref().unwrap().audio().unwrap().clone();
        let mut decoder = symphonia::default::get_codecs().make_audio_decoder(&params, &AudioDecoderOptions::default()).unwrap();
        let rate = params.sample_rate.unwrap() as f64;
        let duration = track.num_frames.map(|n| n as f64 / rate);
        println!("durée du conteneur : {duration:?} s, du flux : {:?} s", ep.duration);

        let mut frames = 0usize;
        while frames < (rate as usize) * 5 {
            let packet = format.next_packet().unwrap().unwrap();
            frames += decoder.decode(&packet).unwrap().frames();
        }
        println!("5 s décodées à {:?}", t0.elapsed());

        let target = ep.duration.unwrap_or(600) as f64 * 0.6;
        format.seek(SeekMode::Coarse, SeekTo::Time { time: Time::try_from_secs_f64(target).unwrap(), track_id: Some(track.id) }).unwrap();
        decoder.reset();
        let packet = format.next_packet().unwrap().unwrap();
        decoder.decode(&packet).unwrap();
        println!("saut à {target:.0} s décodé à {:?} ({} Ko reçus)", t0.elapsed(), dl.progress().0 / 1024);
    }

    fn tempfile_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rustmusic-podcast-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }
}
