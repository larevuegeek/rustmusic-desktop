use serde::Deserialize;
use tauri::State;

use crate::{
    entity::profil::profil::Profil,
    repository::profil::profil_repository::ProfilRepository, state::AppState
};

#[tauri::command]
pub async fn get_profile(
    state: State<'_, AppState>,
    profil_id: i64
) -> Result<Profil, String> {

    let profil: Profil = match ProfilRepository::find_profile_by_id(&state.pool, profil_id).await {
        Ok(profil) => profil,
        Err(e) => return Err(format!("Failed to get profil : {}", e))
    };

    Ok(profil)
}

#[tauri::command]
pub async fn get_all_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<Profil>, String> {

    match ProfilRepository::find_all_active(&state.pool).await {
        Ok(profils) => Ok(profils),
        Err(e) => Err(format!("Failed to get profils : {}", e))
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateProfilPayload {
    pub name: String,
    pub avatar: Option<String>,
    pub color: String,
}

#[tauri::command]
pub async fn create_profile(
    state: State<'_, AppState>,
    payload: CreateProfilPayload,
) -> Result<Profil, String> {

    match ProfilRepository::create(&state.pool, &state.pool, &payload.name, payload.avatar.as_deref(), &payload.color).await {
        Ok(profil) => Ok(profil),
        Err(e) => Err(format!("Failed to create profil : {}", e))
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfilPayload {
    pub id: i64,
    pub name: String,
    pub avatar: Option<String>,
    pub color: String,
}

#[tauri::command]
pub async fn update_profile(
    state: State<'_, AppState>,
    payload: UpdateProfilPayload,
) -> Result<Profil, String> {

    match ProfilRepository::update(&state.pool, &state.pool, payload.id, &payload.name, payload.avatar.as_deref(), &payload.color).await {
        Ok(profil) => Ok(profil),
        Err(e) => Err(format!("Failed to update profil : {}", e))
    }
}

#[tauri::command]
pub async fn delete_profile(
    state: State<'_, AppState>,
    profil_id: i64,
) -> Result<(), String> {

    if profil_id == 1 {
        return Err("Impossible de supprimer le profil administrateur".into());
    }

    match ProfilRepository::delete(&state.pool, profil_id).await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to delete profil : {}", e))
    }
}