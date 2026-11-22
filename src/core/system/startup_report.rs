//! En-tête du journal à chaque lancement : version, système, session graphique et
//! carte graphique. De quoi lire un rapport de plantage sans questionner le testeur.

/// Écrit l'en-tête de ce lancement dans le journal.
pub fn log_startup() {
    log::info!(
        "🚀 RustMusic {} — {} {} ({})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        os_description()
    );
    #[cfg(target_os = "linux")]
    linux::log_session();
}

/// Nom et version du système, autant qu'on puisse les lire.
fn os_description() -> String {
    #[cfg(target_os = "linux")]
    {
        let distribution = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| linux::os_release_field(&content, "PRETTY_NAME"))
            .unwrap_or_else(|| "distribution inconnue".into());
        let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease").unwrap_or_default();
        format!("{distribution}, noyau {}", kernel.trim())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map(|o| format!("macOS {}", String::from_utf8_lossy(&o.stdout).trim()))
            .unwrap_or_else(|_| "macOS".into())
    }
    #[cfg(target_os = "windows")]
    {
        "Windows".into()
    }
}

#[cfg(target_os = "linux")]
mod linux {
    /// Valeur d'un champ de `/etc/os-release`, guillemets ôtés.
    pub fn os_release_field(content: &str, key: &str) -> Option<String> {
        content.lines().find_map(|line| {
            let value = line.strip_prefix(key)?.strip_prefix('=')?;
            Some(value.trim().trim_matches('"').to_string())
        })
    }

    /// Session (Wayland, X11), bureau, et chaque carte graphique avec son pilote.
    pub fn log_session() {
        let var = |name: &str| std::env::var(name).unwrap_or_else(|_| "-".into());
        log::info!(
            "🖥  Session {} · bureau {} · WAYLAND_DISPLAY={} · DISPLAY={}",
            var("XDG_SESSION_TYPE"),
            var("XDG_CURRENT_DESKTOP"),
            var("WAYLAND_DISPLAY"),
            var("DISPLAY")
        );
        for card in graphics_cards() {
            log::info!("🖥  Carte graphique : {card}");
        }
        if let Ok(version) = std::fs::read_to_string("/proc/driver/nvidia/version") {
            if let Some(first) = version.lines().next() {
                log::info!("🖥  {}", first.trim());
            }
        }
    }

    /// `card0 : 1002:1638 (amdgpu)` pour chaque carte de `/sys/class/drm`.
    fn graphics_cards() -> Vec<String> {
        let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
            return Vec::new();
        };
        let mut cards: Vec<String> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                // `card0`, pas ses sorties (`card0-HDMI-A-1`).
                if !name.starts_with("card") || name.contains('-') {
                    return None;
                }
                let device = entry.path().join("device");
                let read = |file: &str| {
                    std::fs::read_to_string(device.join(file))
                        .map(|s| s.trim().trim_start_matches("0x").to_string())
                        .unwrap_or_else(|_| "?".into())
                };
                let driver = std::fs::read_link(device.join("driver"))
                    .ok()
                    .and_then(|d| d.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| "sans pilote".into());
                Some(format!("{name} : {}:{} ({driver})", read("vendor"), read("device")))
            })
            .collect();
        cards.sort();
        cards
    }

    #[cfg(test)]
    mod tests {
        use super::os_release_field;

        #[test]
        fn quoted_os_release_values_are_unwrapped() {
            let content = "NAME=\"Kubuntu\"\nPRETTY_NAME=\"Kubuntu 26.04 LTS\"\nID=ubuntu\n";
            assert_eq!(os_release_field(content, "PRETTY_NAME").as_deref(), Some("Kubuntu 26.04 LTS"));
            assert_eq!(os_release_field(content, "ID").as_deref(), Some("ubuntu"));
            assert_eq!(os_release_field(content, "VERSION"), None);
        }
    }
}
