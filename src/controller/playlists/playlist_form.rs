use crate::models::playlists::*;
use crate::mapper::playlists::playlist_form::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::models::layout::ToastKind;
use crate::commands::playlist::playlist_command;

impl MusicApp {
    pub(crate) fn playlist_form(&mut self) -> Option<&mut PlaylistForm> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::PlaylistForm(f)) => Some(f),
            _ => None,
        }
    }

    pub(crate) fn playlist_form_update(&mut self, msg: PlaylistFormMsg) -> bool {
        match msg {
            PlaylistFormMsg::New(then_add) => {
                let form = PlaylistForm { edit: None, name: String::new(), desc: String::new(), color: 0, icon: 0, error: None, busy: false, then_add };
                self.open_popin(Popin::new(t("playlist.new"), PopinBody::PlaylistForm(Box::new(form))));
            }
            PlaylistFormMsg::Edit(id) => {
                let Some(p) = self.data.playlists.iter().find(|p| p.id == id) else { return false };
                let form = PlaylistForm {
                    edit: Some(id),
                    name: p.name.clone(),
                    desc: p.description.clone().unwrap_or_default(),
                    color: COLORS.iter().position(|c| c.eq_ignore_ascii_case(&p.color)).unwrap_or(0),
                    icon: ICONS.iter().position(|i| *i == p.icon).unwrap_or(0),
                    error: None,
                    busy: false,
                    then_add: None,
                };
                self.open_popin(Popin::new(t("playlist.edit"), PopinBody::PlaylistForm(Box::new(form))));
            }
            PlaylistFormMsg::Name(v) => {
                let Some(f) = self.playlist_form() else { return false };
                f.name = v;
                f.error = None;
            }
            PlaylistFormMsg::Desc(v) => {
                if let Some(f) = self.playlist_form() {
                    f.desc = v;
                }
            }
            PlaylistFormMsg::Color(i) => {
                if let Some(f) = self.playlist_form() {
                    f.color = i;
                }
            }
            PlaylistFormMsg::Icon(i) => {
                if let Some(f) = self.playlist_form() {
                    f.icon = i;
                }
            }
            PlaylistFormMsg::Submit => return self.playlist_form_submit(),
        }
        true
    }

    pub(crate) fn playlist_form_submit(&mut self) -> bool {
        let profile = self.data.profile_id().unwrap_or(0);
        let Some(f) = self.playlist_form() else { return false };
        if f.name.trim().is_empty() {
            f.error = Some(t("forms.name_required"));
            return true;
        }
        f.busy = true;
        let desc = Some(f.desc.trim().to_string()).filter(|d| !d.is_empty());
        let payload = serde_json::json!({
            "id": f.edit, "profil_id": profile, "name": f.name.trim(), "description": desc,
            "color": f.color_hex(), "icon": f.icon_id(), "server_id": null, "source_id": null,
        });
        let (edit, then_add) = (f.edit, f.then_add.clone());
        let appending = then_add.is_some();
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                match edit {
                    Some(_) => match serde_json::from_value(payload) {
                        Ok(p) => playlist_command::update_playlist(app.state(), p).await,
                        Err(e) => Err(e.to_string()),
                    },
                    None => {
                        let created_item = match serde_json::from_value(payload) {
                            Ok(p) => playlist_command::create_playlist(app.state(), p).await,
                            Err(e) => Err(e.to_string()),
                        }?;
                        if let Some((id, path)) = then_add {
                            let _ = playlist_command::add_track_to_playlist(app.state(), created_item.id, Some(id), Some(path)).await;
                        }
                        Ok(created_item)
                    }
                }
            },
            move |me, r| {
                match r {
                    Ok(p) => {
                        me.views.popin = None;
                        me.load_playlists();
                        if appending {
                            me.toast(ToastKind::Success, t("notify.added"), Some(t("notify.added_to").replace("{name}", &p.name)));
                        }
                    }
                    Err(e) => {
                        let Some(f) = me.playlist_form() else { return true };
                        f.busy = false;
                        f.error = Some(t(if e.contains("UNIQUE") || e.contains("duplicate") { "popins.playlist_name_taken" } else { "forms.error_generic" }));
                    }
                }
                true
            },
        );
        true
    }
}
