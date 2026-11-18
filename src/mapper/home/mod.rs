//! Les modèles que les templates lisent, dérivés de `Data` à chaque vue : fiche du
//! morceau, cartes de l'accueil, lignes de la barre latérale, sorties audio…

pub(crate) mod home_genres;
pub(crate) mod home_mixes;
pub(crate) mod home_page;
pub(crate) mod home_playlists;
pub(crate) mod home_resume;
pub(crate) mod home_sources;
pub(crate) mod home_welcome;

pub(crate) use home_genres::*;
pub(crate) use home_mixes::*;
pub(crate) use home_page::*;
pub(crate) use home_playlists::*;
pub(crate) use home_resume::*;
pub(crate) use home_sources::*;
pub(crate) use home_welcome::*;
