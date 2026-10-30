//! Aperçu des sous-dossiers d'un dossier : pochettes, nombre de titres, meilleure qualité.

use std::collections::HashMap;

use serde::Serialize;
use sqlx::SqlitePool;

use crate::repository::library::library_track_repository::{FolderTrackRow, LibraryTrackRepository};

/// Pochettes montrées sur une tuile de dossier (mosaïque 2×2 au plus).
const MAX_COVERS: usize = 4;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct FolderPreview {
    /// Une par album, dans l'ordre des chemins.
    pub covers: Vec<String>,
    pub track_count: u32,
    pub album_count: u32,
    /// Meilleure piste du dossier : DSD, puis profondeur, puis fréquence.
    pub best_format: Option<String>,
    pub best_bits: Option<u32>,
    pub best_rate: Option<u32>,
}

/// Aperçus des sous-dossiers directs de `dir`, indexés par leur nom.
pub async fn subfolder_previews(pool: &SqlitePool, library_id: i64, dir: &str) -> HashMap<String, FolderPreview> {
    let sep = std::path::MAIN_SEPARATOR;
    let prefix = if dir.ends_with(sep) { dir.to_string() } else { format!("{dir}{sep}") };
    match LibraryTrackRepository::find_under_prefix(pool, library_id, &prefix).await {
        Ok(rows) => summarize(&prefix, rows),
        Err(e) => {
            log::warn!("[dossiers] aperçu de {dir} : {e}");
            HashMap::new()
        }
    }
}

fn summarize(prefix: &str, rows: Vec<FolderTrackRow>) -> HashMap<String, FolderPreview> {
    let mut previews: HashMap<String, FolderPreview> = HashMap::new();
    let mut albums: HashMap<String, Vec<String>> = HashMap::new();
    let mut best: HashMap<String, (u64, String, u32, u32)> = HashMap::new();

    for row in rows {
        let Some(rest) = row.path.get(prefix.len()..) else { continue };
        // Un fichier posé directement dans le dossier n'appartient à aucun sous-dossier.
        let Some((child, _)) = rest.split_once(['\\', '/']) else { continue };
        let preview = previews.entry(child.to_string()).or_default();
        preview.track_count += 1;

        let seen = albums.entry(child.to_string()).or_default();
        let album = row.album_id.clone().or_else(|| row.cover.clone()).unwrap_or_default();
        if !seen.contains(&album) {
            seen.push(album);
            if let Some(cover) = row.cover.filter(|c| !c.is_empty()) {
                if preview.covers.len() < MAX_COVERS && !preview.covers.contains(&cover) {
                    preview.covers.push(cover);
                }
            }
        }

        let bits = row.bits_per_sample.unwrap_or(0).max(0) as u32;
        let rate = row.sample_rate.unwrap_or(0).max(0) as u32;
        let rank = if bits == 1 { u64::MAX } else { (bits as u64) << 32 | rate as u64 };
        let current = best.entry(child.to_string()).or_insert((0, String::new(), 0, 0));
        if rank > current.0 || current.1.is_empty() {
            *current = (rank, row.audio_format.unwrap_or_default(), bits, rate);
        }
    }

    for (child, preview) in previews.iter_mut() {
        preview.album_count = albums.get(child).map_or(0, |a| a.iter().filter(|x| !x.is_empty()).count()) as u32;
        if let Some((_, format, bits, rate)) = best.remove(child) {
            preview.best_format = Some(format).filter(|f| !f.is_empty() && f != "Unknown");
            preview.best_bits = Some(bits).filter(|&b| b > 0);
            preview.best_rate = Some(rate).filter(|&r| r > 0);
        }
    }
    previews
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(path: &str, album: &str, cover: Option<&str>, format: &str, bits: i64, rate: i64) -> FolderTrackRow {
        FolderTrackRow {
            path: path.into(),
            album_id: Some(album.into()),
            cover: cover.map(Into::into),
            audio_format: Some(format.into()),
            bits_per_sample: Some(bits),
            sample_rate: Some(rate),
        }
    }

    #[test]
    fn each_subfolder_gets_its_covers_count_and_best_quality() {
        let p = r"S:\HD\";
        let previews = summarize(p, vec![
            row(r"S:\HD\Adele\01.flac", "a1", Some("c1"), "FLAC", 16, 44100),
            row(r"S:\HD\Adele\02.flac", "a1", Some("c1"), "FLAC", 24, 96000),
            row(r"S:\HD\Adele\CD2\01.flac", "a2", Some("c2"), "FLAC", 16, 44100),
            row(r"S:\HD\Bowie\01.dsf", "b1", Some("c3"), "DSF", 1, 2822400),
            row(r"S:\HD\Bowie\02.flac", "b2", None, "FLAC", 24, 192000),
            row(r"S:\HD\seul.flac", "x", Some("cx"), "FLAC", 16, 44100),
        ]);
        assert_eq!(previews.len(), 2, "un fichier à la racine n'est pas un sous-dossier");

        let adele = &previews["Adele"];
        assert_eq!((adele.track_count, adele.album_count), (3, 2), "CD2 compte dans Adele");
        assert_eq!(adele.covers, vec!["c1", "c2"], "une pochette par album, pas par piste");
        assert_eq!((adele.best_format.as_deref(), adele.best_bits, adele.best_rate), (Some("FLAC"), Some(24), Some(96000)));

        let bowie = &previews["Bowie"];
        assert_eq!(bowie.covers, vec!["c3"], "un album sans pochette n'en invente pas");
        assert_eq!((bowie.best_format.as_deref(), bowie.best_bits), (Some("DSF"), Some(1)), "le DSD passe devant le 24/192");
    }

    #[test]
    fn covers_stop_at_four_and_shared_covers_are_not_repeated() {
        // Deux albums partagent une pochette, puis cinq autres : « même » une fois, et quatre au total.
        let rows = (0..7).map(|i| {
            let cover = if i < 2 { "même".to_string() } else { format!("c{i}") };
            row(&format!(r"S:\M\Coffret\{i}\01.flac"), &format!("a{i}"), Some(&cover), "FLAC", 16, 44100)
        });
        let previews = summarize(r"S:\M\", rows.collect());
        assert_eq!(previews["Coffret"].covers, vec!["même", "c2", "c3", "c4"]);
        assert_eq!(previews["Coffret"].album_count, 7);
    }
}
