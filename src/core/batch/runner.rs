//! Moteur de traitement par lot.
//!
//! Brique commune à toutes les opérations de masse — écriture de tags,
//! renommage, déplacement. Elle ne sait rien de ce qu'elle exécute : on lui
//! donne une liste de chemins et une opération, elle rend compte.
//!
//! # Trois garanties, et pourquoi
//!
//! **Un échec n'interrompt jamais le lot.** Sur cinq cents fichiers, il y en
//! aura toujours un verrouillé, en lecture seule, ou dans un format qu'on ne
//! sait pas réécrire. S'arrêter au premier annulerait les quatre cent
//! quatre-vingt-dix-neuf autres pour rien. Chaque échec est collecté avec sa
//! raison, et le lot continue.
//!
//! **L'annulation est constatée entre deux fichiers, jamais pendant.** Couper
//! une écriture atomique en cours laisserait un fichier temporaire orphelin à
//! côté du morceau — précisément ce que `atomic_write` existe pour éviter.
//!
//! **Le compte rendu est produit dans tous les cas**, y compris après une
//! annulation. Un lot interrompu doit dire ce qu'il a déjà fait : sans ça,
//! l'utilisateur ne sait pas dans quel état est sa bibliothèque.
//!
//! # Pourquoi un rapporteur plutôt qu'un `AppHandle`
//! Le moteur reçoit une fonction de progression au lieu d'émettre lui-même les
//! événements Tauri. C'est ce qui le rend testable sans application : les tests
//! ci-dessous collectent la progression dans un vecteur.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;

/// Nombre de fichiers traités de front.
///
/// Trois et non seize : l'essentiel du temps part en va-et-vient réseau, que
/// le recouvrement masque bien, mais saturer un partage le rend plus lent, pas
/// plus rapide. Valeur de départ, à mesurer.
pub const DEFAULT_CONCURRENCY: usize = 3;

/// Un fichier qui n'a pas pu être traité, et pourquoi.
#[derive(Debug, Clone, Serialize)]
pub struct BatchFailure {
    pub path: String,
    pub error: String,
}

/// Émis après chaque fichier.
#[derive(Debug, Clone, Serialize)]
pub struct BatchProgress {
    pub job_id: String,
    pub done: usize,
    pub total: usize,
    pub failed: usize,
    /// Le fichier qui vient d'être traité.
    pub current: String,
}

/// Émis une fois, à la fin, quelle qu'en soit la cause.
#[derive(Debug, Clone, Serialize)]
pub struct BatchReport {
    pub job_id: String,
    pub total: usize,
    pub succeeded: usize,
    /// Seuls les échecs sont listés : c'est sur eux qu'on agit ensuite.
    pub failures: Vec<BatchFailure>,
    pub cancelled: bool,
}

impl BatchReport {
    /// Fichiers jamais atteints — non nul seulement après une annulation.
    pub fn skipped(&self) -> usize {
        self.total - self.succeeded - self.failures.len()
    }
}

/// Exécute `operation` sur chaque chemin.
///
/// `on_progress` est appelée après chaque fichier, dans l'ordre de la liste.
pub async fn run<F, Fut, R>(
    job_id: String,
    cancel: Arc<AtomicBool>,
    paths: Vec<String>,
    concurrency: usize,
    operation: F,
    on_progress: R,
) -> BatchReport
where
    F: Fn(String) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = Result<(), String>> + Send + 'static,
    R: Fn(BatchProgress),
{
    let total = paths.len();
    let concurrency = concurrency.max(1);

    let mut succeeded = 0usize;
    let mut failures: Vec<BatchFailure> = Vec::new();
    let mut cancelled = false;
    let mut index = 0usize;

    while index < total {
        // Seul endroit où l'annulation est lue : entre deux paquets, donc
        // jamais au milieu d'une écriture.
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }

        let end = (index + concurrency).min(total);
        let mut handles = Vec::with_capacity(end - index);
        for path in &paths[index..end] {
            let operation = operation.clone();
            let path = path.clone();
            handles.push(tokio::spawn(async move { operation(path).await }));
        }

        for (offset, handle) in handles.into_iter().enumerate() {
            let path = paths[index + offset].clone();

            let result = match handle.await {
                Ok(result) => result,
                // Une tâche qui panique ne doit pas emporter le lot : elle
                // devient un échec comme un autre.
                Err(e) => Err(format!("tâche interrompue : {e}")),
            };

            match result {
                Ok(()) => succeeded += 1,
                Err(error) => failures.push(BatchFailure {
                    path: path.clone(),
                    error,
                }),
            }

            on_progress(BatchProgress {
                job_id: job_id.clone(),
                done: index + offset + 1,
                total,
                failed: failures.len(),
                current: path,
            });
        }

        index = end;
    }

    BatchReport {
        job_id,
        total,
        succeeded,
        failures,
        cancelled,
    }
}

