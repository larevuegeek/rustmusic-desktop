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
use crate::repository::library::library_cache_repository::LibraryCacheRepository;
use crate::repository::library::library_track_repository::LibraryTrackRepository;
use crate::repository::playlist::playlist_item_repository::PlaylistItemRepository;
use crate::repository::playlist::playlist_repository::PlaylistRepository;
use crate::repository::playlist::track_liked_repository::TrackLikedRepository;
use crate::repository::profile::profile_repository::ProfileRepository;
use crate::repository::settings::settings_repository::SettingsRepository;

/// Réglages qu'on n'écrit jamais, quoi que dise le fichier.
///
/// `dlna_uuid` identifie ce serveur sur le réseau local. La restaurer sur une
/// seconde machine ferait apparaître deux serveurs sous la même identité, et
/// les appareils qui les découvrent choisiraient l'un ou l'autre au hasard.
const NON_IMPORTABLE_SETTINGS: &[&str] = &["dlna_uuid"];

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
    #[serde(rename = "profils_created")]
    pub profiles_created: usize,
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
const MISSING_EXAMPLES: usize = 8;

/// Lit et valide un fichier d'export.
pub fn read_bundle(content: &str) -> Result<ExportBundle, String> {
    let bundle: ExportBundle = serde_json::from_str(content)
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
    by_path: HashMap<String, String>,
    by_tags: HashMap<String, String>,
}

