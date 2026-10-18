//! Partage réseau remonté sous un autre nom. GVFS met le serveur tel qu'il a été
//! saisi dans le chemin : le même NAS monté par son adresse (`server=10.0.0.170`)
//! puis par son nom (`server=nas.local`) donne deux dossiers différents, et une
//! bibliothèque importée sous l'un ne trouve plus rien sous l'autre.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Même partage, monté ailleurs : les chemins sous `old_root` sont sous `new_root`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Remount {
    pub old_root: String,
    pub new_root: String,
}

/// Valeur de `share=` dans le nom d'un montage (`server=…,share=music,user=…`).
fn share_of(params: &str) -> Option<String> {
    params.split(',').find_map(|kv| kv.strip_prefix("share=")).map(str::to_ascii_lowercase)
}

/// Racine du montage GVFS SMB d'un chemin (`…/gvfs/smb-share:…`), son partage et le
/// reste du chemin ; `None` hors d'un partage GVFS.
fn split_smb(path: &Path) -> Option<(PathBuf, String, PathBuf)> {
    let mut racine = PathBuf::new();
    let mut composants = path.components();
    while let Some(c) = composants.next() {
        racine.push(c);
        let Some(params) = c.as_os_str().to_str().and_then(|n| n.strip_prefix("smb-share:")) else {
            continue;
        };
        if racine.parent()?.file_name()? != OsStr::new("gvfs") {
            return None;
        }
        return Some((racine, share_of(params)?, composants.as_path().to_path_buf()));
    }
    None
}

/// Autre montage du même partage où `path` existe. `None` si le chemin est
/// accessible, hors d'un partage GVFS, ou retrouvé nulle part.
pub fn find_remount(path: &Path) -> Option<Remount> {
    if path.exists() {
        return None;
    }
    let (racine, share, reste) = split_smb(path)?;
    let mut candidats: Vec<PathBuf> = std::fs::read_dir(racine.parent()?)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| *p != racine)
        .filter(|p| {
            p.file_name()
                .and_then(OsStr::to_str)
                .and_then(|n| n.strip_prefix("smb-share:"))
                .and_then(share_of)
                .is_some_and(|s| s == share)
        })
        .collect();
    candidats.sort();
    let nouvelle = candidats.into_iter().find(|c| c.join(&reste).exists())?;
    Some(Remount {
        old_root: racine.to_string_lossy().into_owned(),
        new_root: nouvelle.to_string_lossy().into_owned(),
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn gvfs(nom: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rustmusic-remount-{nom}-{}", std::process::id())).join("gvfs");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_the_share_mounted_by_name_instead_of_address() {
        let g = gvfs("nom");
        let nouveau = g.join("smb-share:server=nas.local,share=music");
        std::fs::create_dir_all(nouveau.join("Mp3 - 320 kbits/BO")).unwrap();
        let ancien = g.join("smb-share:server=10.0.0.170,share=music");

        let trouve = find_remount(&ancien.join("Mp3 - 320 kbits"));
        assert_eq!(
            trouve,
            Some(Remount {
                old_root: ancien.to_string_lossy().into_owned(),
                new_root: nouveau.to_string_lossy().into_owned(),
            })
        );
        let _ = std::fs::remove_dir_all(g.parent().unwrap());
    }

    #[test]
    fn ignores_another_share_and_a_missing_subfolder() {
        let g = gvfs("autre");
        std::fs::create_dir_all(g.join("smb-share:server=nas.local,share=photos/Mp3")).unwrap();
        std::fs::create_dir_all(g.join("smb-share:server=nas.local,share=music/Flac")).unwrap();
        let ancien = g.join("smb-share:server=10.0.0.170,share=music/Mp3");
        assert_eq!(find_remount(&ancien), None);
        let _ = std::fs::remove_dir_all(g.parent().unwrap());
    }

    #[test]
    fn leaves_local_and_reachable_paths_alone() {
        assert_eq!(find_remount(Path::new("/chemin/qui/n/existe/pas")), None);
        assert_eq!(find_remount(&std::env::temp_dir()), None);
    }
}
