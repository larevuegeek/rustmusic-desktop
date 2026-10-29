//! Podcasts : abonnement, rafraîchissement des flux, vérification de fond.
//! Coupé dans les réglages, rien ne part sur le réseau.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};

use crate::core::podcast::chapters::{self, Chapter};
use crate::core::podcast::download;
use crate::core::podcast::fetch::{self, Outcome};
use crate::core::audio_analyser::audio_analyser::AudioAnalyser;
use crate::entity::audio::audio_file::{AudioFile, AudioFormat};
use crate::entity::audio::audio_tags::{AttachedImage, AudioTags};
use crate::helper::library::thumbnail_helper::cover_data_url;
use crate::repository::settings::settings_repository::SettingsRepository;
use crate::core::podcast::feed;
use crate::helper::library::{image_fetch, thumbnail_helper};
use crate::repository::podcast::podcast_repository::PodcastRepository;

/// Réglage « Podcasts » : désactivé par défaut.
pub const ENABLED_KEY: &str = "podcasts";
/// Vitesse de lecture des émissions qui n'en ont pas encore de propre.
pub const SPEED_KEY: &str = "podcast_speed";
/// Délai entre deux vérifications de fond de tous les flux.
/// Réglage de la vérification de fond : `never`, `1h`, `6h` (défaut), `24h`.
const REFRESH_KEY: &str = "podcast_refresh";

fn refresh_every(setting: Option<&str>) -> Option<Duration> {
    let hours = match setting.unwrap_or("6h") {
        "never" => return None,
        "1h" => 1,
        "24h" => 24,
        _ => 6,
    };
    Some(Duration::from_secs(hours * 3600))
}

static ENABLED: AtomicBool = AtomicBool::new(false);
/// Pour prévenir l'interface depuis les tâches de fond (téléchargement fini).
static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app(app: AppHandle) {
    let _ = APP.set(app);
}

