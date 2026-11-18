use crate::models::layout::*;
use karbon_ui::intl;
use karbon_ui::intl::t;

/// « 3 échecs » / « 1 échec ».
pub fn failures_label(n: usize) -> String {
    format!("{} {}", intl::number(n as i64), t(if n > 1 { "batch.failures" } else { "batch.failure" }))
}

impl crate::models::layout::BatchState {
    /// Le dernier lot a laissé des échecs (icône d'alerte de l'en-tête).
    pub fn has_failures(&self) -> bool {
        self.report.as_ref().is_some_and(|r| !r.1.is_empty())
    }

    /// Le bilan du dernier lot, mis en forme.
    pub fn report_view(&self) -> Option<crate::models::layout::BatchReport> {
        let (succeeded, failures, skipped) = self.report.as_ref()?;
        Some(crate::models::layout::BatchReport {
            succeeded: format!("{} {}", intl::number(*succeeded as i64), t("batch.succeeded")),
            failed: (!failures.is_empty()).then(|| failures_label(failures.len())),
            skipped: (*skipped > 0).then(|| format!("{} {}", intl::number(*skipped as i64), t("batch.skipped"))),
            failures: failures
                .iter()
                .map(|(path, message)| crate::models::layout::BatchFailure {
                    path: path.clone(),
                    file: file_name(path),
                    message: message.clone(),
                })
                .collect(),
        })
    }
}

pub fn file_name(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
}

impl BatchState {
    pub fn percent(&self) -> f32 {
        if self.total == 0 {
            0.
        } else {
            self.done as f32 * 100. / self.total as f32
        }
    }

    pub fn counter(&self) -> String {
        format!("{} / {}", intl::number(self.done as i64), intl::number(self.total as i64))
    }
}
