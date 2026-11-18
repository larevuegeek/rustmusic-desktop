//! L'application de la fenêtre : son état et ses accès simples.

use std::sync::mpsc::Receiver;
use std::time::Instant;
use karbon_ui::{
    task::TaskCommand,
    ui::prelude::*,
    window::WindowCommand,
};
use crate::app::resources::assets::Assets;
use crate::app::platform::bridge::Backend;
use karbon_ui::task::Inbox;
use karbon_ui::intl;
use crate::models::app::Data;
use crate::models::app::Status;
use crate::models::app::Menu;
use crate::models::app::SleepTimer;
use crate::models::app::Msg;
use crate::models::app::Route;
use karbon_ui::navigation::History;

pub(crate) const MIX_SIZE: i64 = 50;

pub struct MusicApp {
    pub(crate) backend: Backend,
    /// La boîte aux lettres : résultats des commandes, événements du moteur, horloge.
    pub(crate) inbox: Inbox<MusicApp>,
    pub(crate) assets: Option<Assets>,
    pub data: Data,
    pub history: History<Route>,
    /// L'état des pages traduites.
    pub views: crate::models::app::Views,

    // ─── État de l'interface ───
    /// Thème effectif de la dernière vue (`auto` résolu), tenu à jour par `view`.
    pub dark: std::cell::Cell<bool>,
    /// Taille de la fenêtre à la dernière vue (unités logiques) : place la télécommande.
    pub viewport: std::cell::Cell<(f32, f32)>,
    /// Dernières variables de thème calculées (thème, contraste, teinte de la pochette,
    /// couleur d'accent).
    pub(crate) theme_cache: std::cell::RefCell<Option<((bool, bool, u32, u32, Option<u32>), CssVariables)>>,
    pub footer_collapsed: bool,
    pub sidebar_collapsed: bool,
    /// Écran de chargement : levé une fois le profil et ses bibliothèques chargés.
    pub booting: bool,
    pub search: String,
    /// Résultat en surbrillance.
    pub suggestion: Option<usize>,
    pub(crate) search_generation: u64,
    pub menu: Option<Menu>,
    pub sleep_timer: Option<SleepTimer>,
    pub(crate) sleep_end: Option<Instant>,
    pub sleep_input: String,
    /// Index dans `vm.sources`.
    pub source: usize,

    pub(crate) window_commands: Vec<WindowCommand>,
    pub(crate) tasks: Vec<TaskCommand>,
    /// Une vue a été construite depuis la dernière demande des pochettes de la page :
    /// `task_commands` les recalcule (sinon un `ViewModel` entier à chaque message).
    pub(crate) covers_due: std::cell::Cell<bool>,
    /// Le moteur a demandé la sortie.
    pub(crate) exiting: bool,
    /// Aperçu hors écran (`--export`) : les données sont attendues au chargement (`settle`).
    pub export: bool,
    /// Aperçu : menu ouvert, recherche tapée, page ouverte après le chargement.
    pub export_open: Option<Menu>,
    pub export_search: Option<String>,
    pub export_route: Option<Route>,
    /// Aperçu : défilement simulé de l'accueil (unités logiques).
    pub export_offset: f32,
    /// Messages rejoués après la route (`--folder` : ouvrir ce dossier…).
    pub export_msgs: Vec<Msg>,
    pub export_popup: Option<&'static str>,
    /// Aperçu : l'écran de chargement gardé affiché (« --open boot »).
    pub export_boot: bool,
    /// Aperçu : l'accueil en cours de chargement (« --open home-loading »).
    pub export_skeleton: bool,
    /// Le petit lecteur de la zone de notification, ouvert : sa boîte aux lettres.
    pub(crate) tray_player: Option<karbon_ui::task::InboxSender<crate::app::tray_player::TrayPlayer>>,
    /// Dernier état envoyé au petit lecteur.
    pub(crate) tray_sent: Option<crate::models::tray::TraySnapshot>,
    /// Fermeture du petit lecteur : le clic sur l'icône qui l'a refermé ne le rouvre pas.
    pub(crate) tray_closed_at: Option<Instant>,
    /// Morceau et état de lecture montrés par le menu de l'icône (Linux), pour ne le
    /// reposer que s'ils changent.
    pub(crate) tray_menu: Option<crate::app::platform::tray::TrayMenuState>,
    /// Colonne du tableau des morceaux en cours de redimensionnement.
    pub(crate) column_resize: Option<crate::models::tracks::ColumnResize>,
    /// Aperçu : la télécommande d'ampli avec un ampli fictif (« --open amp-remote[:mini|:sound|:nav|:standby] »).
    pub export_amp: Option<&'static str>,
}

