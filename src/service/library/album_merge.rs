//! Rassemble les fiches d'un même album éclatées par artiste.
//!
//! L'identité d'un album a longtemps inclus son artiste : sur une compilation
//! sans tag « artiste de l'album », chaque piste fondait le sien — 52 fiches
//! pour un seul disque. La migration a introduit `album_dir` ; ce service le
//! remplit et fusionne ce qui doit l'être. Tourne au démarrage, idempotent :
//! une fois `album_dir` posé, il n'a plus rien à lire.

use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::entity::artist::artist::ArtistCreate;
use crate::entity::library::library_artist::LibraryArtistCreate;
use crate::helper::string::string::{normalize_name, normalize_sort_name, split_artists};
use crate::repository::artist::artist_repository::ArtistRepository;
use crate::repository::library::library_album_artist_repository::LibraryAlbumArtistRepository;
use crate::repository::library::library_album_repository::LibraryAlbumRepository;
use crate::repository::library::library_artist_repository::LibraryArtistRepository;
use crate::repository::library::library_pin_repository::LibraryPinRepository;
use crate::repository::library::library_repository::LibraryRepository;
use crate::repository::library::library_track_repository::LibraryTrackRepository;
use crate::service::library::album_folder::album_root;

/// Le nom donné à un album dont les pistes n'ont pas d'artiste commun.
pub(crate) const VARIOUS_ARTISTS: &str = "Various Artists";

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlbumMergeReport {
    /// Fiches examinées, c'est-à-dire sans `album_dir`.
    pub albums_seen: i64,
    /// Groupes ayant fusionné.
    pub groups_merged: i64,
    /// Fiches supprimées au passage.
    pub albums_removed: i64,
    /// Albums passés à « Various Artists ».
    pub compilations: i64,
}

/// Une fiche d'album et ce que ses pistes disent d'elle.
#[derive(Default)]
struct Record {
    id: String,
    library_id: i64,
    title: String,
    artist_id: String,
    cover: Option<String>,
    folder: String,
    tracks_list: i64,
    artists: Vec<String>,
    tags: Vec<String>,
}

