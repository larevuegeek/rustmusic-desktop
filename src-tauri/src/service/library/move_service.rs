//! Déplacer des fichiers sans casser la bibliothèque.
//!
//! # Sept tables stockent un chemin absolu
//! `library_files`, `library_cache`, `recent_files`, `track_liked`,
//! `queue_tracks`, `library_dirs`, et la pochette de la bibliothèque. Déplacer
//! un fichier sans les mettre à jour **orpheline en silence** l'historique
//! d'écoute, les « j'aime » et le cache d'analyse. Les playlists survivent,
//! elles : elles référencent un identifiant de morceau, pas un chemin.
//!
//! `queue_tracks` manquait à l'inventaire d'origine, et c'était la plus
//! coûteuse à oublier : le morceau restait dans la file sous son ancien nom, et
//! devenait illisible sans que rien ne l'explique.
//!
//! Un déplacement est donc une transaction qui couvre le disque **et** la base,
//! ou n'a pas lieu.
//!
//! # Une transaction par fichier, pas une pour le lot
//! Trois mille fichiers dans une seule transaction, c'est un verrou tenu
//! pendant toute la durée des entrées-sorties — et un échec au deux-millième
//! qui annule les mille neuf cent quatre-vingt-dix-neuf précédents, pourtant
//! parfaitement valides. Fichier par fichier, ce qui est passé reste passé, et
//! le journal dit exactement où on en est.
//!
//! # L'ordre, et le retour en arrière
//! On déplace sur le disque, puis on met la base à jour, puis on valide. Si la
//! base refuse — un chemin cible déjà connu viole une contrainte d'unicité —
//! on remet le fichier à sa place avant de rendre l'échec. L'inverse laisserait
//! une base qui décrit une arborescence qui n'existe pas.

use serde::Serialize;
use sqlx::SqlitePool;

use crate::repository::batch::batch_journal_repository::BatchJournalRepository as Journal;
use crate::repository::library::track_path_repository::TrackPathRepository;

/// Ce qu'un lot a produit.
#[derive(Debug, Serialize)]
pub struct MoveOutcome {
    /// Identifiant du lot au journal — c'est par lui qu'on annule.
    pub batch_id: String,
    pub succeeded: usize,
    pub failed: Vec<MoveFailure>,
    /// Les couples avant/après réellement appliqués.
    ///
    /// L'interface en a besoin : ses écrans tiennent des chemins en mémoire, et
    /// recharger avec les anciens ne trouverait plus rien.
    pub moved: Vec<(String, String)>,
}

#[derive(Debug, Serialize)]
pub struct MoveFailure {
    pub path: String,
    pub message: String,
}

/// Ce qui accompagne un déplacement.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct MoveOptions {
    /// Emporter les fichiers compagnons — paroles, feuillet, pochette.
    pub satellites: bool,
    /// Supprimer les dossiers devenus vides.
    pub cleanup_empty: bool,
    /// Racines de la bibliothèque : le nettoyage ne remonte **jamais** au-delà.
    pub roots: Vec<String>,
}

/// Compagnons d'une piste : même nom de base, autre extension.
///
/// Un `.lrc` laissé derrière, ce sont les paroles perdues — et elles ne se
/// retrouvent pas, puisque c'est le nom du fichier qui les relie au morceau.
const TRACK_SIDECARS: [&str; 2] = ["lrc", "cue"];

/// Fichiers qui appartiennent au dossier de l'album, pas à une piste.
///
/// Ils ne suivent que si **tout le dossier** part au même endroit : un album
/// éclaté vers deux destinations rendrait arbitraire le choix de la pochette
/// qui accompagne l'une plutôt que l'autre.
fn is_folder_satellite(name: &str) -> bool {
    let lower = name.to_lowercase();
    let (stem, ext) = lower.rsplit_once('.').unwrap_or((lower.as_str(), ""));

    matches!(ext, "m3u" | "m3u8" | "nfo" | "log" | "cue" | "txt" | "pdf")
        || (matches!(ext, "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp")
            && matches!(
                stem,
                "cover" | "folder" | "front" | "back" | "album" | "artwork" | "thumb"
            ))
}

/// Dossier parent d'un chemin.
fn parent_of(path: &str) -> String {
    match path.rfind(['/', '\\']) {
        Some(i) => path[..i].to_string(),
        None => String::new(),
    }
}

/// Nom sans extension.
fn stem_of(name: &str) -> String {
    match name.rfind('.') {
        Some(i) => name[..i].to_string(),
        None => name.to_string(),
    }
}

