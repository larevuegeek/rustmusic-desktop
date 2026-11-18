//! La télécommande d'ampli : panneau, commandes, appuis maintenus, état publié par la session.

use std::time::{Duration, Instant};
use crate::app::music_app::MusicApp;
use crate::commands::network::amp_remote_command;
use crate::core::amp_remote::model::{Command, Zone};
use crate::core::amp_remote::session;
use crate::models::amp_remote::*;
use crate::models::app::{Msg, Route, SettingsPage};
use crate::models::layout::ToastKind;
use crate::mapper::amp_remote::amp_remote_view::{clamp_top, dock_setting, is_network, DOCK_EDGE, SNAP_ZONE};
use karbon_ui::ui::drag_drop::{PanEvent, PanPhase};

/// −/+ maintenus : un pas, puis un toutes les 120 ms après 380 ms.
const REPEAT_AFTER: u64 = 380;
const REPEAT_EVERY: u64 = 120;
/// Au plus 40 pas répétés par appui (~5 s).
const MAX_REPEATS: u32 = 40;
/// Veille : appui maintenu, pour éviter les coupures par mégarde.
const POWER_HOLD: u64 = 800;
const HINT_FOR: u64 = 1600;
/// Molette : un pas toutes les 70 ms au plus.
const WHEEL_GAP: Duration = Duration::from_millis(70);

impl MusicApp {
    pub(crate) fn amp_remote_update(&mut self, msg: AmpRemoteMsg) -> bool {
        match msg {
            AmpRemoteMsg::Toggle => {
                let open = !self.data.flag("amp_remote_open");
                self.views.amp_remote.slid = true;
                self.set_setting("amp_remote_open", if open { "true" } else { "false" });
            }
            AmpRemoteMsg::Mini => {
                let mini = !self.data.flag("amp_remote_mini");
                self.set_setting("amp_remote_mini", if mini { "true" } else { "false" });
            }
            AmpRemoteMsg::Tab(tab) => self.views.amp_remote.tab = tab,
            AmpRemoteMsg::Send(cmd) => self.amp_send(cmd),
            AmpRemoteMsg::Drag(zone, value) => self.views.amp_remote.drag = Some((zone, value as f32)),
            AmpRemoteMsg::Release(zone, value) => {
                self.views.amp_remote.drag = None;
                self.amp_send(Command::Volume { zone, value: value as f32 });
            }
            AmpRemoteMsg::Step(zone, up, true) => {
                let token = self.amp_token();
                let r = &mut self.views.amp_remote;
                r.repeat = Some((zone, up));
                r.repeats = 0;
                self.amp_after(REPEAT_AFTER, AmpRemoteMsg::Repeat(token));
                return false;
            }
            AmpRemoteMsg::Step(_, _, false) => {
                self.views.amp_remote.repeat = None;
                self.amp_token();
                return false;
            }
            AmpRemoteMsg::StepClick(zone, up) => {
                let r = &mut self.views.amp_remote;
                let repeated = std::mem::take(&mut r.repeats) > 0;
                r.repeat = None;
                self.amp_token();
                if !repeated {
                    self.amp_send(Command::VolumeStep { zone, up });
                }
                return false;
            }
            AmpRemoteMsg::Repeat(token) => {
                let r = &mut self.views.amp_remote;
                let Some((zone, up)) = r.repeat.filter(|_| r.token == token) else { return false };
                r.repeats += 1;
                // Garde-fou : un relâchement perdu ne monte jamais le volume sans fin.
                if r.repeats >= MAX_REPEATS {
                    r.repeat = None;
                }
                self.amp_send(Command::VolumeStep { zone, up });
                if self.views.amp_remote.repeat.is_some() {
                    self.amp_after(REPEAT_EVERY, AmpRemoteMsg::Repeat(token));
                }
                return false;
            }
            AmpRemoteMsg::Power(true) => {
                let on = self.views.amp_remote.view.as_ref().is_some_and(|v| v.status.state.connected && v.status.state.main.power);
                if !on {
                    return false;
                }
                let token = self.amp_token();
                let r = &mut self.views.amp_remote;
                r.power_pressed = true;
                r.holding = true;
                r.hint = false;
                self.amp_after(POWER_HOLD, AmpRemoteMsg::PowerHeld(token));
            }
            AmpRemoteMsg::Power(false) => {
                if !self.views.amp_remote.holding {
                    return false;
                }
                // Lâché trop tôt : l'indice dit qu'il faut maintenir.
                self.views.amp_remote.holding = false;
                self.views.amp_remote.hint = true;
                let token = self.amp_token();
                self.amp_after(HINT_FOR, AmpRemoteMsg::HintDone(token));
            }
            AmpRemoteMsg::PowerClick => {
                let Some(state) = self.views.amp_remote.view.as_ref().map(|v| &v.status.state).filter(|s| s.connected) else { return false };
                let on = state.main.power;
                // À la souris, la veille passe par l'appui maintenu ; au clavier, le clic suffit.
                if std::mem::take(&mut self.views.amp_remote.power_pressed) && on {
                    return false;
                }
                self.amp_send(Command::Power { zone: Zone::Main, on: !on });
            }
            AmpRemoteMsg::PowerHeld(token) => {
                let r = &mut self.views.amp_remote;
                if r.token != token || !r.holding {
                    return false;
                }
                r.holding = false;
                self.amp_send(Command::Power { zone: Zone::Main, on: false });
            }
            AmpRemoteMsg::HintDone(token) => {
                if self.views.amp_remote.token != token {
                    return false;
                }
                self.views.amp_remote.hint = false;
            }
            AmpRemoteMsg::StepInput(delta) => {
                let Some(view) = self.views.amp_remote.view.as_ref() else { return false };
                let inputs = &view.status.caps.inputs;
                if inputs.is_empty() {
                    return false;
                }
                let current = view.status.state.main.input.as_deref().unwrap_or("");
                let index = inputs.iter().position(|i| i.id.eq_ignore_ascii_case(current)).map_or(0, |i| i as i32);
                let next = (index + delta).rem_euclid(inputs.len() as i32) as usize;
                let id = inputs[next].id.clone();
                self.amp_send(Command::Input { zone: Zone::Main, id });
            }
            AmpRemoteMsg::Wheel(up) => {
                let on = self.views.amp_remote.view.as_ref().is_some_and(|v| v.status.state.connected && v.status.state.main.power);
                if !on || !self.amp_wheel_ready() {
                    return false;
                }
                self.amp_send(Command::VolumeStep { zone: Zone::Main, up });
                return false;
            }
            AmpRemoteMsg::NavWheel(key) => {
                if self.amp_wheel_ready() {
                    self.amp_send(Command::Nav { key });
                }
                return false;
            }
            AmpRemoteMsg::Pad(wheel) => self.set_setting("amp_remote_pad", if wheel { "wheel" } else { "keys" }),
            AmpRemoteMsg::Settings => return self.handle(Msg::Go(Route::Settings(SettingsPage::Remote))),
            AmpRemoteMsg::Pan(event) => return self.amp_pan(event),
        }
        true
    }