fn notify(event: &str, payload: i64) {
    if let Some(app) = APP.get() {
        let _ = app.emit(event, payload);
    }
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn ensure_enabled() -> Result<(), String> {
    if enabled() { Ok(()) } else { Err("podcasts désactivés dans les réglages".into()) }
}

/// S'abonne au flux (ou retrouve l'abonnement existant) ; renvoie l'id du podcast.
pub async fn subscribe(pool: &SqlitePool, profil_id: i64, raw_url: &str) -> Result<i64, String> {
    ensure_enabled()?;
    let url = fetch::normalize_url(raw_url)?;
    if let Some(id) = PodcastRepository::find_id_by_url(pool, profil_id, &url).await.map_err(|e| e.to_string())? {
        return Ok(id);
    }
    let Outcome::Fresh(got) = fetch::fetch(&url, None, None).await? else {
        return Err("flux vide".into());
    };
    let parsed = feed::parse(&got.body)?;
    let id = PodcastRepository::insert(pool, profil_id, &url, &parsed, got.etag.as_deref(), got.last_modified.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    log::info!("🎙️  Abonné à « {} » ({} épisodes)", parsed.title, parsed.episodes.len());
    if let Some(image) = parsed.image_url.as_deref() {
        fetch_cover(pool, id, image).await;
    }
    Ok(id)
}

/// Bilan d'un import OPML.
#[derive(Debug, Default, serde::Serialize)]
pub struct OpmlImport {
    pub added: usize,
    pub existing: usize,
    pub failed: usize,
}

/// Abonne le profil aux flux d'un fichier OPML, quatre à la fois ; ceux déjà suivis sont comptés à part.
pub async fn import_opml(pool: &SqlitePool, profil_id: i64, xml: &str) -> Result<OpmlImport, String> {
    ensure_enabled()?;
    let mut summary = OpmlImport::default();
    let mut todo = Vec::new();
    for o in crate::core::podcast::opml::parse(xml) {
        let known = match fetch::normalize_url(&o.feed_url) {
            Ok(url) => PodcastRepository::find_id_by_url(pool, profil_id, &url).await.ok().flatten().is_some(),
            Err(_) => false,
        };
        if known {
            summary.existing += 1;
        } else {
            todo.push(o.feed_url);
        }
    }
    for chunk in todo.chunks(4) {
        let mut set = tokio::task::JoinSet::new();
        for url in chunk {
            let (pool, url) = (pool.clone(), url.clone());
            set.spawn(async move { (subscribe(&pool, profil_id, &url).await, url) });
        }
        while let Some(done) = set.join_next().await {
            match done {
                Ok((Ok(_), _)) => summary.added += 1,
                Ok((Err(e), url)) => {
                    log::warn!("[podcasts] OPML : {url} ignoré : {e}");
                    summary.failed += 1;
                }
                Err(_) => summary.failed += 1,
            }
        }
    }
    Ok(summary)
}

/// Les abonnements du profil en OPML.
pub async fn export_opml(pool: &SqlitePool, profil_id: i64) -> Result<(String, usize), String> {
    let items = PodcastRepository::opml_items(pool, profil_id).await.map_err(|e| e.to_string())?;
    Ok((crate::core::podcast::opml::build(&items), items.len()))
}

/// Aperçu d'un flux sans s'y abonner : rien n'est enregistré.
pub async fn preview(raw_url: &str) -> Result<crate::entity::podcast::podcast::PodcastPreview, String> {
    use crate::entity::podcast::podcast::{PodcastPreview, PreviewEpisode};
    ensure_enabled()?;
    let url = fetch::normalize_url(raw_url)?;
    let Outcome::Fresh(got) = fetch::fetch(&url, None, None).await? else {
        return Err("flux vide".into());
    };
    let f = feed::parse(&got.body)?;
    let mut episodes = f.episodes;
    // Le flux n'est pas toujours dans l'ordre : les plus récents d'abord.
    episodes.sort_by_key(|e| std::cmp::Reverse(e.published_at));
    Ok(PodcastPreview {
        episode_count: episodes.len(),
        episodes: episodes
            .into_iter()
            .take(8)
            .map(|e| PreviewEpisode {
                title: e.title,
                summary: e.summary,
                published_at: e.published_at.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()),
                duration: e.duration,
            })
            .collect(),
        title: f.title,
        author: f.author,
        description: f.description,
        image_url: f.image_url,
        link: f.link,
    })
}

/// Relit un flux ; renvoie le nombre d'épisodes parus depuis.
pub async fn refresh_one(pool: &SqlitePool, podcast_id: i64) -> Result<usize, String> {
    ensure_enabled()?;
    let state = PodcastRepository::feed_state(pool, podcast_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("podcast inconnu")?;
    match fetch::fetch(&state.feed_url, state.etag.as_deref(), state.last_modified.as_deref()).await? {
        Outcome::NotModified => {
            PodcastRepository::touch(pool, podcast_id).await.map_err(|e| e.to_string())?;
            Ok(0)
        }
        Outcome::Fresh(got) => {
            let parsed = feed::parse(&got.body)?;
            let added = PodcastRepository::update(pool, podcast_id, &parsed, got.etag.as_deref(), got.last_modified.as_deref())
                .await
                .map_err(|e| e.to_string())?;
            if added > 0 {
                auto_download(pool, podcast_id, added).await;
            }
            // Pochette absente ou changée par l'éditeur.
            if let Some(image) = parsed.image_url.as_deref() {
                if state.cover_path.is_none() || state.image_url.as_deref() != Some(image) {
                    fetch_cover(pool, podcast_id, image).await;
                }
            }
            Ok(added)
        }
    }
}

/// Podcast en téléchargement automatique : ses nouveaux épisodes (ceux parus à l'instant) se téléchargent.
async fn auto_download(pool: &SqlitePool, podcast_id: i64, added: usize) {
    let ids = PodcastRepository::auto_download_candidates(pool, podcast_id, added as i64).await.unwrap_or_default();
    for id in ids {
        match download_episode(pool, id).await {
            Ok(()) => log::info!("🎙️  Téléchargement automatique de l'épisode {id}"),
            Err(e) => log::warn!("🎙️  Téléchargement automatique impossible ({id}) : {e}"),
        }
    }
}

/// Délai avant d'effacer un épisode écouté : `never`, `now`, `24h` (défaut), `7d`, `30d`.
const CLEANUP_KEY: &str = "podcast_cleanup";

fn cleanup_hours(setting: Option<&str>) -> Option<i64> {
    match setting.unwrap_or("24h") {
        "never" => None,
        "now" => Some(0),
        "7d" => Some(7 * 24),
        "30d" => Some(30 * 24),
        _ => Some(24),
    }
}

/// Efface les épisodes téléchargés écoutés depuis le délai choisi ; rend les octets libérés.
pub async fn cleanup_played(pool: &SqlitePool) -> u64 {
    let setting = SettingsRepository::get(pool, CLEANUP_KEY).await.ok().flatten();
    let Some(hours) = cleanup_hours(setting.as_deref()) else { return 0 };
    let mut freed = 0;
    for id in PodcastRepository::played_downloads(pool, hours).await.unwrap_or_default() {
        // Un fichier encore ouvert (en lecture) refuse l'effacement : il partira au passage suivant.
        if let Ok(Some(bytes)) = delete_download(pool, id).await {
            freed += bytes;
        }
    }
    freed
}

/// Pochettes : `covers/podcasts/{full,1x,2x}/<id>.jpg`, rangées comme les portraits d'artistes.
fn covers_dir() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_default();
    dir.push("com.larevuegeek.rustmusic");
    dir.push("covers");
    dir.push("podcasts");
    dir
}

/// Télécharge et range la pochette ; un échec laisse simplement le podcast sans image.
async fn fetch_cover(pool: &SqlitePool, id: i64, url: &str) {
    let download = image_fetch::download(fetch::client(), url);
    let Ok(Some(bytes)) = tokio::time::timeout(Duration::from_secs(15), download).await else {
        log::warn!("[podcasts] pochette injoignable : {url}");
        return;
    };
    if image::guess_format(&bytes).is_err() {
        log::warn!("[podcasts] pochette illisible : {url}");
        return;
    }
    match thumbnail_helper::save_artist_image(&covers_dir(), &format!("{id}.jpg"), &bytes) {
        Ok(path) => {
            let _ = PodcastRepository::set_cover(pool, id, &path).await;
        }
        Err(e) => log::warn!("[podcasts] pochette non enregistrée : {e}"),
    }
}

/// Retire la pochette d'un podcast quitté, miniatures comprises.
pub fn remove_cover(id: i64) {
    for size in ["full", "1x", "2x"] {
        let _ = std::fs::remove_file(covers_dir().join(size).join(format!("{id}.jpg")));
    }
}

// =========================================
// LECTURE
// =========================================

/// Octets reçus avant de lancer le son : de quoi lire l'en-tête et quelques secondes.
const PREBUFFER: u64 = 384 * 1024;
/// Réglage « mode de lecture » : `stream` (pendant le téléchargement) ou `download` (après).
const PLAY_MODE_KEY: &str = "podcast_play_mode";

/// Rend l'épisode lisible à `path` : déjà téléchargé, ou téléchargement lancé et assez avancé.
/// `whole` : attendre le fichier complet (lecteur réseau, ou réglage « après téléchargement »).
pub async fn prepare(pool: &SqlitePool, path: &Path, whole: bool) -> Result<(), String> {
    if !download::is_episode_path(path) || path.exists() {
        return Ok(());
    }
    let id = episode_by_path(pool, path).await.ok_or("épisode inconnu")?;
    ensure_enabled()?;
    let whole = whole || SettingsRepository::get(pool, PLAY_MODE_KEY).await.ok().flatten().as_deref() == Some("download");
    let dl = start_download(pool, id, path).await?;
    let (min, wait) = if whole { (u64::MAX, Duration::from_secs(3600)) } else { (PREBUFFER, Duration::from_secs(30)) };
    tokio::task::spawn_blocking(move || dl.wait_for(min, wait)).await.map_err(|e| e.to_string())?
}

/// L'épisode lu à `path`, s'il s'agit d'un chemin du cache des podcasts.
pub async fn episode_by_path(pool: &SqlitePool, path: &Path) -> Option<i64> {
    if !download::is_episode_path(path) {
        return None;
    }
    PodcastRepository::find_by_path(pool, &path.to_string_lossy()).await.ok().flatten()
}

/// Lance (ou rejoint) le téléchargement de l'épisode `id` vers `path`.
async fn start_download(pool: &SqlitePool, id: i64, path: &Path) -> Result<std::sync::Arc<download::Download>, String> {
    if let Some(dl) = download::get(path) {
        return Ok(dl);
    }
    let ep = PodcastRepository::playable(pool, id).await.map_err(|e| e.to_string())?.ok_or("épisode inconnu")?;
    let pool = pool.clone();
    Ok(download::start(&ep.audio_url, path.to_path_buf(), ep.duration.map(|d| d as f64), move |result| {
        tauri::async_runtime::spawn(async move {
            if let Ok(p) = result {
                let _ = PodcastRepository::set_file(&pool, id, Some(&p.to_string_lossy())).await;
                notify("podcast-downloaded", id);
            }
        });
    }))
}

/// Télécharge un épisode sans l'écouter ; l'interface est prévenue à la fin (`podcast-downloaded`).
pub async fn download_episode(pool: &SqlitePool, id: i64) -> Result<(), String> {
    ensure_enabled()?;
    let ep = PodcastRepository::episode(pool, id).await.map_err(|e| e.to_string())?.ok_or("épisode inconnu")?;
    let path = PathBuf::from(&ep.path);
    if path.exists() {
        return Ok(());
    }
    start_download(pool, id, &path).await.map(|_| ())
}

/// Efface le fichier d'un épisode ; il reste dans la liste, prêt à être retéléchargé.
/// `None` : rien à effacer (pas de fichier, ou téléchargement en cours qu'on laisse finir).
pub async fn delete_download(pool: &SqlitePool, id: i64) -> Result<Option<u64>, String> {
    let ep = PodcastRepository::episode(pool, id).await.map_err(|e| e.to_string())?.ok_or("épisode inconnu")?;
    let Some(file) = ep.file_path else { return Ok(None) };
    if download::get(Path::new(&file)).is_some() {
        return Ok(None);
    }
    let size = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
    match std::fs::remove_file(&file) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("fichier non supprimé : {e}")),
    }
    if let Some(dir) = Path::new(&file).parent() {
        let _ = std::fs::remove_dir(dir);
    }
    PodcastRepository::set_file(pool, id, None).await.map_err(|e| e.to_string())?;
    Ok(Some(size))
}

