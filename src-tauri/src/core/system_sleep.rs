//! Veille du système : pause juste avant la mise en veille (Linux), et option qui
//! garde la machine éveillée pendant la lecture — l'écran, lui, peut s'éteindre.

use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

use tauri::AppHandle;

static ORDRES: OnceLock<Mutex<Sender<bool>>> = OnceLock::new();

/// Lance le fil qui tient le blocage de la veille, et l'écoute de la mise en veille.
pub fn start(app: AppHandle) {
    let (tx, rx) = channel::<bool>();
    let _ = ORDRES.set(Mutex::new(tx));
    // Un seul fil pour tout : sous Windows, l'état demandé est attaché au fil appelant.
    let _ = std::thread::Builder::new().name("veille".into()).spawn(move || {
        let mut verrou = plateforme::Verrou::default();
        for bloquer in rx {
            verrou.set(bloquer);
        }
    });

    #[cfg(target_os = "linux")]
    linux::watch_sleep(app);
    #[cfg(not(target_os = "linux"))]
    let _ = app;
}

/// Empêche (ou rend possible) la mise en veille automatique de la machine.
pub fn set_blocked(bloquer: bool) {
    if let Some(tx) = ORDRES.get() {
        if let Ok(tx) = tx.lock() {
            let _ = tx.send(bloquer);
        }
    }
}

#[cfg(target_os = "windows")]
mod plateforme {
    use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED};

    #[derive(Default)]
    pub struct Verrou;

    impl Verrou {
        /// Sans ES_DISPLAY_REQUIRED : l'écran garde le droit de s'éteindre.
        pub fn set(&mut self, bloquer: bool) {
            let etat = if bloquer { ES_CONTINUOUS | ES_SYSTEM_REQUIRED } else { ES_CONTINUOUS };
            unsafe {
                SetThreadExecutionState(etat);
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod plateforme {
    use zbus::blocking::Connection;
    use zbus::zvariant::{OwnedFd, OwnedObjectPath};

    /// Blocage en cours : descripteur rendu par logind, ou demande au portail en
    /// Flatpak (le bac à sable n'a pas accès à logind). Le lâcher lève le blocage.
    #[derive(Default)]
    pub enum Verrou {
        #[default]
        Libre,
        // Jamais lu : seule sa fermeture compte, elle lève le blocage.
        Logind(#[allow(dead_code)] OwnedFd),
        Portail(Connection, OwnedObjectPath),
    }

    impl Verrou {
        pub fn set(&mut self, bloquer: bool) {
            if !bloquer {
                if let Verrou::Portail(conn, demande) = std::mem::take(self) {
                    super::linux::portal_release(&conn, &demande);
                }
                return;
            }
            if !matches!(self, Verrou::Libre) {
                return;
            }
            let verrou = if crate::core::sandbox::flatpak_id().is_some() {
                super::linux::portal_inhibit("Lecture en cours").map(|(c, d)| Verrou::Portail(c, d))
            } else {
                super::linux::inhibit("block", "Lecture en cours").map(Verrou::Logind)
            };
            match verrou {
                Ok(v) => *self = v,
                Err(e) => log::warn!("[veille] blocage refusé : {e}"),
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod plateforme {
    use objc2_core_foundation::CFString;

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        fn IOPMAssertionCreateWithName(kind: &CFString, niveau: u32, nom: &CFString, id: *mut u32) -> i32;
        fn IOPMAssertionRelease(id: u32) -> i32;
    }

    /// Identifiant de l'assertion IOKit en cours ; « PreventUserIdleSystemSleep » laisse l'écran s'éteindre.
    #[derive(Default)]
    pub struct Verrou(Option<u32>);

    impl Verrou {
        pub fn set(&mut self, bloquer: bool) {
            match (bloquer, self.0) {
                (true, None) => {
                    let kind = CFString::from_static_str("PreventUserIdleSystemSleep");
                    let nom = CFString::from_static_str("RustMusic : lecture en cours");
                    let mut id = 0u32;
                    // 255 = kIOPMAssertionLevelOn, 0 = kIOReturnSuccess.
                    if unsafe { IOPMAssertionCreateWithName(&kind, 255, &nom, &mut id) } == 0 {
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
mod plateforme {
    #[derive(Default)]
    pub struct Verrou;

    impl Verrou {
        pub fn set(&mut self, _bloquer: bool) {}
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::collections::HashMap;
    use std::time::Duration;

    use tauri::{AppHandle, Emitter};
    use zbus::blocking::{Connection, Proxy};
    use zbus::zvariant::{OwnedFd, OwnedObjectPath, Value};

    fn logind(conn: &Connection) -> zbus::Result<Proxy<'_>> {
        Proxy::new(conn, "org.freedesktop.login1", "/org/freedesktop/login1", "org.freedesktop.login1.Manager")
    }

    /// Inhibiteur de veille logind (`block` ou `delay`), levé quand le descripteur se ferme.
    pub fn inhibit(mode: &str, pourquoi: &str) -> zbus::Result<OwnedFd> {
        let conn = Connection::system()?;
        let fd: OwnedFd = logind(&conn)?.call("Inhibit", &("sleep", "RustMusic", pourquoi, mode))?;
        Ok(fd)
    }

    /// Blocage de la veille demandé au portail `Inhibit` (Flatpak) : levé par
    /// `portal_release`, ou quand la connexion se ferme.
    pub fn portal_inhibit(pourquoi: &str) -> zbus::Result<(Connection, OwnedObjectPath)> {
        let conn = Connection::session()?;
        let mut options: HashMap<&str, Value> = HashMap::new();
        options.insert("reason", Value::from(pourquoi));
        // 4 = mise en veille seulement : l'écran garde le droit de s'éteindre.
        let reponse = conn.call_method(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.portal.Inhibit"),
            "Inhibit",
            &("", 4u32, options),
        )?;
        let demande: OwnedObjectPath = reponse.body().deserialize()?;
        Ok((conn, demande))
    }

    pub fn portal_release(conn: &Connection, demande: &OwnedObjectPath) {
        if let Err(e) = conn.call_method(
            Some("org.freedesktop.portal.Desktop"),
            demande.as_str(),
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
        if crate::core::sandbox::flatpak_id().is_some() {
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
        let mut retard = inhibit("delay", "Mise en pause avant la veille").ok();
        for signal in proxy.receive_signal("PrepareForSleep")? {
            let (entre,): (bool,) = signal.body().deserialize()?;
            let _ = app.emit("systeme-veille", entre);
            if entre {
                // Le temps que la pause atteigne la sortie audio.
                std::thread::sleep(Duration::from_millis(1500));
                retard = None;
            } else if retard.is_none() {
                retard = inhibit("delay", "Mise en pause avant la veille").ok();
            }
        }
        Ok(())
    }
}
