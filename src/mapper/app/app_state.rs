//! Ce que les gabarits lisent de l'état de l'application : lecteur, file, bibliothèque,
//! profils et réglages d'affichage. Les gabarits passent par ces noms, jamais par `data`.

use crate::app::music_app::MusicApp;
use crate::entity::profile::profile::Profile;
use crate::models::app::{Route, Section};

impl MusicApp {
    // Lecteur et file.

    /// Volume, de 0 à 100.
    pub fn volume(&self) -> u8 {
        self.data.volume
    }

    /// Volume en nombre décimal, pour les curseurs.
    pub fn volume_value(&self) -> f64 {
        f64::from(self.data.volume)
    }

    /// Volume sous la moitié : l'icône du haut-parleur bas.
    pub fn volume_low(&self) -> bool {
        self.data.volume < 50
    }

    /// Icône du volume de la barre du lecteur.
    pub fn volume_icon(&self) -> &'static str {
        if self.volume_low() {
            "material-symbols-light:volume-down-outline-rounded"
        } else {
            "material-symbols-light:volume-up-outline-rounded"
        }
    }

    /// Un morceau est chargé dans le lecteur.
    pub fn has_track(&self) -> bool {
        self.data.player.path.is_some()
    }

    /// Position de lecture, en secondes.
    pub fn position(&self) -> f64 {
        self.data.player.position
    }

    /// La file est mélangée.
    pub fn is_shuffled(&self) -> bool {
        self.data.queue.is_shuffled
    }

    /// La répétition est active (la file ou un morceau).
    pub fn repeat_on(&self) -> bool {
        self.data.queue.repeat_mode != "off"
    }

    /// Icône de la répétition : un morceau, ou toute la file.
    pub fn repeat_icon(&self) -> &'static str {
        if self.data.queue.repeat_mode == "one" {
            "material-symbols-light:repeat-one-rounded"
        } else {
            "material-symbols-light:repeat-rounded"
        }
    }

    /// Le fichier est dans les titres likés.
    pub fn is_liked(&self, path: &str) -> bool {
        self.data.liked.contains(path)
    }

    /// La pochette du morceau courant.
    pub fn current_cover(&self) -> Option<&String> {
        self.data.current_track().and_then(|c| c.cover.as_ref())
    }

    /// Aucune sortie audio détectée.
    pub fn no_output_devices(&self) -> bool {
        self.data.devices.is_empty()
    }

    // Bibliothèque.

    /// La bibliothèque ouverte n'a aucun album.
    pub fn albums_empty(&self) -> bool {
        self.data.library().is_some_and(|l| l.total_albums == 0)
    }

    /// La bibliothèque ouverte n'a aucun artiste.
    pub fn artists_empty(&self) -> bool {
        self.data.library().is_some_and(|l| l.total_artists == 0)
    }

    /// La bibliothèque ouverte est sur ce poste (pas sur un serveur).
    pub fn is_local_library(&self) -> bool {
        self.data.library().is_some_and(|l| l.server_id.is_none())
    }

    /// La bibliothèque n'a aucun dossier.
    pub fn no_library_dirs(&self) -> bool {
        self.data.library_dirs.is_empty()
    }

    /// Les statistiques ne sont pas encore arrivées.
    pub fn stats_loading(&self) -> bool {
        self.data.stats.is_none()
    }

    /// Aucun genre connu.
    pub fn genres_empty(&self) -> bool {
        self.data.genres.is_empty()
    }

    /// Pas encore de genre, faute de statistiques.
    pub fn genres_loading(&self) -> bool {
        self.genres_empty() && self.stats_loading()
    }

    /// Aucun abonnement.
    pub fn podcasts_empty(&self) -> bool {
        self.data.podcasts.is_empty()
    }

    /// La page d'un podcast qui n'est pas épinglé : la ligne Podcasts reste allumée.
    pub fn unpinned_podcast_open(&self) -> bool {
        matches!(self.route(), Route::Podcast(id) if !self.data.podcasts.iter().any(|p| p.pinned && p.id == *id))
    }

    /// État joignable d'un serveur, une fois testé.
    pub fn server_online(&self, id: i64) -> Option<bool> {
        self.data.server_online.get(&id).copied()
    }

    // Profils.

    /// Les profils du poste.
    pub fn profile_list(&self) -> &[Profile] {
        &self.data.profiles
    }

    /// Le profil en cours.
    pub fn is_current_profile(&self, id: i64) -> bool {
        self.data.profile_id() == Some(id)
    }

    // Réglages d'affichage.

    pub fn show_sleep_timer(&self) -> bool {
        self.data.flag_default_on("show_sleep_timer")
    }

    /// Le bouton des nouveautés, quand il y a quelque chose à montrer.
    pub fn show_whats_new(&self) -> bool {
        self.data.flag_default_on("show_whats_new_button") && !crate::mapper::updates::update_releases::slides().is_empty()
    }

    pub fn show_theme_toggle(&self) -> bool {
        self.data.flag_default_on("show_theme_toggle")
    }

    pub fn show_favorites(&self) -> bool {
        self.data.flag_default_on("show_favorites")
    }

    /// Les bulles d'aide sur les termes techniques.
    pub fn help_bubbles(&self) -> bool {
        self.data.flag_default_on("show_help_bubbles")
    }

    pub fn podcasts_enabled(&self) -> bool {
        self.data.flag("podcasts")
    }

    pub fn radios_enabled(&self) -> bool {
        self.data.flag("radios")
    }

    /// La section Écouter mène aux podcasts, sinon aux radios.
    pub fn listen_route(&self) -> Route {
        if self.podcasts_enabled() { Route::Podcasts } else { Route::Radios }
    }

    /// Ni lecteurs réseau ni AirPlay : pas de menu de diffusion.
    pub fn network_output_off(&self) -> bool {
        !self.data.flag("network_players") && !self.data.flag("airplay")
    }

    pub fn wasapi_exclusive(&self) -> bool {
        self.data.flag("wasapi_exclusive")
    }

    /// Volume du mini-lecteur en barre horizontale.
    pub fn mini_volume_horizontal(&self) -> bool {
        self.data.setting("mini_volume_layout") == Some("horizontal")
    }

    /// Les onglets de bibliothèque sont dans la barre latérale (seuls ou avec ceux du haut).
    pub fn tabs_in_sidebar(&self) -> bool {
        self.data.tabs_placement() != 1
    }

    /// La position des onglets est celle de rang `i` (barre, haut, les deux).
    pub fn is_tabs_placement(&self, i: usize) -> bool {
        usize::from(self.data.tabs_placement()) == i
    }

    /// Raccourcis de la barre latérale sans épingle : albums et artistes, selon les réglages.
    pub fn pin_shortcuts(&self) -> Vec<(&'static str, String, Section)> {
        [
            ("show_pinned_albums", "material-symbols:album-outline-rounded", "library.albums", Section::Albums),
            ("show_pinned_artists", "material-symbols:mic-external-on-outline-rounded", "library.artists", Section::Artists),
        ]
        .into_iter()
        .filter(|s| self.data.flag_default_on(s.0))
        .map(|s| (s.1, karbon_ui::intl::t(s.2), s.3))
        .collect()
    }
}