/// Les lots en cours, pour pouvoir les annuler depuis une autre commande.
///
/// Vit dans l'état de l'application : la commande d'annulation arrive par un
/// appel séparé de celle qui a lancé le lot, il faut donc un point de rendez-vous.
#[derive(Default)]
pub struct BatchRegistry {
    running: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl BatchRegistry {
    /// Enregistre un lot et rend son drapeau d'annulation.
    pub fn start(&self, job_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.lock().insert(job_id.to_string(), flag.clone());
        flag
    }

    /// Demande l'annulation. Faux si le lot est inconnu ou déjà terminé.
    pub fn cancel(&self, job_id: &str) -> bool {
        match self.lock().get(job_id) {
            Some(flag) => {
                flag.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    pub fn finish(&self, job_id: &str) {
        self.lock().remove(job_id);
    }

    /// Un verrou empoisonné signifie qu'une tâche a paniqué en le tenant. La
    /// table reste exploitable — on récupère plutôt que de propager la panique
    /// à tous les lots suivants.
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<AtomicBool>>> {
        self.running.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    fn paths(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("/musique/piste-{i}.flac")).collect()
    }

    /// Collecte la progression sans avoir besoin d'une application Tauri.
    fn collector() -> (Arc<Mutex<Vec<BatchProgress>>>, impl Fn(BatchProgress)) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        (seen, move |p: BatchProgress| sink.lock().unwrap().push(p))
    }

    #[tokio::test]
    async fn a_failure_does_not_stop_the_batch() {
        // La garantie centrale : un fichier verrouillé au milieu de cinq cents
        // ne doit pas annuler les autres.
        let (_, on_progress) = collector();
        let report = run(
            "job".into(),
            Arc::new(AtomicBool::new(false)),
            paths(5),
            1,
            |path: String| async move {
                if path.ends_with("piste-2.flac") {
                    Err("fichier verrouillé".into())
                } else {
                    Ok(())
                }
            },
            on_progress,
        )
        .await;

        assert_eq!(report.total, 5);
        assert_eq!(report.succeeded, 4);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].error, "fichier verrouillé");
        assert!(report.failures[0].path.ends_with("piste-2.flac"));
        assert!(!report.cancelled);
        assert_eq!(report.skipped(), 0);
    }

    #[tokio::test]
    async fn every_file_reports_its_progress_in_order() {
        let (seen, on_progress) = collector();
        run(
            "job".into(),
            Arc::new(AtomicBool::new(false)),
            paths(4),
            2,
            |_: String| async { Ok(()) },
            on_progress,
        )
        .await;

        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 4);
        assert_eq!(
            seen.iter().map(|p| p.done).collect::<Vec<_>>(),
            vec![1, 2, 3, 4],
            "la progression doit suivre l'ordre de la liste"
        );
        assert!(seen.iter().all(|p| p.total == 4));
    }

    #[tokio::test]
    async fn cancelling_stops_between_files_and_still_reports() {
        // Le drapeau est levé pendant le traitement du premier fichier : le
        // paquet en cours va à son terme, le suivant ne démarre pas.
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let processed = Arc::new(AtomicUsize::new(0));
        let counter = processed.clone();

        let (_, on_progress) = collector();
        let report = run(
            "job".into(),
            cancel,
            paths(10),
            1,
            move |_: String| {
                let flag = flag.clone();
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::Relaxed);
                    flag.store(true, Ordering::Relaxed);
                    Ok(())
                }
            },
            on_progress,
        )
        .await;

        assert!(report.cancelled);
        assert_eq!(processed.load(Ordering::Relaxed), 1, "un fichier de trop a été traité");
        assert_eq!(report.succeeded, 1);
        assert_eq!(report.skipped(), 9, "le compte rendu doit dire ce qui reste");
    }

    #[tokio::test]
    async fn a_panicking_task_becomes_a_failure() {
        // Sans ça, un défaut dans une opération emporterait tout le lot.
        let (_, on_progress) = collector();
        let report = run(
            "job".into(),
            Arc::new(AtomicBool::new(false)),
            paths(3),
            1,
            |path: String| async move {
                if path.ends_with("piste-1.flac") {
                    panic!("défaut simulé");
                }
                Ok(())
            },
            on_progress,
        )
        .await;

        assert_eq!(report.succeeded, 2);
        assert_eq!(report.failures.len(), 1);
        assert!(report.failures[0].error.contains("tâche interrompue"));
    }

    #[tokio::test]
    async fn an_empty_batch_is_harmless() {
        let (seen, on_progress) = collector();
        let report = run(
            "job".into(),
            Arc::new(AtomicBool::new(false)),
            Vec::new(),
            DEFAULT_CONCURRENCY,
            |_: String| async { Ok(()) },
            on_progress,
        )
        .await;

        assert_eq!(report.total, 0);
        assert!(!report.cancelled);
        assert!(seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_registry_only_cancels_running_jobs() {
        let registry = BatchRegistry::default();
        let flag = registry.start("job-1");

        assert!(registry.cancel("job-1"));
        assert!(flag.load(Ordering::Relaxed));

        assert!(!registry.cancel("inconnu"), "un lot inconnu ne doit pas être annulable");

        registry.finish("job-1");
        assert!(!registry.cancel("job-1"), "un lot terminé ne doit plus être annulable");
    }
}
