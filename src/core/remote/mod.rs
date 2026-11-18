//! Serveurs de musique distants (OpenSubsonic, Jellyfin, DLNA) : ce qu'ils ont en commun, derrière [`Backend`].

pub use crate::core::subsonic::catalog::RemoteSong;
pub use crate::core::subsonic::client::{MusicFolder, ServerInfo};
pub use crate::core::subsonic::lyrics::ServerLyrics;

mod model;
mod source;
mod backend;
#[cfg(test)]
mod tests;

pub use model::*;
pub use source::*;
pub use backend::*;