/// Vide les téléchargements du profil (ou d'un podcast) ; `played_only` : les écoutés seulement.
/// Renvoie (fichiers effacés, octets libérés).
pub async fn delete_downloads(pool: &SqlitePool, profil_id: i64, podcast_id: Option<i64>, played_only: bool) -> Result<(usize, u64), String> {
    let list = PodcastRepository::downloads(pool, profil_id, podcast_id).await.map_err(|e| e.to_string())?;
    let (mut count, mut bytes) = (0, 0);
    for d in list.into_iter().filter(|d| !played_only || d.played) {
        if let Ok(Some(n)) = delete_download(pool, d.id).await {
            count += 1;
            bytes += n;
        }
    }
    Ok((count, bytes))
}

/// Épisodes téléchargés, avec leur taille sur le disque.
pub async fn downloads(pool: &SqlitePool, profil_id: i64) -> Result<Vec<crate::entity::podcast::podcast::DownloadView>, String> {
    let mut list = PodcastRepository::downloads(pool, profil_id, None).await.map_err(|e| e.to_string())?;
    for d in &mut list {
        d.size = std::fs::metadata(&d.file_path).map(|m| m.len()).unwrap_or(0);
    }
    Ok(list)
}

/// Fiche « morceau » d'un épisode pour le lecteur : titre, émission, durée, pochette.
pub async fn audio_file(pool: &SqlitePool, path: &Path) -> Option<AudioFile> {
    let id = episode_by_path(pool, path).await?;
    let ep = PodcastRepository::playable(pool, id).await.ok()??;
    // Téléchargé : format, débit et fréquence réels ; sinon le format d'après l'extension.
    let mut file = if path.exists() {
        AudioAnalyser::analyse_audio_file(&path.to_path_buf()).unwrap_or_else(|_| AudioFile::new(path.to_string_lossy().into_owned()))
    } else {
        AudioFile::new(path.to_string_lossy().into_owned())
    };
    if matches!(file.audio_format, AudioFormat::Unknown) {
        file.audio_format = format_of(path);
    }
    if file.duration <= 0.0 {
        file.duration = ep.duration.unwrap_or(0) as f32;
    }
    // Les étiquettes de l'épisode viennent du flux, pas du fichier.
    file.tags = AudioTags::new();
    file.tags.title = Some(ep.title);
    file.tags.artist = Some(ep.author.clone().unwrap_or_else(|| ep.podcast_title.clone()));
    file.tags.album = Some(ep.podcast_title);
    file.tags.genre = Some("Podcast".into());
    // Repère pour l'interface : ni cœur ni note sur un épisode.
    file.tags.media_type = Some("podcast".into());
    file.tags.duration = ep.duration.map(|d| d as u32);
    if let Some(src) = ep.cover_path.as_deref().and_then(|c| cover_data_url(Path::new(c))) {
        file.tags.attached_images.push(AttachedImage::cover(src));
    }
    Some(file)
}

