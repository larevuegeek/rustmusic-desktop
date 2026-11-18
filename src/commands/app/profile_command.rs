use serde::Deserialize;
use crate::app::State;

use crate::{
    entity::profile::profile::Profile,
    repository::profile::profile_repository::ProfileRepository, app::platform::app_state::AppState
};

pub async fn get_profile(
    state: State<'_, AppState>,
    profile_id: i64
) -> Result<Profile, String> {

    let profile: Profile = match ProfileRepository::find_profile_by_id(&state.pool, profile_id).await {
        Ok(profile) => profile,
        Err(e) => return Err(format!("Failed to get profil : {}", e))
    };

    Ok(profile)
}

pub async fn get_all_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<Profile>, String> {

    match ProfileRepository::find_all_active(&state.pool).await {
        Ok(profiles) => Ok(profiles),
        Err(e) => Err(format!("Failed to get profils : {}", e))
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateProfilePayload {
    pub name: String,
    pub avatar: Option<String>,
    pub color: String,
}

pub async fn create_profile(
    state: State<'_, AppState>,
    payload: CreateProfilePayload,
) -> Result<Profile, String> {

    match ProfileRepository::create(&state.pool, &state.pool, &payload.name, payload.avatar.as_deref(), &payload.color).await {
        Ok(profile) => Ok(profile),
        Err(e) => Err(format!("Failed to create profil : {}", e))
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfilePayload {
    pub id: i64,
    pub name: String,
    pub avatar: Option<String>,
    pub color: String,
}

/// Importe une image de profil (recadrée, réduite) ; rend son chemin, à donner à la création ou à la mise à jour.
pub async fn profile_avatar_import(source: String) -> Result<String, String> {
    let dir = crate::helper::profile::avatar_helper::avatars_dir();
    let path = crate::app::platform::runtime::spawn_blocking(move || crate::helper::profile::avatar_helper::import_into(std::path::Path::new(&source), &dir))
        .await
        .map_err(|e| e.to_string())??;
    Ok(path.to_string_lossy().into_owned())
}

pub async fn update_profile(
    state: State<'_, AppState>,
    payload: UpdateProfilePayload,
) -> Result<Profile, String> {
    // Image remplacée ou retirée : l'ancienne n'a plus d'usage.
    if let Ok(old) = ProfileRepository::find_profile_by_id(&state.pool, payload.id).await {
        if old.avatar != payload.avatar {
            crate::helper::profile::avatar_helper::remove_if_ours(old.avatar.as_deref());
        }
    }

    match ProfileRepository::update(&state.pool, &state.pool, payload.id, &payload.name, payload.avatar.as_deref(), &payload.color).await {
        Ok(profile) => Ok(profile),
        Err(e) => Err(format!("Failed to update profil : {}", e))
    }
}

pub async fn delete_profile(
    state: State<'_, AppState>,
    profile_id: i64,
) -> Result<(), String> {

    if profile_id == 1 {
        return Err("Impossible de supprimer le profil administrateur".into());
    }

    let avatar = ProfileRepository::find_profile_by_id(&state.pool, profile_id).await.ok().and_then(|p| p.avatar);
    match ProfileRepository::delete(&state.pool, profile_id).await {
        Ok(_) => {
            crate::helper::profile::avatar_helper::remove_if_ours(avatar.as_deref());
            Ok(())
        }
        Err(e) => Err(format!("Failed to delete profil : {}", e))
    }
}