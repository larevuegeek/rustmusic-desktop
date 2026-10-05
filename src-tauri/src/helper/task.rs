//! Tâche de fond exclusive et annulable : une seule passe à la fois.

use std::sync::atomic::{AtomicBool, Ordering};

pub struct Task {
    running: AtomicBool,
    cancel_requested: AtomicBool,
}

impl Task {
    pub const fn new() -> Self {
        Self { running: AtomicBool::new(false), cancel_requested: AtomicBool::new(false) }
    }

    /// Réserve la tâche ; `None` si elle tourne déjà (deux passes se marchaient dessus).
    pub fn start(&'static self) -> Option<TaskToken> {
        if self.running.swap(true, Ordering::SeqCst) {
            return None;
        }
        self.cancel_requested.store(false, Ordering::SeqCst);
        Some(TaskToken(self))
    }

    /// Lisible sans le jeton, depuis un thread bloquant.
    pub fn is_cancel_requested(&self) -> bool {
        self.cancel_requested.load(Ordering::SeqCst)
    }

    /// Demande l'arrêt ; vrai si une passe était en cours.
    pub fn cancel(&self) -> bool {
        let was_running = self.running.load(Ordering::SeqCst);
        if was_running {
            self.cancel_requested.store(true, Ordering::SeqCst);
        }
        was_running
    }
}

/// Tant qu'il vit, la tâche est occupée.
pub struct TaskToken(&'static Task);

impl TaskToken {
    pub fn is_cancelled(&self) -> bool {
        self.0.cancel_requested.load(Ordering::SeqCst)
    }

    /// Pause entre deux requêtes, interrompue par une annulation.
    pub async fn sleep(&self, ms: u64) {
        let mut remaining = ms;
        while remaining > 0 && !self.is_cancelled() {
            let step = remaining.min(100);
            tokio::time::sleep(std::time::Duration::from_millis(step)).await;
            remaining -= step;
        }
    }
}

impl Drop for TaskToken {
    fn drop(&mut self) {
        self.0.running.store(false, Ordering::SeqCst);
    }
}
