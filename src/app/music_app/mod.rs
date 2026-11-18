//! L'application : état de l'interface, démarrage, construction de la vue, événements du
//! moteur, tâches de fond (pompe d'événements, pochettes). Les messages sont traités par
//! `controller/`.

pub(crate) mod state;
pub(crate) mod startup;
pub(crate) mod engine;
pub(crate) mod region_keys;
pub(crate) mod theme;
pub(crate) mod application;
pub(crate) mod tasks;

pub use state::MusicApp;
pub(crate) use state::*;
pub(crate) use tasks::*;