pub async fn consolidate(pool: &SqlitePool) -> Result<AlbumMergeReport, String> {
    let lines = LibraryAlbumRepository::find_without_dir(pool)
        .await
        .map_err(|e| format!("Lecture des albums : {e}"))?;

    if lines.is_empty() {
        return Ok(AlbumMergeReport::default());
    }

    // ─── Une fiche par album, nourrie de ses pistes ───
    let mut records: HashMap<String, Record> = HashMap::new();
    for l in lines {
        let f = records.entry(l.album_id.clone()).or_insert_with(|| Record {
            id: l.album_id.clone(),
            library_id: l.library_id,
            title: l.title_normalized.clone(),
            artist_id: l.artist_id.clone(),
            cover: l.cover_url.clone(),
            ..Default::default()
        });
        if let Some(p) = l.path.as_deref() {
            if f.tracks_list == 0 {
                f.folder = album_root(p);
            }
            f.tracks_list += 1;
        }
        if let Some(a) = l.track_artist {
            f.artists.push(a);
        }
        // Le tag peut nommer plusieurs artistes : seul le premier porte l'album.
        if let Some(t) = l.tag_album {
            if let Some(first) = split_artists(&t).into_iter().next() {
                f.tags.push(first);
            }
        }
    }

    let mut report = AlbumMergeReport {
        albums_seen: records.len() as i64,
        ..Default::default()
    };

    // ─── Regroupement sur la nouvelle identité ───
    let mut groups: HashMap<(i64, String, String), Vec<Record>> = HashMap::new();
    for (_, f) in records {
        groups
            .entry((f.library_id, f.title.clone(), f.folder.clone()))
            .or_default()
            .push(f);
    }

    let mut known_items: HashMap<String, String> =
        ArtistRepository::find_ids_by_normalized_name(pool)
            .await
            .map_err(|e| format!("Lecture des artistes : {e}"))?
            .into_iter()
            .collect();

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Ouverture de la transaction : {e}"))?;

    let mut libraries: Vec<i64> = Vec::new();

    for ((library_id, _, folder), mut group) in groups {
        if !libraries.contains(&library_id) {
            libraries.push(library_id);
        }

        // La fiche qui a déjà une pochette, puis la mieux fournie.
        group.sort_by(|a, b| {
            b.cover
                .is_some()
                .cmp(&a.cover.is_some())
                .then(b.tracks_list.cmp(&a.tracks_list))
                .then(a.id.cmp(&b.id))
        });

        let guard = group[0].id.clone();

        if group.len() > 1 {
            for lost in &group[1..] {
                merge_into(&mut tx, &guard, &lost.id).await?;
                report.albums_removed += 1;
            }
            report.groups_merged += 1;

            // ─── À qui appartient l'album fusionné ? ───
            let mut counts: HashMap<&str, i64> = HashMap::new();
            for f in &group {
                for t in &f.tags {
                    *counts.entry(t.as_str()).or_insert(0) += 1;
                }
            }
            let artists: std::collections::HashSet<&str> = group
                .iter()
                .flat_map(|f| f.artists.iter().map(|s| s.as_str()))
                .collect();

            // Le tag majoritaire tranche ; un featuring isolé ne fait pas d'une
            // pochette une compilation.
            let owner = if let Some((tag, _)) =
                counts.iter().max_by_key(|(name, n)| (**n, std::cmp::Reverse(*name)))
            {
                resolve_artist(&mut tx, &mut known_items, tag, library_id).await?
            } else if artists.len() > 1 {
                report.compilations += 1;
                LibraryAlbumRepository::set_album_type(&mut *tx, &guard, "compilation")
                    .await
                    .map_err(|e| format!("Type de l'album : {e}"))?;
                resolve_artist(&mut tx, &mut known_items, VARIOUS_ARTISTS, library_id).await?
            } else {
                group[0].artist_id.clone()
            };

            LibraryAlbumRepository::set_artist(&mut *tx, &guard, &owner)
                .await
                .map_err(|e| format!("Propriétaire de l'album : {e}"))?;

            recount_album(&mut tx, &guard).await?;
        }

        // Posé en dernier : c'est lui qui rend la fiche invisible au prochain tour.
        LibraryAlbumRepository::set_album_dir(&mut *tx, &guard, &folder)
            .await
            .map_err(|e| format!("Dossier de l'album : {e}"))?;
    }

    // Les compteurs d'albums ont vu passer des suppressions et des changements
    // d'artiste ; aucun déclencheur ne couvre le second cas.
    // Un décompte groupé, pas une sous-requête par artiste : corrélée, celle-ci
    // relisait toute la bibliothèque pour chacun de ses 941 artistes — 10,9 s.
    for library_id in &libraries {
        recount_library(&mut tx, *library_id).await?;
    }

    tx.commit()
        .await
        .map_err(|e| format!("Validation : {e}"))?;

    if report.groups_merged > 0 {
        log::info!(
            "💿 Albums rassemblés : {} groupe(s), {} fiche(s) en double supprimée(s), \
             {} compilation(s)",
            report.groups_merged,
            report.albums_removed,
            report.compilations
        );
    }

    Ok(report)
}

/// Rattache les pistes, artistes d'album et épingles de la fiche `lost` à `guard`, puis la supprime.
pub(crate) async fn merge_into(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, guard: &str, lost: &str) -> Result<(), String> {
    LibraryTrackRepository::move_to_album(&mut **tx, lost, guard)
        .await
        .map_err(|e| format!("Rattachement des pistes : {e}"))?;

    // Sans ça la cascade les emporterait avec la fiche.
    LibraryAlbumArtistRepository::copy_to_album(&mut **tx, lost, guard)
        .await
        .map_err(|e| format!("Report des artistes d'album : {e}"))?;

    // Même raison ; si `guard` est déjà épinglée, la cascade retire le doublon.
    LibraryPinRepository::move_album(&mut **tx, lost, guard)
        .await
        .map_err(|e| format!("Report des épingles : {e}"))?;

    LibraryAlbumRepository::delete_by_id(&mut **tx, lost)
        .await
        .map_err(|e| format!("Suppression de la fiche en double : {e}"))?;
    Ok(())
}

/// Les déclencheurs comptent les pistes insérées, pas déplacées.
pub(crate) async fn recount_album(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, album_id: &str) -> Result<(), String> {
    LibraryAlbumRepository::recount(&mut **tx, album_id)
        .await
        .map_err(|e| format!("Comptes de l'album : {e}"))?;
    Ok(())
}

