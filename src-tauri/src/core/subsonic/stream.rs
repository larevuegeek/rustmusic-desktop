//! Lecture d'un morceau distant : téléchargé dans un cache pendant qu'on l'écoute, comme un épisode
//! de podcast. Le moteur garde le chemin `remote://` ; seule l'ouverture passe par le cache.

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use symphonia::core::io::MediaSource;

use crate::core::podcast::download;

/// Taille au-delà de laquelle les morceaux les plus anciennement écoutés quittent le cache.
const CACHE_MAX: u64 = 2 * 1024 * 1024 * 1024;

pub fn is_remote(path: &Path) -> bool {
    path.to_str().is_some_and(|p| p.starts_with(crate::core::remote::SCHEME))
}

/// Dossier du cache des morceaux distants.
pub fn cache_dir() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_default();
    dir.push("com.larevuegeek.rustmusic");
    dir.push("servers-cache");
    dir
}

/// Dossier des morceaux gardés hors ligne : jamais vidé par la limite du cache.
pub fn offline_dir() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_default();
    dir.push("com.larevuegeek.rustmusic");
    dir.push("servers-offline");
    dir
}

/// Chemin distant → fichier du cache, rempli par la préparation.
fn prepared() -> &'static Mutex<HashMap<PathBuf, PathBuf>> {
    static PREPARED: OnceLock<Mutex<HashMap<PathBuf, PathBuf>>> = OnceLock::new();
    PREPARED.get_or_init(Default::default)
}

/// Derniers fichiers préparés (en cours, suivant) : jamais retirés du cache.
fn recent() -> &'static Mutex<VecDeque<PathBuf>> {
    static RECENT: OnceLock<Mutex<VecDeque<PathBuf>>> = OnceLock::new();
    RECENT.get_or_init(Default::default)
}

pub fn register(path: &Path, file: PathBuf) {
    let mut recent = recent().lock().unwrap_or_else(|e| e.into_inner());
    recent.retain(|f| f != &file);
    recent.push_back(file.clone());
    if recent.len() > 3 {
        recent.pop_front();
    }
    prepared().lock().unwrap_or_else(|e| e.into_inner()).insert(path.to_path_buf(), file);
}

/// Morceau lancé par l'utilisateur : les autres téléchargements du cache s'arrêtent et libèrent le débit.
pub fn play_only(path: &Path) {
    let cache = cache_dir();
    for (logical, f) in prepared().lock().unwrap_or_else(|e| e.into_inner()).iter() {
        if logical != path && f.starts_with(&cache) && download::get(f).is_some() {
            log::debug!("☁️  Téléchargement abandonné : {}", f.display());
            download::cancel(f);
        }
    }
}

/// Le fichier du cache derrière un chemin distant déjà préparé.
pub fn cached(path: &Path) -> Option<PathBuf> {
    prepared().lock().unwrap_or_else(|e| e.into_inner()).get(path).cloned()
}

/// Source pour le décodeur et extension de son fichier.
pub type Opened = (Box<dyn MediaSource>, Option<String>);

/// La source d'un morceau distant ; `None` pour un chemin local.
pub fn open(path: &Path) -> Option<io::Result<Opened>> {
    if !is_remote(path) {
        return None;
    }
    let Some(file) = cached(path) else {
        return Some(Err(io::Error::new(io::ErrorKind::NotFound, "morceau distant non préparé")));
    };
    let ext = file.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
    let source: io::Result<Box<dyn MediaSource>> = match download::reader(&file) {
        Some(growing) => growing.map(|g| Box::new(g) as Box<dyn MediaSource>),
        None => File::open(&file).map(|f| Box::new(f) as Box<dyn MediaSource>),
    };
    Some(source.map(|s| (s, ext)))
}

