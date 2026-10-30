//! Rescan : un fichier renommé ou déplacé garde sa piste (note, écoutes, playlists) ; les fichiers
//! disparus du disque quittent la bibliothèque. Sans ça, un fichier renommé apparaissait deux fois.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use sqlx::SqlitePool;

use crate::core::audio_analyser::audio_analyser::AudioAnalyser;
use crate::entity::library::library_files::FileFingerprint;
use crate::repository::library::library_album_repository::LibraryAlbumRepository;
use crate::repository::library::library_files_repository::LibraryFilesRepository;
use crate::repository::library::track_path_repository::TrackPathRepository;
use crate::service::library::album_folder::album_root;
use crate::service::library::album_merge::recount_library;
use crate::service::library::library_service::modified_secs;

/// Un nouveau fichier sur le disque : chemin, taille, date (secondes, en texte).
type NewFile = (String, i64, Option<String>);

/// Fichiers disparus et nouveaux de même taille et même date, quand chacun n'a qu'un seul vis-à-vis.
/// Plusieurs candidats (copies identiques) : rien, on ne devine pas.
fn pair_renames(missing: &[FileFingerprint], new: &[NewFile]) -> Vec<(FileFingerprint, String)> {
    let key = |size: i64, date: &Option<String>| date.clone().map(|d| (size, d));
    let mut gone: HashMap<(i64, String), Vec<&FileFingerprint>> = HashMap::new();
    for f in missing {
        if let Some(k) = key(f.size, &f.modified_at) {
            gone.entry(k).or_default().push(f);
        }
    }
    let mut found: HashMap<(i64, String), Vec<&String>> = HashMap::new();
    for (path, size, date) in new {
        if let Some(k) = key(*size, date) {
            found.entry(k).or_default().push(path);
        }
    }
    gone.into_iter()
        .filter_map(|(k, old)| match (old.as_slice(), found.get(&k).map(Vec::as_slice)) {
            ([old], Some([new])) => Some(((*old).clone(), (*new).clone())),
            _ => None,
        })
        .collect()
}

/// Mêmes titre et artiste (sans la casse), durée à une seconde près ; un tag absent d'un côté ne juge pas.
fn same_track(old: &FileFingerprint, title: Option<&str>, artist: Option<&str>, duration: Option<f64>) -> bool {
    let norm = |s: &str| s.trim().to_lowercase();
    let agree = |a: Option<&str>, b: Option<&str>| match (a.map(norm).filter(|s| !s.is_empty()), b.map(norm).filter(|s| !s.is_empty())) {
        (Some(a), Some(b)) => a == b,
        _ => true,
    };
    let durations = match (old.duration.filter(|d| *d > 0.0), duration.filter(|d| *d > 0.0)) {
        (Some(a), Some(b)) => (a - b).abs() <= 1.0,
        _ => true,
    };
    agree(old.title.as_deref(), title) && agree(old.artist.as_deref(), artist) && durations
}

/// Fait suivre chaque piste vers son nouveau chemin (fichier, cache, historique, J'aime, file),
/// puis le dossier de l'album quand il a entièrement déménagé. Rend le nombre de pistes suivies.
async fn apply_relinks(pool: &SqlitePool, pairs: &[(FileFingerprint, String)]) -> usize {
    let mut done = 0;
    let mut albums: HashSet<String> = HashSet::new();
    for (old, new) in pairs {
        // Le nouveau chemin a déjà ses traces (écouté à part) : pas de fusion hasardeuse.
        if TrackPathRepository::known(pool, std::slice::from_ref(new)).await.map_or(true, |k| !k.is_empty()) {
            continue;
        }
        let relinked = async {
            let mut tx = pool.begin().await?;
            TrackPathRepository::repoint(&mut tx, &old.path, new).await?;
            tx.commit().await
        }
        .await;
        match relinked {
            Ok(()) => {
                done += 1;
                if album_root(&old.path) != album_root(new) {
                    albums.extend(old.library_album_id.clone());
                }
            }
            Err(e) => log::warn!("📁 {} → {new} non suivi : {e}", old.path),
        }
    }
    for album in albums {
        let paths = LibraryAlbumRepository::track_paths(pool, &album).await.unwrap_or_default();
        let roots: HashSet<String> = paths.iter().map(|p| album_root(p)).collect();
        if let [root] = roots.into_iter().collect::<Vec<_>>().as_slice() {
            if let Err(e) = LibraryAlbumRepository::set_album_dir(pool, &album, root).await {
                log::warn!("📁 Dossier de l'album {album} non mis à jour : {e}");
            }
        }
    }
    done
}