/// Met un chemin sous une forme comparable.
///
/// Windows ne distingue pas la casse et accepte les deux séparateurs : le même
/// fichier peut s'écrire `S:\Musique\a.flac` ou `s:/music/a.flac`. Comparer
/// les chaînes brutes ferait échouer l'appariement sur des différences qui n'en
/// sont pas.
fn path_key(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

/// Signature d'une piste par ses tags.
///
/// La durée n'y entre pas : deux encodages du même morceau diffèrent souvent
/// d'une seconde, et l'exiger ferait manquer des correspondances justes.
fn tags_key(title: Option<&str>, artist_name: Option<&str>, album: Option<&str>) -> Option<String> {
    let title = title?.trim().to_lowercase();
    if title.is_empty() {
        return None;
    }
    let artist_name = artist_name.unwrap_or("").trim().to_lowercase();
    let album = album.unwrap_or("").trim().to_lowercase();
    Some(format!("{title}\u{1f}{artist_name}\u{1f}{album}"))
}

async fn build_index(pool: &SqlitePool) -> Result<Index, sqlx::Error> {
    let lines = LibraryTrackRepository::find_all_refs(pool).await?;

    let mut by_path = HashMap::with_capacity(lines.len());
    let mut by_tags = HashMap::with_capacity(lines.len());

    for (id, path, title, artist_name, album) in lines {
        by_path.insert(path_key(&path), id.clone());
        if let Some(key) = tags_key(title.as_deref(), artist_name.as_deref(), album.as_deref()) {
            // Première occurrence gardée : plusieurs fichiers peuvent porter les
            // mêmes tags — un même morceau en FLAC et en MP3. N'importe lequel
            // fait l'affaire, mais il en faut un seul, et toujours le même.
            by_tags.entry(key).or_insert(id);
        }
    }

    Ok(Index {
        by_path,
        by_tags,
    })
}

/// Résultat d'un appariement, pour tenir les comptes.
enum Found {
    ByPath(String),
    ByTags(String),
    NotFound,
}

fn pair_up(index: &Index, track: &ExportTrackRef) -> Found {
    if let Some(id) = index.by_path.get(&path_key(&track.path)) {
        return Found::ByPath(id.clone());
    }
    if let Some(key) = tags_key(
        track.title.as_deref(),
        track.artist.as_deref(),
        track.album.as_deref(),
    ) {
        if let Some(id) = index.by_tags.get(&key) {
            return Found::ByTags(id.clone());
        }
    }
    Found::NotFound
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

    let mut report = ImportReport {
        app_version: bundle.app_version.clone(),
        exported_at: bundle.exported_at.clone(),
        ..Default::default()
    };

    // ─── Réglages ───
    if options.settings {
        for (key, value) in &bundle.settings {
            if NON_IMPORTABLE_SETTINGS.contains(&key.as_str()) {
                report.settings_skipped += 1;
                continue;
            }
            if !dry_run {
                SettingsRepository::set(pool, key, value).await?;
            }
            report.settings += 1;
        }
    }

    for profile in &bundle.profiles {
        let profile_id = resolve_profile(pool, &profile.name, profile, dry_run, &mut report).await?;

        // En simulation, un profil absent n'a pas d'identifiant : on ne peut
        // pas apparier ses playlists à des lignes existantes, mais on compte
        // quand même les pistes pour annoncer un total juste.
        if options.playlists {
            for list in &profile.playlists {
                let existing = match profile_id {
                    Some(pid) => existing_playlist(pool, pid, &list.name).await?,
                    None => None,
                };

                if existing.is_some() && !options.replace_existing {
                    report.playlists_skipped += 1;
                    continue;
                }

                // Apparier d'abord : on veut les comptes même en simulation.
                let mut paired: Vec<String> = Vec::with_capacity(list.tracks.len());
                for track in &list.tracks {
                    match pair_up(&index, track) {
                        Found::ByPath(id) => {
                            report.tracks_matched_by_path += 1;
                            paired.push(id);
                        }
                        Found::ByTags(id) => {
                            report.tracks_matched_by_tags += 1;
                            paired.push(id);
                        }
                        Found::NotFound => {
                            report.tracks_missing += 1;
                            if report.missing_samples.len() < MISSING_EXAMPLES {
                                report.missing_samples.push(track.path.clone());
                            }
                        }
                    }
                }

                if existing.is_some() {
                    report.playlists_replaced += 1;
                } else {
                    report.playlists_created += 1;
                }

                if dry_run {
                    continue;
                }

                let Some(pid) = profile_id else { continue };

                let playlist_id = match existing {
                    Some(id) => {
                        PlaylistItemRepository::delete_all(pool, id).await?;
                        id
                    }
                    None => {
                        PlaylistRepository::insert_playlist(
                            pool,
                            pid,
                            None,
                            list.name.clone(),
                            list.description.clone(),
                            list.color.clone(),
                            list.icon.clone(),
                            None,
                            list.position,
                            0,
                        )
                        .await?
                        .id
                    }
                };

                PlaylistRepository::set_rules_and_flags(pool, playlist_id, list.rules.as_deref(), list.is_mix, list.pinned).await?;

                for (rank, track_id) in paired.iter().enumerate() {
                    // `OR IGNORE` : une playlist exportée peut contenir deux
                    // fois le même morceau — par deux chemins différents qui
                    // s'apparient au même fichier — et la contrainte d'unicité
                    // ferait échouer tout l'import pour ça.
                    PlaylistItemRepository::insert_if_absent(pool, playlist_id, track_id, rank as i64).await?;
                }

                // Les compteurs affichés dans la barre latérale se recalculent
                // ici. Les laisser à zéro donnerait une playlist qui paraît
                // vide alors qu'elle ne l'est pas.
                PlaylistRepository::recalculate_stats(pool, playlist_id).await?;
            }
        }

        // ─── Titres aimés ───
        if options.liked {
            for track in &profile.liked {
                report.liked += 1;
                if dry_run {
                    continue;
                }
                let Some(pid) = profile_id else { continue };

                // Les titres aimés sont désignés par chemin dès l'origine : on
                // conserve le chemin du fichier même s'il ne correspond à aucune
                // piste connue, exactement comme le fait l'application.
                let cache_id = LibraryCacheRepository::get_library_cache_id_by_path(pool, &track.path).await?;
                TrackLikedRepository::insert_if_absent(pool, &track.path, pid, cache_id).await?;
            }
        }
    }

    Ok(report)
}

/// Retrouve un profil par son nom, ou le crée.
///
/// Le nom est la seule identité portable : les identifiants sont locaux, et
/// deux installations n'attribuent pas les mêmes.
async fn resolve_profile(
    pool: &SqlitePool,
    name: &str,
    profile: &super::export_service::ExportProfile,
    dry_run: bool,
    report: &mut ImportReport,
) -> Result<Option<i64>, sqlx::Error> {
    if let Some(id) = ProfileRepository::find_id_by_name(pool, name).await? {
        return Ok(Some(id));
    }

    report.profiles_created += 1;

    if dry_run {
        return Ok(None);
    }

    let id = ProfileRepository::insert_from_export(pool, name, &profile.color, profile.avatar.as_deref(), profile.bio.as_deref(), &profile.role).await?;

    Ok(Some(id))
}

async fn existing_playlist(
    pool: &SqlitePool,
    profile_id: i64,
    name: &str,
) -> Result<Option<i64>, sqlx::Error> {
    PlaylistRepository::find_id_by_name(pool, profile_id, name).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_foreign_file_is_refused() {
        let err = read_bundle(r#"{"hello":"world"}"#).unwrap_err();
        assert!(err.contains("illisible"), "message : {err}");
    }

    #[test]
    fn a_valid_json_of_another_kind_is_refused() {
        // Le marqueur existe pour ça : un JSON qui a la bonne forme mais vient
        // d'ailleurs ne doit pas être rejoué dans la base.
        let fake = r#"{"format":"autre-chose","version":1,"app_version":"1",
                       "exported_at":"","settings":{},"profils":[]}"#;
        let err = read_bundle(fake).unwrap_err();
        assert!(err.contains("n'est pas un export"), "message : {err}");
    }

    #[test]
    fn a_newer_format_is_refused() {
        // Importer la moitié d'un fichier qu'on ne comprend pas est pire que
        // de ne rien importer.
        let future = format!(
            r#"{{"format":"{FORMAT}","version":{},"app_version":"9",
                "exported_at":"","settings":{{}},"profils":[]}}"#,
            VERSION + 1
        );
        let err = read_bundle(&future).unwrap_err();
        assert!(err.contains("plus récente"), "message : {err}");
    }

    #[test]
    fn paths_compare_ignoring_case_and_separator() {
        // Le même fichier s'écrit de plusieurs façons sous Windows.
        assert_eq!(path_key(r"S:\Musique\a.flac"), path_key("s:/musique/A.FLAC"));
        assert_ne!(path_key(r"S:\Musique\a.flac"), path_key(r"S:\Musique\b.flac"));
    }

    #[test]
    fn the_tag_signature_ignores_case_and_spaces() {
        let a = tags_key(Some(" Bohemian Rhapsody "), Some("Queen"), Some("A Night At The Opera"));
        let b = tags_key(Some("bohemian rhapsody"), Some("queen"), Some("a night at the opera"));
        assert_eq!(a, b);
        assert!(a.is_some());
    }

    #[test]
    fn a_track_without_title_has_no_signature() {
        // Sans titre, la signature ne distinguerait plus rien : deux morceaux
        // du même album s'apparieraient l'un à l'autre.
        assert!(tags_key(None, Some("Queen"), Some("Opera")).is_none());
        assert!(tags_key(Some("   "), Some("Queen"), None).is_none());
    }

    #[test]
    fn the_path_wins_over_the_tags() {
        let index = Index {
            by_path: [(path_key("S:/m/a.flac"), "par-chemin".to_string())]
                .into_iter()
                .collect(),
            by_tags: [(tags_key(Some("t"), Some("a"), Some("al")).unwrap(), "par-tags".to_string())]
                .into_iter()
                .collect(),
        };

        let track = ExportTrackRef {
            path: "S:/m/a.flac".into(),
            title: Some("t".into()),
            artist: Some("a".into()),
            album: Some("al".into()),
            duration: None,
        };

        match pair_up(&index, &track) {
            Found::ByPath(id) => assert_eq!(id, "par-chemin"),
            _ => panic!("le chemin doit primer"),
        }
    }

    #[test]
    fn tags_catch_a_stale_path() {
        // Le cas qui justifie tout le mécanisme : la bibliothèque a changé de
        // lettre de lecteur, aucun chemin ne répond plus.
        let index = Index {
            by_path: HashMap::new(),
            by_tags: [(tags_key(Some("t"), Some("a"), Some("al")).unwrap(), "trouve".to_string())]
                .into_iter()
                .collect(),
        };

        let track = ExportTrackRef {
            path: r"D:\ailleurs\a.flac".into(),
            title: Some("T".into()),
            artist: Some("A".into()),
            album: Some("AL".into()),
            duration: None,
        };

        match pair_up(&index, &track) {
            Found::ByTags(id) => assert_eq!(id, "trouve"),
            _ => panic!("les tags devaient rattraper"),
        }
    }

    #[test]
    fn an_unknown_track_stays_unfound() {
        let index = Index {
            by_path: HashMap::new(),
            by_tags: HashMap::new(),
        };
        let track = ExportTrackRef {
            path: "x".into(),
            title: Some("t".into()),
            artist: None,
            album: None,
            duration: None,
        };
        assert!(matches!(pair_up(&index, &track), Found::NotFound));
    }

    #[test]
    fn the_dlna_identity_is_never_imported() {
        // Deux serveurs sous la même identité sur un réseau, c'est un défaut
        // qu'on ne diagnostique jamais.
        assert!(NON_IMPORTABLE_SETTINGS.contains(&"dlna_uuid"));
    }

    // ────────────────────────────────────────────────────────────────────
    // Contre une vraie base
    // ────────────────────────────────────────────────────────────────────
    //
    // Les tests ci-dessus valident l'appariement, qui est du calcul. Ceux qui
    // suivent exécutent le SQL, et c'est le seul moyen de le valider : sqlx
    // apparie colonnes et champs à l'exécution, jamais à la compilation. Une
    // requête fausse compile parfaitement.

    use crate::service::export::export_service::{ExportPlaylist, ExportProfile};
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
    async fn insert_base(pool: &SqlitePool) {
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
    async fn insert_track(pool: &SqlitePool, id: &str, path: &str, title: &str, artist_name: &str) {
        insert_base(pool).await;
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
        let artist_key = format!("a-{}", artist_name.to_lowercase());
        sqlx::query(
            "INSERT OR IGNORE INTO artists (id, name, name_normalized, sort_name)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&artist_key)
        .bind(artist_name)
        .bind(artist_name.to_lowercase())
        .bind(artist_name)
        .execute(pool)
        .await
        .expect("artiste");
        sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, artist_id, title, title_normalized, duration) VALUES (?, 1, ?, ?, ?, ?, 200)")
            .bind(id)
            .bind(format!("f-{id}"))
            .bind(&artist_key)
            .bind(title)
            .bind(title.to_lowercase())
            .execute(pool)
            .await
            .expect("piste");
    }

    fn bundle_with(tracks_list: Vec<ExportTrackRef>) -> ExportBundle {
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
            profiles: vec![ExportProfile {
                name: "David".into(),
                color: "#22c55e".into(),
                avatar: None,
                bio: None,
                role: "admin".into(),
                playlists: vec![ExportPlaylist {
                    rules: None,
                    is_mix: false,
                    pinned: true,
                    name: "Grunge".into(),
                    description: None,
                    color: "#8b5cf6".into(),
                    icon: "mynaui:music".into(),
                    position: 0,
                    tracks: tracks_list,
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

    fn track_ref(path: &str, title: &str, artist_name: &str) -> ExportTrackRef {
        ExportTrackRef {
            path: path.into(),
            title: Some(title.into()),
            artist: Some(artist_name.into()),
            album: None,
            duration: Some(200.0),
        }
    }

    #[tokio::test]
    async fn an_import_rebuilds_the_playlist_and_its_tracks() {
        let pool = base_de_test().await;
        insert_track(&pool, "t1", r"S:\m\a.flac", "A", "Nirvana").await;
        insert_track(&pool, "t2", r"S:\m\b.flac", "B", "Nirvana").await;

        let bundle = bundle_with(vec![
            track_ref(r"S:\m\a.flac", "A", "Nirvana"),
            track_ref(r"S:\m\b.flac", "B", "Nirvana"),
        ]);

        let r = apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import");

        assert_eq!(r.playlists_created, 1);
        assert_eq!(r.tracks_matched_by_path, 2);
        assert_eq!(r.tracks_missing, 0);

        let (name, count): (String, i64) =
            sqlx::query_as("SELECT name, track_count FROM playlists")
                .fetch_one(&pool)
                .await
                .expect("playlist");
        assert_eq!(name, "Grunge");
        // Le compteur doit refléter le contenu : c'est lui qui s'affiche dans
        // la barre latérale.
        assert_eq!(count, 2);

        // L'ordre exporté doit être l'ordre restauré.
        let order: Vec<(String, i64)> = sqlx::query_as(
            "SELECT library_track_id, sort_index FROM playlist_items ORDER BY sort_index",
        )
        .fetch_all(&pool)
        .await
        .expect("items");
        assert_eq!(order[0].0, "t1");
        assert_eq!(order[1].0, "t2");
    }

    #[tokio::test]
    async fn the_dlna_setting_does_not_cross_the_import() {
        let pool = base_de_test().await;
        let bundle = bundle_with(vec![]);

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
    async fn an_existing_playlist_is_left_intact() {
        let pool = base_de_test().await;
        insert_track(&pool, "t1", r"S:\m\a.flac", "A", "Nirvana").await;

        let bundle = bundle_with(vec![track_ref(r"S:\m\a.flac", "A", "Nirvana")]);

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

        let how_many: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playlists")
            .fetch_one(&pool)
            .await
            .expect("compte");
        assert_eq!(how_many, 1, "l'import a dupliqué la playlist");
    }

    #[tokio::test]
    async fn replacement_is_requested_and_duplicates_nothing() {
        let pool = base_de_test().await;
        insert_track(&pool, "t1", r"S:\m\a.flac", "A", "Nirvana").await;
        let bundle = bundle_with(vec![track_ref(r"S:\m\a.flac", "A", "Nirvana")]);

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
    async fn a_stale_path_is_caught_by_the_tags() {
        let pool = base_de_test().await;
        // La bibliothèque est sur D:, l'export venait de S:.
        insert_track(&pool, "t1", r"D:\ailleurs\a.flac", "A", "Nirvana").await;

        let bundle = bundle_with(vec![track_ref(r"S:\m\a.flac", "A", "Nirvana")]);

        let r = apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import");

        assert_eq!(r.tracks_matched_by_path, 0);
        assert_eq!(r.tracks_matched_by_tags, 1);
        assert_eq!(r.tracks_missing, 0);
    }

    #[tokio::test]
    async fn the_preview_writes_nothing() {
        let pool = base_de_test().await;
        insert_track(&pool, "t1", r"S:\m\a.flac", "A", "Nirvana").await;
        let bundle = bundle_with(vec![track_ref(r"S:\m\a.flac", "A", "Nirvana")]);

        let r = apply(&pool, &bundle, ImportOptions::default(), true)
            .await
            .expect("aperçu");

        // Les comptes doivent être justes…
        assert_eq!(r.playlists_created, 1);
        assert_eq!(r.tracks_matched_by_path, 1);

        // …et la base intacte.
        let lists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playlists")
            .fetch_one(&pool)
            .await
            .expect("compte");
        assert_eq!(lists, 0, "l'aperçu a écrit dans la base");

        let theme: Option<String> =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'theme'")
                .fetch_optional(&pool)
                .await
                .expect("lecture");
        assert!(theme.is_none(), "l'aperçu a écrit un réglage");
    }

    #[tokio::test]
    async fn an_unfound_track_is_reported_without_blocking_the_rest() {
        let pool = base_de_test().await;
        insert_track(&pool, "t1", r"S:\m\a.flac", "A", "Nirvana").await;

        let bundle = bundle_with(vec![
            track_ref(r"S:\m\a.flac", "A", "Nirvana"),
            track_ref(r"S:\m\disparu.flac", "Disparu", "Inconnu"),
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
    async fn liked_tracks_are_restored_without_duplicates() {
        let pool = base_de_test().await;
        let bundle = bundle_with(vec![]);

        apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import 1");
        apply(&pool, &bundle, ImportOptions::default(), false)
            .await
            .expect("import 2");

        let how_many: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM track_liked")
            .fetch_one(&pool)
            .await
            .expect("compte");
        assert_eq!(how_many, 1, "un second import a dupliqué les titres aimés");
    }
}
