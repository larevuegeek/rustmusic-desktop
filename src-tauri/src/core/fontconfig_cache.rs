//! Caches fontconfig d'une autre version que celle du système (Linux).
//!
//! Une fontconfig plus récente (application en conteneur) pose des liens
//! `…cache-9 → …cache-12`. Celle du système suit le lien et accepte ce format
//! plus récent : des polices perdent leur nom et le processus web de WebKitGTK
//! boucle avant d'afficher quoi que ce soit. On retire, avant le démarrage de
//! WebKit, tout cache dont l'en-tête ne correspond pas à la version de son nom.

use std::io::Read;
use std::path::Path;

/// `FC_CACHE_MAGIC_MMAP` (fcint.h), suivi de la version du format.
const CACHE_MAGIC: u32 = 0xFC02_FC04;

pub fn remove_mismatched_caches() {
    let dirs = [
        dirs::cache_dir().map(|d| d.join("fontconfig")),
        dirs::home_dir().map(|d| d.join(".fontconfig")),
    ];
    for dir in dirs.into_iter().flatten() {
        let removed = remove_mismatched_caches_in(&dir);
        if removed > 0 {
            log::warn!(
                "🔤 Cache fontconfig : {} cache(s) d'une autre version retiré(s) de {}",
                removed,
                dir.display()
            );
        }
    }
}

fn remove_mismatched_caches_in(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
    let mut removed = 0;
    for entry in entries.flatten() {
        let Some(expected) = name_version(&entry.file_name().to_string_lossy()) else { continue };
        // `File::open` suit les liens : c'est le contenu que fontconfig lira. Un lien est retiré, pas sa cible.
        let Some(found) = header_version(&entry.path()) else { continue };
        if found != expected && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// `…-le64.cache-9` → `Some(9)`.
fn name_version(name: &str) -> Option<u32> {
    let (_, version) = name.rsplit_once(".cache-")?;
    if version.is_empty() || !version.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    version.parse().ok()
}

fn header_version(path: &Path) -> Option<u32> {
    let mut header = [0u8; 8];
    std::fs::File::open(path).ok()?.read_exact(&mut header).ok()?;
    let [m0, m1, m2, m3, v0, v1, v2, v3] = header;
    (u32::from_ne_bytes([m0, m1, m2, m3]) == CACHE_MAGIC).then(|| u32::from_ne_bytes([v0, v1, v2, v3]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn cache(version: u32) -> Vec<u8> {
        let mut bytes = CACHE_MAGIC.to_ne_bytes().to_vec();
        bytes.extend(version.to_ne_bytes());
        bytes.extend([0u8; 32]);
        bytes
    }

    #[test]
    fn version_from_name() {
        assert_eq!(name_version("3bd6a00a1059ca9725da3f0d7afa935d-le64.cache-9"), Some(9));
        assert_eq!(name_version("3bd6a00a1059ca9725da3f0d7afa935d-le32d4.cache-12"), Some(12));
        assert_eq!(name_version("da43223dd54fb3bb4243ae19d4b583b2-le64.cache-reindex1-10"), None);
        assert_eq!(name_version("CACHEDIR.TAG"), None);
    }

    #[test]
    fn only_mismatched_caches_are_removed() {
        let dir = std::env::temp_dir().join(format!("rustmusic-fc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a-le64.cache-12"), cache(12)).unwrap();
        std::fs::write(dir.join("b-le64.cache-9"), cache(9)).unwrap();
        std::fs::write(dir.join("d-le64.cache-9"), cache(12)).unwrap();
        std::fs::write(dir.join("e-le64.cache-9"), b"pas un cache").unwrap();
        symlink("a-le64.cache-12", dir.join("a-le64.cache-9")).unwrap();
        symlink("a-le64.cache-12", dir.join("a-le64.cache-11")).unwrap();
        symlink("b-le64.cache-9", dir.join("c-le64.cache-9")).unwrap();
        symlink("absent-le64.cache-12", dir.join("f-le64.cache-9")).unwrap();

        assert_eq!(remove_mismatched_caches_in(&dir), 3);
        assert!(std::fs::symlink_metadata(dir.join("a-le64.cache-9")).is_err());
        assert!(std::fs::symlink_metadata(dir.join("a-le64.cache-11")).is_err());
        assert!(!dir.join("d-le64.cache-9").exists());
        assert!(dir.join("a-le64.cache-12").exists());
        assert!(dir.join("b-le64.cache-9").exists());
        assert!(dir.join("c-le64.cache-9").exists());
        assert!(dir.join("e-le64.cache-9").exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