/// Comparaison de chemins, insensible à la casse sous Windows.
fn same_path(a: &str, b: &str) -> bool {
    let norm = |p: &str| {
        let unified = p.replace('\\', "/");
        let trimmed = unified.trim_end_matches('/').to_string();
        if cfg!(windows) {
            trimmed.to_lowercase()
        } else {
            trimmed
        }
    };
    norm(a) == norm(b)
}

/// Le chemin est-il sous l'une des racines, sans être l'une d'elles ?
fn under_roots(path: &str, roots: &[String]) -> bool {
    let norm = |p: &str| {
        let unified = p.replace('\\', "/");
        let trimmed = unified.trim_end_matches('/').to_string();
        if cfg!(windows) {
            trimmed.to_lowercase()
        } else {
            trimmed
        }
    };
    let target = norm(path);
    roots.iter().any(|root| {
        let root = norm(root);
        !root.is_empty() && target != root && target.starts_with(&format!("{root}/"))
    })
}

/// Les compagnons d'une piste, avec leur destination.
fn track_sidecars(from: &str, to: &str) -> Vec<(String, String)> {
    let from_dir = parent_of(from);
    let to_dir = parent_of(to);
    let from_stem = stem_of(&file_name(from));
    let to_stem = stem_of(&file_name(to));

    TRACK_SIDECARS
        .iter()
        .filter_map(|ext| {
            let source = format!("{from_dir}/{from_stem}.{ext}");
            if !std::path::Path::new(&source).exists() {
                // Certains systèmes rendent `.LRC` : on tente aussi la
                // majuscule plutôt que d'abandonner les paroles pour une casse.
                let upper = format!("{from_dir}/{from_stem}.{}", ext.to_uppercase());
                if !std::path::Path::new(&upper).exists() {
                    return None;
                }
                return Some((upper, format!("{to_dir}/{to_stem}.{}", ext.to_uppercase())));
            }
            Some((source, format!("{to_dir}/{to_stem}.{ext}")))
        })
        .collect()
}

/// Les fichiers de dossier à emporter, quand le dossier part d'un bloc.
fn folder_satellites(from_dir: &str, to_dir: &str) -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(from_dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().ok()?.is_file() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if !is_folder_satellite(&name) {
                return None;
            }
            Some((format!("{from_dir}/{name}"), format!("{to_dir}/{name}")))
        })
        .collect()
}

/// Supprime les dossiers devenus vides, en remontant.
///
/// Jamais au-delà des racines de la bibliothèque : une remontée récursive sans
/// garde-fou finirait par proposer de supprimer le disque.
fn cleanup_empty_dirs(dirs: &[String], roots: &[String]) {
    for dir in dirs {
        let mut current = dir.clone();
        // Cent niveaux : une profondeur qu'aucune arborescence musicale
        // n'atteint, et qui garantit qu'on ne boucle pas si `parent_of` rend
        // toujours la même chose.
        for _ in 0..100 {
            if !under_roots(&current, roots) {
                break;
            }
            let empty = std::fs::read_dir(&current)
                .map(|mut e| e.next().is_none())
                .unwrap_or(false);
            if !empty || std::fs::remove_dir(&current).is_err() {
                break;
            }
            current = parent_of(&current);
        }
    }
}

/// Déplace un fichier, y compris d'un volume à l'autre.
///
/// `rename` cesse d'être atomique entre deux disques : il faut copier,
/// vérifier, puis supprimer — et ne supprimer qu'**après** vérification. Une
/// copie tronquée suivie d'une suppression, c'est le fichier perdu.
fn move_file(from: &str, to: &str) -> std::io::Result<()> {
    let source = std::path::Path::new(from);
    let target = std::path::Path::new(to);

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    match std::fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            // Le repli couvre le changement de volume, mais aussi les systèmes
            // de fichiers réseau qui refusent `rename` pour d'autres raisons.
            // On ne cherche pas à distinguer : ce qui compte est de ne rien
            // supprimer avant d'avoir vérifié.
            let copied = std::fs::copy(source, target)?;
            let expected = std::fs::metadata(source)?.len();
            if copied != expected {
                let _ = std::fs::remove_file(target);
                return Err(rename_error);
            }
            std::fs::remove_file(source)?;
            Ok(())
        }
    }
}

/// Nom de fichier seul.
fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

/// Date de modification, en secondes depuis l'époque.
///
/// Relevée juste après le déplacement : si elle a changé au moment d'annuler,
/// le fichier a été retouché entre-temps, et le remettre à son ancien nom
/// laisserait croire qu'on a tout rétabli.
fn modified_at(path: &str) -> Option<i64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

