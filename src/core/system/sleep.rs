//! Veille du système : pause juste avant la mise en veille (Linux), et option qui
//! garde la machine éveillée pendant la lecture — l'écran, lui, peut s'éteindre.

use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

use crate::app::AppHandle;

static ORDERS: OnceLock<Mutex<Sender<bool>>> = OnceLock::new();

/// Lance le fil qui tient le blocage de la veille, et l'écoute de la mise en veille.
pub fn start(app: AppHandle) {
    let (tx, rx) = channel::<bool>();
    let _ = ORDERS.set(Mutex::new(tx));
    // Un seul fil pour tout : sous Windows, l'état demandé est attaché au fil appelant.
    let _ = std::thread::Builder::new().name("veille".into()).spawn(move || {
        let mut lock = platform::Lock::default();
        for block_on in rx {
            lock.set(block_on);
        }
    });

    #[cfg(target_os = "linux")]
    linux::watch_sleep(app);
    #[cfg(not(target_os = "linux"))]
    let _ = app;
}

/// Empêche (ou rend possible) la mise en veille automatique de la machine.
pub fn set_blocked(block_on: bool) {
    if let Some(tx) = ORDERS.get() {
        if let Ok(tx) = tx.lock() {
            let _ = tx.send(block_on);
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED};

    #[derive(Default)]
    pub struct Lock;

    impl Lock {
        /// Sans ES_DISPLAY_REQUIRED : l'écran garde le droit de s'éteindre.
        pub fn set(&mut self, block_on: bool) {
            let state = if block_on { ES_CONTINUOUS | ES_SYSTEM_REQUIRED } else { ES_CONTINUOUS };
            unsafe {
                SetThreadExecutionState(state);
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use zbus::blocking::Connection;
    use zbus::zvariant::{OwnedFd, OwnedObjectPath};

    /// Blocage en cours : descripteur rendu par logind, ou demande au portail en
    /// Flatpak (le bac à sable n'a pas accès à logind). Le lâcher lève le blocage.
    #[derive(Default)]
    pub enum Lock {
        #[default]
        Free,
        // Jamais lu : seule sa fermeture compte, elle lève le blocage.
        Logind(#[allow(dead_code)] OwnedFd),
        Portal(Connection, OwnedObjectPath),
    }

    impl Lock {
        pub fn set(&mut self, block_on: bool) {
            if !block_on {
                if let Lock::Portal(conn, request) = std::mem::take(self) {
                    super::linux::portal_release(&conn, &request);
                }
                return;
            }
            if !matches!(self, Lock::Free) {
                return;
            }
            let lock = if crate::core::system::sandbox::flatpak_id().is_some() {
                super::linux::portal_inhibit("Lecture en cours").map(|(c, d)| Lock::Portal(c, d))
            } else {
                super::linux::inhibit("block", "Lecture en cours").map(Lock::Logind)
            };
            match lock {
                Ok(v) => *self = v,
                Err(e) => log::warn!("[veille] blocage refusé : {e}"),
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use objc2_core_foundation::CFString;

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        fn IOPMAssertionCreateWithName(kind: &CFString, level: u32, name: &CFString, id: *mut u32) -> i32;
        fn IOPMAssertionRelease(id: u32) -> i32;
    }

    /// Identifiant de l'assertion IOKit en cours ; « PreventUserIdleSystemSleep » laisse l'écran s'éteindre.
    #[derive(Default)]
    pub struct Lock(Option<u32>);

    impl Lock {
        pub fn set(&mut self, block_on: bool) {
            match (block_on, self.0) {
                (true, None) => {
                    let kind = CFString::from_static_str("PreventUserIdleSystemSleep");
                    let name = CFString::from_static_str("RustMusic : lecture en cours");
                    let mut id = 0u32;
                    // 255 = kIOPMAssertionLevelOn, 0 = kIOReturnSuccess.
                    if unsafe { IOPMAssertionCreateWithName(&kind, 255, &name, &mut id) } == 0 {
                        self.0 = Some(id);
                    }
                }
                (false, Some(id)) => {
                    unsafe {
                        IOPMAssertionRelease(id);
                    }
                    self.0 = None;
                }
                _ => {}
            }
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod platform {
    #[derive(Default)]
    pub struct Lock;

    impl Lock {
        pub fn set(&mut self, _block: bool) {}
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::collections::HashMap;
    use std::time::Duration;

    use crate::app::AppHandle;
    use zbus::blocking::{Connection, Proxy};
    use zbus::zvariant::{OwnedFd, OwnedObjectPath, Value};

    fn logind(conn: &Connection) -> zbus::Result<Proxy<'_>> {
        Proxy::new(conn, "org.freedesktop.login1", "/org/freedesktop/login1", "org.freedesktop.login1.Manager")
    }

    /// Inhibiteur de veille logind (`block` ou `delay`), levé quand le descripteur se ferme.
    pub fn inhibit(mode: &str, why: &str) -> zbus::Result<OwnedFd> {
        let conn = Connection::system()?;
        let fd: OwnedFd = logind(&conn)?.call("Inhibit", &("sleep", "RustMusic", why, mode))?;
        Ok(fd)
    }

    /// Blocage de la veille demandé au portail `Inhibit` (Flatpak) : levé par
    /// `portal_release`, ou quand la connexion se ferme.
    pub fn portal_inhibit(why: &str) -> zbus::Result<(Connection, OwnedObjectPath)> {
        let conn = Connection::session()?;
        let mut options: HashMap<&str, Value> = HashMap::new();
        options.insert("reason", Value::from(why));
        // 4 = mise en veille seulement : l'écran garde le droit de s'éteindre.
        let response = conn.call_method(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.portal.Inhibit"),
            "Inhibit",
            &("", 4u32, options),
        )?;
        let request: OwnedObjectPath = response.body().deserialize()?;
        Ok((conn, request))
    }

    pub fn portal_release(conn: &Connection, request: &OwnedObjectPath) {
        if let Err(e) = conn.call_method(
            Some("org.freedesktop.portal.Desktop"),
            request.as_str(),
            Some("org.freedesktop.portal.Request"),
            "Close",
            &(),
        ) {
            log::warn!("[veille] fin du blocage par le portail : {e}");
        }
    }

    /// Prévient l'interface avant la veille (elle met en pause) et au réveil.
    pub fn watch_sleep(app: AppHandle) {
        // Aucun portail n'annonce la mise en veille, et Flathub refuse l'accès à logind.
        if crate::core::system::sandbox::flatpak_id().is_some() {
            log::info!("[veille] Flatpak : pas de pause automatique avant la veille");
            return;
        }
        let _ = std::thread::Builder::new().name("veille-logind".into()).spawn(move || {
            if let Err(e) = run_loop(&app) {
                log::warn!("[veille] écoute de logind impossible : {e}");
            }
        });
    }

    fn run_loop(app: &AppHandle) -> zbus::Result<()> {
        let conn = Connection::system()?;
        let proxy = logind(&conn)?;
        // « delay » : logind attend qu'on rende ce descripteur avant d'endormir la machine.
        let mut delay = inhibit("delay", "Mise en pause avant la veille").ok();
        for signal in proxy.receive_signal("PrepareForSleep")? {
            let (between,): (bool,) = signal.body().deserialize()?;
            let _ = app.emit("systeme-veille", between);
            if between {
                // Le temps que la pause atteigne la sortie audio.
                std::thread::sleep(Duration::from_millis(1500));
                delay = None;
            } else if delay.is_none() {
                delay = inhibit("delay", "Mise en pause avant la veille").ok();
            }
        }
        Ok(())
    }
}
