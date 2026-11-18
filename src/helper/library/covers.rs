//! Pochettes sur disque : le cache de RustMusic garde chaque pochette entière
//! (`…/full/…`) et des miniatures déjà calculées (`…/2x/…` 500 px, `…/1x/…` 160 px).

use std::path::{Path, PathBuf};

/// Côté des vignettes (`size="1x"` de la source) : mosaïques de genre, petites cases.
pub const SMALL_EDGE: u32 = 160;

/// Le fichier à décoder pour une pochette demandée à `edge` pixels : la miniature 1x
/// (vignettes) ou 2x quand elle existe, bien plus légère à décoder que la pochette
/// entière pour le même rendu ; sinon la pochette elle-même.
pub fn on_disk(path: &str, edge: u32) -> PathBuf {
    let dir = if edge <= SMALL_EDGE { "1x" } else { "2x" };
    let small = path
        .replace("\\full\\", &format!("\\{dir}\\"))
        .replace("/full/", &format!("/{dir}/"));
    if small != path && Path::new(&small).is_file() {
        PathBuf::from(small)
    } else {
        PathBuf::from(path)
    }
}
