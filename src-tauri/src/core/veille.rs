//! Veille du système : pause juste avant la mise en veille (Linux), et option qui
//! garde la machine éveillée pendant la lecture — l'écran, lui, peut s'éteindre.

use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

use tauri::AppHandle;

static ORDRES: OnceLock<Mutex<Sender<bool>>> = OnceLock::new();

/// Lance le fil qui tient le blocage de la veille, et l'écoute de la mise en veille.
pub fn demarrer(app: AppHandle) {
    let (tx, rx) = channel::<bool>();
    let _ = ORDRES.set(Mutex::new(tx));
    // Un seul fil pour tout : sous Windows, l'état demandé est attaché au fil appelant.
    let _ = std::thread::Builder::new().name("veille".into()).spawn(move || {
        let mut verrou = plateforme::Verrou::default();
        for bloquer in rx {
            verrou.regler(bloquer);
        }
    });

    #[cfg(target_os = "linux")]
    linux::ecouter_mise_en_veille(app);
    #[cfg(not(target_os = "linux"))]
    let _ = app;
}

/// Empêche (ou rend possible) la mise en veille automatique de la machine.
pub fn bloquer(bloquer: bool) {
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
        pub fn regler(&mut self, bloquer: bool) {
            let etat = if bloquer { ES_CONTINUOUS | ES_SYSTEM_REQUIRED } else { ES_CONTINUOUS };
            unsafe {
                SetThreadExecutionState(etat);
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod plateforme {
    use zbus::zvariant::OwnedFd;

    /// Tant que le descripteur rendu par logind est ouvert, la veille est bloquée.
    #[derive(Default)]
    pub struct Verrou(Option<OwnedFd>);

    impl Verrou {
        pub fn regler(&mut self, bloquer: bool) {
            if !bloquer {
                self.0 = None;
            } else if self.0.is_none() {
                match super::linux::inhiber("block", "Lecture en cours") {
                    Ok(fd) => self.0 = Some(fd),
                    Err(e) => log::warn!("[veille] blocage refusé par logind : {e}"),
                }
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
        pub fn regler(&mut self, bloquer: bool) {
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
        pub fn regler(&mut self, _bloquer: bool) {}
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::time::Duration;

    use tauri::{AppHandle, Emitter};
    use zbus::blocking::{Connection, Proxy};
    use zbus::zvariant::OwnedFd;

    fn logind(conn: &Connection) -> zbus::Result<Proxy<'_>> {
        Proxy::new(conn, "org.freedesktop.login1", "/org/freedesktop/login1", "org.freedesktop.login1.Manager")
    }

    /// Inhibiteur de veille logind (`block` ou `delay`), levé quand le descripteur se ferme.
    pub fn inhiber(mode: &str, pourquoi: &str) -> zbus::Result<OwnedFd> {
        let conn = Connection::system()?;
        let fd: OwnedFd = logind(&conn)?.call("Inhibit", &("sleep", "RustMusic", pourquoi, mode))?;
        Ok(fd)
    }

    /// Prévient l'interface avant la veille (elle met en pause) et au réveil.
    pub fn ecouter_mise_en_veille(app: AppHandle) {
        let _ = std::thread::Builder::new().name("veille-logind".into()).spawn(move || {
            if let Err(e) = boucle(&app) {
                log::warn!("[veille] écoute de logind impossible : {e}");
            }
        });
    }

    fn boucle(app: &AppHandle) -> zbus::Result<()> {
        let conn = Connection::system()?;
        let proxy = logind(&conn)?;
        // « delay » : logind attend qu'on rende ce descripteur avant d'endormir la machine.
        let mut retard = inhiber("delay", "Mise en pause avant la veille").ok();
        for signal in proxy.receive_signal("PrepareForSleep")? {
            let (entre,): (bool,) = signal.body().deserialize()?;
            let _ = app.emit("systeme-veille", entre);
            if entre {
                // Le temps que la pause atteigne la sortie audio.
                std::thread::sleep(Duration::from_millis(1500));
                retard = None;
            } else if retard.is_none() {
                retard = inhiber("delay", "Mise en pause avant la veille").ok();
            }
        }
        Ok(())
    }
}
