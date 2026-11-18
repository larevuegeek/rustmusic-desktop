use crate::models::library::*;
use chrono::Utc;
use karbon_ui::intl;
use crate::helper::date::elapsed_seconds;
use karbon_ui::intl::t;
use crate::entity::library::library_dirs::LibraryDir;

pub(crate) fn nb(n: i64, one: &str, many: &str) -> String {
    if n == 1 {
        t(one)
    } else {
        t(many).replace("{n}", &intl::number(n))
    }
}

impl LibraryFolders {
    /// `resume` : dossiers, fichiers, taille.
    pub fn summary(&self) -> String {
        if self.loading {
            return t("common.loading");
        }
        let files: i64 = self.dirs.iter().map(|d| d.total_files).sum();
        let size: i64 = self.dirs.iter().map(|d| d.total_size).sum();
        [
            nb(self.dirs.len() as i64, "folders_popin.folders_one", "folders_popin.folders_n"),
            if files > 0 { nb(files, "folders_popin.files_one", "folders_popin.files_n") } else { String::new() },
            if size > 0 { karbon_ui::intl::bytes(size as u64) } else { String::new() },
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
    }

    /// Pastilles d'un dossier : fichiers, taille, état ; et l'état (0 fait, 1 en cours, 2 jamais).
    pub fn pills(&self, d: &LibraryDir) -> (Option<String>, Option<String>, String, u8) {
        let busy = self.rescanning.as_deref() == Some(d.id.as_str()) || d.scan_status == "scanning";
        let state = if busy {
            t("folders_popin.scanning")
        } else if d.scan_status == "cancelled" {
            t("folders_popin.cancelled")
        } else if let Some(at) = d.last_scan_at {
            t("folders_popin.scanned").replace("{ago}", &intl::relative_time_short(elapsed_seconds(&at.to_rfc3339(), Utc::now()).unwrap_or(0)))
        } else {
            t("folders_popin.never_scanned")
        };
        let tone = if busy { 1 } else if d.last_scan_at.is_some() { 0 } else { 2 };
        (
            (d.total_files > 0).then(|| nb(d.total_files, "folders_popin.files_one", "folders_popin.files_n")),
            (d.total_size > 0).then(|| karbon_ui::intl::bytes(d.total_size as u64)),
            state,
            tone,
        )
    }
}
