//! Types communs aux serveurs distants : playlist, sorte de serveur, identifiants gardés.

use super::*;
use serde::{Deserialize, Serialize};

/// Une playlist côté serveur, avec ses morceaux.
pub struct RemotePlaylist {
    pub id: String,
    pub name: String,
    pub comment: Option<String>,
    pub songs: Vec<RemoteSong>,
}

/// La sorte de serveur, gardée en base (`media_servers.kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Subsonic,
    Jellyfin,
    Dlna,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Subsonic => "subsonic",
            Kind::Jellyfin => "jellyfin",
            Kind::Dlna => "dlna",
        }
    }

    pub fn parse(s: &str) -> Kind {
        match s.to_ascii_lowercase().as_str() {
            "jellyfin" => Kind::Jellyfin,
            "dlna" => Kind::Dlna,
            _ => Kind::Subsonic,
        }
    }
}

/// Ce qui se garde pour rejoindre le serveur (jamais le mot de passe).
#[derive(Debug, Default, PartialEq)]
pub struct Stored {
    pub token: Option<String>,
    pub salt: Option<String>,
    pub api_key: Option<String>,
    pub user_id: Option<String>,
}
