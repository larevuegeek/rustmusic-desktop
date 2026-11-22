//! Mini et micro-lecteur : passage d'un mode de fenêtre à l'autre, animation, réponse aux
//! messages.

use crate::models::player::*;
use crate::mapper::player::mini::*;
use std::time::{Duration, Instant};
use karbon_ui::{
task::{CancellationToken, Task, TaskCommand, TaskId},
window::{WindowAction, WindowChange, WindowCommand, WindowId, WindowState},
};
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use karbon_ui::storage;

/// Plancher du mode normal (comme `ui::run`) et taille de départ quand la taille gardée
/// n'est pas fiable.
pub const NORMAL_MIN: (i32, i32) = (1100, 760);

pub(crate) const NORMAL_SIZE: (i32, i32) = (1280, 900);

/// Largeur du mini-lecteur, hauteur de sa partie repliée (barre 34, pochette 4 + 84,
/// commandes 12 + 46, onglets 14 + 40 + 12) et du panneau déroulé.
pub const MINI_W: i32 = 380;

pub(crate) const PANEL_H: i32 = 300;

pub(crate) const MICRO_POS_KEY: &str = "rustmusic:micro-pos";

/// Pas de l'animation de hauteur (easeOutCubic, 220 ms) : une tâche par image.
pub const ANIM_TASK: TaskId = TaskId(2);

pub(crate) const ANIM: Duration = Duration::from_millis(220);

pub(crate) const FRAME: Duration = Duration::from_millis(16);

/// Wayland ne donne ni ne laisse choisir la position : rien à retenir sous Linux.
pub(crate) const POSITIONS: bool = !cfg!(target_os = "linux");

pub(crate) fn change(change: WindowChange) -> WindowCommand {
    WindowCommand::Change(WindowId::MAIN, change)
}

/// Position du micro-lecteur gardée (`rustmusic:micro-pos`).
pub(crate) fn saved_micro_position() -> Option<(i32, i32)> {
    let value: serde_json::Value = serde_json::from_str(&storage::local().get(MICRO_POS_KEY)?).ok()?;
    Some((
        value.get("x")?.as_i64()? as i32,
        value.get("y")?.as_i64()? as i32,
    ))
}

impl MusicApp {
    pub(crate) fn mini_update(&mut self, message: MiniMsg) -> bool {
        match message {
            MiniMsg::Toggle if self.views.mini.active => self.mini_exit(),
            MiniMsg::Toggle => self.mini_enter(),
            MiniMsg::Restore => self.mini_exit(),
            // Fermer : même règle que la barre de titre.
            MiniMsg::Close if self.closes_to_tray() => {
                self.window_commands
                    .push(change(WindowChange::Visible(false)));
            }
            MiniMsg::Close => {
                // Masquée d'abord : on ne voit pas la fenêtre reprendre sa taille.
                self.window_commands
                    .push(change(WindowChange::Visible(false)));
                self.mini_exit();
                self.window_commands
                    .push(WindowCommand::Action(WindowId::MAIN, WindowAction::Close));
            }
            MiniMsg::Pin => {
                let m = &mut self.views.mini;
                m.pinned = !m.pinned;
                let pinned = m.pinned;
                self.window_commands
                    .push(change(WindowChange::Topmost(pinned)));
            }
            MiniMsg::Micro => self.micro_enter(),
            MiniMsg::BackToMini => self.micro_exit(),
            MiniMsg::Tab(tab) => {
                let m = &mut self.views.mini;
                m.tab = if m.tab == Some(tab) { None } else { Some(tab) };
                let open = m.tab.is_some();
                self.lyrics_sync();
                self.mini_expand(open);
            }
            MiniMsg::ClearUpcoming => return self.queue_clear_upcoming(),
            MiniMsg::Remove(id) => return self.queue_remove(&id),
            MiniMsg::Play(index) => return self.handle(Msg::PlayQueue(index)),
            MiniMsg::VolumeMenu(open) => self.views.mini.volume_open = open,
            MiniMsg::Mute => return self.handle(Msg::Mute),
            MiniMsg::Wheel { up, hint } => {
                let target = (i32::from(self.data.volume) + if up { 5 } else { -5 }).clamp(0, 100);
                if hint {
                    let m = &mut self.views.mini;
                    m.volume_hint = true;
                    m.hint_turn = m.hint_turn.wrapping_add(1);
                    let turn = m.hint_turn;
                    self.backend.call(
                        async { tokio::time::sleep(Duration::from_millis(900)).await },
                        move |me, ()| me.mini_update(MiniMsg::HintEnd(turn)),
                    );
                }
                return self.handle(Msg::Volume(f64::from(target))) || hint;
            }
            MiniMsg::HintEnd(turn) => {
                let m = &mut self.views.mini;
                if m.hint_turn != turn || !m.volume_hint {
                    return false;
                }
                m.volume_hint = false;
            }
        }
        true
    }