/// Applique un lot de déplacements.
///
/// `moves` est supposé **déjà ordonné** par le planificateur : les chaînes
/// `a → b → c` ne se dénouent que dans le bon sens.
pub async fn apply_moves(
    pool: &SqlitePool,
    library_id: Option<i64>,
    kind: &str,
    pattern: &str,
    moves: Vec<(String, String)>,
    options: MoveOptions,
) -> Result<MoveOutcome, String> {
    let batch_id = Journal::open(pool, library_id, kind, Some(pattern), moves.len())
        .await
        .map_err(|e| format!("Ouverture du journal : {e}"))?;

    let mut succeeded = 0usize;
    let mut failed: Vec<MoveFailure> = Vec::new();
    let mut applied: Vec<(String, String)> = Vec::new();

    // Dossiers d'origine, pour les compagnons puis le nettoyage. On note la
    // destination de chacun : un dossier qui se disperse vers plusieurs cibles
    // ne peut pas emporter sa pochette quelque part en particulier.
    let mut folder_targets: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();
    for (from, to) in &moves {
        let from_dir = parent_of(from);
        let to_dir = parent_of(to);
        folder_targets
            .entry(from_dir)
            .and_modify(|current| {
                if !matches!(current.as_deref(), Some(d) if same_path(d, &to_dir)) {
                    *current = None;
                }
            })
            .or_insert(Some(to_dir));
    }

    for (from, to) in moves {
        // Le déplacement est bloquant et peut porter sur des dizaines de méga-
        // octets à travers le réseau : il n'a rien à faire sur la boucle
        // asynchrone, qui sert aussi l'interface pendant ce temps.
        let (f, t) = (from.clone(), to.clone());
        let moved = tokio::task::spawn_blocking(move || move_file(&f, &t))
            .await
            .map_err(|e| format!("Déplacement : {e}"))?;

        if let Err(e) = moved {
            failed.push(MoveFailure {
                path: from.clone(),
                message: format!("{e}"),
            });
            let _ = Journal::record_failure(pool, &batch_id, &from, &to, &format!("{e}")).await;
            continue;
        }

        let outcome = async {
            let mut tx = pool.begin().await?;
            TrackPathRepository::repoint(&mut tx, &from, &to).await?;
            tx.commit().await
        }
        .await;

        match outcome {
            Ok(()) => {
                succeeded += 1;
                applied.push((from.clone(), to.clone()));
                let _ = Journal::record_move(pool, &batch_id, &from, &to, modified_at(&to)).await;

                // Les paroles et le feuillet suivent la piste. Ils ne sont dans
                // aucune table — seul le journal les connaît, et c'est lui qui
                // permettra de les ramener.
                if options.satellites {
                    for (sfrom, sto) in track_sidecars(&from, &to) {
                        let (f, t) = (sfrom.clone(), sto.clone());
                        let moved =
                            tokio::task::spawn_blocking(move || move_file(&f, &t)).await;
                        if matches!(moved, Ok(Ok(()))) {
                            let _ =
                                Journal::record_move(pool, &batch_id, &sfrom, &sto, modified_at(&sto))
                                    .await;
                        }
                    }
                }
            }
            Err(e) => {
                // La base a refusé : on remet le fichier où il était, sans quoi
                // elle décrirait une arborescence qui n'existe plus.
                let (f, t) = (to.clone(), from.clone());
                let _ = tokio::task::spawn_blocking(move || move_file(&f, &t)).await;
                let message = format!("Base de données : {e}");
                failed.push(MoveFailure {
                    path: from.clone(),
                    message: message.clone(),
                });
                let _ = Journal::record_failure(pool, &batch_id, &from, &to, &message).await;
            }
        }
    }

    // ─── Fichiers de dossier ───
    if options.satellites {
        for (from_dir, to_dir) in &folder_targets {
            let Some(to_dir) = to_dir else { continue };
            if same_path(from_dir, to_dir) {
                continue;
            }
            for (sfrom, sto) in folder_satellites(from_dir, to_dir) {
                let (f, t) = (sfrom.clone(), sto.clone());
                let moved = tokio::task::spawn_blocking(move || move_file(&f, &t)).await;
                if matches!(moved, Ok(Ok(()))) {
                    let _ =
                        Journal::record_move(pool, &batch_id, &sfrom, &sto, modified_at(&sto)).await;
                }
            }
        }
    }

    // ─── Dossiers vides ───
    if options.cleanup_empty && !options.roots.is_empty() {
        let dirs: Vec<String> = folder_targets.keys().cloned().collect();
        let roots = options.roots.clone();
        let _ = tokio::task::spawn_blocking(move || cleanup_empty_dirs(&dirs, &roots)).await;
    }

    let _ = Journal::set_counts(pool, &batch_id, succeeded, failed.len()).await;
    let _ = Journal::purge(pool, KEEP_BATCHES).await;

    Ok(MoveOutcome {
        batch_id,
        succeeded,
        failed,
        moved: applied,
    })
}

