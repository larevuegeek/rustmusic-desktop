use serde::Serialize;
use tauri::State;

use crate::service::export::{export_service, import_service};
use crate::state::AppState;

/// Ce que l'export a emporté, pour le dire à l'utilisateur.
#[derive(Debug, Serialize)]
pub struct ExportSummary {
    pub path: String,
    pub settings: usize,
    pub profils: usize,
    pub playlists: usize,
    pub tracks: usize,
    pub liked: usize,
    pub bytes: u64,
}

/// Écrit réglages, profils, playlists et titres aimés dans un fichier JSON.
///
/// # Pourquoi l'écriture se fait ici et non dans l'interface
/// Faire transiter le fichier par l'interface pour qu'elle l'écrive imposerait
/// de sérialiser deux fois et d'ouvrir au frontend un droit d'écriture sur
/// n'importe quel chemin. Le chemin vient d'un sélecteur système — donc d'un
/// choix explicite de l'utilisateur — et l'écriture reste du côté qui détient
/// déjà les données.
#[tauri::command]
pub async fn export_settings_and_playlists(
    state: State<'_, AppState>,
    path: String,
) -> Result<ExportSummary, String> {
    let maintenant = chrono::Utc::now().to_rfc3339();

    let bundle = export_service::collect(&state.pool, env!("CARGO_PKG_VERSION"), &maintenant)
        .await
        .map_err(|e| format!("Lecture des données : {e}"))?;

    // Indenté : un export sert aussi à être relu, comparé, corrigé à la main.
    // Le surcoût en octets est sans commune mesure avec ce que ça fait gagner
    // le jour où l'on cherche pourquoi une playlist manque.
    let json = serde_json::to_string_pretty(&bundle)
        .map_err(|e| format!("Sérialisation : {e}"))?;

    std::fs::write(&path, &json).map_err(|e| format!("Écriture de {path} : {e}"))?;

    let playlists: usize = bundle.profils.iter().map(|p| p.playlists.len()).sum();
    let tracks: usize = bundle
        .profils
        .iter()
        .flat_map(|p| p.playlists.iter())
        .map(|l| l.tracks.len())
        .sum();
    let liked: usize = bundle.profils.iter().map(|p| p.liked.len()).sum();

    log::info!(
        "💾 Export : {} réglages, {} playlists, {} pistes, {} titres aimés → {}",
        bundle.settings.len(),
        playlists,
        tracks,
        liked,
        path
    );

    Ok(ExportSummary {
        path: path.clone(),
        settings: bundle.settings.len(),
        profils: bundle.profils.len(),
        playlists,
        tracks,
        liked,
        bytes: json.len() as u64,
    })
}

/// Lit un fichier d'export et annonce ce qu'il ferait, sans rien écrire.
///
/// L'aperçu n'est pas un ornement : un import touche les réglages et les
/// playlists, et l'utilisateur doit pouvoir voir combien de morceaux seront
/// retrouvés — et lesquels manqueront — avant de s'engager.
#[tauri::command]
pub async fn preview_import(
    state: State<'_, AppState>,
    path: String,
    options: Option<import_service::ImportOptions>,
) -> Result<import_service::ImportReport, String> {
    let contenu = std::fs::read_to_string(&path).map_err(|e| format!("Lecture de {path} : {e}"))?;
    let bundle = import_service::read_bundle(&contenu)?;

    import_service::apply(
        &state.pool,
        &bundle,
        options.unwrap_or_default(),
        true,
    )
    .await
    .map_err(|e| format!("Analyse : {e}"))
}

/// Rejoue un fichier d'export dans la base.
#[tauri::command]
pub async fn import_settings_and_playlists(
    state: State<'_, AppState>,
    path: String,
    options: Option<import_service::ImportOptions>,
) -> Result<import_service::ImportReport, String> {
    let contenu = std::fs::read_to_string(&path).map_err(|e| format!("Lecture de {path} : {e}"))?;
    let bundle = import_service::read_bundle(&contenu)?;

    let rapport = import_service::apply(
        &state.pool,
        &bundle,
        options.unwrap_or_default(),
        false,
    )
    .await
    .map_err(|e| format!("Import : {e}"))?;

    log::info!(
        "📥 Import : {} réglages, {} playlists créées, {} remplacées, {} ignorées, \
         {} pistes par chemin, {} par tags, {} introuvables",
        rapport.settings,
        rapport.playlists_created,
        rapport.playlists_replaced,
        rapport.playlists_skipped,
        rapport.tracks_matched_by_path,
        rapport.tracks_matched_by_tags,
        rapport.tracks_missing
    );

    Ok(rapport)
}
