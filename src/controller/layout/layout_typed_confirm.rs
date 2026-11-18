use crate::models::layout::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::layout::ToastKind;
use crate::commands::app::settings_command;

impl MusicApp {
    pub(crate) fn typed_update(&mut self, msg: TypedMsg) -> bool {
        match msg {
            TypedMsg::Reset => self.views.typed = Some(TypedConfirm { kind: TypedKind::Reset, typed: String::new(), busy: false }),
            TypedMsg::RemoveServer(id) => {
                let Some(s) = self.data.servers.iter().find(|s| s.server.id == id) else { return false };
                let libraries: Vec<(String, i64)> = s.libraries.iter().map(|l| (l.name.clone(), l.total_tracks)).collect();
                let tracks = libraries.iter().map(|l| l.1).sum();
                let kind = TypedKind::RemoveServer { id, name: s.server.name.clone(), libraries, tracks };
                self.views.typed = Some(TypedConfirm { kind, typed: String::new(), busy: false });
            }
            TypedMsg::Input(v) => {
                let Some(d) = self.views.typed.as_mut() else { return false };
                d.typed = v;
            }
            TypedMsg::Cancel => {
                if self.views.typed.as_ref().is_some_and(|d| !d.busy) {
                    self.views.typed = None;
                }
            }
            TypedMsg::Confirm => return self.typed_confirm(),
        }
        true
    }

    pub(crate) fn typed_confirm(&mut self) -> bool {
        let Some(d) = self.views.typed.as_mut() else { return false };
        if d.locked() {
            return false;
        }
        d.busy = true;
        match d.kind.clone() {
            TypedKind::Reset => {
                let app = self.backend.app.clone();
                self.backend.call(async move { settings_command::reset_application(app.state()).await }, |me, r| {
                    match r {
                        // La base sera recréée par les migrations : relancer l'application.
                        Ok(()) => {
                            if let Ok(exe) = std::env::current_exe() {
                                let _ = std::process::Command::new(exe).spawn();
                            }
                            me.handle(Msg::Quit);
                        }
                        Err(e) => {
                            if let Some(d) = me.views.typed.as_mut() {
                                d.busy = false;
                            }
                            log::error!("[confirmation] {e}");
                        }
                    }
                    true
                });
            }
            TypedKind::RemoveServer { id, .. } => {
                let app = self.backend.app.clone();
                self.backend.call(async move { crate::commands::network::server_command::server_remove(app.state(), id).await }, move |me, r| {
                    match r {
                        Ok(_) => {
                            me.views.typed = None;
                            me.reload_servers();
                        }
                        Err(e) => {
                            if let Some(d) = me.views.typed.as_mut() {
                                d.busy = false;
                            }
                            me.toast(ToastKind::Error, t("notify.error"), Some(e));
                        }
                    }
                    true
                });
            }
        }
        true
    }
}