/// Nombre de lots conservés au journal.
///
/// Un journal qui grossit sans fin finit par peser plus que la bibliothèque
/// qu'il décrit. Cinquante lots couvrent largement le besoin réel — on annule
/// dans la minute qui suit, pas six mois plus tard.
const KEEP_BATCHES: i64 = 50;

/// Enregistre l'état des tags **avant** une réécriture.
///
/// Le journal ne connaissait que des couples de chemins : de quoi annuler un
/// renommage, pas une correction. Or corriger cinq cents fichiers avec un
/// mauvais remplissage n'est pas plus rattrapable à la main qu'un renommage.
///
/// L'instantané est pris **avant** que quoi que ce soit ne soit écrit, et un
/// fichier illisible est simplement omis : mieux vaut un lot partiellement
/// annulable qu'un lot refusé parce qu'un fichier sur cinq cents est abîmé.
pub async fn journal_tags(
    pool: &SqlitePool,
    library_id: Option<i64>,
    paths: &[String],
    snapshot: impl Fn(&str) -> Option<String> + Send + Sync,
) -> Result<String, String> {
    let batch_id = Journal::open(pool, library_id, "tags", None, paths.len())
        .await
        .map_err(|e| format!("Ouverture du journal : {e}"))?;

    let mut kept = 0usize;
    for path in paths {
        let Some(blob) = snapshot(path) else { continue };
        let _ = Journal::record_tags(pool, &batch_id, path, &blob).await;
        kept += 1;
    }

    let _ = Journal::set_counts(pool, &batch_id, kept, 0).await;
    let _ = Journal::purge(pool, KEEP_BATCHES).await;
    Ok(batch_id)
}

/// Les tags à restaurer d'un lot, dans l'ordre du journal.
pub async fn tags_to_restore(
    pool: &SqlitePool,
    batch_id: &str,
) -> Result<Vec<(i64, String, String)>, String> {
    Ok(Journal::tags_to_restore(pool, batch_id)
        .await
        .map_err(|e| format!("Lecture du journal : {e}"))?
        .into_iter()
        .map(|s| (s.item_id, s.path, s.blob))
        .collect())
}

/// Marque un lot de tags comme annulé.
pub async fn mark_tags_undone(pool: &SqlitePool, batch_id: &str, restored: &[i64]) {
    for id in restored {
        let _ = Journal::mark_item_undone(pool, *id).await;
    }
    let _ = Journal::mark_undone(pool, batch_id).await;
}

/// La nature d'un lot, pour savoir comment l'annuler.
pub async fn batch_kind(pool: &SqlitePool, batch_id: &str) -> Result<(String, bool), String> {
    Journal::kind(pool, batch_id)
        .await
        .map_err(|e| format!("Lecture du journal : {e}"))?
        .ok_or_else(|| "Lot introuvable.".to_string())
}

pub use crate::repository::batch::batch_journal_repository::JournalEntry;

/// Les derniers lots, du plus récent au plus ancien.
pub async fn list_batches(pool: &SqlitePool, limit: i64) -> Result<Vec<JournalEntry>, String> {
    Journal::list(pool, limit)
        .await
        .map_err(|e| format!("Lecture du journal : {e}"))
}

