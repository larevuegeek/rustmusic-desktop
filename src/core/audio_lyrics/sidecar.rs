use std::path::Path;

/// Cherche un fichier `.lrc` à côté du fichier audio.
/// Exemple : `Nirvana - Smells Like Teen Spirit.flac` → cherche
/// `Nirvana - Smells Like Teen Spirit.lrc` dans le même dossier.
///
/// Retourne le contenu brut du LRC (toutes lignes, timestamps inclus).
/// `None` si pas de fichier sidecar trouvé ou s'il est vide.
pub fn read_sidecar_lrc(audio_path: &str) -> Option<String> {
    let path = Path::new(audio_path);
    let lrc_path = path.with_extension("lrc");

    if !lrc_path.exists() {
        return None;
    }

    match std::fs::read_to_string(&lrc_path) {
        Ok(content) if !content.trim().is_empty() => Some(content),
        _ => None,
    }
}

/// Heuristique : un contenu LRC contient au moins une ligne `[mm:ss.cs]…`
/// Sinon c'est probablement un .lrc "fake" qui ne contient que du texte brut.
pub fn has_synced_timestamps(content: &str) -> bool {
    content.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with('[')
            && trimmed
                .chars()
                .nth(1)
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
    })
}