fn format_of(path: &Path) -> AudioFormat {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("mp3") => AudioFormat::MP3,
        Some("m4a" | "aac" | "mp4") => AudioFormat::AAC,
        Some("ogg" | "oga" | "opus") => AudioFormat::OGG,
        Some("flac") => AudioFormat::FLAC,
        Some("wav") => AudioFormat::WAV,
        _ => AudioFormat::Unknown,
    }
}

/// Position d'écoute d'un épisode ; vers la fin, il est marqué écouté.
/// `duration` : celle du lecteur. Celle du flux peut mentir (publicités insérées à la volée).
pub async fn save_position(pool: &SqlitePool, path: &Path, position: f64, duration: Option<f64>) -> bool {
    if !download::is_episode_path(path) {
        return false;
    }
    let Some(id) = episode_by_path(pool, path).await else { return true };
    let _ = PodcastRepository::set_position(pool, id, position).await;
    if let Ok(Some(ep)) = PodcastRepository::playable(pool, id).await {
        let d = duration.filter(|d| *d > 0.0).or(ep.duration.map(|d| d as f64));
        if !ep.played && d.is_some_and(|d| d > 60.0 && position >= d - 30.0) {
            let _ = PodcastRepository::set_played(pool, id, true).await;
        }
    }
    true
}

