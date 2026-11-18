use crate::models::library::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::commands::library::library_command;
use crate::entity::library::library::LibraryCreate;

impl MusicApp {
    pub(crate) fn library_form(&mut self) -> Option<&mut LibraryForm> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::LibraryForm(f)) => Some(f),
            _ => None,
        }
    }

    /// Le formulaire « Renommer » de la bibliothèque `id`.
    pub(crate) fn rename_library(&mut self, id: i64) -> bool {
        let Some(l) = self.data.libraries.iter().find(|l| l.id == id).cloned() else { return false };
        let f = LibraryForm { edit: Some((l.id, l.name.clone(), l.total_tracks)), name: l.name, desc: String::new(), error: None, global_error: None, busy: false };
        self.open_popin(Popin::new(t("selector.rename_title"), PopinBody::LibraryForm(Box::new(f))));
        true
    }

    /// La confirmation avant de supprimer la bibliothèque `id`.
    pub(crate) fn ask_delete_library(&mut self, id: i64) -> bool {
        let Some(l) = self.data.libraries.iter().find(|l| l.id == id).cloned() else { return false };
        self.menu = None;
        self.open_popin(Popin::confirm(
            t("library_head.delete"),
            t("library_head.delete_confirm").replace("{name}", &l.name),
            t("common.delete"),
            t("common.cancel"),
            true,
            Msg::LibraryForm(LibraryFormMsg::Delete(l.id)),
        ));
        true
    }

    pub(crate) fn library_form_update(&mut self, msg: LibraryFormMsg) -> bool {
        match msg {
            LibraryFormMsg::Create => {
                let f = LibraryForm { edit: None, name: String::new(), desc: String::new(), error: None, global_error: None, busy: false };
                self.open_popin(Popin::new(t("library.create_library"), PopinBody::LibraryForm(Box::new(f))));
            }
            LibraryFormMsg::Rename => {
                let Some(id) = self.data.library_id else { return false };
                return self.rename_library(id);
            }
            LibraryFormMsg::Name(v) => {
                let Some(f) = self.library_form() else { return false };
                f.name = v;
                f.error = None;
                f.global_error = None;
            }
            LibraryFormMsg::Desc(v) => {
                let Some(f) = self.library_form() else { return false };
                f.desc = v;
            }
            LibraryFormMsg::Submit => return self.library_form_submit(),
            LibraryFormMsg::AskDelete => {
                let Some(id) = self.data.library_id else { return false };
                return self.ask_delete_library(id);
            }
            LibraryFormMsg::Delete(id) => {
                let app = self.backend.app.clone();
                self.backend.call(async move { library_command::remove_library(app.state(), id).await }, |me, r| {
                    match r {
                        Ok(_) => {
                            me.load_profile_data();
                            return me.handle(Msg::Go(Route::Home));
                        }
                        Err(e) => log::error!("[bibliothèque] suppression : {e}"),
                    }
                    true
                });
            }
        }
        true
    }

    pub(crate) fn library_form_submit(&mut self) -> bool {
        let profile = self.data.profile_id();
        let Some(f) = self.library_form() else { return false };
        f.error = None;
        f.global_error = None;
        if f.name.trim().is_empty() {
            f.error = Some("forms.name_required");
            return true;
        }
        let Some(profile) = profile else {
            f.global_error = Some("forms.no_profil");
            return true;
        };
        f.busy = true;
        let (edit, name, desc) = (f.edit.clone(), f.name.trim().to_string(), Some(f.desc.trim().to_string()).filter(|d| !d.is_empty()));
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                match edit {
                    Some((id, old, _)) if old != name => library_command::rename_library(app.state(), id, name).await,
                    Some(_) => Ok(()),
                    None => library_command::create_library(app.state(), LibraryCreate { profile_id: profile, name, description: desc }).await.map(|_| ()),
                }
            },
            |me, r| {
                match r {
                    Ok(()) => {
                        me.views.popin = None;
                        me.load_profile_data();
                    }
                    Err(e) => {
                        let Some(f) = me.library_form() else { return true };
                        f.busy = false;
                        if e.contains("UNIQUE") || e.contains("duplicate") {
                            f.error = Some("popins.library_name_taken");
                        } else {
                            f.global_error = Some("forms.error_retry");
                        }
                    }
                }
                true
            },
        );
        true
    }
}
