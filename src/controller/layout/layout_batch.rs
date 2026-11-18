use crate::models::layout::*;
use crate::app::music_app::MusicApp;

impl MusicApp {
    pub(crate) fn batch_begin(&mut self, job_id: String, label: String) {
        self.views.batch = Some(BatchState { job_id, label, running: true, ..Default::default() });
    }

    /// Événements du moteur `batch-progress` et `batch-done`.
    pub(crate) fn batch_event(&mut self, name: &str, payload: &serde_json::Value) -> bool {
        let Some(b) = self.views.batch.as_mut() else { return false };
        if payload.get("job_id").and_then(|v| v.as_str()) != Some(b.job_id.as_str()) {
            return false;
        }
        let n = |k: &str| payload.get(k).and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        if name == "batch-progress" {
            b.done = n("done");
            b.total = n("total");
            b.failed = n("failed");
            b.current = payload.get("current").and_then(|v| v.as_str()).unwrap_or("").to_string();
        } else {
            let failures: Vec<(String, String)> = payload
                .get("failures")
                .and_then(|v| v.as_array())
                .map(|l| {
                    l.iter()
                        .map(|f| (f["path"].as_str().unwrap_or("").to_string(), f["error"].as_str().unwrap_or("").to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let succeeded = n("succeeded");
            let ignores = n("total").saturating_sub(succeeded + failures.len());
            b.running = false;
            b.report = Some((succeeded, failures, ignores));
        }
        true
    }

    pub(crate) fn batch_cancel(&mut self) -> bool {
        let Some(b) = self.views.batch.as_mut() else { return false };
        b.cancelling = true;
        let id = b.job_id.clone();
        let app = self.backend.app.clone();
        self.backend.fire(async move {
            let _ = crate::commands::tags::batch_command::cancel_batch(app.state(), id);
        });
        true
    }
}
