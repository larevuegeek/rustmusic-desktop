//! Export automatique des playlists (option par playlist) : un .m3u8 ou .m3u par playlist dans un dossier
//! choisi, à copier avec la musique sur une clé ou un téléphone. Chemins relatifs au dossier par défaut.
//! Un fichier n'est réécrit que si son contenu change : les outils de synchronisation ne recopient rien pour rien.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sqlx::SqlitePool;

use crate::commands::playlist::playlist_command::tracks_of;
use crate::core::playlist_file::{self, M3uEntry};
use crate::repository::settings::settings_repository::SettingsRepository;

pub const DIR_KEY: &str = "playlist_export_dir";
pub const FORMAT_KEY: &str = "playlist_export_format";
pub const RELATIVE_KEY: &str = "playlist_export_relative";

/// Une passe à la fois : deux passes rapprochées écriraient les mêmes fichiers.
static RUNNING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug, Default, Serialize)]
pub struct ExportReport {
    /// Pas de dossier choisi : rien n'a été écrit.
    pub no_dir: bool,
    pub playlists: usize,
    /// Fichiers réécrits (les autres étaient déjà à jour).
    pub written: usize,
    pub tracks: usize,
    /// Morceaux sur un autre disque que le dossier : écrits avec leur chemin complet.
    pub absolute: usize,
    /// Morceaux d'un serveur, sans fichier sur le disque : laissés de côté.
    pub skipped: usize,
    pub errors: Vec<String>,
}

struct Options {
    dir: PathBuf,
    extension: &'static str,
    relative: bool,
}

async fn options(pool: &SqlitePool) -> Option<Options> {
    let get = |k: &'static str| async move { SettingsRepository::get(pool, k).await.ok().flatten().unwrap_or_default() };
    let dir = get(DIR_KEY).await;
    if dir.trim().is_empty() {
        return None;
    }
    Some(Options {
        dir: PathBuf::from(dir.trim()),
        extension: if get(FORMAT_KEY).await == "m3u" { "m3u" } else { "m3u8" },
        relative: get(RELATIVE_KEY).await != "false",
    })
}

/// Nom de fichier sûr sur Windows, macOS, Linux et les cartes FAT d'un téléphone.
fn file_stem(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() { '_' } else { c })
        .collect();
    let clean = clean.trim().trim_end_matches('.').trim();
    if clean.is_empty() { "Playlist".into() } else { clean.chars().take(120).collect() }
}

/// Retire un fichier écrit par nous (jamais autre chose qu'une liste .m3u / .m3u8).
fn remove_ours(path: &str) {
    let p = Path::new(path);
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if (ext == "m3u" || ext == "m3u8") && p.is_file() {
        if let Err(e) = std::fs::remove_file(p) {
            log::warn!("[export auto] {path} : {e}");
        }
    }
}

/// Toutes les playlists marquées « Exporter automatiquement ».
pub async fn run(pool: &SqlitePool) -> ExportReport {
    let _guard = RUNNING.lock().await;
    let mut report = ExportReport::default();
    let rows: Vec<(i64, String, Option<String>)> =
        match sqlx::query_as("SELECT id, name, export_file FROM playlists WHERE auto_export = 1 ORDER BY profil_id, position, id")
            .fetch_all(pool)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                report.errors.push(e.to_string());
                return report;
            }
        };
    if rows.is_empty() {
        return report;
    }
    let Some(opts) = options(pool).await else {
        report.no_dir = true;
        return report;
    };
    if let Err(e) = std::fs::create_dir_all(&opts.dir) {
        report.errors.push(format!("{} : {e}", opts.dir.display()));
        return report;
    }
    let dir_str = opts.dir.to_string_lossy().to_string();
    let mut taken: HashSet<String> = HashSet::new();

    for (id, name, previous) in rows {
        // Deux playlists du même nom (deux profils) : la seconde prend son numéro.
        let mut stem = file_stem(&name);
        if !taken.insert(stem.to_lowercase()) {
            stem = format!("{stem} ({id})");
            taken.insert(stem.to_lowercase());
        }
        let target = opts.dir.join(format!("{stem}.{}", opts.extension));
        let target_str = target.to_string_lossy().to_string();

        let tracks = match tracks_of(pool, id).await {
            Ok(t) => t,
            Err(e) => {
                report.errors.push(format!("{name} : {e}"));
                continue;
            }
        };
        let mut entries = Vec::with_capacity(tracks.len());
        for t in &tracks {
            let Some(path) = t.path.clone().filter(|p| !p.starts_with(crate::core::remote::SCHEME)) else {
                report.skipped += 1;
                continue;
            };
            let written_path = if opts.relative {
                playlist_file::relative_path(&dir_str, &path).unwrap_or_else(|| {
                    report.absolute += 1;
                    path.clone()
                })
            } else {
                path.clone()
            };
            let label = match (&t.artist_name, &t.title) {
                (Some(a), Some(ti)) => format!("{a} - {ti}"),
                (None, Some(ti)) => ti.clone(),
                _ => Path::new(&path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
            };
            entries.push(M3uEntry { path: written_path, duration: t.duration, label });
        }
        report.playlists += 1;
        report.tracks += entries.len();

        let content = playlist_file::build(&entries);
        if std::fs::read_to_string(&target).ok().as_deref() != Some(content.as_str()) {
            match std::fs::write(&target, &content) {
                Ok(()) => report.written += 1,
                Err(e) => {
                    report.errors.push(format!("{} : {e}", target.display()));
                    continue;
                }
            }
        }
        // Renommée, autre format ou autre dossier : l'ancien fichier part.
        if let Some(old) = previous.filter(|old| *old != target_str) {
            remove_ours(&old);
        }
        let _ = sqlx::query("UPDATE playlists SET export_file = ? WHERE id = ?").bind(&target_str).bind(id).execute(pool).await;
    }
    report
}

