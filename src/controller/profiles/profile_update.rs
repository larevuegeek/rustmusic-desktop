use crate::models::profiles::*;
use crate::mapper::profiles::profile_list::*;
use crate::app::music_app::MusicApp;
use crate::commands::app::profile_command::{self, CreateProfilePayload, UpdateProfilePayload};

impl MusicApp {
    pub(crate) fn profiles(&mut self) -> Option<&mut ProfileScreen> {
        self.views.profiles.as_mut()
    }

    pub(crate) fn profiles_update(&mut self, msg: ProfileMsg) -> bool {
        match msg {
            ProfileMsg::Open => {
                self.menu = None;
                self.views.profiles = Some(ProfileScreen {
                    mode: ProfileMode::Select,
                    name: String::new(),
                    color: 0,
                    avatar: None,
                    error: None,
                    busy: false,
                    delete_confirm: false,
                    delete_text: String::new(),
                });
            }
            ProfileMsg::Close => self.views.profiles = None,
            ProfileMsg::Select(id) => {
                self.views.profiles = None;
                if self.data.profile_id() == Some(id) {
                    return true;
                }
                self.data.profile = self.data.profiles.iter().find(|p| p.id == id).cloned();
                // Bibliothèques, file, likés.
                self.load_profile_data();
            }
            ProfileMsg::Create => {
                let Some(s) = self.profiles() else { return false };
                *s = ProfileScreen { mode: ProfileMode::Create, name: String::new(), avatar: None, error: None, ..s.clone() };
            }
            ProfileMsg::Edit(id) => {
                let Some(p) = self.data.profiles.iter().find(|p| p.id == id).cloned() else { return false };
                let Some(s) = self.profiles() else { return false };
                s.mode = ProfileMode::Edit(id);
                s.name = p.name;
                s.avatar = p.avatar;
                s.color = PALETTE.iter().position(|c| *c == p.color).unwrap_or(0);
                s.error = None;
            }
            ProfileMsg::Back => {
                let Some(s) = self.profiles() else { return false };
                s.mode = ProfileMode::Select;
                s.name.clear();
                s.error = None;
                s.delete_confirm = false;
                s.delete_text.clear();
            }
            ProfileMsg::Name(v) => {
                let Some(s) = self.profiles() else { return false };
                s.name = v;
                s.error = None;
            }
            ProfileMsg::Color(i) => {
                let Some(s) = self.profiles() else { return false };
                s.color = i;
            }
            ProfileMsg::ChooseAvatar => {
                if self.profiles().is_none() {
                    return false;
                }
                self.ask(crate::models::layout::Choice::ProfileAvatar);
                return false;
            }
            ProfileMsg::PresetAvatar(i) => {
                let Some(s) = self.profiles() else { return false };
                s.avatar = crate::mapper::profiles::profile_list::preset_value(i);
                s.error = None;
            }
            ProfileMsg::RemoveAvatar => {
                let Some(s) = self.profiles() else { return false };
                s.avatar = None;
            }
            ProfileMsg::DeleteAsk => {
                let Some(s) = self.profiles() else { return false };
                s.delete_confirm = true;
            }
            ProfileMsg::DeleteText(v) => {
                let Some(s) = self.profiles() else { return false };
                s.delete_text = v;
            }
            ProfileMsg::DeleteCancel => {
                let Some(s) = self.profiles() else { return false };
                s.delete_confirm = false;
                s.delete_text.clear();
            }
            ProfileMsg::Submit => return self.profiles_submit(),
            ProfileMsg::Delete => return self.profiles_delete(),
        }
        true
    }

    pub(crate) fn profiles_submit(&mut self) -> bool {
        let Some(s) = self.profiles() else { return false };
        if s.name.trim().is_empty() {
            s.error = Some("forms.name_required");
            return true;
        }
        s.busy = true;
        let (mode, name, color, avatar) = (s.mode, s.name.trim().to_string(), s.color_hex().to_string(), s.avatar.clone());
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                match mode {
                    ProfileMode::Edit(id) => profile_command::update_profile(app.state(), UpdateProfilePayload { id, name, avatar, color }).await,
                    _ => profile_command::create_profile(app.state(), CreateProfilePayload { name, avatar, color }).await,
                }
            },
            |me, r| {
                match r {
                    Ok(p) => {
                        match me.data.profiles.iter_mut().find(|x| x.id == p.id) {
                            Some(x) => *x = p.clone(),
                            None => me.data.profiles.push(p.clone()),
                        }
                        if me.data.profile_id() == Some(p.id) {
                            me.data.profile = Some(p);
                        }
                        let _ = me.profiles_update(ProfileMsg::Back);
                        if let Some(s) = me.profiles() {
                            s.busy = false;
                        }
                    }
                    Err(e) => {
                        let Some(s) = me.profiles() else { return true };
                        s.busy = false;
                        s.error = Some(if e.contains("UNIQUE") || e.contains("duplicate") { "forms.name_taken" } else { "forms.error_generic" });
                    }
                }
                true
            },
        );
        true
    }

    /// Image choisie pour le profil en cours de création ou d'édition : importée (recadrée
    /// en carré, réduite) dans le dossier de l'application, montrée tout de suite, gardée
    /// à l'enregistrement du profil.
    pub(crate) fn profile_avatar_import(&mut self, source: String) {
        let Some(s) = self.profiles() else { return };
        s.busy = true;
        s.error = None;
        self.backend.call(async move { profile_command::profile_avatar_import(source).await }, |me, r| {
            let Some(s) = me.profiles() else { return false };
            s.busy = false;
            match r {
                Ok(path) => s.avatar = Some(path),
                Err(e) => {
                    log::warn!("Image de profil : {e}");
                    s.error = Some("profil.avatar_error");
                }
            }
            true
        });
    }

    pub(crate) fn profiles_delete(&mut self) -> bool {
        let Some(s) = self.profiles() else { return false };
        let ProfileMode::Edit(id) = s.mode else { return false };
        if !s.delete_ready() || s.busy {
            return false;
        }
        let app = self.backend.app.clone();
        self.backend.call(async move { profile_command::delete_profile(app.state(), id).await }, move |me, r| {
            match r {
                Ok(()) => {
                    me.data.profiles.retain(|p| p.id != id);
                    if me.data.profile_id() == Some(id) {
                        me.data.profile = me.data.profiles.first().cloned();
                        me.load_profile_data();
                    }
                    let _ = me.profiles_update(ProfileMsg::Back);
                }
                Err(e) => log::error!("Suppression du profil : {e}"),
            }
            true
        });
        true
    }
}
