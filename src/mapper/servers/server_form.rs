//! Le formulaire d'un serveur : ce qu'affichent ses gabarits, bâti depuis l'état de la page,
//! et les pochettes à charger.

use crate::models::servers::*;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::core::dlna_client::client::Found;

/// Les tuiles : titre, description.
pub fn kinds() -> Vec<(String, String)> {
    vec![
        (t("servers.kind_subsonic"), t("servers.intro")),
        ("Jellyfin".into(), t("servers.intro_jellyfin")),
        ("DLNA".into(), t("servers.intro_dlna")),
    ]
}

/// Modèle et hôte d'un serveur trouvé.
pub fn found_detail(s: &Found) -> String {
    let host = s.location.split("://").nth(1).and_then(|r| r.split('/').next()).unwrap_or_default();
    [s.model.as_str(), host].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ")
}

impl AddServer {
    pub fn is_dlna(&self) -> bool {
        self.kind == 2
    }

    pub fn jellyfin(&self) -> bool {
        self.kind == 1
    }

    /// Nombre de bibliothèques qui seront ajoutées.
    pub fn count(&self) -> usize {
        if self.is_dlna() {
            self.dlna.as_ref().map_or(0, |d| d.selected.len())
        } else {
            self.chosen.len()
        }
    }

    /// La note sous les identifiants.
    pub fn note(&self) -> String {
        t(&format!("servers.{}{}", if self.with_key { "api_key_note" } else { "password_note" }, if self.jellyfin() { "_jellyfin" } else { "" }))
    }
}

impl ServerFolders {
    /// `new_items` : les dossiers à ajouter.
    pub fn new_ids(&self) -> Vec<String> {
        match &self.dlna {
            Some(d) => d.selected.iter().map(|s| s.0.clone()).filter(|id| !self.existing.contains(id)).collect(),
            None => self.chosen.clone(),
        }
    }
}

impl Popin {
    /// Pied des fenêtres de serveur : annuler, libellé, icône, armé, occupé.
    pub fn server_form_footer(&self) -> Option<(String, String, &'static str, bool, bool)> {
        match &self.body {
            PopinBody::AddServer(f) => Some(match &f.probe {
                Some(_) => (t("servers.back"), t("servers.add_libraries").replace("{n}", &f.count().to_string()), "material-symbols:add-rounded", f.count() > 0, f.busy),
                None => (t("common.cancel"), t("servers.connect"), "material-symbols:link-rounded", true, f.busy),
            }),
            PopinBody::ServerFolders(f) => {
                let n = f.new_ids().len();
                Some((t("common.cancel"), t("servers.add_libraries").replace("{n}", &n.to_string()), "material-symbols:add-rounded", n > 0, f.busy))
            }
            _ => None,
        }
    }

    pub fn add_server(&self) -> Option<&AddServer> {
        match &self.body {
            PopinBody::AddServer(f) => Some(f),
            _ => None,
        }
    }

    pub fn server_folders(&self) -> Option<&ServerFolders> {
        match &self.body {
            PopinBody::ServerFolders(f) => Some(f),
            _ => None,
        }
    }
}
