//! Tâche de fond exclusive et annulable : une seule passe à la fois.

use std::sync::atomic::{AtomicBool, Ordering};

pub struct Tache {
    en_cours: AtomicBool,
    annuler: AtomicBool,
}

impl Tache {
    pub const fn new() -> Self {
        Self { en_cours: AtomicBool::new(false), annuler: AtomicBool::new(false) }
    }

    /// Réserve la tâche ; `None` si elle tourne déjà (deux passes se marchaient dessus).
    pub fn demarrer(&'static self) -> Option<Jeton> {
        if self.en_cours.swap(true, Ordering::SeqCst) {
            return None;
        }
        self.annuler.store(false, Ordering::SeqCst);
        Some(Jeton(self))
    }

    /// Lisible sans le jeton, depuis un thread bloquant.
    pub fn annulation_demandee(&self) -> bool {
        self.annuler.load(Ordering::SeqCst)
    }

    /// Demande l'arrêt ; vrai si une passe était en cours.
    pub fn annuler(&self) -> bool {
        let actif = self.en_cours.load(Ordering::SeqCst);
        if actif {
            self.annuler.store(true, Ordering::SeqCst);
        }
        actif
    }
}

/// Tant qu'il vit, la tâche est occupée.
pub struct Jeton(&'static Tache);

impl Jeton {
    pub fn annule(&self) -> bool {
        self.0.annuler.load(Ordering::SeqCst)
    }

    /// Pause entre deux requêtes, interrompue par une annulation.
    pub async fn patienter(&self, ms: u64) {
        let mut reste = ms;
        while reste > 0 && !self.annule() {
            let pas = reste.min(100);
            tokio::time::sleep(std::time::Duration::from_millis(pas)).await;
            reste -= pas;
        }
    }
}

impl Drop for Jeton {
    fn drop(&mut self) {
        self.0.en_cours.store(false, Ordering::SeqCst);
    }
}