/// Annule un lot.
///
/// L'annulation **rend compte comme un lot normal** : elle peut échouer, et
/// prétendre le contraire serait pire que de ne pas la proposer. Un fichier
/// retouché depuis l'opération n'est pas remis en place — le remettre à son
/// ancien nom donnerait à croire qu'on a tout rétabli, alors que son contenu a
/// changé entre-temps.
pub async fn undo_batch(pool: &SqlitePool, batch_id: &str) -> Result<MoveOutcome, String> {
    let (_, already_undone) = batch_kind(pool, batch_id).await?;
    if already_undone {
        return Err("Ce lot a déjà été annulé.".to_string());
    }

    let items = Journal::moves_to_undo(pool, batch_id)
        .await
        .map_err(|e| format!("Lecture du journal : {e}"))?;

    let mut succeeded = 0usize;
    let mut failed: Vec<MoveFailure> = Vec::new();
    let mut applied: Vec<(String, String)> = Vec::new();

    for record in items {
        let (item_id, before, after, stamp) = (
            record.item_id,
            record.before_path,
            record.after_path,
            record.modified_at,
        );
        // Le fichier a-t-il bougé ou changé depuis ?
        let current = modified_at(&after);
        if current.is_none() {
            failed.push(MoveFailure {
                path: after.clone(),
                message: "Le fichier n'est plus là.".to_string(),
            });
            continue;
        }
        if let (Some(recorded), Some(now)) = (stamp, current) {
            if recorded != now {
                failed.push(MoveFailure {
                    path: after.clone(),
                    message: "Le fichier a été modifié depuis l'opération.".to_string(),
                });
                continue;
            }
        }

        let (f, t) = (after.clone(), before.clone());
        let moved = tokio::task::spawn_blocking(move || move_file(&f, &t))
            .await
            .map_err(|e| format!("Déplacement : {e}"))?;

        if let Err(e) = moved {
            failed.push(MoveFailure {
                path: after.clone(),
                message: format!("{e}"),
            });
            continue;
        }

        let outcome = async {
            let mut tx = pool.begin().await?;
            TrackPathRepository::repoint(&mut tx, &after, &before).await?;
            tx.commit().await
        }
        .await;

        match outcome {
            Ok(()) => {
                succeeded += 1;
                applied.push((after.clone(), before.clone()));
                let _ = Journal::mark_item_undone(pool, item_id).await;
            }
            Err(e) => {
                let (f, t) = (before.clone(), after.clone());
                let _ = tokio::task::spawn_blocking(move || move_file(&f, &t)).await;
                failed.push(MoveFailure {
                    path: after.clone(),
                    message: format!("Base de données : {e}"),
                });
            }
        }
    }

    // Marqué comme annulé même en cas d'échecs partiels : ce lot ne doit pas
    // pouvoir être rejoué, et l'écran d'historique montre le détail.
    let _ = Journal::mark_undone(pool, batch_id).await;

    Ok(MoveOutcome {
        batch_id: batch_id.to_string(),
        succeeded,
        failed,
        moved: applied,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fichiers_de_dossier_reconnus() {
        assert!(is_folder_satellite("cover.jpg"));
        assert!(is_folder_satellite("Folder.PNG"));
        assert!(is_folder_satellite("album.m3u"));
        assert!(is_folder_satellite("rip.log"));
        assert!(is_folder_satellite("livret.pdf"));
        // Une image qui n'est pas une pochette reste où elle est : emporter
        // toutes les images d'un dossier ferait suivre n'importe quoi.
        assert!(!is_folder_satellite("photo-concert.jpg"));
        assert!(!is_folder_satellite("piste01.flac"));
    }

    #[test]
    fn le_nettoyage_ne_remonte_pas_au_dela_des_racines() {
        let roots = vec!["/musique".to_string()];
        assert!(under_roots("/musique/Angele/Brol", &roots));
        assert!(under_roots("/musique/Angele", &roots));
        // La racine elle-même n'est jamais supprimable.
        assert!(!under_roots("/musique", &roots));
        assert!(!under_roots("/", &roots));
        assert!(!under_roots("/autre/chose", &roots));
    }

    #[test]
    fn une_racine_prefixe_n_en_est_pas_une() {
        // « /musique2 » commence par « /musique » sans être dedans : sans le
        // séparateur dans la comparaison, on effacerait le dossier du voisin.
        let roots = vec!["/musique".to_string()];
        assert!(!under_roots("/musique2/Album", &roots));
    }

    #[test]
    fn aucune_racine_interdit_tout_nettoyage() {
        assert!(!under_roots("/musique/Album", &[]));
    }

    #[test]
    fn comparaison_de_chemins() {
        assert!(same_path("/a/b", "/a/b/"));
        assert!(same_path(r"C:\a\b", "C:/a/b"));
        assert!(!same_path("/a/b", "/a/c"));
    }

    #[test]
    fn nom_sans_extension() {
        assert_eq!(stem_of("01 - Titre.flac"), "01 - Titre");
        assert_eq!(stem_of("R.E.M. - Losing.mp3"), "R.E.M. - Losing");
        assert_eq!(stem_of("sansextension"), "sansextension");
    }
}