    /// La languette glissée : le moteur déplace la télécommande (`data-pan-frame`) ;
    /// lâchée près d'un bord, elle s'y colle, sinon elle reste où on la pose. Le moteur la
    /// fait glisser du point de lâcher jusqu'à cette place.
    fn amp_pan(&mut self, event: PanEvent) -> bool {
        match event.phase {
            PanPhase::Start => {
                self.views.amp_remote.pan_from = Some(self.amp_dock());
                true
            }
            PanPhase::Move => false,
            PanPhase::End => {
                let (width, height) = self.viewport.get();
                let span = self.amp_span();
                let Some(from) = self.views.amp_remote.pan_from.take() else { return false };
                let inset = if from.left { from.inset + event.dx } else { from.inset - event.dx };
                // Écarts aux deux bords de la fenêtre.
                let left_gap = if from.left { inset } else { width - inset - span };
                let right_gap = width - left_gap - span;
                let snap_left = left_gap < SNAP_ZONE && left_gap <= right_gap;
                let snap_right = right_gap < SNAP_ZONE && !snap_left;
                let dock = match (snap_left, snap_right) {
                    (true, _) => AmpDock { left: true, inset: DOCK_EDGE, top: from.top + event.dy },
                    (_, true) => AmpDock { left: false, inset: DOCK_EDGE, top: from.top + event.dy },
                    _ => AmpDock { left: from.left, inset: inset.max(0.), top: from.top + event.dy },
                };
                let dock = AmpDock { top: clamp_top(dock.top, height), ..dock };
                self.set_setting("amp_remote_dock", dock_setting(dock, height));
                true
            }
        }
    }

    /// Commande envoyée à l'ampli ; l'état revient par l'ampli lui-même.
    fn amp_send(&mut self, cmd: Command) {
        if let Err(e) = amp_remote_command::amp_remote_send(cmd) {
            self.toast(ToastKind::Error, e, None);
        }
    }

    /// Un nouveau jeton : les minuteries en cours sont périmées.
    fn amp_token(&mut self) -> u64 {
        self.views.amp_remote.token += 1;
        self.views.amp_remote.token
    }

    fn amp_after(&mut self, ms: u64, msg: AmpRemoteMsg) {
        self.backend.call(async move { tokio::time::sleep(Duration::from_millis(ms)).await }, move |me, ()| me.amp_remote_update(msg));
    }

    fn amp_wheel_ready(&mut self) -> bool {
        let r = &mut self.views.amp_remote;
        if r.last_wheel.is_some_and(|t| t.elapsed() < WHEEL_GAP) {
            return false;
        }
        r.last_wheel = Some(Instant::now());
        true
    }

    /// Nouvel état publié par la session (`amp-remote-state`). Le passage sur NET ou USB
    /// ouvre l'onglet Navigation.
    pub(crate) fn amp_remote_refresh(&mut self) -> bool {
        let r = &mut self.views.amp_remote;
        // Aperçu : l'ampli fictif reste en place.
        if self.export_amp.is_none() {
            r.view = session::view();
        }
        let input = r
            .view
            .as_ref()
            .and_then(|v| {
                let id = v.status.state.main.input.as_deref()?;
                v.status.caps.inputs.iter().find(|i| i.id.eq_ignore_ascii_case(id)).map(|i| i.name.clone())
            })
            .unwrap_or_default();
        if input != r.last_input {
            if is_network(&input) {
                r.tab = AmpTab::Nav;
            }
            r.last_input = input;
        }
        if r.view.is_none() {
            r.cover = None;
        }
        true
    }

    /// Nouvelle pochette de l'écran réseau (`amp-remote-cover`), décodée ici.
    pub(crate) fn amp_remote_cover(&mut self) -> bool {
        let cover = session::cover_bytes().and_then(|(_, bytes)| self.assets.as_ref()?.decode(&bytes));
        self.views.amp_remote.cover = cover;
        true
    }
}
