//! Le formulaire d'un serveur : chargement des données et réponse aux messages.

use crate::models::servers::*;
use crate::models::servers::DlnaPick;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::models::layout::PopinSize;
use crate::models::layout::ToastKind;
use crate::commands::network::server_command;

/// Les trois sortes de serveur, dans l'ordre des tuiles.
pub const KINDS: [&str; 3] = ["subsonic", "jellyfin", "dlna"];

pub(crate) fn type_label(kind: usize, info: &crate::core::subsonic::client::ServerInfo) -> String {
    let mut parts = Vec::new();
    if let Some(k) = info.kind.as_ref().filter(|k| !k.is_empty()) {
        parts.push(k[..1].to_uppercase() + &k[1..]);
    }
    if let Some(v) = info.version.as_ref().and_then(|v| v.split(' ').next()).filter(|v| !v.is_empty()) {
        parts.push(v.to_string());
    }
    if kind == 0 {
        parts.push(if info.open_subsonic { "OpenSubsonic".into() } else { "Subsonic".into() });
    }
    parts.join(" · ")
}

impl MusicApp {
    pub(crate) fn add_server(&mut self) -> Option<&mut AddServer> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::AddServer(f)) => Some(f),
            _ => None,
        }
    }

    pub(crate) fn server_folders(&mut self) -> Option<&mut ServerFolders> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::ServerFolders(f)) => Some(f),
            _ => None,
        }
    }

    pub(crate) fn server_form_update(&mut self, msg: ServerFormMsg) -> bool {
        match msg {
            ServerFormMsg::OpenAdd => {
                let form = AddServer {
                    kind: 0,
                    url: String::new(),
                    username: String::new(),
                    password: String::new(),
                    api_key: String::new(),
                    with_key: false,
                    probe: None,
                    name: String::new(),
                    chosen: Vec::new(),
                    dlna: None,
                    found: Vec::new(),
                    searching: false,
                    busy: false,
                    error: None,
                };
                self.open_popin(Popin {
                    title: t("servers.add"),
                    icon: Some("material-symbols:cloud-outline-rounded"),
                    body: PopinBody::AddServer(Box::new(form)),
                    size: PopinSize::Lg,
                    bare: false,
                    flush: false,
                });
                return true;
            }
            ServerFormMsg::OpenFolders(id) => return self.server_form_open_folders(id),
            ServerFormMsg::Submit if self.server_folders().is_some() => return self.server_form_folders_add(),
            ServerFormMsg::Back if self.server_folders().is_some_and(|f| !f.busy) => {
                self.views.popin = None;
                return true;
            }
            ServerFormMsg::Toggle(id) if self.server_folders().is_some() => {
                let Some(f) = self.server_folders() else { return false };
                if f.chosen.contains(&id) {
                    f.chosen.retain(|x| *x != id);
                } else if !f.existing.contains(&id) {
                    f.chosen.push(id);
                }
                return true;
            }
            _ => {}
        }
        let Some(f) = self.add_server() else { return false };
        match msg {
            ServerFormMsg::Kind(k) => {
                if f.busy {
                    return false;
                }
                f.kind = k;
                f.error = None;
                if k == 2 && !f.searching && f.found.is_empty() {
                    return self.server_form_update(ServerFormMsg::Discover);
                }
            }
            ServerFormMsg::Url(v) => {
                f.url = v;
                f.error = None;
            }
            ServerFormMsg::User(v) => {
                f.username = v;
                f.error = None;
            }
            ServerFormMsg::Pass(v) => {
                f.password = v;
                f.error = None;
            }
            ServerFormMsg::Key(v) => {
                f.api_key = v;
                f.error = None;
            }
            ServerFormMsg::ToggleKey => {
                f.with_key = !f.with_key;
                f.error = None;
            }
            ServerFormMsg::Discover => {
                f.searching = true;
                let app = self.backend.app.clone();
                self.backend.call(async move { server_command::server_discover(app.state()).await }, |me, r| {
                    let Some(f) = me.add_server() else { return false };
                    f.searching = false;
                    f.found = r.unwrap_or_default();
                    true
                });
            }
            ServerFormMsg::Found(i) => {
                let Some(s) = f.found.get(i) else { return false };
                f.url = s.location.clone();
                return self.server_form_connect();
            }
            ServerFormMsg::Connect => return self.server_form_connect(),
            ServerFormMsg::Back => {
                if f.busy {
                    return false;
                }
                if f.probe.is_some() {
                    f.probe = None;
                    f.dlna = None;
                    f.error = None;
                } else {
                    self.views.popin = None;
                }
            }
            ServerFormMsg::Name(v) => f.name = v,
            ServerFormMsg::Toggle(id) => {
                if f.chosen.contains(&id) {
                    f.chosen.retain(|x| *x != id);
                } else {
                    f.chosen.push(id);
                }
            }
            ServerFormMsg::Submit => {
                if f.probe.is_some() {
                    return self.server_form_add();
                }
                return self.server_form_connect();
            }
            ServerFormMsg::OpenAdd | ServerFormMsg::OpenFolders(_) => {}
        }
        true
    }

    pub(crate) fn server_form_connect(&mut self) -> bool {
        let Some(f) = self.add_server() else { return false };
        f.error = None;
        let missing = if f.is_dlna() {
            false
        } else if f.with_key {
            f.api_key.trim().is_empty() || (f.jellyfin() && f.username.trim().is_empty())
        } else {
            f.username.trim().is_empty() || (!f.jellyfin() && f.password.is_empty())
        };
        if f.url.trim().is_empty() || missing {
            f.error = Some(t("servers.fill_fields"));
            return true;
        }
        f.busy = true;
        let kind = f.kind;
        let (url, user) = (f.url.trim().to_string(), f.username.trim().to_string());
        let (pass, key) = if f.with_key { (None, Some(f.api_key.trim().to_string())) } else { (Some(f.password.clone()), None) };
        self.backend.call(
            async move {
                server_command::server_probe(Some(KINDS[kind].into()), url, user, pass, key)
                    .await
                    .map(|p| Probe { url: p.url, name: p.name, kind_label: type_label(kind, &p.info), folders: p.folders.into_iter().map(|f| (f.id, f.name)).collect() })
            },
            |me, r| {
                let Some(f) = me.add_server() else { return false };
                f.busy = false;
                match r {
                    Ok(p) => {
                        f.name = p.name.clone();
                        f.chosen = if f.is_dlna() { Vec::new() } else { p.folders.iter().map(|x| x.0.clone()).collect() };
                        f.dlna = f.is_dlna().then(|| DlnaPick::new(p.url.clone()));
                        let dlna = f.dlna.is_some();
                        f.probe = Some(p);
                        if dlna {
                            me.dlna_load();
                        }
                    }
                    Err(e) => f.error = Some(e),
                }
                true
            },
        );
        true
    }

    pub(crate) fn server_form_add(&mut self) -> bool {
        let profile = self.data.profile_id();
        let Some(f) = self.add_server() else { return false };
        let Some(probe) = &f.probe else { return false };
        let ids: Vec<String> = match &f.dlna {
            Some(d) => d.selected.iter().map(|s| s.0.clone()).collect(),
            None => f.chosen.clone(),
        };
        if ids.is_empty() {
            return false;
        }
        let Some(profile) = profile else { return false };
        f.error = None;
        f.busy = true;
        let kind = KINDS[f.kind].to_string();
        let (url, user) = (probe.url.clone(), f.username.trim().to_string());
        let (pass, key) = if f.with_key { (None, Some(f.api_key.trim().to_string())) } else { (Some(f.password.clone()), None) };
        let name = Some(f.name.trim().to_string()).filter(|n| !n.is_empty());
        let app = self.backend.app.clone();
        self.backend.call(
            async move { server_command::server_add(app.clone(), app.state(), profile, Some(kind), url, user, pass, key, name, ids).await },
            |me, r| {
                match r {
                    Ok(s) => {
                        me.views.popin = None;
                        me.toast(ToastKind::Success, t("servers.added").replace("{name}", &s.server.name), None);
                        me.reload_servers();
                        me.load_profile_data();
                    }
                    Err(e) => {
                        let Some(f) = me.add_server() else { return true };
                        f.busy = false;
                        f.error = Some(e);
                    }
                }
                true
            },
        );
        true
    }

    pub(crate) fn server_form_open_folders(&mut self, id: i64) -> bool {
        let Some(s) = self.data.servers.iter().find(|s| s.server.id == id) else { return false };
        let dlna = s.server.kind == "dlna";
        let form = ServerFolders {
            server_id: id,
            existing: s.libraries.iter().filter_map(|l| l.remote_folder_id.clone()).collect(),
            folders: Vec::new(),
            loading: !dlna,
            chosen: Vec::new(),
            dlna: dlna.then(|| DlnaPick::new(s.server.url.clone())),
            busy: false,
            error: None,
        };
        let title = t("servers.folders_title").replace("{name}", &s.server.name);
        self.open_popin(Popin {
            title,
            icon: Some("material-symbols:create-new-folder-outline-rounded"),
            body: PopinBody::ServerFolders(Box::new(form)),
            size: PopinSize::Lg,
            bare: false,
            flush: false,
        });
        if dlna {
            self.dlna_load();
            return true;
        }
        let app = self.backend.app.clone();
        self.backend.call(async move { server_command::server_folders(app.state(), id).await }, |me, r| {
            let Some(f) = me.server_folders() else { return false };
            f.loading = false;
            match r {
                Ok(v) => f.folders = v.into_iter().map(|x| (x.id, x.name)).collect(),
                Err(e) => f.error = Some(e),
            }
            true
        });
        true
    }

    pub(crate) fn server_form_folders_add(&mut self) -> bool {
        let Some(f) = self.server_folders() else { return false };
        let ids = f.new_ids();
        if ids.is_empty() {
            return false;
        }
        f.busy = true;
        f.error = None;
        let (id, n) = (f.server_id, ids.len());
        let app = self.backend.app.clone();
        self.backend.call(async move { server_command::server_add_libraries(app.clone(), app.state(), id, ids).await }, move |me, r| {
            match r {
                Ok(_) => {
                    me.views.popin = None;
                    me.toast(ToastKind::Success, t("servers.folders_added").replace("{n}", &n.to_string()), None);
                    me.reload_servers();
                    me.load_profile_data();
                }
                Err(e) => {
                    let Some(f) = me.server_folders() else { return true };
                    f.busy = false;
                    f.error = Some(e);
                }
            }
            true
        });
        true
    }
}
