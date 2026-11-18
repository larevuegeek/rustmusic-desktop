use std::path::PathBuf;

pub fn get_database_url(directory: &str, db_name: &str) -> String {

    let mut dir: PathBuf = crate::app::platform::paths::data_root().unwrap_or_else(|| PathBuf::from("."));
    dir.push(directory);

    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::error!("Impossible de créer le dossier AppData: {}", e);
    }

    let db_path: PathBuf = dir.join(db_name);

    format!("sqlite:{}?mode=rwc", db_path.display())
}