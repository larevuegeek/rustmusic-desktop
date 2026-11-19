//! Lancement à l'ouverture de la session (Réglages → Général → « Lancer au démarrage du
//! système »). Mêmes emplacements et même nom que la 0.4 (module de lancement automatique
//! de Tauri, crate `auto-launch`) : une entrée de la 0.4 est reprise, jamais doublée.
//!
//! - Windows : valeur `RustMusic` de `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` ;
//! - macOS : `~/Library/LaunchAgents/RustMusic.plist` ;
//! - Linux : `~/.config/autostart/RustMusic.desktop`, ou le portail Background sous Flatpak.

/// Nom de l'entrée, comme en 0.4.
pub const NAME: &str = "RustMusic";

/// Clé du réglage dans la table `settings`.
pub const KEY: &str = "auto_start";

/// Programme à lancer : l'AppImage elle-même (pas son point de montage temporaire), sinon
/// l'exécutable courant.
fn program() -> Result<std::path::PathBuf, String> {
    if let Some(appimage) = std::env::var_os("APPIMAGE").filter(|p| !p.is_empty()) {
        return Ok(appimage.into());
    }
    std::env::current_exe().map_err(|e| format!("exécutable courant : {e}"))
}

/// Active ou retire le lancement au démarrage. Activé, l'entrée est réécrite : elle suit
/// l'appli si elle a été déplacée ou mise à jour.
pub fn set(enabled: bool) -> Result<(), String> {
    if crate::core::system::sandbox::flatpak_id().is_some() {
        return crate::core::system::sandbox::request_autostart(enabled);
    }
    platform::set(enabled, &program()?)
}

#[cfg(target_os = "windows")]
mod platform {
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
        RegCreateKeyExW, RegDeleteValueW, RegSetValueExW,
    };
    use windows::core::{HSTRING, PCWSTR};

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    /// État « désactivé » posé par le Gestionnaire des tâches : retiré à l'activation.
    const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

    fn open(path: &str) -> Result<HKEY, String> {
        let mut key = HKEY::default();
        let path = HSTRING::from(path);
        // SAFETY: arguments valides le temps de l'appel ; la clé est refermée par l'appelant.
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut key,
                None,
            )
        };
        status.ok().map_err(|e| format!("registre {path} : {e}"))?;
        Ok(key)
    }

    pub(super) fn set(enabled: bool, program: &std::path::Path) -> Result<(), String> {
        let name = HSTRING::from(super::NAME);
        let key = open(RUN)?;
        let result = if enabled {
            let value = format!("\"{}\"", program.display());
            let wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
            // SAFETY: `wide` vit jusqu'à la fin de l'appel.
            let bytes = unsafe { std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), wide.len() * 2) };
            // SAFETY: clé ouverte en écriture, nom et données valides.
            unsafe { RegSetValueExW(key, PCWSTR(name.as_ptr()), None, REG_SZ, Some(bytes)) }
                .ok()
                .map_err(|e| format!("registre Run : {e}"))
        } else {
            // SAFETY: clé ouverte en écriture ; une valeur absente n'est pas une erreur.
            let _ = unsafe { RegDeleteValueW(key, PCWSTR(name.as_ptr())) };
            Ok(())
        };
        // SAFETY: clé ouverte ci-dessus.
        let _ = unsafe { RegCloseKey(key) };
        if enabled {
            if let Ok(approved) = open(APPROVED) {
                // SAFETY: clé ouverte en écriture ; une valeur absente n'est pas une erreur.
                let _ = unsafe { RegDeleteValueW(approved, PCWSTR(name.as_ptr())) };
                // SAFETY: clé ouverte ci-dessus.
                let _ = unsafe { RegCloseKey(approved) };
            }
        }
        result
    }
}

#[cfg(target_os = "macos")]
mod platform {
    fn plist() -> Result<std::path::PathBuf, String> {
        let home = dirs::home_dir().ok_or("dossier personnel introuvable")?;
        Ok(home.join("Library/LaunchAgents").join(format!("{}.plist", super::NAME)))
    }

    /// Échappement XML d'un chemin.
    fn xml(text: &str) -> String {
        text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
    }

    pub(super) fn set(enabled: bool, program: &std::path::Path) -> Result<(), String> {
        let path = plist()?;
        if !enabled {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("{} : {e}", path.display())),
                _ => Ok(()),
            };
        }
        let content = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key>\n  <string>{name}</string>\n  <key>ProgramArguments</key>\n  <array>\n    <string>{program}</string>\n  </array>\n  <key>RunAtLoad</key>\n  <true/>\n</dict>\n</plist>\n",
            name = super::NAME,
            program = xml(&program.display().to_string()),
        );
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
        }
        std::fs::write(&path, content).map_err(|e| format!("{} : {e}", path.display()))
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod platform {
    fn desktop_file() -> Result<std::path::PathBuf, String> {
        let config = dirs::config_dir().ok_or("dossier de configuration introuvable")?;
        Ok(config.join("autostart").join(format!("{}.desktop", super::NAME)))
    }

    pub(super) fn set(enabled: bool, program: &std::path::Path) -> Result<(), String> {
        let path = desktop_file()?;
        if !enabled {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("{} : {e}", path.display())),
                _ => Ok(()),
            };
        }
        // Chemin entre guillemets (espaces) ; `"`, `\`, `` ` `` et `$` échappés (spécification
        // Desktop Entry).
        let quoted = program
            .display()
            .to_string()
            .chars()
            .flat_map(|c| match c {
                '"' | '\\' | '`' | '$' => vec!['\\', c],
                c => vec![c],
            })
            .collect::<String>();
        let content = format!(
            "[Desktop Entry]\nType=Application\nVersion=1.0\nName={name}\nComment={name} startup script\nExec=\"{quoted}\"\nStartupNotify=false\nTerminal=false\n",
            name = super::NAME,
        );
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
        }
        std::fs::write(&path, content).map_err(|e| format!("{} : {e}", path.display()))
    }
}
