use crate::commands::network::network_player_command::DeviceView;

/// Lecteurs réseau (UPnP/DLNA, AirPlay) proposés dans le menu de sortie.
#[derive(Default)]
pub struct NetworkPlayers {
    /// Appareils trouvés par la dernière recherche.
    pub devices: Vec<DeviceView>,
    /// L'appareil qui joue à la place de la sortie locale.
    pub selected: Option<DeviceView>,
    pub searching: bool,
    /// Une recherche a eu lieu : « aucun lecteur trouvé » a un sens.
    pub searched: bool,
}
