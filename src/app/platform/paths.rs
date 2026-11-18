//! Dossiers de l'application, à l'identique de ceux que Tauri résolvait
//! (`BaseDirectory::AppData`, `app_config_dir`) : `%APPDATA%\com.larevuegeek.rustmusic`
//! sous Windows, `~/Library/Application Support/…` sous macOS, `~/.local/share/…` et
//! `~/.config/…` sous Linux.

use std::path::PathBuf;

/// Le dossier parent des données (`%APPDATA%`…), ou `RUSTMUSIC_DATA_DIR` s'il est posé :
/// un clone de travail teste sur une copie sans toucher aux vraies données.
pub fn data_root() -> Option<PathBuf> {
    match std::env::var_os("RUSTMUSIC_DATA_DIR") {
        Some(dir) if !dir.is_empty() => Some(PathBuf::from(dir)),
        _ => dirs::data_dir(),
    }
}

/// Identifiant de l'application, nom de son dossier de données.
pub const IDENTIFIER: &str = "com.larevuegeek.rustmusic";

/// Données de l'application (base, pochettes, journaux).
pub fn app_data_dir() -> PathBuf {
    data_root()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(IDENTIFIER)
}

/// Configuration de l'application (état de la fenêtre).
pub fn app_config_dir() -> PathBuf {
    std::env::var_os("RUSTMUSIC_DATA_DIR")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::config_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(IDENTIFIER)
}

/// Le cache des pochettes.
pub fn covers_dir() -> PathBuf {
    app_data_dir().join("covers")
}