/// Compteurs d'albums et de pistes d'une bibliothèque, après suppressions et changements d'artiste
/// (aucun déclencheur ne couvre le second cas).
pub(crate) async fn recount_library(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, library_id: i64) -> Result<(), String> {
    LibraryArtistRepository::recount_albums(&mut **tx, library_id)
        .await
        .map_err(|e| format!("Compteur d'albums par artiste : {e}"))?;

    LibraryArtistRepository::recount_tracks(&mut **tx, library_id)
        .await
        .map_err(|e| format!("Compteur de pistes par artiste : {e}"))?;

    LibraryRepository::recount_albums(&mut **tx, library_id)
        .await
        .map_err(|e| format!("Compteur d'albums : {e}"))?;
    Ok(())
}

/// L'artiste portant ce nom, créé et rattaché à la bibliothèque s'il manque.
pub(crate) async fn resolve_artist(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    known_items: &mut HashMap<String, String>,
    name: &str,
    library_id: i64,
) -> Result<String, String> {
    let key = normalize_name(name);
    let artist_id = match known_items.get(&key) {
        Some(id) => id.clone(),
        None => {
            let created = ArtistRepository::insert_artist(
                &mut **tx,
                ArtistCreate {
                    name: name.to_string(),
                    name_normalized: key.clone(),
                    sort_name: normalize_sort_name(name),
                },
            )
            .await
            .map_err(|e| format!("Création de l'artiste « {name} » : {e}"))?;
            known_items.insert(key, created.id.clone());
            created.id
        }
    };

    LibraryArtistRepository::insert_library_artist(
        &mut **tx,
        LibraryArtistCreate {
            library_id,
            artist_id: artist_id.clone(),
        },
    )
    .await
    .map_err(|e| format!("Rattachement de « {name} » à la bibliothèque : {e}"))?;

    Ok(artist_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn base() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("base mémoire");
        sqlx::migrate!("./SQL/").run(&pool).await.expect("migrations");
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')")
            .execute(&pool).await.expect("profil");
        sqlx::query("INSERT OR IGNORE INTO library (id, profil_id, name) VALUES (1, 1, 'test')")
            .execute(&pool).await.expect("bibliothèque");
        pool
    }

    async fn insert_artist(pool: &SqlitePool, id: &str, name: &str) {
        sqlx::query("INSERT INTO artists (id, name, name_normalized, sort_name) VALUES (?, ?, ?, ?)")
            .bind(id).bind(name).bind(normalize_name(name)).bind(name)
            .execute(pool).await.expect("artiste");
        sqlx::query("INSERT INTO library_artists (library_id, artist_id) VALUES (1, ?)")
            .bind(id).execute(pool).await.expect("lien bibliothèque");
    }

    /// Une fiche d'album telle que l'ancienne indexation la posait : une par
    /// artiste, `album_dir` vide.
    async fn insert_album(pool: &SqlitePool, id: &str, title: &str, owner_id: &str) {
        sqlx::query("INSERT INTO library_albums (id, library_id, artist_id, title, title_normalized) VALUES (?, 1, ?, ?, ?)")
            .bind(id).bind(owner_id).bind(title).bind(normalize_name(title))
            .execute(pool).await.expect("album");
    }

    async fn insert_track(pool: &SqlitePool, id: &str, album: &str, artist_key: &str, path_part: &str, tag_album: Option<&str>) {
        sqlx::query("INSERT INTO library_cache (path, album_artist) VALUES (?, ?)")
            .bind(path_part).bind(tag_album)
            .execute(pool).await.expect("cache");
        let cache_id: i64 = sqlx::query_scalar("SELECT id FROM library_cache WHERE path = ?")
            .bind(path_part).fetch_one(pool).await.expect("id de cache");
        sqlx::query("INSERT INTO library_files (id, library_id, path, filename, extension, size, status) VALUES (?, 1, ?, 'f', 'flac', 1, 'indexed')")
            .bind(format!("f-{id}")).bind(path_part)
            .execute(pool).await.expect("fichier");
        sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, cache_id, library_album_id, artist_id, title, title_normalized) VALUES (?, 1, ?, ?, ?, ?, 'T', 't')")
            .bind(id).bind(format!("f-{id}")).bind(cache_id).bind(album).bind(artist_key)
            .execute(pool).await.expect("piste");
    }

    async fn albums(pool: &SqlitePool) -> Vec<(String, String, String)> {
        sqlx::query_as::<_, (String, String, String)>(
            "SELECT b.title, a.name, b.album_type FROM library_albums b
             JOIN artists a ON a.id = b.artist_id ORDER BY b.title")
            .fetch_all(pool).await.expect("albums")
    }

    /// Deux fiches, deux artistes, un seul dossier : le cas « NRJ ».
    async fn split_compilation(pool: &SqlitePool, folder1: &str, folder2: &str) {
        insert_artist(pool, "a1", "Ava Max").await;
        insert_artist(pool, "a2", "Soprano").await;
        insert_album(pool, "alb1", "NRJ Hits", "a1").await;
        insert_album(pool, "alb2", "NRJ Hits", "a2").await;
        insert_track(pool, "t1", "alb1", "a1", folder1, None).await;
        insert_track(pool, "t2", "alb2", "a2", folder2, None).await;
    }

    #[tokio::test]
    async fn a_split_compilation_becomes_one_album_again() {
        let pool = base().await;
        split_compilation(&pool, r"S:\M\NRJ Hits\01.flac", r"S:\M\NRJ Hits\02.flac").await;

        let r = consolidate(&pool).await.expect("consolidation");
        assert_eq!((r.groups_merged, r.albums_removed, r.compilations), (1, 1, 1));
        assert_eq!(albums(&pool).await,
                   vec![("NRJ Hits".into(), "Various Artists".into(), "compilation".into())]);

        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM library_tracks WHERE library_album_id = 'alb1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(n, 2, "les deux pistes auraient dû suivre");
    }

    #[tokio::test]
    async fn box_set_discs_come_together() {
        let pool = base().await;
        split_compilation(&pool, r"S:\M\NRJ Hits\CD1\01.flac", r"S:\M\NRJ Hits\CD2\01.flac").await;

        assert_eq!(consolidate(&pool).await.expect("consolidation").albums_removed, 1);
    }

    #[tokio::test]
    async fn two_same_named_albums_stay_apart() {
        let pool = base().await;
        insert_artist(&pool, "a1", "AC/DC").await;
        insert_artist(&pool, "a2", "Suprême NTM").await;
        insert_album(&pool, "alb1", "Best Of", "a1").await;
        insert_album(&pool, "alb2", "Best Of", "a2").await;
        insert_track(&pool, "t1", "alb1", "a1", r"S:\M\ACDC\Best Of\01.flac", Some("AC/DC")).await;
        insert_track(&pool, "t2", "alb2", "a2", r"S:\M\NTM\Best Of\01.flac", Some("Suprême NTM")).await;

        let r = consolidate(&pool).await.expect("consolidation");
        assert_eq!((r.groups_merged, r.albums_removed), (0, 0));
        assert_eq!(albums(&pool).await.len(), 2);
    }

    #[tokio::test]
    async fn a_lone_featuring_does_not_make_a_compilation() {
        let pool = base().await;
        insert_artist(&pool, "a1", "Sam Smith").await;
        insert_artist(&pool, "a2", "Naughty Boy").await;
        insert_album(&pool, "alb1", "In The Lonely Hour", "a1").await;
        insert_album(&pool, "alb2", "In The Lonely Hour", "a2").await;
        let d = r"S:\M\Sam Smith\In The Lonely Hour";
        insert_track(&pool, "t1", "alb1", "a1", &format!(r"{d}\01.flac"), Some("Sam Smith")).await;
        insert_track(&pool, "t2", "alb1", "a1", &format!(r"{d}\02.flac"), Some("Sam Smith")).await;
        insert_track(&pool, "t3", "alb2", "a2", &format!(r"{d}\03.flac"), Some("Naughty Boy")).await;

        let r = consolidate(&pool).await.expect("consolidation");
        assert_eq!((r.groups_merged, r.compilations), (1, 0));
        assert_eq!(albums(&pool).await,
                   vec![("In The Lonely Hour".into(), "Sam Smith".into(), "album".into())],
                   "le tag majoritaire garde l'album");
    }

    #[tokio::test]
    async fn rerunning_changes_nothing_more() {
        let pool = base().await;
        split_compilation(&pool, r"S:\M\NRJ Hits\01.flac", r"S:\M\NRJ Hits\02.flac").await;

        consolidate(&pool).await.expect("premier passage");
        let second = consolidate(&pool).await.expect("second passage");
        assert_eq!((second.albums_seen, second.albums_removed), (0, 0));
    }

    #[tokio::test]
    async fn counters_follow_the_merge() {
        let pool = base().await;
        split_compilation(&pool, r"S:\M\NRJ Hits\01.flac", r"S:\M\NRJ Hits\02.flac").await;
        consolidate(&pool).await.expect("consolidation");

        let tracks_list: i64 = sqlx::query_scalar("SELECT total_tracks FROM library_albums WHERE id = 'alb1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(tracks_list, 2);

        // Ava Max ne possède plus l'album : son compteur est retombé.
        let n: i64 = sqlx::query_scalar("SELECT total_albums FROM library_artists WHERE artist_id = 'a1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn a_multi_name_tag_does_not_create_a_hybrid_artist() {
        let pool = base().await;
        insert_artist(&pool, "a1", "Ava Max").await;
        insert_artist(&pool, "a2", "Soprano").await;
        insert_album(&pool, "alb1", "B.O.", "a1").await;
        insert_album(&pool, "alb2", "B.O.", "a2").await;
        insert_track(&pool, "t1", "alb1", "a1", r"S:\M\BO\01.flac", Some("Divers; Divers")).await;
        insert_track(&pool, "t2", "alb2", "a2", r"S:\M\BO\02.flac", Some("Divers; Divers")).await;

        consolidate(&pool).await.expect("consolidation");
        let names: Vec<String> = sqlx::query_scalar("SELECT name FROM artists ORDER BY name")
            .fetch_all(&pool).await.unwrap();
        assert!(!names.iter().any(|n| n.contains(';')), "artistes : {names:?}");
        assert_eq!(albums(&pool).await,
                   vec![("B.O.".into(), "Divers".into(), "album".into())]);
    }

    #[tokio::test]
    async fn the_deleted_record_artists_follow() {
        let pool = base().await;
        insert_artist(&pool, "a1", "Sam Smith").await;
        insert_artist(&pool, "a2", "Naughty Boy").await;
        insert_album(&pool, "alb1", "Lonely Hour", "a1").await;
        insert_album(&pool, "alb2", "Lonely Hour", "a2").await;
        insert_track(&pool, "t1", "alb1", "a1", r"S:\M\Sam\01.flac", Some("Sam Smith")).await;
        insert_track(&pool, "t2", "alb2", "a2", r"S:\M\Sam\02.flac", Some("Sam Smith")).await;
        sqlx::query("INSERT INTO library_album_artists (id, library_id, library_album_id, artist_id) VALUES ('x', 1, 'alb2', 'a2')")
            .execute(&pool).await.expect("liaison d'album");

        consolidate(&pool).await.expect("consolidation");
        let remaining_items: Vec<(String, String)> = sqlx::query_as(
            "SELECT library_album_id, artist_id FROM library_album_artists")
            .fetch_all(&pool).await.unwrap();
        assert_eq!(remaining_items, vec![("alb1".to_string(), "a2".to_string())],
                   "la liaison aurait dû suivre la fusion, pas disparaître");
    }

    #[tokio::test]
    async fn the_deleted_record_pins_follow() {
        let pool = base().await;
        split_compilation(&pool, r"S:\M\NRJ Hits\01.flac", r"S:\M\NRJ Hits\02.flac").await;
        sqlx::query("INSERT INTO library_pins (library_id, library_album_id) VALUES (1, 'alb2')")
            .execute(&pool).await.expect("épingle");

        consolidate(&pool).await.expect("consolidation");
        let remaining: Vec<String> = sqlx::query_scalar("SELECT library_album_id FROM library_pins")
            .fetch_all(&pool).await.unwrap();
        assert_eq!(remaining, vec!["alb1".to_string()]);
    }

    #[tokio::test]
    async fn an_album_without_tracks_is_not_reread_forever() {
        let pool = base().await;
        insert_artist(&pool, "a1", "Ava Max").await;
        insert_album(&pool, "alb1", "Orphelin", "a1").await;

        assert_eq!(consolidate(&pool).await.expect("1er").albums_seen, 1);
        assert_eq!(consolidate(&pool).await.expect("2e").albums_seen, 0);
    }
}
