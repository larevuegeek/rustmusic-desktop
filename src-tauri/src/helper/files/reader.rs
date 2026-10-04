use std::{fs::{self}, path::{Path, PathBuf}};

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "aiff", "aif",
    "ogg", "opus", "m4a", "aac",
    "dsf", "dff",
];

// Insensible à la casse : « .FLAC » ou « .Mp3 » (vieux rips) étaient ignorés.
fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

pub fn read_dir_deep(
    directory: &str,
    files: &mut Vec<PathBuf>
) {
    read_dir_deep_suivi(directory, files, &mut |_| true);
}

/// `suivi` reçoit le nombre de fichiers trouvés ; faux arrête le parcours.
pub fn read_dir_deep_suivi(
    directory: &str,
    files: &mut Vec<PathBuf>,
    suivi: &mut dyn FnMut(usize) -> bool,
) -> bool {
    let path_directory: PathBuf = PathBuf::from(&directory);
    let entries = match fs::read_dir(&path_directory) {
        Ok(e) => e,
        Err(e) => {
            log::error!("Impossible de lire le dossier {:?}: {}", path_directory, e);
            return true;
        }
    };

    for entry in entries.flatten() {
        let path: PathBuf = entry.path();

        if path.is_dir() {
            if let Some(p) = path.to_str() {
                if !read_dir_deep_suivi(p, files, suivi) {
                    return false;
                }
            }
        } else if is_audio(&path) {
            files.push(path);
            if !suivi(files.len()) {
                return false;
            }
        }
    }

    suivi(files.len())
}
