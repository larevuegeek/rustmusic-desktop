//! Relecture d'un fichier d'export.
//!
//! # Retrouver les morceaux, pas seulement les chemins
//! Une playlist exportée ne contient pas d'identifiants — ils n'auraient aucun
//! sens ici. Chaque piste est décrite par son chemin et par de quoi la
//! reconnaître : titre, artiste, album, durée.
//!
//! L'appariement se fait donc en deux temps. D'abord le chemin, qui répond dans
//! le cas courant — même machine, même bibliothèque. Puis, s'il ne donne rien,
//! les tags : c'est ce qui sauve une bibliothèque déplacée de `S:\` vers `D:\`,
//! où tous les chemins ont changé d'un coup sans qu'un seul morceau ait bougé.
//!
//! # Rien n'est détruit sans qu'on l'ait demandé
//! Une playlist qui porte déjà ce nom est laissée telle quelle, et signalée
//! comme ignorée. Écraser par défaut ferait d'une fausse manœuvre une perte
//! définitive. Le remplacement existe, mais il se demande.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;

use super::export_service::{ExportBundle, ExportTrackRef, FORMAT, VERSION};

/// Réglages qu'on n'écrit jamais, quoi que dise le fichier.
///
/// `dlna_uuid` identifie ce serveur sur le réseau local. La restaurer sur une
/// seconde machine ferait apparaître deux serveurs sous la même identité, et
/// les appareils qui les découvrent choisiraient l'un ou l'autre au hasard.
const REGLAGES_NON_IMPORTABLES: &[&str] = &["dlna_uuid"];

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ImportOptions {
    pub settings: bool,
    pub playlists: bool,
    pub liked: bool,
    /// Vide et réécrit une playlist qui porte déjà ce nom, au lieu de l'ignorer.
    pub replace_existing: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            settings: true,
            playlists: true,
            liked: true,
            replace_existing: false,
        }
    }
}

/// Ce que l'import a fait, ou ferait.
#[derive(Debug, Default, Serialize)]
pub struct ImportReport {
    pub app_version: String,
    pub exported_at: String,
    pub settings: usize,
    pub settings_skipped: usize,
    pub profils_created: usize,
    pub playlists_created: usize,
    pub playlists_replaced: usize,
    pub playlists_skipped: usize,
    pub tracks_matched_by_path: usize,
    pub tracks_matched_by_tags: usize,
    pub tracks_missing: usize,
    pub liked: usize,
    /// Quelques pistes introuvables, pour que l'utilisateur voie lesquelles.
    pub missing_samples: Vec<String>,
}

/// Nombre d'exemples de pistes introuvables rapportés.
///
/// Assez pour reconnaître un motif — un dossier entier absent, une lettre de
/// lecteur qui a changé — sans noyer le message quand il en manque des milliers.
const EXEMPLES_MANQUANTS: usize = 8;

/// Lit et valide un fichier d'export.
pub fn read_bundle(contenu: &str) -> Result<ExportBundle, String> {
    let bundle: ExportBundle = serde_json::from_str(contenu)
        .map_err(|e| format!("Fichier illisible : {e}"))?;

    if bundle.format != FORMAT {
        return Err(format!(
            "Ce fichier n'est pas un export RustMusic (nature « {} »).",
            bundle.format
        ));
    }

    // Un fichier plus récent peut contenir des notions que cette version ne
    // sait pas placer. Mieux vaut refuser que d'en importer une moitié.
    if bundle.version > VERSION {
        return Err(format!(
            "Fichier écrit par une version plus récente (format {} contre {}). \
             Mets l'application à jour avant de l'importer.",
            bundle.version, VERSION
        ));
    }

    Ok(bundle)
}

/// Index des pistes de la base, pour l'appariement.
struct Index {
    par_chemin: HashMap<String, String>,
    par_tags: HashMap<String, String>,
}

