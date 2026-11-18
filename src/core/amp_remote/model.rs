//! Types communs à toutes les marques : l'ampli, ce qu'il sait faire, son état, les commandes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Brand {
    /// Onkyo, Pioneer et Integra : même protocole (eISCP).
    Onkyo,
    Denon,
    Yamaha,
}

/// Un ampli trouvé sur le réseau.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Amp {
    /// « onkyo:<mac> » : stable d'une session à l'autre.
    pub id: String,
    pub name: String,
    pub brand: Brand,
    pub model: String,
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Zone {
    Main,
    Zone2,
}

/// Un choix proposé : une entrée ou un mode d'écoute.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Choice {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Range {
    pub min: i32,
    pub max: i32,
    pub step: i32,
}

/// Ce que l'ampli sait faire, lu sur l'ampli quand il le dit.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Caps {
    pub inputs: Vec<Choice>,
    /// Modes directs (Stéréo, Direct…) puis touches qui font défiler une famille (Film, Musique…).
    pub modes: Vec<Choice>,
    pub volume_max: f32,
    pub volume_step: f32,
    pub tone: Option<Range>,
    pub zone2: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ZoneState {
    pub power: bool,
    pub volume: Option<f32>,
    pub muted: bool,
    pub input: Option<String>,
}

/// État de l'ampli, tenu à jour par ses propres messages.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct State {
    pub connected: bool,
    pub main: ZoneState,
    pub zone2: Option<ZoneState>,
    /// Code et nom du mode d'écoute en cours.
    pub mode: Option<Choice>,
    pub bass: Option<i32>,
    pub treble: Option<i32>,
    /// Écran des services réseau (NET, USB) : liste en cours et lecture.
    pub net: Net,
}

/// Une ligne de la liste affichée par l'ampli.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct NetLine {
    pub text: String,
    /// Nature : dossier, morceau, artiste, album, liste, recherche, en lecture…
    pub kind: String,
}

/// Ce que l'écran de l'ampli montre pour les services réseau.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Net {
    pub title: String,
    /// Nombre total d'éléments de la liste (toutes pages).
    pub count: u32,
    /// Ligne du curseur sur la page affichée.
    pub cursor: Option<u8>,
    /// Page affichée (dix lignes au plus).
    pub lines: Vec<NetLine>,
    /// L'écran montre la lecture plutôt qu'une liste.
    pub playback: bool,
    pub track: String,
    pub artist: String,
    pub album: String,
    /// « 01:23 / 04:56 », vide si inconnu.
    pub time: String,
    /// « play », « pause », « stop ».
    pub status: String,
    /// Révision de la pochette, servie à part (`amp-remote-cover`).
    pub cover_rev: u32,
}

/// Capacités et état, ce que l'interface affiche.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Status {
    pub caps: Caps,
    pub state: State,
    /// Pochette en cours (type, octets) : trop lourde pour voyager avec chaque état.
    #[serde(skip)]
    pub cover: Option<(String, Vec<u8>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavKey {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Back,
    Home,
    Menu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    PlayPause,
    Stop,
    Next,
    Previous,
    Repeat,
    Shuffle,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Power { zone: Zone, on: bool },
    Volume { zone: Zone, value: f32 },
    VolumeStep { zone: Zone, up: bool },
    Mute { zone: Zone, on: bool },
    Input { zone: Zone, id: String },
    Mode { id: String },
    Bass { value: i32 },
    Treble { value: i32 },
    /// Flèches et validation : liste réseau sur NET/USB, menu à l'écran sinon.
    Nav { key: NavKey },
    /// Ouvre la ligne `line` de la page affichée.
    NetSelect { line: u8 },
    Transport { action: Transport },
}

/// Protocole texte en TCP (Onkyo, Denon) : encode les commandes, lit les réponses.
/// Sans entrée/sortie, pour être testé à l'octet près.
pub trait Codec: Send {
    /// Requêtes envoyées à la connexion : capacités et état complet.
    fn hello(&self) -> Vec<Vec<u8>>;
    /// Question légère posée régulièrement : une connexion morte se voit vite.
    fn keepalive(&self) -> Vec<u8>;
    /// Pause entre deux envois : certains amplis perdent les commandes reçues à la file.
    fn gap(&self) -> std::time::Duration {
        std::time::Duration::ZERO
    }
    fn encode(&self, cmd: &Command, status: &Status) -> Vec<Vec<u8>>;
    /// Lit les messages complets de `buf` ; rend le nombre d'octets consommés.
    fn feed(&mut self, buf: &[u8], status: &mut Status) -> usize;
}
