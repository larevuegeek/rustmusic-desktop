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
use crate::repository::library::library_artist_repository::LibraryArtistRepository;
use crate::service::library::album_folder::racine_album;

/// Le nom donné à un album dont les pistes n'ont pas d'artiste commun.
const ARTISTES_DIVERS: &str = "Various Artists";

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

#[derive(sqlx::FromRow)]
struct Ligne {
    album_id: String,
    library_id: i64,
    title_normalized: String,
    artist_id: String,
    cover_url: Option<String>,
    path: Option<String>,
    track_artist: Option<String>,
    tag_album: Option<String>,
}

/// Une fiche d'album et ce que ses pistes disent d'elle.
#[derive(Default)]
struct Fiche {
    id: String,
    library_id: i64,
    titre: String,
    artist_id: String,
    cover: Option<String>,
    dossier: String,
    pistes: i64,
    artistes: Vec<String>,
    tags: Vec<String>,
}

pub async fn consolider(pool: &SqlitePool) -> Result<AlbumMergeReport, String> {
    let lignes: Vec<Ligne> = sqlx::query_as::<_, Ligne>(
        r#"
        SELECT b.id               AS album_id,
               b.library_id       AS library_id,
               b.title_normalized AS title_normalized,
               b.artist_id        AS artist_id,
               b.cover_url        AS cover_url,
               f.path             AS path,
               t.artist_id        AS track_artist,
               lc.album_artist    AS tag_album
        FROM library_albums b
        LEFT JOIN library_tracks t ON t.library_album_id = b.id
        LEFT JOIN library_files f ON f.id = t.file_id
        LEFT JOIN library_cache lc ON lc.id = t.cache_id
        WHERE b.album_dir IS NULL
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Lecture des albums : {e}"))?;

    if lignes.is_empty() {
        return Ok(AlbumMergeReport::default());
    }

    // ─── Une fiche par album, nourrie de ses pistes ───
    let mut fiches: HashMap<String, Fiche> = HashMap::new();
    for l in lignes {
        let f = fiches.entry(l.album_id.clone()).or_insert_with(|| Fiche {
            id: l.album_id.clone(),
            library_id: l.library_id,
            titre: l.title_normalized.clone(),
            artist_id: l.artist_id.clone(),
            cover: l.cover_url.clone(),
            ..Default::default()
        });
        if let Some(p) = l.path.as_deref() {
            if f.pistes == 0 {
                f.dossier = racine_album(p);
            }
            f.pistes += 1;
        }
        if let Some(a) = l.track_artist {
            f.artistes.push(a);
        }
        // Le tag peut nommer plusieurs artistes : seul le premier porte l'album.
        if let Some(t) = l.tag_album {
            if let Some(premier) = split_artists(&t).into_iter().next() {
                f.tags.push(premier);
            }
        }
    }

    let mut rapport = AlbumMergeReport {
        albums_seen: fiches.len() as i64,
        ..Default::default()
    };

    // ─── Regroupement sur la nouvelle identité ───
    let mut groupes: HashMap<(i64, String, String), Vec<Fiche>> = HashMap::new();
    for (_, f) in fiches {
        groupes
            .entry((f.library_id, f.titre.clone(), f.dossier.clone()))
            .or_default()
            .push(f);
    }

    let mut connus: HashMap<String, String> =
        sqlx::query_as::<_, (String, String)>("SELECT name_normalized, id FROM artists")
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Lecture des artistes : {e}"))?
            .into_iter()
            .collect();

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Ouverture de la transaction : {e}"))?;

    let mut bibliotheques: Vec<i64> = Vec::new();

    for ((library_id, _, dossier), mut groupe) in groupes {
        if !bibliotheques.contains(&library_id) {
            bibliotheques.push(library_id);
        }

        // La fiche qui a déjà une pochette, puis la mieux fournie.
        groupe.sort_by(|a, b| {
            b.cover
                .is_some()
                .cmp(&a.cover.is_some())
                .then(b.pistes.cmp(&a.pistes))
                .then(a.id.cmp(&b.id))
        });

        let garde = groupe[0].id.clone();

        if groupe.len() > 1 {
            for perdue in &groupe[1..] {
                sqlx::query("UPDATE library_tracks SET library_album_id = ? WHERE library_album_id = ?")
                    .bind(&garde)
                    .bind(&perdue.id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Rattachement des pistes : {e}"))?;

                // Sans ça la cascade les emporterait avec la fiche.
                sqlx::query(
                    "INSERT OR IGNORE INTO library_album_artists (library_id, library_album_id, artist_id, role)
                     SELECT library_id, ?1, artist_id, role
                     FROM library_album_artists WHERE library_album_id = ?2",
                )
                .bind(&garde)
                .bind(&perdue.id)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Report des artistes d'album : {e}"))?;
                sqlx::query("DELETE FROM library_albums WHERE id = ?")
                    .bind(&perdue.id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Suppression de la fiche en double : {e}"))?;
                rapport.albums_removed += 1;
            }
            rapport.groups_merged += 1;

            // ─── À qui appartient l'album fusionné ? ───
            let mut comptes: HashMap<&str, i64> = HashMap::new();
            for f in &groupe {
                for t in &f.tags {
                    *comptes.entry(t.as_str()).or_insert(0) += 1;
                }
            }
            let artistes: std::collections::HashSet<&str> = groupe
                .iter()
                .flat_map(|f| f.artistes.iter().map(|s| s.as_str()))
                .collect();

            // Le tag majoritaire tranche ; un featuring isolé ne fait pas d'une
            // pochette une compilation.
            let proprietaire = if let Some((tag, _)) =
                comptes.iter().max_by_key(|(nom, n)| (**n, std::cmp::Reverse(*nom)))
            {
                resoudre_artiste(&mut tx, &mut connus, tag, library_id).await?
            } else if artistes.len() > 1 {
                rapport.compilations += 1;
                sqlx::query("UPDATE library_albums SET album_type = 'compilation' WHERE id = ?")
                    .bind(&garde)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Type de l'album : {e}"))?;
                resoudre_artiste(&mut tx, &mut connus, ARTISTES_DIVERS, library_id).await?
            } else {
                groupe[0].artist_id.clone()
            };

            sqlx::query("UPDATE library_albums SET artist_id = ? WHERE id = ?")
                .bind(&proprietaire)
                .bind(&garde)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Propriétaire de l'album : {e}"))?;

            // Les déclencheurs comptent les pistes insérées, pas déplacées.
            sqlx::query(
                "UPDATE library_albums SET
                   total_tracks = (SELECT COUNT(*) FROM library_tracks WHERE library_album_id = ?1),
                   total_duration = (SELECT COALESCE(SUM(duration), 0) FROM library_tracks WHERE library_album_id = ?1)
                 WHERE id = ?1",
            )
            .bind(&garde)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Comptes de l'album : {e}"))?;
        }

        // Posé en dernier : c'est lui qui rend la fiche invisible au prochain tour.
        sqlx::query("UPDATE library_albums SET album_dir = ? WHERE id = ?")
            .bind(&dossier)
            .bind(&garde)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Dossier de l'album : {e}"))?;
    }

    // Les compteurs d'albums ont vu passer des suppressions et des changements
    // d'artiste ; aucun déclencheur ne couvre le second cas.
    // Un décompte groupé, pas une sous-requête par artiste : corrélée, celle-ci
    // relisait toute la bibliothèque pour chacun de ses 941 artistes — 10,9 s.
    for library_id in &bibliotheques {
        sqlx::query(
            "UPDATE library_artists SET total_albums = COALESCE((
                SELECT c.n FROM (
                    SELECT library_id, artist_id, COUNT(*) AS n
                    FROM library_albums GROUP BY library_id, artist_id
                ) c
                WHERE c.library_id = library_artists.library_id
                  AND c.artist_id = library_artists.artist_id), 0)
             WHERE library_id = ?",
        )
        .bind(library_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Compteur d'albums par artiste : {e}"))?;

        sqlx::query(
            "UPDATE library_artists SET total_tracks = COALESCE((
                SELECT c.n FROM (
                    SELECT library_id, artist_id, COUNT(*) AS n
                    FROM library_tracks WHERE artist_id IS NOT NULL
                    GROUP BY library_id, artist_id
                ) c
                WHERE c.library_id = library_artists.library_id
                  AND c.artist_id = library_artists.artist_id), 0)
             WHERE library_id = ?",
        )
        .bind(library_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Compteur de pistes par artiste : {e}"))?;

        sqlx::query(
            "UPDATE library SET total_albums =
               (SELECT COUNT(*) FROM library_albums b WHERE b.library_id = library.id)
             WHERE id = ?",
        )
        .bind(library_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Compteur d'albums : {e}"))?;
    }

    tx.commit()
        .await
        .map_err(|e| format!("Validation : {e}"))?;

    if rapport.groups_merged > 0 {
        log::info!(
            "💿 Albums rassemblés : {} groupe(s), {} fiche(s) en double supprimée(s), \
             {} compilation(s)",
            rapport.groups_merged,
            rapport.albums_removed,
            rapport.compilations
        );
    }

    Ok(rapport)
}

/// L'artiste portant ce nom, créé et rattaché à la bibliothèque s'il manque.
async fn resoudre_artiste(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    connus: &mut HashMap<String, String>,
    nom: &str,
    library_id: i64,
) -> Result<String, String> {
    let cle = normalize_name(nom);
    let artist_id = match connus.get(&cle) {
        Some(id) => id.clone(),
        None => {
            let cree = ArtistRepository::insert_artist(
                &mut **tx,
                ArtistCreate {
                    name: nom.to_string(),
                    name_normalized: cle.clone(),
                    sort_name: normalize_sort_name(nom),
                },
            )
            .await
            .map_err(|e| format!("Création de l'artiste « {nom} » : {e}"))?;
            connus.insert(cle, cree.id.clone());
            cree.id
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
    .map_err(|e| format!("Rattachement de « {nom} » à la bibliothèque : {e}"))?;

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

    async fn artiste(pool: &SqlitePool, id: &str, nom: &str) {
        sqlx::query("INSERT INTO artists (id, name, name_normalized, sort_name) VALUES (?, ?, ?, ?)")
            .bind(id).bind(nom).bind(normalize_name(nom)).bind(nom)
            .execute(pool).await.expect("artiste");
        sqlx::query("INSERT INTO library_artists (library_id, artist_id) VALUES (1, ?)")
            .bind(id).execute(pool).await.expect("lien bibliothèque");
    }

    /// Une fiche d'album telle que l'ancienne indexation la posait : une par
    /// artiste, `album_dir` vide.
    async fn fiche(pool: &SqlitePool, id: &str, titre: &str, proprio: &str) {
        sqlx::query("INSERT INTO library_albums (id, library_id, artist_id, title, title_normalized) VALUES (?, 1, ?, ?, ?)")
            .bind(id).bind(proprio).bind(titre).bind(normalize_name(titre))
            .execute(pool).await.expect("album");
    }

    async fn piste(pool: &SqlitePool, id: &str, album: &str, artiste_id: &str, chemin: &str, tag_album: Option<&str>) {
        sqlx::query("INSERT INTO library_cache (path, album_artist) VALUES (?, ?)")
            .bind(chemin).bind(tag_album)
            .execute(pool).await.expect("cache");
        let cache_id: i64 = sqlx::query_scalar("SELECT id FROM library_cache WHERE path = ?")
            .bind(chemin).fetch_one(pool).await.expect("id de cache");
        sqlx::query("INSERT INTO library_files (id, library_id, path, filename, extension, size, status) VALUES (?, 1, ?, 'f', 'flac', 1, 'indexed')")
            .bind(format!("f-{id}")).bind(chemin)
            .execute(pool).await.expect("fichier");
        sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, cache_id, library_album_id, artist_id, title, title_normalized) VALUES (?, 1, ?, ?, ?, ?, 'T', 't')")
            .bind(id).bind(format!("f-{id}")).bind(cache_id).bind(album).bind(artiste_id)
            .execute(pool).await.expect("piste");
    }

    async fn albums(pool: &SqlitePool) -> Vec<(String, String, String)> {
        sqlx::query_as::<_, (String, String, String)>(
            "SELECT b.title, a.name, b.album_type FROM library_albums b
             JOIN artists a ON a.id = b.artist_id ORDER BY b.title")
            .fetch_all(pool).await.expect("albums")
    }

    /// Deux fiches, deux artistes, un seul dossier : le cas « NRJ ».
    async fn compilation_eclatee(pool: &SqlitePool, dossier1: &str, dossier2: &str) {
        artiste(pool, "a1", "Ava Max").await;
        artiste(pool, "a2", "Soprano").await;
        fiche(pool, "alb1", "NRJ Hits", "a1").await;
        fiche(pool, "alb2", "NRJ Hits", "a2").await;
        piste(pool, "t1", "alb1", "a1", dossier1, None).await;
        piste(pool, "t2", "alb2", "a2", dossier2, None).await;
    }

    #[tokio::test]
    async fn une_compilation_eclatee_redevient_un_album() {
        let pool = base().await;
        compilation_eclatee(&pool, r"S:\M\NRJ Hits\01.flac", r"S:\M\NRJ Hits\02.flac").await;

        let r = consolider(&pool).await.expect("consolidation");
        assert_eq!((r.groups_merged, r.albums_removed, r.compilations), (1, 1, 1));
        assert_eq!(albums(&pool).await,
                   vec![("NRJ Hits".into(), "Various Artists".into(), "compilation".into())]);

        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM library_tracks WHERE library_album_id = 'alb1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(n, 2, "les deux pistes auraient dû suivre");
    }

    #[tokio::test]
    async fn les_disques_d_un_coffret_se_rejoignent() {
        let pool = base().await;
        compilation_eclatee(&pool, r"S:\M\NRJ Hits\CD1\01.flac", r"S:\M\NRJ Hits\CD2\01.flac").await;

        assert_eq!(consolider(&pool).await.expect("consolidation").albums_removed, 1);
    }

    #[tokio::test]
    async fn deux_albums_homonymes_restent_separes() {
        let pool = base().await;
        artiste(&pool, "a1", "AC/DC").await;
        artiste(&pool, "a2", "Suprême NTM").await;
        fiche(&pool, "alb1", "Best Of", "a1").await;
        fiche(&pool, "alb2", "Best Of", "a2").await;
        piste(&pool, "t1", "alb1", "a1", r"S:\M\ACDC\Best Of\01.flac", Some("AC/DC")).await;
        piste(&pool, "t2", "alb2", "a2", r"S:\M\NTM\Best Of\01.flac", Some("Suprême NTM")).await;

        let r = consolider(&pool).await.expect("consolidation");
        assert_eq!((r.groups_merged, r.albums_removed), (0, 0));
        assert_eq!(albums(&pool).await.len(), 2);
    }

    #[tokio::test]
    async fn un_featuring_isole_ne_fait_pas_une_compilation() {
        let pool = base().await;
        artiste(&pool, "a1", "Sam Smith").await;
        artiste(&pool, "a2", "Naughty Boy").await;
        fiche(&pool, "alb1", "In The Lonely Hour", "a1").await;
        fiche(&pool, "alb2", "In The Lonely Hour", "a2").await;
        let d = r"S:\M\Sam Smith\In The Lonely Hour";
        piste(&pool, "t1", "alb1", "a1", &format!(r"{d}\01.flac"), Some("Sam Smith")).await;
        piste(&pool, "t2", "alb1", "a1", &format!(r"{d}\02.flac"), Some("Sam Smith")).await;
        piste(&pool, "t3", "alb2", "a2", &format!(r"{d}\03.flac"), Some("Naughty Boy")).await;

        let r = consolider(&pool).await.expect("consolidation");
        assert_eq!((r.groups_merged, r.compilations), (1, 0));
        assert_eq!(albums(&pool).await,
                   vec![("In The Lonely Hour".into(), "Sam Smith".into(), "album".into())],
                   "le tag majoritaire garde l'album");
    }

    #[tokio::test]
    async fn relancer_ne_change_plus_rien() {
        let pool = base().await;
        compilation_eclatee(&pool, r"S:\M\NRJ Hits\01.flac", r"S:\M\NRJ Hits\02.flac").await;

        consolider(&pool).await.expect("premier passage");
        let deuxieme = consolider(&pool).await.expect("second passage");
        assert_eq!((deuxieme.albums_seen, deuxieme.albums_removed), (0, 0));
    }

    #[tokio::test]
    async fn les_compteurs_suivent_la_fusion() {
        let pool = base().await;
        compilation_eclatee(&pool, r"S:\M\NRJ Hits\01.flac", r"S:\M\NRJ Hits\02.flac").await;
        consolider(&pool).await.expect("consolidation");

        let pistes: i64 = sqlx::query_scalar("SELECT total_tracks FROM library_albums WHERE id = 'alb1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(pistes, 2);

        // Ava Max ne possède plus l'album : son compteur est retombé.
        let n: i64 = sqlx::query_scalar("SELECT total_albums FROM library_artists WHERE artist_id = 'a1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn un_tag_a_plusieurs_noms_ne_cree_pas_un_artiste_batard() {
        let pool = base().await;
        artiste(&pool, "a1", "Ava Max").await;
        artiste(&pool, "a2", "Soprano").await;
        fiche(&pool, "alb1", "B.O.", "a1").await;
        fiche(&pool, "alb2", "B.O.", "a2").await;
        piste(&pool, "t1", "alb1", "a1", r"S:\M\BO\01.flac", Some("Divers; Divers")).await;
        piste(&pool, "t2", "alb2", "a2", r"S:\M\BO\02.flac", Some("Divers; Divers")).await;

        consolider(&pool).await.expect("consolidation");
        let noms: Vec<String> = sqlx::query_scalar("SELECT name FROM artists ORDER BY name")
            .fetch_all(&pool).await.unwrap();
        assert!(!noms.iter().any(|n| n.contains(';')), "artistes : {noms:?}");
        assert_eq!(albums(&pool).await,
                   vec![("B.O.".into(), "Divers".into(), "album".into())]);
    }

    #[tokio::test]
    async fn les_artistes_de_la_fiche_supprimee_suivent() {
        let pool = base().await;
        artiste(&pool, "a1", "Sam Smith").await;
        artiste(&pool, "a2", "Naughty Boy").await;
        fiche(&pool, "alb1", "Lonely Hour", "a1").await;
        fiche(&pool, "alb2", "Lonely Hour", "a2").await;
        piste(&pool, "t1", "alb1", "a1", r"S:\M\Sam\01.flac", Some("Sam Smith")).await;
        piste(&pool, "t2", "alb2", "a2", r"S:\M\Sam\02.flac", Some("Sam Smith")).await;
        sqlx::query("INSERT INTO library_album_artists (id, library_id, library_album_id, artist_id) VALUES ('x', 1, 'alb2', 'a2')")
            .execute(&pool).await.expect("liaison d'album");

        consolider(&pool).await.expect("consolidation");
        let restants: Vec<(String, String)> = sqlx::query_as(
            "SELECT library_album_id, artist_id FROM library_album_artists")
            .fetch_all(&pool).await.unwrap();
        assert_eq!(restants, vec![("alb1".to_string(), "a2".to_string())],
                   "la liaison aurait dû suivre la fusion, pas disparaître");
    }

    #[tokio::test]
    async fn un_album_sans_piste_n_est_pas_relu_indefiniment() {
        let pool = base().await;
        artiste(&pool, "a1", "Ava Max").await;
        fiche(&pool, "alb1", "Orphelin", "a1").await;

        assert_eq!(consolider(&pool).await.expect("1er").albums_seen, 1);
        assert_eq!(consolider(&pool).await.expect("2e").albums_seen, 0);
    }
}
