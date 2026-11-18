use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// L'état global du lecteur
#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct QueueState {
    #[serde(rename = "profilId")]
    #[sqlx(rename = "profil_id")]
    pub profile_id: i64,
    pub current_index: i32, // -1 quand rien n'est lu
    pub is_shuffled: bool,
    pub repeat_mode: String, // "off", "one", "all"
}