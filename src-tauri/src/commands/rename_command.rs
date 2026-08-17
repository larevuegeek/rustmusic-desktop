//! Aperçu du renommage et de la restructuration.
//!
//! # Cette commande n'écrit rien
//! Elle calcule où chaque fichier irait, et ce qui l'en empêche. C'est la
//! condition posée par la roadmap : corriger cinq mille fichiers avec un motif
//! erroné n'est pas rattrapable à la main. L'aperçu à blanc n'est donc pas du
//! confort, c'est ce qui permet d'oser lancer le lot.
//!
//! # Le motif s'applique aux tags **corrigés**
//! Les valeurs viennent de l'atelier, modifications en attente comprises, et
//! non de la base. C'est l'enchaînement naturel : on corrige les tags, on
//! constate que les noms de fichiers ne suivent plus, on renomme d'après ce
//! qu'on vient d'écrire. Repartir de la base ferait renommer d'après les
//! anciennes valeurs — exactement celles qu'on venait de corriger.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use tauri::State;

use crate::core::tag_pattern::pattern::{self, PatternError};
use crate::core::tag_pattern::planner::{self, FileInput, Mode, PlanIssue, PlannedMove};
use crate::service::library::move_service::{self, MoveOptions, MoveOutcome};
use crate::state::AppState;

/// Un fichier de l'atelier, tel que l'interface l'envoie.
#[derive(Debug, Deserialize)]
pub struct RenameInput {
    pub path: String,
    /// Les tags, sous les noms de l'atelier (`album_artist`, `track_number`…).
    pub tags: HashMap<String, String>,
}

