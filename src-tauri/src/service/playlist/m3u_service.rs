//! Import et export de playlists .m3u / .m3u8 (foobar2000, VLC, Winamp…), pour « Cet ordinateur ».

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;
use sqlx::SqlitePool;

use crate::core::playlist_file::{self, M3uEntry};
use crate::mapper::playlist::playlist::playlist_track_view::PlaylistTrackView;
use crate::repository::playlist::playlist_repository::PlaylistRepository;

#[derive(Debug, Serialize)]
pub struct M3uImport {
    pub name: String,
    pub playlist_id: i64,
    /// Morceaux retrouvés dans la bibliothèque.
    pub added: usize,
    /// Lignes de la liste sans morceau correspondant.
    pub missing: usize,
}

/// Texte d'une liste : UTF-8, sinon Latin-1 (vieilles .m3u de Winamp).
fn read_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{} : {e}", path.display()))?;
    Ok(String::from_utf8(bytes).unwrap_or_else(|e| e.into_bytes().iter().map(|&b| b as char).collect()))
}

/// Les morceaux locaux du profil : par chemin exact (sans la casse) et par dossier/fichier.
async fn local_index(pool: &SqlitePool, profil_id: i64) -> Result<(HashMap<String, String>, HashMap<String, Option<String>>), sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT lf.path, lt.id FROM library_files lf
         JOIN library_tracks lt ON lt.file_id = lf.id
         JOIN library l ON l.id = lf.library_id
         WHERE l.profil_id = ? AND l.server_id IS NULL",
    )
    .bind(profil_id)
    .fetch_all(pool)
    .await?;
    let mut exact = HashMap::new();
    // Une fin de chemin partagée par deux morceaux est ambiguë : écartée.
    let mut tails: HashMap<String, Option<String>> = HashMap::new();
    for (path, id) in rows {
        tails.entry(playlist_file::tail_key(&path)).and_modify(|v| *v = None).or_insert_with(|| Some(id.clone()));
        exact.insert(path.to_lowercase(), id);
    }
    Ok((exact, tails))
}

/// Nom libre parmi les playlists du profil : « Nom », sinon « Nom (2) »…
async fn free_name(pool: &SqlitePool, profil_id: i64, wanted: &str) -> Result<String, sqlx::Error> {
    let taken: Vec<String> = sqlx::query_scalar("SELECT lower(name) FROM playlists WHERE profil_id = ?").bind(profil_id).fetch_all(pool).await?;
    let mut name = wanted.to_string();
    let mut n = 1;
    while taken.contains(&name.to_lowercase()) {
        n += 1;
        name = format!("{wanted} ({n})");
    }
    Ok(name)
}

/// Une liste devient une playlist de « Cet ordinateur » ; les morceaux absents de la bibliothèque sont comptés.
pub async fn import(pool: &SqlitePool, profil_id: i64, file: &Path) -> Result<M3uImport, String> {
    let text = read_text(file)?;
    let base = file.parent().unwrap_or(Path::new(""));
    let paths = playlist_file::parse(&text, base);
    let (exact, tails) = local_index(pool, profil_id).await.map_err(|e| e.to_string())?;

    let mut ids: Vec<String> = Vec::new();
    let mut missing = 0;
    for p in &paths {
        let s = p.to_string_lossy();
        let found = exact.get(&s.to_lowercase()).cloned().or_else(|| tails.get(&playlist_file::tail_key(&s)).cloned().flatten());
        match found {
            Some(id) if !ids.contains(&id) => ids.push(id),
            Some(_) => {}
            None => missing += 1,
        }
    }

    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "Playlist".into());
    let name = free_name(pool, profil_id, stem.trim()).await.map_err(|e| e.to_string())?;
    let playlist = PlaylistRepository::insert_playlist(pool, profil_id, None, name.clone(), None, "#8b5cf6".into(), "mynaui:music".into(), None, 0, 0)
        .await
        .map_err(|e| e.to_string())?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    for (i, id) in ids.iter().enumerate() {
        sqlx::query("INSERT INTO playlist_items (playlist_id, library_track_id, sort_index) VALUES (?, ?, ?)")
            .bind(playlist.id)
            .bind(id)
            .bind(i as i64)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    PlaylistRepository::recalculate_stats(&mut *tx, playlist.id).await.map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(M3uImport { name, playlist_id: playlist.id, added: ids.len(), missing })
}

/// Écrit les morceaux dans `dest` ; ceux d'un serveur (sans fichier sur le disque) sont laissés de côté.
/// Rend (écrits, laissés de côté).
pub fn export(tracks: &[PlaylistTrackView], dest: &Path) -> Result<(usize, usize), String> {
    let entries: Vec<M3uEntry> = tracks
        .iter()
        .filter_map(|t| {
            let path = t.path.clone().filter(|p| !p.starts_with(crate::core::remote::SCHEME))?;
            let label = match (&t.artist_name, &t.title) {
                (Some(a), Some(ti)) => format!("{a} - {ti}"),
                (None, Some(ti)) => ti.clone(),
                _ => Path::new(&path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
            };
            Some(M3uEntry { path, duration: t.duration, label })
        })
        .collect();
    std::fs::write(dest, playlist_file::build(&entries)).map_err(|e| format!("{} : {e}", dest.display()))?;
    Ok((entries.len(), tracks.len() - entries.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn an_m3u_from_another_machine_finds_the_tracks() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library (id, profil_id, name) VALUES (5, 1, 'L')").execute(&pool).await.unwrap();
        for (i, p) in ["S:/Musique/Queen/01 Bohemian.flac", "S:/Musique/Queen/02 Love.flac"].iter().enumerate() {
            sqlx::query("INSERT INTO library_files (id, library_id, path, filename) VALUES (?, 5, ?, 'x')").bind(format!("f{i}")).bind(p).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, title, title_normalized, duration) VALUES (?, 5, ?, 't', 't', 100)")
                .bind(format!("t{i}")).bind(format!("f{i}")).execute(&pool).await.unwrap();
        }
        let dir = std::env::temp_dir().join(format!("rm-m3u-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Soirée.m3u8");
        // Exacte (autre casse), même fin de chemin sur un autre disque, absente, doublon.
        std::fs::write(&file, "#EXTM3U\ns:/musique/queen/01 bohemian.flac\nD:\\Old\\Queen\\02 Love.flac\n/x/absent.mp3\nS:/Musique/Queen/01 Bohemian.flac\n").unwrap();
        let r = import(&pool, 1, &file).await.unwrap();
        assert_eq!((r.name.as_str(), r.added, r.missing), ("Soirée", 2, 1));
        // Même nom : suffixé.
        assert_eq!(import(&pool, 1, &file).await.unwrap().name, "Soirée (2)");
        let count: i64 = sqlx::query_scalar("SELECT track_count FROM playlists WHERE id = ?").bind(r.playlist_id).fetch_one(&pool).await.unwrap();
        assert_eq!(count, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