/// Avant le scan : un fichier disparu et un nouveau de même taille, même date et mêmes tags sont le
/// même fichier renommé ou déplacé ; sa piste le suit au lieu d'être recréée. Rend le nombre suivi.
pub async fn relink_renamed(
    pool: &SqlitePool,
    library_id: i64,
    dirs: &[String],
    listed: &[PathBuf],
    unreadable: &[PathBuf],
) -> Result<usize, String> {
    let known = LibraryFilesRepository::find_fingerprints(pool, library_id)
        .await
        .map_err(|e| format!("Fichiers connus : {e}"))?;
    let all_known: HashSet<String> = LibraryFilesRepository::find_paths_by_library(pool, library_id)
        .await
        .map_err(|e| format!("Fichiers connus : {e}"))?
        .into_iter()
        .map(|(_, p)| p)
        .collect();
    let seen: HashSet<&str> = listed.iter().filter_map(|p| p.to_str()).collect();
    let missing: Vec<FileFingerprint> = known
        .into_iter()
        .filter(|f| {
            let p = Path::new(&f.path);
            dirs.iter().any(|d| p.starts_with(d)) && !seen.contains(f.path.as_str()) && !unreadable.iter().any(|u| p.starts_with(u))
        })
        .collect();
    let fresh: Vec<PathBuf> = listed.iter().filter(|p| p.to_str().is_some_and(|s| !all_known.contains(s))).cloned().collect();
    if missing.is_empty() || fresh.is_empty() {
        return Ok(0);
    }

    // Taille et date sur le disque, puis les tags des seuls candidats.
    let confirmed = tauri::async_runtime::spawn_blocking(move || {
        let new: Vec<NewFile> = fresh
            .iter()
            .filter_map(|p| {
                let meta = std::fs::metadata(p).ok()?;
                Some((p.to_str()?.to_string(), meta.len() as i64, modified_secs(&meta)))
            })
            .collect();
        pair_renames(&missing, &new)
            .into_iter()
            .filter(|(old, new)| match AudioAnalyser::analyse_for_import(&PathBuf::from(new)) {
                Ok(audio) => same_track(old, audio.tags.title.as_deref(), audio.tags.artist.as_deref(), Some(audio.duration as f64)),
                Err(_) => false,
            })
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| format!("Rapprochement : {e}"))?;

    let done = apply_relinks(pool, &confirmed).await;
    if done > 0 {
        log::info!("📁 {done} fichier(s) renommé(s) ou déplacé(s) suivi(s) : notes, écoutes et playlists gardées");
    }
    Ok(done)
}

/// Retire de la bibliothèque les fichiers connus sous `dir` absents de `listed` (le listage complet
/// du dossier) ; pistes, places en playlist et albums vides suivent. Rien sous un dossier `unreadable` :
/// un sous-dossier illisible (réseau, droits) n'est pas un dossier vidé. Rend le nombre de fichiers retirés.
pub async fn remove_missing_files(
    pool: &SqlitePool,
    library_id: i64,
    dir: &str,
    listed: &[PathBuf],
    unreadable: &[PathBuf],
) -> Result<usize, String> {
    let known = LibraryFilesRepository::find_paths_by_library(pool, library_id)
        .await
        .map_err(|e| format!("Fichiers connus : {e}"))?;
    let seen: HashSet<&str> = listed.iter().filter_map(|p| p.to_str()).collect();
    let gone: Vec<(String, String)> = known
        .into_iter()
        .filter(|(_, path)| {
            let p = Path::new(path);
            p.starts_with(dir) && !seen.contains(path.as_str()) && !unreadable.iter().any(|u| p.starts_with(u))
        })
        .collect();
    if gone.is_empty() {
        return Ok(0);
    }
    // Un dossier qui se liste vide alors qu'il avait des morceaux : disque mal monté, pas une suppression.
    if listed.is_empty() {
        log::warn!("📁 {dir} se liste vide : ses {} fichier(s) connus sont gardés", gone.len());
        return Ok(0);
    }

    let mut tx = pool.begin().await.map_err(|e| format!("Transaction : {e}"))?;
    for (id, _) in &gone {
        LibraryFilesRepository::delete_by_id(&mut *tx, id).await.map_err(|e| format!("Retrait d'un fichier : {e}"))?;
    }
    LibraryAlbumRepository::delete_empty(&mut *tx, library_id).await.map_err(|e| format!("Albums vides : {e}"))?;
    recount_library(&mut tx, library_id).await?;
    tx.commit().await.map_err(|e| format!("Validation : {e}"))?;

    log::info!("📁 {} fichier(s) disparu(s) de {dir} retiré(s) de la bibliothèque", gone.len());
    Ok(gone.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn base() -> SqlitePool {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'T')").execute(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO library (id, profil_id, name) VALUES (1, 1, 'L')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO artists (id, name, name_normalized, sort_name) VALUES ('a1', 'A', 'a', 'A')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library_albums (id, library_id, artist_id, title, title_normalized) VALUES ('al1', 1, 'a1', 'Un', 'un'), ('al2', 1, 'a1', 'Deux', 'deux')")
            .execute(&pool).await.unwrap();
        // Deux morceaux sur l'album Un, un sur Deux, un hors du dossier scanné.
        for (n, path, album) in [(1, "/m/un/1.flac", "al1"), (2, "/m/un/2.flac", "al1"), (3, "/m/deux/1.flac", "al2"), (4, "/autre/x.flac", "al2")] {
            sqlx::query("INSERT INTO library_files (id, library_id, path, filename, extension, size, status) VALUES (?, 1, ?, 'x', 'flac', 1, 'ok')")
                .bind(format!("f{n}")).bind(path).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, artist_id, library_album_id, title, title_normalized, duration) VALUES (?, 1, ?, 'a1', ?, 't', 't', 200)")
                .bind(format!("t{n}")).bind(format!("f{n}")).bind(album).execute(&pool).await.unwrap();
        }
        pool
    }

    async fn count(pool: &SqlitePool, sql: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_string())).fetch_one(pool).await.unwrap()
    }

    #[tokio::test]
    async fn a_renamed_file_leaves_no_duplicate() {
        let pool = base().await;
        // `2.flac` renommé (le nouveau nom est déjà enregistré par le scan), `deux/1.flac` supprimé.
        let listed = [PathBuf::from("/m/un/1.flac"), PathBuf::from("/m/un/2 - renamed.flac")];
        assert_eq!(remove_missing_files(&pool, 1, "/m", &listed, &[]).await.unwrap(), 2);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM library_files").await, 2, "reste 1.flac et le fichier hors du dossier");
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM library_tracks").await, 2);
        // L'album Deux garde le morceau hors dossier ; aucun album vide ne reste.
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM library_albums").await, 2);
        assert_eq!(count(&pool, "SELECT total_albums FROM library WHERE id = 1").await, 2);
    }

    fn fp(path: &str, size: i64, date: &str, title: &str) -> FileFingerprint {
        FileFingerprint {
            path: path.into(), size, modified_at: Some(date.into()),
            title: Some(title.into()), artist: Some("Bach".into()), duration: Some(200.0), library_album_id: Some("al1".into()),
        }
    }

    #[test]
    fn renames_pair_on_size_and_date_only_when_unambiguous() {
        let missing = [fp("/m/a.flac", 10, "100", "A"), fp("/m/b.flac", 20, "200", "B"), fp("/m/c.flac", 20, "200", "C")];
        let new = [("/m/a2.flac".to_string(), 10, Some("100".to_string())), ("/m/x.flac".to_string(), 20, Some("200".to_string()))];
        let pairs = pair_renames(&missing, &new);
        // a → a2 ; b et c ont la même empreinte : ambigu, rien.
        assert_eq!(pairs.iter().map(|(o, n)| (o.path.as_str(), n.as_str())).collect::<Vec<_>>(), [("/m/a.flac", "/m/a2.flac")]);
        // Date différente : pas le même fichier.
        assert!(pair_renames(&missing[..1], &[("/m/a2.flac".into(), 10, Some("101".into()))]).is_empty());
    }

    #[test]
    fn tags_confirm_the_rename() {
        let old = fp("/m/a.flac", 10, "100", "Air");
        assert!(same_track(&old, Some(" air "), Some("BACH"), Some(200.6)));
        assert!(same_track(&old, None, None, None), "sans tags lus, taille et date suffisent");
        assert!(!same_track(&old, Some("Prélude"), Some("Bach"), Some(200.0)));
        assert!(!same_track(&old, Some("Air"), Some("Bach"), Some(260.0)));
    }

    #[tokio::test]
    async fn a_renamed_file_keeps_its_track_rating_and_playlists() {
        let pool = base().await;
        sqlx::query("UPDATE library_tracks SET rating = 4.5, play_count = 7 WHERE id = 't1'").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO playlists (id, profil_id, name, color, icon) VALUES (9, 1, 'P', '#000', 'x')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO playlist_items (playlist_id, library_track_id, sort_index) VALUES (9, 't1', 0)").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO track_liked (path, profil_id) VALUES ('/m/un/1.flac', 1)").execute(&pool).await.unwrap();

        // Tout l'album Un déménage dans un nouveau dossier.
        let pairs = [
            (fp("/m/un/1.flac", 1, "1", "t"), "/m/Un (2020)/1.flac".to_string()),
            (fp("/m/un/2.flac", 1, "1", "t"), "/m/Un (2020)/2.flac".to_string()),
        ];
        assert_eq!(apply_relinks(&pool, &pairs).await, 2);

        let (rating, plays): (f64, i64) = sqlx::query_as("SELECT rating, play_count FROM library_tracks lt JOIN library_files lf ON lf.id = lt.file_id WHERE lf.path = '/m/Un (2020)/1.flac'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!((rating, plays), (4.5, 7));
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM playlist_items WHERE library_track_id = 't1'").await, 1);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM track_liked WHERE path = '/m/Un (2020)/1.flac'").await, 1);
        let dir: String = sqlx::query_scalar("SELECT album_dir FROM library_albums WHERE id = 'al1'").fetch_one(&pool).await.unwrap();
        assert_eq!(dir, album_root("/m/Un (2020)/1.flac"));
        // Plus rien de disparu : le retrait n'enlève rien.
        let listed = [PathBuf::from("/m/Un (2020)/1.flac"), PathBuf::from("/m/Un (2020)/2.flac"), PathBuf::from("/m/deux/1.flac")];
        assert_eq!(remove_missing_files(&pool, 1, "/m", &listed, &[]).await.unwrap(), 0);
    }

    /// Sur le vrai disque : un FLAC renommé dans un autre dossier est suivi ; un autre, modifié en plus, non.
    #[tokio::test]
    async fn a_file_renamed_on_disk_is_followed() {
        let pool = base().await;
        let root = std::env::temp_dir().join(format!("rustmusic-relink-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("avant")).unwrap();
        std::fs::create_dir_all(root.join("apres")).unwrap();
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let (a, b) = (root.join("avant/1.flac"), root.join("avant/2.flac"));
        std::fs::copy(fixture.join("flac16.flac"), &a).unwrap();
        std::fs::copy(fixture.join("flac24.flac"), &b).unwrap();

        // Indexés comme au scan : taille, date et tags lus du fichier.
        for (n, f) in [(5, &a), (6, &b)] {
            let meta = std::fs::metadata(f).unwrap();
            let audio = AudioAnalyser::analyse_for_import(f).unwrap();
            let path = f.to_str().unwrap();
            sqlx::query("INSERT INTO library_cache (id, path, title, artist, duration) VALUES (?, ?, ?, ?, ?)")
                .bind(n).bind(path).bind(&audio.tags.title).bind(&audio.tags.artist).bind(audio.duration as f64).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_files (id, library_id, cache_id, path, filename, extension, size, status, modified_at) VALUES (?, 1, ?, ?, 'x', 'flac', ?, 'indexed', ?)")
                .bind(format!("f{n}")).bind(n).bind(path).bind(meta.len() as i64).bind(modified_secs(&meta)).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, cache_id, artist_id, title, title_normalized, duration, rating) VALUES (?, 1, ?, ?, 'a1', 't', 't', 1, 3.0)")
                .bind(format!("t{n}")).bind(format!("f{n}")).bind(n).execute(&pool).await.unwrap();
        }

        let (a2, b2) = (root.join("apres/un.flac"), root.join("apres/deux.flac"));
        std::fs::rename(&a, &a2).unwrap();
        std::fs::rename(&b, &b2).unwrap();
        // Le second est aussi modifié (taille changée) : ce n'est plus le même fichier.
        use std::io::Write;
        std::fs::OpenOptions::new().append(true).open(&b2).unwrap().write_all(b"x").unwrap();

        let dirs = [root.to_str().unwrap().to_string()];
        let listed = [a2.clone(), b2.clone()];
        assert_eq!(relink_renamed(&pool, 1, &dirs, &listed, &[]).await.unwrap(), 1);
        let suivi: String = sqlx::query_scalar("SELECT path FROM library_files WHERE id = 'f5'").fetch_one(&pool).await.unwrap();
        assert_eq!(suivi, a2.to_str().unwrap());
        let ancien: String = sqlx::query_scalar("SELECT path FROM library_files WHERE id = 'f6'").fetch_one(&pool).await.unwrap();
        assert_eq!(ancien, b.to_str().unwrap(), "modifié : pas suivi, le retrait s'en chargera");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn an_unreadable_subfolder_keeps_its_files() {
        let pool = base().await;
        // `/m/deux` illisible pendant le scan : `deux/1.flac` n'est pas listé, mais pas supprimé.
        let listed = [PathBuf::from("/m/un/1.flac"), PathBuf::from("/m/un/2.flac")];
        assert_eq!(remove_missing_files(&pool, 1, "/m", &listed, &[PathBuf::from("/m/deux")]).await.unwrap(), 0);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM library_files").await, 4);
    }

    #[tokio::test]
    async fn an_empty_listing_removes_nothing() {
        let pool = base().await;
        assert_eq!(remove_missing_files(&pool, 1, "/m", &[], &[]).await.unwrap(), 0);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM library_files").await, 4);
    }

    #[tokio::test]
    async fn an_album_left_without_tracks_goes_away() {
        let pool = base().await;
        // L'album Deux n'a plus que `deux/1.flac`, qui disparaît (le dossier garde un autre fichier).
        sqlx::query("DELETE FROM library_files WHERE id = 'f4'").execute(&pool).await.unwrap();
        assert_eq!(remove_missing_files(&pool, 1, "/m/deux", &[PathBuf::from("/m/deux/autre.flac")], &[]).await.unwrap(), 1);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM library_albums WHERE id = 'al2'").await, 0);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM library_albums WHERE id = 'al1'").await, 1);
        // Un dossier voisin au nom proche (`/m/un` vs `/m/u`) n'est pas concerné.
        assert_eq!(remove_missing_files(&pool, 1, "/m/u", &[PathBuf::from("/m/u/x.flac")], &[]).await.unwrap(), 0);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM library_files").await, 2);
    }
}
