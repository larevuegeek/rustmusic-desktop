//! Commandes des artistes multiples : vérification (toute la bibliothèque ou un artiste), bilan, décisions.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, State};

use crate::service::library::artist_split::{self, CheckReport};
use crate::state::AppState;

/// Une seule vérification complète à la fois.
static RUNNING: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub fn artist_check_running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

/// Lance la vérification et rend la main aussitôt. Événements : `artist-check-progress`
/// ({ done, total, name, decision, parts, images } ; `decision` vide = nom en cours), puis `artist-check-done` ({ run_id, error }).
#[tauri::command]
pub async fn artist_check_start(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Err("busy".into());
    }
    let pool = state.pool.clone();
    tauri::async_runtime::spawn(async move {
        let result = match artist_split::candidates(&pool, false).await {
            Ok(names) => {
                let _ = app.emit("artist-check-progress", serde_json::json!({ "done": 0, "total": names.len(), "name": "", "decision": "", "parts": [] }));
                let progress_app = app.clone();
                let photos = artist_split::photos(&pool).await;
                artist_split::check(&pool, &names, |s| {
                    let (decision, parts) = s.result.unwrap_or(("", &[]));
                    let images = artist_split::photos_of(&photos, parts);
                    let _ = progress_app.emit(
                        "artist-check-progress",
                        serde_json::json!({ "done": s.done, "total": s.total, "name": s.name, "decision": decision, "parts": parts, "images": images }),
                    );
                })
                .await
            }
            Err(e) => Err(e.to_string()),
        };
        RUNNING.store(false, Ordering::SeqCst);
        let payload = match result {
            Ok(run) => serde_json::json!({ "run_id": run, "error": null }),
            Err(e) => serde_json::json!({ "run_id": null, "error": e }),
        };
        let _ = app.emit("artist-check-done", payload);
    });
    Ok(())
}

/// Vérifie un seul nom (menu « … » d'un artiste), même déjà décidé ; rend l'identifiant du bilan.
#[tauri::command]
pub async fn artist_check_name(state: State<'_, AppState>, name: String) -> Result<i64, String> {
    if !artist_split::doubtful(&name) {
        return Err("not_doubtful".into());
    }
    artist_split::check(&state.pool, &[name], |_| {}).await
}

#[tauri::command]
pub async fn artist_check_report(state: State<'_, AppState>, run_id: Option<i64>) -> Result<CheckReport, String> {
    artist_split::report(&state.pool, run_id).await.map_err(|e| e.to_string())
}

/// `decision` : « whole » (un seul artiste, annule une séparation) ou « split » avec `parts`.
#[tauri::command]
pub async fn artist_rule_set(state: State<'_, AppState>, name: String, decision: String, parts: Vec<String>) -> Result<(), String> {
    if decision != artist_split::WHOLE && decision != artist_split::SPLIT {
        return Err(format!("décision inconnue : {decision}"));
    }
    artist_split::set_rule(&state.pool, &name, &decision, &parts).await
}

/// Le nom est-il douteux (le menu « … » ne propose la vérification que dans ce cas) ?
#[tauri::command]
pub fn artist_is_doubtful(name: String) -> bool {
    artist_split::doubtful(&name)
}

/// Au lancement : les règles en mémoire, pour le scan.
pub async fn init(state: &AppState) {
    artist_split::reload_rules(&state.pool).await;
}
