use serde::{Deserialize, Serialize};

use crate::entity::queue::queue_track::QueueTrack;

// L'état global du lecteur
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueueStateRow {
    #[serde(rename = "profilId")]
    pub profile_id: i64,
    pub current_index: i32,
    pub is_shuffled: bool,
    pub repeat_mode: String,
    pub tracks: Vec<QueueTrack>
}