/// Option coupée (ou playlist supprimée) : son fichier part avec elle.
pub async fn forget(pool: &SqlitePool, playlist_id: i64) {
    let previous: Option<String> = sqlx::query_scalar("SELECT export_file FROM playlists WHERE id = ?")
        .bind(playlist_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .flatten();
    if let Some(old) = previous {
        remove_ours(&old);
    }
    let _ = sqlx::query("UPDATE playlists SET export_file = NULL WHERE id = ?").bind(playlist_id).execute(pool).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_are_safe_everywhere() {
        assert_eq!(file_stem("Rock: 70's / 80's?"), "Rock_ 70's _ 80's_");
        assert_eq!(file_stem("  ...  "), "Playlist");
        assert_eq!(file_stem("Soirée."), "Soirée");
    }

    /// Bout en bout sur une base en mémoire : relatif, inchangé non réécrit, renommage, retrait.
    #[tokio::test]
    async fn exports_rewrites_only_on_change_and_cleans_up() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        let root = std::env::temp_dir().join(format!("rm-autoexport-{}", uuid::Uuid::new_v4()));
        let music = root.join("Musique").join("Muse");
        std::fs::create_dir_all(&music).unwrap();
        let song = music.join("01 Uprising.flac");
        std::fs::write(&song, b"x").unwrap();
        let dir = root.join("Musique").join("Playlists");

        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library (id, profil_id, name) VALUES (1, 1, 'L')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library_files (id, library_id, path, filename) VALUES ('f1', 1, ?, 'x')")
            .bind(song.to_string_lossy().to_string()).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, title, title_normalized, duration) VALUES ('t1', 1, 'f1', 'Uprising', 'uprising', 305)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO playlists (id, profil_id, name, auto_export) VALUES (7, 1, 'En route', 1)").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO playlist_items (playlist_id, library_track_id, sort_index) VALUES (7, 't1', 0)").execute(&pool).await.unwrap();

        // Sans dossier : rien.
        assert!(run(&pool).await.no_dir);
        SettingsRepository::set(&pool, DIR_KEY, &dir.to_string_lossy()).await.unwrap();

        let r = run(&pool).await;
        assert_eq!((r.playlists, r.written, r.tracks, r.absolute), (1, 1, 1, 0), "{:?}", r.errors);
        let file = dir.join("En route.m3u8");
        assert!(std::fs::read_to_string(&file).unwrap().contains("../Muse/01 Uprising.flac"));

        // Rien de changé : pas réécrit.
        assert_eq!(run(&pool).await.written, 0);

        // Renommée : nouveau fichier, l'ancien retiré.
        sqlx::query("UPDATE playlists SET name = 'Voiture' WHERE id = 7").execute(&pool).await.unwrap();
        assert_eq!(run(&pool).await.written, 1);
        assert!(!file.exists() && dir.join("Voiture.m3u8").exists());

        // Option coupée : fichier retiré.
        forget(&pool, 7).await;
        assert!(!dir.join("Voiture.m3u8").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
