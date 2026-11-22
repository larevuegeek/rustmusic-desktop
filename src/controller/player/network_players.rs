//! Lecteurs réseau dans le menu de sortie : recherche, choix de l'appareil, et reprise
//! du morceau en cours sur la nouvelle sortie, au même endroit.

use karbon_ui::intl::t;
use crate::app::music_app::*;
use crate::commands::network::network_player_command;
use crate::commands::playback::{player_command, volume_command};
use crate::models::app::Status;
use crate::models::layout::ToastKind;

/// Le morceau à reprendre après un changement de sortie : chemin, position, en lecture.
type ResumePoint = Option<(String, f64, bool)>;

impl MusicApp {
    /// Cherche les appareils des familles activées (~3 s).
    pub(crate) fn network_search(&mut self) {
        if self.network_output_off() || self.data.network.searching {
            return;
        }
        self.data.network.searching = true;
        self.backend.call(network_player_command::network_players_discover(), |me, devices| {
            let network = &mut me.data.network;
            network.devices = devices;
            network.searching = false;
            network.searched = true;
            network.selected = network_player_command::network_player_selected();
            true
        });
    }

    /// Ouverture du menu de sortie : l'appareil retrouvé au lancement, et la première recherche.
    pub(crate) fn network_menu_opened(&mut self) {
        self.data.network.selected = network_player_command::network_player_selected();
        if !self.data.network.searched {
            self.network_search();
        }
    }

    /// Joue sur le lecteur réseau `id`, ou revient à la sortie locale (`None`, avec le
    /// périphérique `local_device` s'il est choisi en même temps).
    pub(crate) fn network_choose(&mut self, id: Option<String>, local_device: Option<String>) {
        self.menu = None;
        if self.data.network.selected.as_ref().map(|d| d.id.as_str()) == id.as_deref() {
            return;
        }
        let resume = self.resume_point();
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                // Le périphérique d'abord : la reprise locale part dessus.
                if let Some(device) = local_device {
                    volume_command::set_device(Some(device)).await;
                }
                let selected = network_player_command::network_player_select(app.state(), id).await;
                (selected, crate::commands::playback::playback::volume().await)
            },
            move |me, (selected, volume)| {
                match selected {
                    Ok(selected) => me.data.network.selected = selected,
                    Err(e) => {
                        me.toast(ToastKind::Error, t("network_player.error"), Some(e));
                        return true;
                    }
                }
                me.data.volume = volume;
                // La chaîne locale ne décrit plus la sortie.
                me.data.player.pipeline = None;
                me.resume_on_new_output(resume);
                true
            },
        );
    }

    /// Une famille (UPnP/DLNA ou AirPlay) activée ou coupée. Coupée pendant qu'elle joue :
    /// retour à la sortie locale, le morceau y reprend.
    pub(crate) fn network_family_toggled(&mut self, kind: &'static str, on: bool) {
        let playing_there = self.data.network.selected.as_ref().is_some_and(|d| d.kind == kind);
        let resume = if !on && playing_there { self.resume_point() } else { None };
        self.backend.call(
            async move {
                let _ = if kind == "airplay" {
                    network_player_command::set_airplay_enabled(on).await
                } else {
                    network_player_command::set_network_players_enabled(on).await
                };
            },
            move |me, ()| {
                let network = &mut me.data.network;
                network.selected = network_player_command::network_player_selected();
                network.devices.retain(|d| on || d.kind != kind);
                network.searched = false;
                me.resume_on_new_output(resume);
                true
            },
        );
    }

    /// `network-player-lost` : l'appareil ne répond plus, la lecture s'est arrêtée.
    pub(crate) fn network_player_lost(&mut self, name: String) {
        self.data.network.selected = network_player_command::network_player_selected();
        self.toast(ToastKind::Error, t("network_player.lost"), Some(name));
    }

    /// `network-player-error` : le morceau n'a pas pu partir sur l'appareil.
    pub(crate) fn network_player_error(&mut self, message: String) {
        self.data.player.preparing = false;
        self.set_status(Status::Ended);
        self.toast(ToastKind::Error, t("network_player.error"), Some(message));
    }

    /// `playback-preparing` à faux : un morceau relancé sur une nouvelle sortie reprend à sa position.
    pub(crate) fn seek_if_resumed(&mut self) {
        let Some((path, position)) = self.data.player.seek_when_ready.take() else {
            return;
        };
        if self.data.player.path.as_deref() == Some(path.as_str()) {
            self.seek(position);
        }
    }

    /// Relance le morceau chargé à `position`, sur la sortie actuelle, sans le compter
    /// comme une nouvelle écoute.
    pub(crate) fn restart_current(&mut self, position: f64) {
        let Some(path) = self.data.player.path.clone() else {
            return;
        };
        self.data.player.stale_output = false;
        self.data.player.seek_when_ready = (position > 2.).then(|| (path.clone(), position));
        self.data.player.preparing = true;
        self.set_status(Status::Playing);
        let app = self.backend.app.clone();
        self.backend.fire(async move {
            player_command::stop_play().await;
            player_command::play_file(app, path).await;
        });
    }

    fn resume_point(&self) -> ResumePoint {
        let player = &self.data.player;
        let playing = match player.status {
            Status::Playing => true,
            Status::Paused => false,
            Status::Ready | Status::Ended => return None,
        };
        Some((player.path.clone()?, player.position, playing))
    }

    /// La sortie a changé (l'ancienne s'est arrêtée) : le morceau qui jouait repart sur
    /// la nouvelle ; en pause, il y repartira à la reprise.
    fn resume_on_new_output(&mut self, resume: ResumePoint) {
        let Some((path, position, playing)) = resume else {
            return;
        };
        if self.data.player.path.as_deref() != Some(path.as_str()) {
            return;
        }
        if playing {
            self.restart_current(position);
        } else {
            self.data.player.stale_output = true;
        }
    }
}
