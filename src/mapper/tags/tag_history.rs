use crate::models::tags::*;
use crate::helper::date::parse_date;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;

impl HistoryState {
    /// Le bilan de la dernière restauration.
    pub fn result_view(&self) -> Option<RestoreReport> {
        let (restored, failed, error) = self.result.as_ref()?;
        Some(RestoreReport {
            has_failures: *failed > 0,
            summary: if *failed > 0 {
                format!("{restored} {} · {failed} {}", t("history.restored"), t("rename.failed"))
            } else {
                format!("{restored} {}", t("history.restored"))
            },
            detail: error.clone().unwrap_or_default(),
        })
    }
}

impl Entry {
    /// Infobulle du motif de renommage, vide sans motif.
    pub fn pattern_title(&self) -> &str {
        self.pattern.as_deref().unwrap_or("")
    }

    /// Le motif de renommage, ou « — ».
    pub fn pattern_text(&self) -> &str {
        self.pattern.as_deref().unwrap_or("—")
    }

    /// `when` : la date locale.
    pub fn when(&self) -> String {
        parse_date(&self.created_at)
            .map(|d| d.with_timezone(&chrono::Local).format("%d/%m/%Y %H:%M:%S").to_string())
            .unwrap_or_else(|| self.created_at.clone())
    }

    pub fn icon(&self) -> &'static str {
        if self.kind == "restructure" {
            "lucide:folder-tree"
        } else {
            "lucide:file-pen-line"
        }
    }

    pub fn kind_label(&self) -> String {
        t(&format!("history.kind.{}", self.kind))
    }
}

impl Popin {
    pub fn history(&self) -> Option<&HistoryState> {
        match &self.body {
            PopinBody::History(h) => Some(h),
            _ => None,
        }
    }
}
