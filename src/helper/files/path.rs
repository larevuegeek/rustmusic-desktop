//! Découpage d'un chemin de fichier, Windows ou Unix.

/// Chemin de fichier → (dossier avec séparateur final, nom du fichier).
pub fn split_path(path: &str) -> (String, String) {
    match path.rfind(['/', '\\']) {
        Some(i) => (path[..=i].to_string(), path[i + 1..].to_string()),
        None => (String::new(), path.to_string()),
    }
}
