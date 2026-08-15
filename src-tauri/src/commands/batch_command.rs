//! Pilotage des traitements par lot.
//!
//! Générique à tous les lots — écriture de tags aujourd'hui, renommage et
//! déplacement demain. La commande qui lance un lot rend la main tout de suite ;
//! c'est par ici qu'on l'interrompt.

use tauri::State;

use crate::state::AppState;

/// Demande l'annulation d'un lot en cours.
///
/// Renvoie `false` si le lot est inconnu ou déjà terminé — l'interface peut
/// alors retirer son bouton sans rien signaler d'anormal.
///
/// L'annulation n'est pas immédiate : elle est constatée entre deux fichiers,
/// jamais pendant une écriture. Le compte rendu final arrive normalement, avec
/// le décompte de ce qui n'a pas été traité.
#[tauri::command]
pub fn cancel_batch(state: State<'_, AppState>, job_id: String) -> bool {
    let cancelled = state.batch.cancel(&job_id);
    if cancelled {
        log::info!("🛑 Annulation demandée pour le lot {job_id}");
    }
    cancelled
}