/// Où reprendre un épisode (`None` : pas un épisode).
pub async fn resume_position(pool: &SqlitePool, path: &Path) -> Option<f64> {
    let id = episode_by_path(pool, path).await?;
    let ep = PodcastRepository::playable(pool, id).await.ok()??;
    Some(if ep.played { 0.0 } else { ep.position })
}

/// Chapitres d'un épisode : déjà connus, sinon fichier JSON du flux, sinon trames ID3 du MP3 (retenus ensuite).
pub async fn chapters(pool: &SqlitePool, path: &Path) -> Vec<Chapter> {
    let Some(id) = episode_by_path(pool, path).await else { return Vec::new() };
    let Ok(Some((known, url))) = PodcastRepository::chapters_of(pool, id).await else { return Vec::new() };
    if let Some(json) = known {
        return serde_json::from_str(&json).unwrap_or_default();
    }
    let mut found = match url {
        Some(url) => json_chapters(&url).await,
        None => None,
    };
    if found.as_ref().is_none_or(|c| c.is_empty()) {
        found = id3_chapters(path).await.or(found);
    }
    let Some(list) = found else { return Vec::new() };
    let _ = PodcastRepository::set_chapters(pool, id, &serde_json::to_string(&list).unwrap_or_else(|_| "[]".into())).await;
    list
}

async fn json_chapters(url: &str) -> Option<Vec<Chapter>> {
    let res = fetch::client().get(url).timeout(Duration::from_secs(15)).send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }
    Some(chapters::parse_json(&res.text().await.ok()?))
}

/// Tag ID3 en tête du fichier, pendant le téléchargement (attend les octets manquants) ou une fois en place.
async fn id3_chapters(path: &Path) -> Option<Vec<Chapter>> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || match download::reader(&path) {
        Some(reader) => chapters::from_reader(reader.ok()?),
        None => chapters::from_reader(std::fs::File::open(&path).ok()?),
    })
    .await
    .ok()
    .flatten()
}

/// Vitesse d'un épisode : celle de son émission, sinon celle des réglages ; `None` hors podcast.
pub async fn speed_for(pool: &SqlitePool, path: &Path) -> Option<f32> {
    let id = episode_by_path(pool, path).await?;
    let own = PodcastRepository::speed_of_episode(pool, id).await.ok().flatten().and_then(|(_, s)| s);
    let speed = match own {
        Some(s) => s as f32,
        None => SettingsRepository::get(pool, SPEED_KEY).await.ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(1.0),
    };
    Some(speed)
}

