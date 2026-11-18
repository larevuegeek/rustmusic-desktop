//! Le handle natif de la fenêtre principale, posé par la couche d'interface quand la
//! fenêtre existe : les contrôles média du système (SMTC sous Windows) en ont besoin.

use std::sync::atomic::{AtomicIsize, Ordering};

static MAIN: AtomicIsize = AtomicIsize::new(0);

/// Enregistre le handle natif (HWND sous Windows) de la fenêtre principale.
pub fn set_main(handle: isize) {
    MAIN.store(handle, Ordering::Release);
}

/// Le handle natif de la fenêtre principale, s'il est connu.
pub fn main() -> Option<isize> {
    match MAIN.load(Ordering::Acquire) {
        0 => None,
        handle => Some(handle),
    }
}