impl MusicApp {
    pub fn new(app: crate::app::AppHandle, events: Receiver<crate::app::Event>) -> Self {
        let inbox = Inbox::new();
        Self {
            backend: Backend::new(app, events, &inbox),
            inbox,
            assets: None,
            data: Data::default(),
            history: History::new(Route::Home),
            views: crate::models::app::Views { whats_new: crate::models::updates::WhatsNewState::load(), ..Default::default() },
            dark: std::cell::Cell::new(true),
            viewport: std::cell::Cell::new((1280., 800.)),
            theme_cache: std::cell::RefCell::new(None),
            footer_collapsed: true,
            sidebar_collapsed: false,
            booting: true,
            search: String::new(),
            suggestion: None,
            search_generation: 0,
            menu: None,
            sleep_timer: None,
            sleep_end: None,
            sleep_input: "20".into(),
            source: 0,
            window_commands: Vec::new(),
            tasks: Vec::new(),
            covers_due: std::cell::Cell::new(true),
            exiting: false,
            export: false,
            export_open: None,
            export_search: None,
            export_route: None,
            export_offset: 0.,
            export_msgs: Vec::new(),
            export_popup: None,
            export_boot: false,
            export_skeleton: false,
            tray_player: None,
            tray_sent: None,
            tray_closed_at: None,
            tray_menu: None,
            column_resize: None,
            export_amp: None,
        }
    }

    pub fn route(&self) -> &Route {
        self.history.current()
    }

    pub fn open(&self, menu: Menu) -> bool {
        self.menu == Some(menu)
    }

    pub fn playing(&self) -> bool {
        self.data.player.status == Status::Playing
    }

    /// Titre du bouton du minuteur et sous-titre de son panneau.
    pub fn sleep_title(&self) -> String {
        match (self.sleep_timer, self.sleep_remaining()) {
            (Some(SleepTimer::TrackEnd), _) => "Arrêt en fin de morceau".into(),
            (_, Some(rest)) => format!("Veille dans {rest}"),
            _ => "Minuteur de veille".into(),
        }
    }

    /// Temps restant du minuteur.
    pub fn sleep_remaining(&self) -> Option<String> {
        let end = self.sleep_end?;
        let rest = end.saturating_duration_since(Instant::now()).as_secs_f64();
        Some(intl::clock(rest))
    }

    /// Le résumé de la pastille « Réseau ».
    pub fn network_summary(&self) -> String {
        let flags = [
            ("Lecteurs UPnP/DLNA", self.data.flag("network_players")),
            ("AirPlay (v1)", self.data.flag("airplay")),
            ("Serveur DLNA", self.data.flag("dlna_enabled")),
        ];
        let active_flags = flags
            .iter()
            .filter(|(_, on)| *on)
            .map(|(n, _)| *n)
            .collect::<Vec<_>>()
            .join(" · ");
        if active_flags.is_empty() {
            "Réseau : rien d'activé".into()
        } else {
            active_flags
        }
    }

    /// Ligne d'état d'un service réseau.
    pub fn network_status(&self, i: usize) -> String {
        let on = match i {
            0 => self.data.flag("network_players"),
            1 => self.data.flag("airplay"),
            _ => self.data.flag("dlna_enabled"),
        };
        match (i, on) {
            (_, false) => "Désactivé".into(),
            (2, true) => "En ligne".into(),
            _ => "Prêt : à choisir dans Sortie audio".into(),
        }
    }
}