    /// Taille et position du mode normal gardées, fenêtre réduite, fixe
    /// et au premier plan.
    pub(crate) fn mini_enter(&mut self) {
        let m = &mut self.views.mini;
        if m.active {
            return;
        }
        crate::core::system::window_geometry::set_compact(true);
        let w = m.window;
        m.saved = Some(Saved {
            geometry: m.normal.unwrap_or((w.x, w.y, w.width, w.height)),
            maximized: w.maximized,
            fullscreen: w.fullscreen,
        });
        m.active = true;
        m.micro = false;
        m.tab = None;
        m.volume_open = false;
        m.height = COLLAPSED_H;
        m.anim = None;
        let pinned = m.pinned;
        self.menu = None;
        let mut commands = Vec::new();
        if w.fullscreen {
            commands.push(WindowChange::Fullscreen(false));
        }
        if w.maximized {
            commands.push(WindowChange::Maximized(false));
        }
        commands.extend([
            WindowChange::MinSize(None),
            WindowChange::Topmost(pinned),
            WindowChange::Resizable(false),
            WindowChange::Size {
                width: MINI_W,
                height: COLLAPSED_H,
            },
            WindowChange::KeepOnScreen,
        ]);
        self.window_commands
            .extend(commands.into_iter().map(change));
    }

    /// Bornes, taille, position et état d'avant ; une taille gardée trop
    /// petite (bascule ratée) laisse place à la taille d'origine, centrée.
    pub(crate) fn mini_exit(&mut self) {
        let m = &mut self.views.mini;
        if !m.active {
            return;
        }
        m.active = false;
        m.micro = false;
        m.anim = None;
        m.tab = None;
        m.volume_open = false;
        let saved = m.saved.take();
        let mut commands = vec![
            WindowChange::Topmost(false),
            WindowChange::AspectRatio(None),
            WindowChange::MaxSize(None),
            WindowChange::Resizable(true),
            WindowChange::MinSize(Some(NORMAL_MIN)),
        ];
        let reliable =
            saved.filter(|s| s.geometry.2 >= NORMAL_MIN.0 && s.geometry.3 >= NORMAL_MIN.1);
        match reliable {
            Some(Saved {
                geometry: (x, y, width, height),
                ..
            }) => {
                commands.push(WindowChange::Size { width, height });
                commands.push(WindowChange::Position { x, y });
            }
            None => {
                commands.push(WindowChange::Size {
                    width: NORMAL_SIZE.0,
                    height: NORMAL_SIZE.1,
                });
                commands.push(WindowChange::Center);
            }
        }
        if saved.is_some_and(|s| s.maximized) {
            commands.push(WindowChange::Maximized(true));
        }
        if saved.is_some_and(|s| s.fullscreen) {
            commands.push(WindowChange::Fullscreen(true));
        }
        self.window_commands
            .extend(commands.into_iter().map(change));
        crate::core::system::window_geometry::set_compact(false);
    }

    /// La fenêtre devient un carré, redimensionnable par sa poignée.
    pub(crate) fn micro_enter(&mut self) {
        let m = &mut self.views.mini;
        if !m.active || m.micro {
            return;
        }
        m.micro = true;
        m.anim = None;
        m.tab = None;
        m.volume_open = false;
        let size = m.micro_size;
        let mut commands = vec![
            WindowChange::MinSize(Some((MICRO_MIN, MICRO_MIN))),
            WindowChange::MaxSize(Some((MICRO_MAX, MICRO_MAX))),
            WindowChange::AspectRatio(Some((1, 1))),
            WindowChange::Size {
                width: size,
                height: size,
            },
        ];
        // Écran débranché depuis : la fenêtre est ramenée dans l'écran ci-dessous.
        if let Some((x, y)) = POSITIONS.then(saved_micro_position).flatten() {
            commands.push(WindowChange::Position { x, y });
        }
        commands.push(WindowChange::KeepOnScreen);
        self.window_commands
            .extend(commands.into_iter().map(change));
    }

