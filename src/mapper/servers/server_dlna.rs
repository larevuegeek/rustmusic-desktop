use crate::models::servers::*;

impl crate::models::servers::Probe {
    /// « Navidrome · https://music.example.com ».
    pub fn subtitle(&self) -> String {
        format!("{} · {}", self.kind_label, self.url)
    }
}

impl crate::models::servers::AddServer {
    /// Le nom tapé, ou celui que le serveur donne.
    pub fn display_name(&self, probe: &crate::models::servers::Probe) -> String {
        let typed = self.name.trim();
        if typed.is_empty() { probe.name.clone() } else { typed.to_string() }
    }
}

impl crate::entity::server::server::ServerLibrary {
    /// « · 1 234 » titres.
    pub fn tracks_suffix(&self) -> String {
        format!("· {}", karbon_ui::intl::number(self.total_tracks))
    }
}

impl crate::core::dlna_client::catalog::Folder {
    /// « 42 éléments », quand le serveur donne le nombre.
    pub fn items_label(&self) -> Option<String> {
        self.children
            .map(|n| karbon_ui::intl::t_with("servers.dlna_items", &[("n", &karbon_ui::intl::number(i64::from(n)))]))
    }
}

impl DlnaPick {
    /// Le fil d'Ariane, de la racine au dossier ouvert.
    pub fn crumbs(&self) -> Vec<DlnaCrumb> {
        let last = self.trail.len().saturating_sub(1);
        self.trail
            .iter()
            .enumerate()
            .map(|(index, (_, name))| DlnaCrumb {
                index,
                label: if index == 0 { karbon_ui::intl::t("servers.dlna_root") } else { name.clone() },
                current: index == last,
            })
            .collect()
    }

    pub fn new(url: String) -> Self {
        DlnaPick { url, trail: vec![("0".into(), String::new())], folders: Vec::new(), loading: true, error: None, selected: Vec::new() }
    }

    pub fn is_selected(&self, id: &str) -> bool {
        self.selected.iter().any(|s| s.0 == id)
    }
}