/// Ce qui arriverait à un fichier.
#[derive(Debug, Serialize)]
pub struct MoveView {
    pub from: String,
    /// Nom court, pour l'affichage — le chemin complet ne tient pas.
    pub from_name: String,
    pub to: String,
    pub to_name: String,
    pub unchanged: bool,
    pub blocked: bool,
    /// Identifiants d'obstacles, clés de traduction côté interface.
    pub issues: Vec<String>,
    /// Champs cités par le motif et vides sur ce fichier.
    pub missing: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RenamePreview {
    pub moves: Vec<MoveView>,
    pub ready: usize,
    pub blocked: usize,
    pub unchanged: usize,
}

/// Traduit les noms de champs de l'atelier vers ceux du langage de motifs.
///
/// Deux vocabulaires, et c'est voulu : l'atelier parle le langage des tags
/// (`album_artist`, `track_number`), le motif celui qu'on tape à la main, où
/// `{albumartist}` et `{track}` sont plus courts à écrire et plus lisibles à
/// relire. La correspondance vit ici, en un seul endroit.
fn to_pattern_values(path: &str, tags: &HashMap<String, String>) -> HashMap<String, String> {
    let get = |key: &str| tags.get(key).cloned().unwrap_or_default();

    let mut values = HashMap::new();
    values.insert("title".into(), get("title"));
    values.insert("artist".into(), get("artist"));
    values.insert("album".into(), get("album"));
    values.insert("albumartist".into(), get("album_artist"));
    values.insert("year".into(), get("year"));
    values.insert("genre".into(), get("genre"));
    values.insert("composer".into(), get("composer"));
    values.insert("track".into(), get("track_number"));
    values.insert("disc".into(), get("disc_number"));

    // L'extension ne vient pas des tags : c'est une propriété du fichier, et
    // la changer en changerait le format déclaré.
    let ext = path
        .rsplit_once('.')
        .map(|(_, e)| e.to_string())
        .filter(|e| !e.contains(['/', '\\']))
        .unwrap_or_default();
    values.insert("ext".into(), ext);

    values
}

fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

/// Vérifie un motif, sans rien calculer d'autre.
///
/// Sert à la saisie : le message d'erreur s'affiche à la frappe, avant qu'on
/// ait chargé quoi que ce soit.
#[tauri::command]
pub fn check_pattern(pattern: String) -> Result<Vec<String>, String> {
    match pattern::parse(&pattern) {
        Ok(tokens) => Ok(pattern::referenced_fields(&tokens)),
        Err(e) => Err(e.to_string()),
    }
}

/// Calcule le plan, sans rien exécuter.
#[tauri::command]
pub async fn preview_rename(
    state: State<'_, AppState>,
    files: Vec<RenameInput>,
    pattern: String,
    restructure: bool,
    library_root: Option<String>,
) -> Result<RenamePreview, String> {
    let tokens = pattern::parse(&pattern).map_err(|e: PatternError| e.to_string())?;

    let mode = if restructure {
        Mode::Restructure
    } else {
        Mode::Rename
    };
    let root = library_root.unwrap_or_default();
    if restructure && root.trim().is_empty() {
        return Err("Aucun dossier de destination.".to_string());
    }

    let inputs: Vec<FileInput> = files
        .iter()
        .map(|f| FileInput {
            path: f.path.clone(),
            values: to_pattern_values(&f.path, &f.tags),
        })
        .collect();

    // Sous Windows, `A.flac` et `a.flac` sont le même fichier : comparer
    // exactement laisserait passer un renommage qui en écrase un autre.
    let case_insensitive = cfg!(windows);

    // L'existence se demande au disque, et un lot peut compter des milliers de
    // fichiers sur un partage réseau : hors de la boucle asynchrone.
    let plan = tokio::task::spawn_blocking(move || {
        planner::plan(&inputs, &tokens, mode, &root, case_insensitive, |path| {
            std::path::Path::new(path).exists()
        })
    })
    .await
    .map_err(|e| format!("Calcul du plan : {e}"))?;

    // ─── Conflits d'unicité en base ───
    //
    // `library_cache.path` et `recent_files.path` sont UNIQUE. Un renommage
    // vers un chemin déjà connu de la base — typiquement la trace d'un fichier
    // supprimé du disque mais pas de la bibliothèque — échouerait au milieu du
    // lot. La roadmap est explicite : le détecter à l'aperçu, pas à
    // l'exécution.
    let targets: Vec<String> = plan
        .moves
        .iter()
        .filter(|m| !m.unchanged && !m.blocked())
        .map(|m| m.to.clone())
        .collect();
    let known = known_paths(&state.pool, &targets).await?;

    let moves = plan
        .moves
        .iter()
        .map(|m| MoveView {
            from: m.from.clone(),
            from_name: file_name(&m.from),
            to: m.to.clone(),
            to_name: file_name(&m.to),
            unchanged: m.unchanged,
            blocked: m.blocked() || known.contains(&m.to),
            issues: {
                let mut issues: Vec<String> =
                    m.issues.iter().map(|i| i.as_str().to_string()).collect();
                if known.contains(&m.to) {
                    issues.push("known_in_library".to_string());
                }
                issues
            },
            missing: m
                .issues
                .iter()
                .find_map(|i| match i {
                    PlanIssue::MissingFields(fields) => Some(fields.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
        })
        .collect();

    let moves: Vec<MoveView> = moves;
    let blocked = moves.iter().filter(|m| m.blocked).count();
    let unchanged = moves.iter().filter(|m| m.unchanged && !m.blocked).count();

    Ok(RenamePreview {
        ready: moves.len() - blocked - unchanged,
        blocked,
        unchanged,
        moves,
    })
}

/// Les chemins déjà connus de la base parmi ceux qu'on vise.
async fn known_paths(
    pool: &sqlx::SqlitePool,
    targets: &[String],
) -> Result<std::collections::HashSet<String>, String> {
    let mut known = std::collections::HashSet::new();
    if targets.is_empty() {
        return Ok(known);
    }

    // Une requête par lot de cinq cents : SQLite plafonne le nombre de
    // paramètres liés, et un album de trente morceaux n'est pas la limite —
    // une bibliothèque entière, si.
    for chunk in targets.chunks(500) {
        let holders = vec!["?"; chunk.len()].join(",");
        let sql = format!(
            "SELECT path FROM library_cache WHERE path IN ({holders})
             UNION SELECT path FROM recent_files WHERE path IN ({holders})"
        );
        let mut query = sqlx::query_as::<_, (String,)>(&sql);
        for path in chunk {
            query = query.bind(path);
        }
        for path in chunk {
            query = query.bind(path);
        }
        let rows = query
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Vérification des chemins : {e}"))?;
        known.extend(rows.into_iter().map(|r| r.0));
    }
    Ok(known)
}

/// Exécute le lot : déplace les fichiers **et** met la base à jour.
///
/// Ne reçoit que les déplacements, jamais le motif : l'aperçu a déjà tranché,
/// et recalculer ici ouvrirait la porte à un écart entre ce qui a été montré et
/// ce qui est fait.
#[tauri::command]
pub async fn apply_rename(
    state: State<'_, AppState>,
    library_id: Option<i64>,
    pattern: String,
    restructure: bool,
    moves: Vec<(String, String)>,
    options: Option<MoveOptions>,
) -> Result<MoveOutcome, String> {
    if moves.is_empty() {
        return Err("Aucun déplacement à appliquer.".to_string());
    }
    let kind = if restructure { "restructure" } else { "rename" };

    // Les compagnons et le nettoyage n'ont de sens qu'en restructuration : un
    // simple renommage ne change pas de dossier, donc rien ne se vide et rien
    // n'a besoin de suivre.
    let mut options = options.unwrap_or_default();
    if !restructure {
        options.satellites = false;
        options.cleanup_empty = false;
    }

    // Les racines viennent de la base et non de l'interface : c'est le
    // garde-fou du nettoyage, et un garde-fou qu'on peut passer en paramètre
    // n'en est pas un.
    if options.cleanup_empty {
        options.roots = sqlx::query_as::<_, (String,)>(
            "SELECT path FROM library_dirs WHERE library_id = ? AND is_active = 1",
        )
        .bind(library_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| format!("Lecture des dossiers : {e}"))?
        .into_iter()
        .map(|r| r.0)
        .collect();
    }

    move_service::apply_moves(&state.pool, library_id, kind, &pattern, moves, options).await
}

/// Les derniers lots du journal.
#[tauri::command]
pub async fn list_batch_journal(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<move_service::JournalEntry>, String> {
    move_service::list_batches(&state.pool, limit.unwrap_or(20)).await
}

/// Annule un lot.
///
/// Deux natures de lot, deux façons d'annuler : un déplacement se défait en
/// remettant les fichiers où ils étaient, une correction de tags en réécrivant
/// ce que le journal a conservé. L'aiguillage se fait ici, sur la nature
/// enregistrée — pas sur ce que l'interface croit savoir.
#[tauri::command]
pub async fn undo_batch(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    batch_id: String,
) -> Result<MoveOutcome, String> {
    let (kind, undone) = move_service::batch_kind(&state.pool, &batch_id).await?;
    if undone {
        return Err("Ce lot a déjà été annulé.".to_string());
    }

    if kind != "tags" {
        return move_service::undo_batch(&state.pool, &batch_id).await;
    }

    let items = move_service::tags_to_restore(&state.pool, &batch_id).await?;
    let mut restored: Vec<i64> = Vec::new();
    let mut failed = Vec::new();

    for (id, path, blob) in items {
        match crate::commands::tag_command::restore_tags(&app, &state.pool, &path, &blob).await {
            Ok(()) => restored.push(id),
            Err(message) => failed.push(move_service::MoveFailure { path, message }),
        }
    }

    move_service::mark_tags_undone(&state.pool, &batch_id, &restored).await;

    Ok(MoveOutcome {
        batch_id,
        succeeded: restored.len(),
        failed,
        // Une restauration de tags ne déplace rien : les chemins sont intacts.
        moved: Vec::new(),
    })
}

/// Ordonne les déplacements pour que les chaînes se résolvent.
///
/// `a → b` et `b → c` : déplacer `a` en premier écraserait `b`. Exposé à part
/// pour que l'interface envoie une liste déjà sûre.
#[tauri::command]
pub fn order_moves(moves: Vec<(String, String)>) -> Result<Vec<(String, String)>, String> {
    let planned: Vec<PlannedMove> = moves
        .iter()
        .map(|(from, to)| PlannedMove {
            from: from.clone(),
            to: to.clone(),
            unchanged: false,
            issues: Vec::new(),
        })
        .collect();

    let (ordered, cycles) = planner::order(&planned, cfg!(windows));
    if !cycles.is_empty() {
        // Un cycle `a → b` et `b → a` ne se dénoue qu'avec un nom temporaire.
        // Plutôt que de l'improviser au milieu d'un lot, on le refuse et on le
        // dit : deux passes successives règlent le cas.
        return Err(format!(
            "{} fichiers forment une permutation circulaire. Renommez-les en deux fois.",
            cycles.len()
        ));
    }

    Ok(ordered.into_iter().map(|i| moves[i].clone()).collect())
}

/// Propose une version nettoyée d'un jeu de valeurs.
///
/// Ne rend qu'une proposition : le résultat va dans les modifications en
/// attente de l'atelier, où on le relit avant d'écrire — et où le journal le
/// rattrape si l'on s'est trompé. Aucune règle ne s'applique d'office.
#[tauri::command]
pub fn clean_tags(
    values: Vec<(String, String)>,
    rules: Vec<String>,
) -> Result<Vec<(String, String)>, String> {
    use crate::core::tag_clean::rules::{self as clean, Rule};

    let selected: Vec<Rule> = rules.iter().filter_map(|r| Rule::parse(r)).collect();
    if selected.is_empty() {
        return Ok(Vec::new());
    }

    Ok(values
        .into_iter()
        .filter_map(|(key, value)| {
            let cleaned = clean::apply(&value, &selected);
            // Seules les valeurs qui changent reviennent : l'interface n'a rien
            // à faire des autres, et les renvoyer ferait passer tout le lot
            // pour modifié.
            (cleaned != value).then_some((key, cleaned))
        })
        .collect())
}
