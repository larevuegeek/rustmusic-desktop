use crate::models::settings::*;
use karbon_ui::intl;
use crate::helper::date::parse_date;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::settings::Control;
use crate::models::settings::Ghost;
use crate::models::settings::GroupAction;
use crate::models::settings::OptionGroup;
use crate::models::settings::OptionRow;
use crate::models::settings::SettingsMsg;
use crate::entity::server::server::ServerEntry;

/// Qualité de lecture : l'original, ou un transcodage fait par le serveur.
pub const QUALITIES: [(Option<&str>, Option<i64>); 4] = [(None, None), (Some("mp3"), Some(320)), (Some("mp3"), Some(192)), (Some("mp3"), Some(128))];

/// Les libellés de `QUALITIES`.
pub fn quality_labels() -> Vec<String> {
    vec![t("servers.quality_original"), "MP3 320 kbit/s".into(), "MP3 192 kbit/s".into(), "MP3 128 kbit/s".into()]
}

/// L'indice de la qualité du serveur.
pub fn quality_of(s: &ServerEntry) -> u64 {
    match (&s.server.transcode_format, s.server.transcode_bitrate.unwrap_or(192)) {
        (None, _) => 0,
        (Some(_), 320) => 1,
        (Some(_), 128) => 3,
        _ => 2,
    }
}

/// `describe` : type, version, utilisateur, adresse.
pub fn describe(s: &ServerEntry) -> String {
    let v = &s.server;
    let mut type_ = v.server_type.clone().unwrap_or_default();
    if let Some(c) = type_.get(..1) {
        type_ = c.to_uppercase() + &type_[1..];
    }
    [Some(type_), v.server_version.as_ref().and_then(|x| x.split(' ').next()).map(String::from), Some(v.username.clone()), Some(v.url.clone())]
        .into_iter()
        .flatten()
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// « Synchronisé le … » ou « Jamais synchronisé ».
pub fn synced_label(s: &ServerEntry) -> String {
    match &s.server.last_sync_at {
        Some(d) => t("servers.synced_at").replace("{date}", &parse_date(d).map_or_else(|| (d).to_string(), |d| intl::date_time(d.into()))),
        None => t("servers.never_synced"),
    }
}

impl MusicApp {
    pub(crate) fn settings_servers(&self) -> OptionGroup {
        let mut lines: Vec<Option<OptionRow>> = self
            .data
            .servers
            .iter()
            .map(|s| {
                self.keep(
                    OptionRow::block(&s.server.id.to_string(), Control::Block("server")),
                    &format!("serveur subsonic opensubsonic navidrome jellyfin dlna minidlna {} {}", s.server.name, s.server.url),
                )
            })
            .collect();
        if self.data.servers.is_empty() {
            lines.push(self.keep(
                OptionRow::new("servers_empty", String::new(), t("servers.empty"), Control::Block("servers-empty")),
                "serveur subsonic opensubsonic navidrome jellyfin dlna minidlna gonic airsonic",
            ));
        } else {
            lines.push(self.keep(
                OptionRow::new(
                    "duplicate_version",
                    t("sources.duplicate"),
                    t("sources.duplicate_hint"),
                    Control::Select(
                        "duplicate_version",
                        self.data.setting("duplicate_version").unwrap_or("local").to_string(),
                        vec![("local".into(), t("sources.duplicate_local")), ("server".into(), t("sources.duplicate_server"))],
                    ),
                ),
                "double doublon version locale serveur toutes les sources",
            ));
        }
        lines.push(self.keep(OptionRow::block("servers_add", Control::Block("servers-add")), "ajouter serveur subsonic navidrome jellyfin dlna"));
        self.option_group("servers", Some(t("servers.group")), Some(t("servers.group_hint")), Some("servers.group"), lines)
            .with_action(GroupAction::Badge(t("settings.beta_badge"), "amber"))
    }

    /// Le serveur d'une ligne `server`.
    pub fn server_of(&self, key: &str) -> Option<&ServerEntry> {
        self.data.servers.iter().find(|s| s.server.id.to_string() == key)
    }

    /// Ses boutons : dossiers, synchroniser (actif), retirer.
    pub fn server_buttons(&self, s: &ServerEntry) -> Vec<Ghost> {
        let sm = |m: ServerMsg| Msg::Settings(SettingsMsg::Server(m));
        let mut b = Vec::new();
        if s.server.enabled {
            b.push(Ghost::new("material-symbols:create-new-folder-outline-rounded", t("servers.folders"), Msg::ServerForm(crate::models::servers::ServerFormMsg::OpenFolders(s.server.id))));
            let sync = self.views.settings.syncing;
            b.push(Ghost::new("material-symbols:sync-rounded", t("servers.sync"), sm(ServerMsg::Sync(s.server.id))).busy(sync == Some(s.server.id)).disabled(sync.is_some()));
        }
        b.push(Ghost::new("material-symbols:delete-outline-rounded", t("servers.remove"), sm(ServerMsg::AskRemove(s.server.id))));
        b
    }
}
