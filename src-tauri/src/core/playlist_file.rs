//! Listes .m3u / .m3u8 : lecture des chemins (relatifs, absolus ou `file://`) et écriture étendue (#EXTINF).

use std::path::{Path, PathBuf};

/// Un morceau à écrire : chemin, durée en secondes, « Artiste - Titre ».
pub struct M3uEntry {
    pub path: String,
    pub duration: Option<f64>,
    pub label: String,
}

/// Chemins des morceaux d'une liste, dans l'ordre ; les relatifs sont résolus depuis `base` (son dossier).
/// Les adresses web (radios) sont ignorées.
pub fn parse(text: &str, base: &Path) -> Vec<PathBuf> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            if let Some(rest) = l.strip_prefix("file://") {
                // file:///C:/Musique/a%20b.flac ou file:///home/x/a.flac
                let decoded = urlencoding::decode(rest).map(|s| s.into_owned()).unwrap_or_else(|_| rest.to_string());
                let p = decoded.strip_prefix('/').filter(|s| s.as_bytes().get(1) == Some(&b':')).map(str::to_string).unwrap_or(decoded);
                return Some(PathBuf::from(p));
            }
            if l.contains("://") {
                return None;
            }
            let p = PathBuf::from(l.replace(['\\', '/'], std::path::MAIN_SEPARATOR_STR));
            Some(if p.is_absolute() || l.as_bytes().get(1) == Some(&b':') { p } else { base.join(p) })
        })
        .collect()
}

/// Liste étendue en UTF-8, chemins absolus.
pub fn build(entries: &[M3uEntry]) -> String {
    let mut out = String::from("#EXTM3U\n");
    for e in entries {
        let secs = e.duration.map(|d| d.round() as i64).unwrap_or(-1);
        out.push_str(&format!("#EXTINF:{secs},{}\n{}\n", e.label.replace(['\r', '\n'], " "), e.path));
    }
    out
}

/// Chemin de `target` vu depuis le dossier `from`, avec des `/` (lus partout : Android, clés USB, Windows).
/// `None` si les deux ne partagent pas de racine (autre lecteur) : il faut alors le chemin complet.
/// Lettre de lecteur et casse ignorées sous Windows.
pub fn relative_path(from: &str, target: &str) -> Option<String> {
    fn parts(p: &str) -> Vec<String> {
        p.split(['\\', '/']).filter(|s| !s.is_empty() && *s != ".").map(str::to_string).collect()
    }
    // Éléments qui font la racine : « C: » (lecteur), « serveur/partage » (UNC), rien sous Linux/macOS (« / »).
    let root = |p: &str| if p.as_bytes().get(1) == Some(&b':') { 1 } else if p.starts_with(r"\\") { 2 } else { 0 };
    let windows = root(from) > 0;
    let same = |a: &str, b: &str| if windows { a.eq_ignore_ascii_case(b) } else { a == b };
    let (base, dest) = (parts(from), parts(target));
    // Racines différentes (C: et S:, autre partage…) : pas de chemin relatif possible.
    let n = root(from);
    if n != root(target) || base.len() < n || dest.len() < n || (0..n).any(|i| !same(&base[i], &dest[i])) {
        return None;
    }
    let common = base.iter().zip(&dest).take_while(|(a, b)| same(a, b)).count();
    let mut out: Vec<&str> = vec![".."; base.len() - common];
    out.extend(dest[common..].iter().map(String::as_str));
    Some(out.join("/"))
}

/// Les deux derniers éléments d'un chemin (dossier/fichier), sans la casse : reconnaît un morceau
/// d'une liste écrite sur un autre ordinateur (autre lecteur, autre dossier racine).
pub fn tail_key(path: &str) -> String {
    let parts: Vec<&str> = path.rsplit(['\\', '/']).filter(|s| !s.is_empty()).take(2).collect();
    parts.into_iter().rev().collect::<Vec<_>>().join("/").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_kind_of_line() {
        let base = Path::new("/home/david/Musique/Listes");
        let text = "\u{feff}#EXTM3U\n#EXTINF:120,A - B\n../Queen/01.flac\n\n/abs/02.mp3\nfile:///mnt/m/03%20x.flac\nhttps://radio.example/stream.mp3\n";
        let got = parse(text, base);
        assert_eq!(got.len(), 3);
        assert!(got[0].ends_with("01.flac") && got[0].starts_with("/home/david/Musique/Listes"));
        assert_eq!(got[2], PathBuf::from("/mnt/m/03 x.flac"));
    }

    #[test]
    fn windows_paths_and_file_urls() {
        let got = parse("C:\\Musique\\a.flac\nfile:///D:/Musique/b%C3%A9.flac\n", Path::new("E:\\Listes"));
        assert_eq!(got[0], PathBuf::from("C:\\Musique\\a.flac".replace('\\', std::path::MAIN_SEPARATOR_STR)));
        assert_eq!(got[1], PathBuf::from("D:/Musique/bé.flac"));
    }

    #[test]
    fn writes_extended_m3u() {
        let s = build(&[M3uEntry { path: "/m/a.flac".into(), duration: Some(201.6), label: "Queen - Bohemian\nRhapsody".into() }]);
        assert_eq!(s, "#EXTM3U\n#EXTINF:202,Queen - Bohemian Rhapsody\n/m/a.flac\n");
    }

    #[test]
    fn relative_paths_from_the_playlist_folder() {
        // Liste à la racine de la musique, liste dans un sous-dossier, même dossier.
        assert_eq!(relative_path(r"S:\Musique", r"S:\Musique\Muse\01.flac").as_deref(), Some("Muse/01.flac"));
        assert_eq!(relative_path(r"S:\Musique\Playlists", r"s:\musique\Muse\01.flac").as_deref(), Some("../Muse/01.flac"));
        assert_eq!(relative_path(r"/mnt/m/Listes", r"/mnt/m/Listes/a.flac").as_deref(), Some("a.flac"));
        assert_eq!(relative_path(r"/home/d/Listes", r"/mnt/nas/a.flac").as_deref(), Some("../../../mnt/nas/a.flac"));
        // Autre lecteur, ou casse différente sous Linux : impossible / distinct.
        assert_eq!(relative_path(r"C:\Listes", r"S:\Musique\a.flac"), None);
        assert_eq!(relative_path(r"/home/d", r"/Home/d/a.flac").as_deref(), Some("../../Home/d/a.flac"));
        assert_eq!(relative_path(r"\\nas\music\Listes", r"\\NAS\Music\Muse\a.flac").as_deref(), Some("../Muse/a.flac"));
        assert_eq!(relative_path(r"\\nas\music", r"\\nas\films\a.mkv"), None);
    }

    #[test]
    fn tail_keys_match_across_machines() {
        assert_eq!(tail_key("C:\\Musique\\Queen\\01.flac"), tail_key("/mnt/nas/music/queen/01.FLAC"));
    }
}