    /// Retour au mini, panneau replié.
    pub(crate) fn micro_exit(&mut self) {
        let m = &mut self.views.mini;
        if !m.micro {
            return;
        }
        m.micro = false;
        m.tab = None;
        m.volume_open = false;
        m.height = COLLAPSED_H;
        self.window_commands.extend(
            [
                WindowChange::AspectRatio(None),
                WindowChange::MaxSize(None),
                WindowChange::MinSize(None),
                WindowChange::Size {
                    width: MINI_W,
                    height: COLLAPSED_H,
                },
                WindowChange::KeepOnScreen,
            ]
            .into_iter()
            .map(change),
        );
    }

    /// Anime la hauteur vers celle du panneau ouvert ou replié.
    pub(crate) fn mini_expand(&mut self, open: bool) {
        let m = &mut self.views.mini;
        let to = if open {
            COLLAPSED_H + PANEL_H
        } else {
            COLLAPSED_H
        };
        if self.export {
            m.height = to;
            return;
        }
        m.anim = Some((m.height, to, Instant::now()));
        self.mini_anim_step();
    }

    /// Une image de l'animation de hauteur (easeOutCubic) ; la suivante dans 16 ms.
    pub(crate) fn mini_anim_step(&mut self) -> bool {
        let m = &mut self.views.mini;
        let Some((from, to, start)) = m.anim else {
            return false;
        };
        if !m.active || m.micro {
            m.anim = None;
            return false;
        }
        let t = (start.elapsed().as_secs_f64() / ANIM.as_secs_f64()).min(1.);
        let eased = 1. - (1. - t).powi(3);
        let height = (f64::from(from) + f64::from(to - from) * eased).round() as i32;
        if height != m.height {
            let grows = height > m.height;
            m.height = height;
            self.window_commands.push(change(WindowChange::Size {
                width: MINI_W,
                height,
            }));
            // Le panneau ne dépasse pas en bas de l'écran.
            if grows {
                self.window_commands
                    .push(change(WindowChange::KeepOnScreen));
            }
        }
        if t < 1. {
            self.tasks.push(TaskCommand::Spawn(Task::new(
                ANIM_TASK,
                |_: CancellationToken| {
                    std::thread::sleep(FRAME);
                },
            )));
        } else {
            self.views.mini.anim = None;
        }
        false
    }

    /// `Application::window_changed` : géométrie normale retenue (et écrite pour le
    /// prochain lancement), taille et position du micro-lecteur retenues à la fin d'un geste.
    pub(crate) fn mini_window(&mut self, state: WindowState) -> bool {
        let m = &mut self.views.mini;
        let previous = std::mem::replace(&mut m.window, state);
        if !m.active {
            if !state.minimized && !state.maximized && !state.fullscreen {
                m.normal = Some((state.x, state.y, state.width, state.height));
            }
            if let Some((x, y, width, height)) = m.normal.filter(|_| !state.minimized) {
                crate::core::system::window_geometry::report(crate::core::system::window_geometry::Geometry {
                    x,
                    y,
                    width,
                    height,
                    maximized: state.maximized,
                    fullscreen: state.fullscreen,
                });
            }
            return false;
        }
        if !m.micro || state.minimized || state.dragging {
            return false;
        }
        // Poignée relâchée : taille retenue, fenêtre ramenée dans l'écran.
        let size = state.width.clamp(MICRO_MIN, MICRO_MAX);
        if size != m.micro_size && state.width == state.height {
            m.micro_size = size;
            if let Err(e) = storage::local().set(MICRO_SIZE_KEY, &size.to_string()) {
                log::warn!("Préférences locales : {e}");
            }
            self.window_commands
                .push(change(WindowChange::KeepOnScreen));
        }
        if POSITIONS && (state.x, state.y) != (previous.x, previous.y) {
            if let Err(e) = storage::local().set(
                MICRO_POS_KEY,
                &serde_json::json!({ "x": state.x, "y": state.y }).to_string(),
            ) {
                log::warn!("Préférences locales : {e}");
            }
        }
        false
    }
}