/// Nouvelle vitesse pour l'épisode en cours : appliquée tout de suite, retenue pour son émission.
pub async fn set_speed(pool: &SqlitePool, path: &Path, speed: f32) -> Result<(), String> {
    crate::core::audio_player::time_stretch::set_speed(speed);
    let Some(id) = episode_by_path(pool, path).await else { return Ok(()) };
    if let Some((podcast_id, _)) = PodcastRepository::speed_of_episode(pool, id).await.map_err(|e| e.to_string())? {
        let speed = crate::core::audio_player::time_stretch::speed();
        PodcastRepository::set_speed(pool, podcast_id, speed as f64).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Fichiers d'un podcast quitté : à lister avant de l'effacer de la base, à supprimer après.
pub async fn files_of(pool: &SqlitePool, podcast_id: i64) -> Vec<String> {
    PodcastRepository::files_of(pool, podcast_id).await.unwrap_or_default()
}

/// Téléchargements en cours arrêtés : sinon ils finiraient après coup, en orphelins.
pub fn remove_files(files: &[String]) {
    for f in files {
        download::cancel(Path::new(f));
        let _ = std::fs::remove_file(f);
        if let Some(dir) = Path::new(f).parent() {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

/// Relit tous les flux, l'un après l'autre ; un flux en panne n'arrête pas les autres.
pub async fn refresh_all(pool: &SqlitePool) -> usize {
    let Ok(states) = PodcastRepository::all_feed_states(pool).await else { return 0 };
    let mut added = 0;
    for s in states {
        if !enabled() {
            break;
        }
        match refresh_one(pool, s.id).await {
            Ok(n) => added += n,
            Err(e) => log::warn!("[podcasts] {} : {e}", s.feed_url),
        }
    }
    added
}

/// Vérification de fond : peu après l'activation, puis selon le réglage (toutes les 6 h par défaut, ou jamais).
/// L'interface est prévenue par `podcasts-updated` quand des épisodes paraissent.
pub fn start_background(app: AppHandle, pool: SqlitePool) {
    tauri::async_runtime::spawn(async move {
        let mut last: Option<Instant> = None;
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            if !enabled() {
                continue;
            }
            let freed = cleanup_played(&pool).await;
            if freed > 0 {
                log::info!("🎙️  Ménage des épisodes écoutés : {} Mo libérés", freed / 1_000_000);
                let _ = app.emit("podcast-downloaded", 0);
            }
            let setting = SettingsRepository::get(&pool, REFRESH_KEY).await.ok().flatten();
            let Some(every) = refresh_every(setting.as_deref()) else { continue };
            if last.is_some_and(|t| t.elapsed() < every) {
                continue;
            }
            last = Some(Instant::now());
            let added = refresh_all(&pool).await;
            if added > 0 {
                log::info!("🎙️  {added} nouvel(s) épisode(s)");
                let _ = app.emit("podcasts-updated", added);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn refresh_delays() {
        assert_eq!(refresh_every(None), Some(Duration::from_secs(6 * 3600)));
        assert_eq!(refresh_every(Some("never")), None);
        assert_eq!(refresh_every(Some("1h")), Some(Duration::from_secs(3600)));
        assert_eq!(refresh_every(Some("24h")), Some(Duration::from_secs(24 * 3600)));
    }

    #[test]
    fn cleanup_delays() {
        assert_eq!(cleanup_hours(None), Some(24));
        assert_eq!(cleanup_hours(Some("never")), None);
        assert_eq!(cleanup_hours(Some("now")), Some(0));
        assert_eq!(cleanup_hours(Some("7d")), Some(168));
    }

    #[tokio::test]
    async fn only_old_enough_played_downloads_are_picked() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO podcasts (id, profil_id, feed_url, title, auto_download) VALUES (1, 1, 'http://f', 'P', 1)").execute(&pool).await.unwrap();
        // 1 : écouté il y a 2 jours ; 2 : écouté à l'instant ; 3 : pas écouté ; 4 : écouté mais pas téléchargé ; 5 : nouveau.
        for (id, played, ago, file, new) in [(1, 1, "-48 hours", true, 0), (2, 1, "0 hours", true, 0), (3, 0, "", true, 0), (4, 1, "-48 hours", false, 0), (5, 0, "", false, 1)] {
            sqlx::query(
                "INSERT INTO podcast_episodes (id, podcast_id, guid, title, audio_url, played, played_at, file_path, is_new, published_at)
                 VALUES (?1, 1, ?1, 't', 'http://a', ?2, CASE WHEN ?2 THEN datetime('now', ?3) END, ?4, ?5, datetime('now', '-' || ?1 || ' days'))",
            )
            .bind(id).bind(played).bind(ago).bind(file.then_some("x.mp3")).bind(new)
            .execute(&pool).await.unwrap();
        }
        assert_eq!(PodcastRepository::played_downloads(&pool, 24).await.unwrap(), vec![1]);
        let mut now = PodcastRepository::played_downloads(&pool, 0).await.unwrap();
        now.sort();
        assert_eq!(now, vec![1, 2]);
        assert_eq!(PodcastRepository::auto_download_candidates(&pool, 1, 3).await.unwrap(), vec![5]);
    }
}