/// Serveur retiré : ses téléchargements s'arrêtent, son cache est effacé.
pub fn forget_server(server_id: i64) {
    let dir = cache_dir().join(server_id.to_string());
    let _ = std::fs::remove_dir_all(offline_dir().join(server_id.to_string()));
    let gone: Vec<PathBuf> = {
        let mut map = prepared().lock().unwrap_or_else(|e| e.into_inner());
        let files = map.values().filter(|f| f.starts_with(&dir)).cloned().collect();
        map.retain(|_, f| !f.starts_with(&dir));
        files
    };
    for file in &gone {
        download::cancel(file);
    }
    recent().lock().unwrap_or_else(|e| e.into_inner()).retain(|f| !f.starts_with(&dir));
    // Un téléchargement interrompu tient son `.part` ouvert un instant (Windows refuse alors d'effacer).
    for attempt in 0..10 {
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => return,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(e) if attempt == 9 => log::warn!("☁️  Cache du serveur {server_id} non effacé : {e}"),
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(200)),
        }
    }
}

/// Écouté à l'instant : remonte en tête du cache.
pub fn touch(file: &Path) {
    if let Ok(f) = File::options().write(true).open(file) {
        let _ = f.set_modified(std::time::SystemTime::now());
    }
}

/// Retire les morceaux les plus anciens au-delà de [`CACHE_MAX`] (jamais ceux en cours).
pub fn prune() {
    prune_dir(&cache_dir(), CACHE_MAX);
}

fn prune_dir(dir: &Path, max: u64) {
    let in_use: Vec<PathBuf> = recent().lock().unwrap_or_else(|e| e.into_inner()).iter().cloned().collect();
    let mut files: Vec<(std::time::SystemTime, u64, PathBuf)> = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "part") {
                // Téléchargement interrompu (appli fermée en cours) : plus personne ne le finira.
                if download::get(&path.with_extension("")).is_none() {
                    let _ = std::fs::remove_file(&path);
                }
            } else {
                files.push((meta.modified().unwrap_or(std::time::UNIX_EPOCH), meta.len(), path));
            }
        }
    }
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    files.sort_by_key(|f| f.0);
    for (_, size, path) in files {
        if total <= max {
            break;
        }
        if download::get(&path).is_some() || in_use.contains(&path) {
            continue;
        }
        if std::fs::remove_file(&path).is_ok() {
            total -= size;
            prepared().lock().unwrap_or_else(|e| e.into_inner()).retain(|_, f| f != &path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_drops_the_oldest_first() {
        let dir = std::env::temp_dir().join("rustmusic-stream-prune");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("1")).unwrap();
        let now = std::time::SystemTime::now();
        for (i, name) in ["old.mp3", "mid.mp3", "new.mp3"].iter().enumerate() {
            let p = dir.join("1").join(name);
            std::fs::write(&p, vec![0u8; 100]).unwrap();
            let f = File::options().write(true).open(&p).unwrap();
            f.set_modified(now - std::time::Duration::from_secs(100 - i as u64 * 10)).unwrap();
        }
        std::fs::write(dir.join("1").join("x.mp3.part"), vec![0u8; 500]).unwrap();
        prune_dir(&dir, 200);
        let mut left: Vec<String> = std::fs::read_dir(dir.join("1")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        // Le `.part` sans téléchargement en cours est un reste d'interruption : effacé.
        assert_eq!(left, vec!["mid.mp3", "new.mp3"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn forgetting_a_server_clears_its_cache() {
        let dir = cache_dir().join("987654");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("s1.mp3");
        std::fs::write(&file, b"x").unwrap();
        register(Path::new("remote://987654/al/s1"), file);
        forget_server(987654);
        assert!(!dir.exists());
        assert!(cached(Path::new("remote://987654/al/s1")).is_none());
    }

    #[test]
    fn local_paths_are_not_remote() {
        assert!(is_remote(Path::new("remote://1/al/s1")));
        assert!(!is_remote(Path::new(r"C:\Musique\a.flac")));
        assert!(open(Path::new(r"C:\Musique\a.flac")).is_none());
    }
}