/// Met un chemin sous une forme comparable.
///
/// Windows ne distingue pas la casse et accepte les deux séparateurs : le même
/// fichier peut s'écrire `S:\Musique\a.flac` ou `s:/musique/a.flac`. Comparer
/// les chaînes brutes ferait échouer l'appariement sur des différences qui n'en
/// sont pas.
fn cle_chemin(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

/// Signature d'une piste par ses tags.
///
/// La durée n'y entre pas : deux encodages du même morceau diffèrent souvent
/// d'une seconde, et l'exiger ferait manquer des correspondances justes.
fn cle_tags(titre: Option<&str>, artiste: Option<&str>, album: Option<&str>) -> Option<String> {
    let titre = titre?.trim().to_lowercase();
    if titre.is_empty() {
        return None;
    }
    let artiste = artiste.unwrap_or("").trim().to_lowercase();
    let album = album.unwrap_or("").trim().to_lowercase();
    Some(format!("{titre}\u{1f}{artiste}\u{1f}{album}"))
}

async fn build_index(pool: &SqlitePool) -> Result<Index, sqlx::Error> {
    let lignes: Vec<(String, String, Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT lt.id, lf.path, lt.title, a.name, la.title
               FROM library_tracks lt
               JOIN library_files lf ON lf.id = lt.file_id
          LEFT JOIN library_albums la ON la.id = lt.library_album_id
          LEFT JOIN artists a ON a.id = lt.artist_id",
        )
        .fetch_all(pool)
        .await?;

    let mut par_chemin = HashMap::with_capacity(lignes.len());
    let mut par_tags = HashMap::with_capacity(lignes.len());

    for (id, path, titre, artiste, album) in lignes {
        par_chemin.insert(cle_chemin(&path), id.clone());
        if let Some(cle) = cle_tags(titre.as_deref(), artiste.as_deref(), album.as_deref()) {
            // Première occurrence gardée : plusieurs fichiers peuvent porter les
            // mêmes tags — un même morceau en FLAC et en MP3. N'importe lequel
            // fait l'affaire, mais il en faut un seul, et toujours le même.
            par_tags.entry(cle).or_insert(id);
        }
    }

    Ok(Index {
        par_chemin,
        par_tags,
    })
}

/// Résultat d'un appariement, pour tenir les comptes.
enum Trouvee {
    ParChemin(String),
    ParTags(String),
    Aucune,
}

fn apparier(index: &Index, piste: &ExportTrackRef) -> Trouvee {
    if let Some(id) = index.par_chemin.get(&cle_chemin(&piste.path)) {
        return Trouvee::ParChemin(id.clone());
    }
    if let Some(cle) = cle_tags(
        piste.title.as_deref(),
        piste.artist.as_deref(),
        piste.album.as_deref(),
    ) {
        if let Some(id) = index.par_tags.get(&cle) {
            return Trouvee::ParTags(id.clone());
        }
    }
    Trouvee::Aucune
}

/// Rejoue un export dans la base.
///
/// `dry_run` fait tout le travail d'appariement sans rien écrire : c'est ce qui
/// permet d'annoncer à l'utilisateur ce qui va se passer avant qu'il ne
/// s'engage. Le même code sert les deux, sans quoi l'aperçu finirait par mentir.
pub async fn apply(
    pool: &SqlitePool,
    bundle: &ExportBundle,
    options: ImportOptions,
    dry_run: bool,
) -> Result<ImportReport, sqlx::Error> {
    let index = build_index(pool).await?;

    let mut rapport = ImportReport {
        app_version: bundle.app_version.clone(),
        exported_at: bundle.exported_at.clone(),
        ..Default::default()
    };

    // ─── Réglages ───
    if options.settings {
        for (cle, valeur) in &bundle.settings {
            if REGLAGES_NON_IMPORTABLES.contains(&cle.as_str()) {
                rapport.settings_skipped += 1;
                continue;
            }
            if !dry_run {
                sqlx::query(
                    "INSERT INTO settings (key, value) VALUES (?, ?)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                )
                .bind(cle)
                .bind(valeur)
                .execute(pool)
                .await?;
            }
            rapport.settings += 1;
        }
    }

    for profil in &bundle.profils {
        let profil_id = resoudre_profil(pool, &profil.name, profil, dry_run, &mut rapport).await?;

        // En simulation, un profil absent n'a pas d'identifiant : on ne peut
        // pas apparier ses playlists à des lignes existantes, mais on compte
        // quand même les pistes pour annoncer un total juste.
        if options.playlists {
            for liste in &profil.playlists {
                let existante = match profil_id {
                    Some(pid) => playlist_existante(pool, pid, &liste.name).await?,
                    None => None,
                };

                if existante.is_some() && !options.replace_existing {
                    rapport.playlists_skipped += 1;
                    continue;
                }

                // Apparier d'abord : on veut les comptes même en simulation.
                let mut appariees: Vec<String> = Vec::with_capacity(liste.tracks.len());
                for piste in &liste.tracks {
                    match apparier(&index, piste) {
                        Trouvee::ParChemin(id) => {
                            rapport.tracks_matched_by_path += 1;
                            appariees.push(id);
                        }
                        Trouvee::ParTags(id) => {
                            rapport.tracks_matched_by_tags += 1;
                            appariees.push(id);
                        }
                        Trouvee::Aucune => {
                            rapport.tracks_missing += 1;
                            if rapport.missing_samples.len() < EXEMPLES_MANQUANTS {
                                rapport.missing_samples.push(piste.path.clone());
                            }
                        }
                    }
                }

                if existante.is_some() {
                    rapport.playlists_replaced += 1;
                } else {
                    rapport.playlists_created += 1;
                }

                if dry_run {
                    continue;
                }

                let Some(pid) = profil_id else { continue };

                let playlist_id = match existante {
                    Some(id) => {
                        sqlx::query("DELETE FROM playlist_items WHERE playlist_id = ?")
                            .bind(id)
                            .execute(pool)
                            .await?;
                        id
                    }
                    None => {
                        let (id,): (i64,) = sqlx::query_as(
                            "INSERT INTO playlists
                                 (profil_id, name, description, color, icon, position)
                             VALUES (?, ?, ?, ?, ?, ?)
                             RETURNING id",
                        )
                        .bind(pid)
                        .bind(&liste.name)
                        .bind(&liste.description)
                        .bind(&liste.color)
                        .bind(&liste.icon)
                        .bind(liste.position)
                        .fetch_one(pool)
                        .await?;
                        id
                    }
                };

                for (rang, track_id) in appariees.iter().enumerate() {
                    // `OR IGNORE` : une playlist exportée peut contenir deux
                    // fois le même morceau — par deux chemins différents qui
                    // s'apparient au même fichier — et la contrainte d'unicité
                    // ferait échouer tout l'import pour ça.
                    sqlx::query(
                        "INSERT OR IGNORE INTO playlist_items
                             (playlist_id, library_track_id, sort_index)
                         VALUES (?, ?, ?)",
                    )
                    .bind(playlist_id)
                    .bind(track_id)
                    .bind(rang as i64)
                    .execute(pool)
                    .await?;
                }

                // Les compteurs affichés dans la barre latérale se recalculent
                // ici. Les laisser à zéro donnerait une playlist qui paraît
                // vide alors qu'elle ne l'est pas.
                sqlx::query(
                    "UPDATE playlists SET
                         track_count = (SELECT COUNT(*) FROM playlist_items WHERE playlist_id = ?),
                         duration = CAST(COALESCE((
                             SELECT SUM(lt.duration) FROM playlist_items pi
                             JOIN library_tracks lt ON lt.id = pi.library_track_id
                             WHERE pi.playlist_id = ?), 0) AS INTEGER),
                         updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?",
                )
                .bind(playlist_id)
                .bind(playlist_id)
                .bind(playlist_id)
                .execute(pool)
                .await?;
            }
        }

        // ─── Titres aimés ───
        if options.liked {
            for piste in &profil.liked {
                rapport.liked += 1;
                if dry_run {
                    continue;
                }
                let Some(pid) = profil_id else { continue };

                // Les titres aimés sont désignés par chemin dès l'origine : on
                // conserve le chemin du fichier même s'il ne correspond à aucune
                // piste connue, exactement comme le fait l'application.
                let cache_id: Option<i64> =
                    sqlx::query_scalar("SELECT id FROM library_cache WHERE path = ?")
                        .bind(&piste.path)
                        .fetch_optional(pool)
                        .await?;

                sqlx::query(
                    "INSERT OR IGNORE INTO track_liked (profil_id, path, library_cache_id)
                     VALUES (?, ?, ?)",
                )
                .bind(pid)
                .bind(&piste.path)
                .bind(cache_id)
                .execute(pool)
                .await?;
            }
        }
    }

    Ok(rapport)
}

/// Retrouve un profil par son nom, ou le crée.
///
/// Le nom est la seule identité portable : les identifiants sont locaux, et
/// deux installations n'attribuent pas les mêmes.
async fn resoudre_profil(
    pool: &SqlitePool,
    nom: &str,
    profil: &super::export_service::ExportProfil,
    dry_run: bool,
    rapport: &mut ImportReport,
) -> Result<Option<i64>, sqlx::Error> {
    let existant: Option<i64> = sqlx::query_scalar("SELECT id FROM profil WHERE name = ?")
        .bind(nom)
        .fetch_optional(pool)
        .await?;

    if let Some(id) = existant {
        return Ok(Some(id));
    }

    rapport.profils_created += 1;

    if dry_run {
        return Ok(None);
    }

    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO profil (name, color, avatar, bio, role) VALUES (?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(nom)
    .bind(&profil.color)
    .bind(&profil.avatar)
    .bind(&profil.bio)
    .bind(&profil.role)
    .fetch_one(pool)
    .await?;

    Ok(Some(id))
}

async fn playlist_existante(
    pool: &SqlitePool,
    profil_id: i64,
    nom: &str,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM playlists WHERE profil_id = ? AND name = ?")
        .bind(profil_id)
        .bind(nom)
        .fetch_optional(pool)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_fichier_etranger_est_refuse() {
        let err = read_bundle(r#"{"hello":"world"}"#).unwrap_err();
        assert!(err.contains("illisible"), "message : {err}");
    }

    #[test]
    fn un_json_valide_mais_d_une_autre_nature_est_refuse() {
        // Le marqueur existe pour ça : un JSON qui a la bonne forme mais vient
        // d'ailleurs ne doit pas être rejoué dans la base.
        let faux = r#"{"format":"autre-chose","version":1,"app_version":"1",
                       "exported_at":"","settings":{},"profils":[]}"#;
        let err = read_bundle(faux).unwrap_err();
        assert!(err.contains("n'est pas un export"), "message : {err}");
    }

    #[test]
    fn un_format_plus_recent_est_refuse() {
        // Importer la moitié d'un fichier qu'on ne comprend pas est pire que
        // de ne rien importer.
        let futur = format!(
            r#"{{"format":"{FORMAT}","version":{},"app_version":"9",
                "exported_at":"","settings":{{}},"profils":[]}}"#,
            VERSION + 1
        );
        let err = read_bundle(&futur).unwrap_err();
        assert!(err.contains("plus récente"), "message : {err}");
    }

    #[test]
    fn les_chemins_se_comparent_sans_casse_ni_separateur() {
        // Le même fichier s'écrit de plusieurs façons sous Windows.
        assert_eq!(cle_chemin(r"S:\Musique\a.flac"), cle_chemin("s:/musique/A.FLAC"));
        assert_ne!(cle_chemin(r"S:\Musique\a.flac"), cle_chemin(r"S:\Musique\b.flac"));
    }

    #[test]
    fn la_signature_par_tags_ignore_casse_et_espaces() {
        let a = cle_tags(Some(" Bohemian Rhapsody "), Some("Queen"), Some("A Night At The Opera"));
        let b = cle_tags(Some("bohemian rhapsody"), Some("queen"), Some("a night at the opera"));
        assert_eq!(a, b);
        assert!(a.is_some());
    }

    #[test]
    fn une_piste_sans_titre_n_a_pas_de_signature() {
        // Sans titre, la signature ne distinguerait plus rien : deux morceaux
        // du même album s'apparieraient l'un à l'autre.
        assert!(cle_tags(None, Some("Queen"), Some("Opera")).is_none());
        assert!(cle_tags(Some("   "), Some("Queen"), None).is_none());
    }

    #[test]
    fn le_chemin_prime_sur_les_tags() {
        let index = Index {
            par_chemin: [(cle_chemin("S:/m/a.flac"), "par-chemin".to_string())]
                .into_iter()
                .collect(),
            par_tags: [(cle_tags(Some("t"), Some("a"), Some("al")).unwrap(), "par-tags".to_string())]
                .into_iter()
                .collect(),
        };

        let piste = ExportTrackRef {
            path: "S:/m/a.flac".into(),
            title: Some("t".into()),
            artist: Some("a".into()),
            album: Some("al".into()),
            duration: None,
        };

        match apparier(&index, &piste) {
            Trouvee::ParChemin(id) => assert_eq!(id, "par-chemin"),
            _ => panic!("le chemin doit primer"),
        }
    }

    #[test]
    fn les_tags_rattrapent_un_chemin_devenu_faux() {
        // Le cas qui justifie tout le mécanisme : la bibliothèque a changé de
        // lettre de lecteur, aucun chemin ne répond plus.
        let index = Index {
            par_chemin: HashMap::new(),
            par_tags: [(cle_tags(Some("t"), Some("a"), Some("al")).unwrap(), "trouve".to_string())]
                .into_iter()
                .collect(),
        };

        let piste = ExportTrackRef {
            path: r"D:\ailleurs\a.flac".into(),
            title: Some("T".into()),
            artist: Some("A".into()),
            album: Some("AL".into()),
            duration: None,
        };

        match apparier(&index, &piste) {
            Trouvee::ParTags(id) => assert_eq!(id, "trouve"),
            _ => panic!("les tags devaient rattraper"),
        }
    }

    #[test]
    fn une_piste_inconnue_reste_introuvable() {
        let index = Index {
            par_chemin: HashMap::new(),
            par_tags: HashMap::new(),
        };
        let piste = ExportTrackRef {
            path: "x".into(),
            title: Some("t".into()),
            artist: None,
            album: None,
            duration: None,
        };
        assert!(matches!(apparier(&index, &piste), Trouvee::Aucune));
    }

    #[test]
    fn l_identite_dlna_n_est_jamais_importee() {
        // Deux serveurs sous la même identité sur un réseau, c'est un défaut
        // qu'on ne diagnostique jamais.
        assert!(REGLAGES_NON_IMPORTABLES.contains(&"dlna_uuid"));
    }

    // ────────────────────────────────────────────────────────────────────
    // Contre une vraie base
    // ────────────────────────────────────────────────────────────────────
    //
    // Les tests ci-dessus valident l'appariement, qui est du calcul. Ceux qui
    // suivent exécutent le SQL, et c'est le seul moyen de le valider : sqlx
    // apparie colonnes et champs à l'exécution, jamais à la compilation. Une
    // requête fausse compile parfaitement.

    use crate::service::export::export_service::{ExportPlaylist, ExportProfil};
    use sqlx::sqlite::SqlitePoolOptions;

    async fn base_de_test() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("base mémoire");

        sqlx::migrate!("./SQL/").run(&pool).await.expect("migrations");
        pool
    }

    /// Pose le profil et la bibliothèque dont dépendent les pistes.
    ///
    /// Les clés étrangères sont actives — sqlx les allume sur chaque connexion —
    /// donc l'ordre d'insertion n'est pas négociable : profil, puis
    /// bibliothèque, puis fichiers et pistes.
    async fn poser_socle(pool: &SqlitePool) {
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')")
            .execute(pool)
            .await
            .expect("profil");
        sqlx::query("INSERT OR IGNORE INTO library (id, profil_id, name) VALUES (1, 1, 'test')")
            .execute(pool)
            .await
            .expect("bibliothèque");
    }

    /// Pose une piste complète — fichier, artiste, piste.
    async fn poser_piste(pool: &SqlitePool, id: &str, path: &str, titre: &str, artiste: &str) {
        poser_socle(pool).await;
        sqlx::query("INSERT INTO library_files (id, library_id, path, filename, extension, size, status) VALUES (?, 1, ?, 'f', 'flac', 1, 'ok')")
            .bind(format!("f-{id}"))
            .bind(path)
            .execute(pool)
            .await
            .expect("fichier");
        // `OR IGNORE` parce que deux pistes peuvent partager un artiste, et que
        // `name_normalized` porte un index unique. Toutes les colonnes NOT NULL
        // sont fournies : `OR IGNORE` avalerait aussi bien une violation de
        // NOT NULL, et l'absence de la ligne ne se verrait qu'au moment où une
        // clé étrangère y renvoie.
        // L'identifiant dérive du **nom**, pas de la piste : `name_normalized`
        // porte un index unique, et deux pistes du même artiste ne créent donc
        // qu'une ligne. Un identifiant par piste ferait rejeter la seconde, que
        // `OR IGNORE` tairait — et la clé étrangère casserait juste après.
        let artiste_id = format!("a-{}", artiste.to_lowercase());
        sqlx::query(
            "INSERT OR IGNORE INTO artists (id, name, name_normalized, sort_name)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&artiste_id)
        .bind(artiste)
        .bind(artiste.to_lowercase())
        .bind(artiste)
        .execute(pool)
        .await
        .expect("artiste");
        sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, artist_id, title, title_normalized, duration) VALUES (?, 1, ?, ?, ?, ?, 200)")
            .bind(id)
            .bind(format!("f-{id}"))
            .bind(&artiste_id)
            .bind(titre)
            .bind(titre.to_lowercase())
            .execute(pool)
            .await
            .expect("piste");
    }

    fn bundle_avec(pistes: Vec<ExportTrackRef>) -> ExportBundle {
        ExportBundle {
            format: FORMAT.to_string(),
            version: VERSION,
            app_version: "0.2.1".into(),
            exported_at: "2026-08-24T00:00:00Z".into(),
            settings: [
                ("theme".to_string(), "light".to_string()),
                ("dlna_uuid".to_string(), "ne-doit-pas-passer".to_string()),
            ]
            .into_iter()
            .collect(),
            profils: vec![ExportProfil {
                name: "David".into(),
                color: "#22c55e".into(),
                avatar: None,
                bio: None,
                role: "admin".into(),
                playlists: vec![ExportPlaylist {
                    name: "Grunge".into(),
                    description: None,
                    color: "#8b5cf6".into(),
                    icon: "mynaui:music".into(),
                    position: 0,
                    tracks: pistes,
                }],
                liked: vec![ExportTrackRef {
                    path: "S:/m/aime.flac".into(),
                    title: Some("Aimé".into()),
                    artist: None,
                    album: None,
                    duration: None,
                }],
            }],
        }
    }

    fn piste_ref(path: &str, titre: &str, artiste: &str) -> ExportTrackRef {
        ExportTrackRef {
            path: path.into(),
            title: Some(titre.into()),
            artist: Some(artiste.into()),
            album: None,
            duration: Some(200.0),
        }
    }

    #[tokio::test]
    async fn un_import_reconstruit_la_playlist_et_ses_pistes() {
        let pool = base_de_test().await;
        poser_piste(&pool, "t1", r"S:\m\a.flac", "A", "Nirvana").await;
        poser_piste(&pool, "t2", r"S:\m\b.flac", "B", "Nirvana").await;

        let bundle = bundle_avec(vec![
            piste_ref(r"S:\m\a.flac", "A", "Nirvana"),
            piste_ref(r"S:\m\b.flac", "B", "Nirvana"),
        ]);

        let r = apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import");

        assert_eq!(r.playlists_created, 1);
        assert_eq!(r.tracks_matched_by_path, 2);
        assert_eq!(r.tracks_missing, 0);

        let (nom, compte): (String, i64) =
            sqlx::query_as("SELECT name, track_count FROM playlists")
                .fetch_one(&pool)
                .await
                .expect("playlist");
        assert_eq!(nom, "Grunge");
        // Le compteur doit refléter le contenu : c'est lui qui s'affiche dans
        // la barre latérale.
        assert_eq!(compte, 2);

        // L'ordre exporté doit être l'ordre restauré.
        let ordre: Vec<(String, i64)> = sqlx::query_as(
            "SELECT library_track_id, sort_index FROM playlist_items ORDER BY sort_index",
        )
        .fetch_all(&pool)
        .await
        .expect("items");
        assert_eq!(ordre[0].0, "t1");
        assert_eq!(ordre[1].0, "t2");
    }

    #[tokio::test]
    async fn le_reglage_dlna_ne_franchit_pas_l_import() {
        let pool = base_de_test().await;
        let bundle = bundle_avec(vec![]);

        let r = apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import");

        assert_eq!(r.settings, 1);
        assert_eq!(r.settings_skipped, 1);

        let uuid: Option<String> =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'dlna_uuid'")
                .fetch_optional(&pool)
                .await
                .expect("lecture");
        assert!(uuid.is_none(), "l'identité DLNA a été importée");
    }

    #[tokio::test]
    async fn une_playlist_existante_est_laissee_intacte() {
        let pool = base_de_test().await;
        poser_piste(&pool, "t1", r"S:\m\a.flac", "A", "Nirvana").await;

        let bundle = bundle_avec(vec![piste_ref(r"S:\m\a.flac", "A", "Nirvana")]);

        // Premier import : la playlist est créée.
        apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import 1");

        // Second import : elle existe, on n'y touche pas.
        let r = apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import 2");

        assert_eq!(r.playlists_skipped, 1);
        assert_eq!(r.playlists_created, 0);

        let combien: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playlists")
            .fetch_one(&pool)
            .await
            .expect("compte");
        assert_eq!(combien, 1, "l'import a dupliqué la playlist");
    }

    #[tokio::test]
    async fn le_remplacement_se_demande_et_ne_duplique_rien() {
        let pool = base_de_test().await;
        poser_piste(&pool, "t1", r"S:\m\a.flac", "A", "Nirvana").await;
        let bundle = bundle_avec(vec![piste_ref(r"S:\m\a.flac", "A", "Nirvana")]);

        apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import 1");

        let options = ImportOptions {
            replace_existing: true,
            ..Default::default()
        };
        let r = apply(&pool, &bundle, options, false).await.expect("import 2");

        assert_eq!(r.playlists_replaced, 1);

        let items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playlist_items")
            .fetch_one(&pool)
            .await
            .expect("compte");
        assert_eq!(items, 1, "le remplacement a empilé au lieu de remplacer");
    }

    #[tokio::test]
    async fn un_chemin_devenu_faux_est_rattrape_par_les_tags() {
        let pool = base_de_test().await;
        // La bibliothèque est sur D:, l'export venait de S:.
        poser_piste(&pool, "t1", r"D:\ailleurs\a.flac", "A", "Nirvana").await;

        let bundle = bundle_avec(vec![piste_ref(r"S:\m\a.flac", "A", "Nirvana")]);

        let r = apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import");

        assert_eq!(r.tracks_matched_by_path, 0);
        assert_eq!(r.tracks_matched_by_tags, 1);
        assert_eq!(r.tracks_missing, 0);
    }

    #[tokio::test]
    async fn l_apercu_n_ecrit_rien() {
        let pool = base_de_test().await;
        poser_piste(&pool, "t1", r"S:\m\a.flac", "A", "Nirvana").await;
        let bundle = bundle_avec(vec![piste_ref(r"S:\m\a.flac", "A", "Nirvana")]);

        let r = apply(&pool, &bundle, ImportOptions::default(), true)
            .await
            .expect("aperçu");

        // Les comptes doivent être justes…
        assert_eq!(r.playlists_created, 1);
        assert_eq!(r.tracks_matched_by_path, 1);

        // …et la base intacte.
        let listes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playlists")
            .fetch_one(&pool)
            .await
            .expect("compte");
        assert_eq!(listes, 0, "l'aperçu a écrit dans la base");

        let theme: Option<String> =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'theme'")
                .fetch_optional(&pool)
                .await
                .expect("lecture");
        assert!(theme.is_none(), "l'aperçu a écrit un réglage");
    }

    #[tokio::test]
    async fn une_piste_introuvable_est_signalee_sans_bloquer_le_reste() {
        let pool = base_de_test().await;
        poser_piste(&pool, "t1", r"S:\m\a.flac", "A", "Nirvana").await;

        let bundle = bundle_avec(vec![
            piste_ref(r"S:\m\a.flac", "A", "Nirvana"),
            piste_ref(r"S:\m\disparu.flac", "Disparu", "Inconnu"),
        ]);

        let r = apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import");

        assert_eq!(r.tracks_missing, 1);
        assert_eq!(r.missing_samples.len(), 1);
        assert!(r.missing_samples[0].contains("disparu"));

        // La playlist existe quand même, avec ce qui a pu être retrouvé.
        let items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playlist_items")
            .fetch_one(&pool)
            .await
            .expect("compte");
        assert_eq!(items, 1);
    }

    #[tokio::test]
    async fn les_titres_aimes_sont_restaures_sans_doublon() {
        let pool = base_de_test().await;
        let bundle = bundle_avec(vec![]);

        apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import 1");
        apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import 2");

        let combien: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM track_liked")
            .fetch_one(&pool)
            .await
            .expect("compte");
        assert_eq!(combien, 1, "un second import a dupliqué les titres aimés");
    }
}
