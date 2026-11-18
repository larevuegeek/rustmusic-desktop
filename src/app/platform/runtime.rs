//! Le runtime tokio de l'application, accessible de partout : remplace
//! `tauri::async_runtime`. Le runtime est créé par `main`, puis sa poignée est posée
//! ici une fois pour toutes ; la boucle de fenêtres tourne sur le thread principal,
//! hors du runtime.

use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::Handle;

pub use tokio::task::JoinHandle;

static HANDLE: OnceLock<Handle> = OnceLock::new();

/// Pose la poignée du runtime ; sans effet si elle l'est déjà.
pub fn install(handle: Handle) {
    let _ = HANDLE.set(handle);
}

fn handle() -> Handle {
    match HANDLE.get() {
        Some(handle) => handle.clone(),
        // Tests unitaires lancés dans leur propre runtime.
        None => Handle::current(),
    }
}

/// Lance une tâche asynchrone sur le runtime.
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    handle().spawn(future)
}

/// Lance un travail bloquant sur le pool prévu pour cela.
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    handle().spawn_blocking(f)
}

/// Attend un futur depuis un thread qui n'est pas un worker du runtime (thread UI,
/// thread audio). Depuis un worker, `block_in_place` évite de bloquer l'ordonnanceur.
pub fn block_on<F: Future>(future: F) -> F::Output {
    let handle = handle();
    match Handle::try_current() {
        Ok(_) => tokio::task::block_in_place(|| handle.block_on(future)),
        Err(_) => handle.block_on(future),
    }
}
