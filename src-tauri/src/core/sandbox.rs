//! Paquet Flatpak : ce que le bac à sable change pour l'appli (mise à jour,
//! lancement au démarrage, accès direct à la carte son).

use serde::Serialize;

/// Identifiant Flatpak de l'appli (« dev.rustmusic.RustMusic »), posé par Flatpak au lancement.
pub fn flatpak_id() -> Option<String> {
    std::env::var("FLATPAK_ID").ok().filter(|v| !v.is_empty())
}

/// Sortie exclusive et DSD natif possibles : la carte son doit être visible. Un Flatpak sans
/// accès aux périphériques ne voit pas `/dev/snd` ; l'accorder suffit à les rétablir.
pub fn exclusive_possible() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new("/dev/snd").exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

/// Ce que l'interface doit savoir pour n'afficher que ce qui marche.
#[derive(Debug, Clone, Serialize)]
pub struct PlatformFeatures {
    /// Mises à jour et lancement au démarrage passent par Flathub et ses portails.
    pub flatpak: bool,
    pub exclusive: bool,
}

pub fn features() -> PlatformFeatures {
    PlatformFeatures { flatpak: flatpak_id().is_some(), exclusive: exclusive_possible() }
}

/// Lancement à l'ouverture de session, demandé au portail « Background » : le fichier
/// d'autostart du plugin, écrit dans le bac à sable, n'est pas lu par la session.
#[cfg(target_os = "linux")]
pub fn request_autostart(enable: bool) -> Result<(), String> {
    use std::collections::HashMap;
    use zbus::zvariant::Value;

    let conn = zbus::blocking::Connection::session().map_err(|e| format!("session D-Bus : {e}"))?;
    let mut options: HashMap<&str, Value> = HashMap::new();
    options.insert("reason", Value::from("RustMusic au démarrage de la session"));
    options.insert("autostart", Value::from(enable));
    options.insert("commandline", Value::from(vec!["rustmusic"]));
    conn.call_method(
        Some("org.freedesktop.portal.Desktop"),
        "/org/freedesktop/portal/desktop",
        Some("org.freedesktop.portal.Background"),
        "RequestBackground",
        &("", options),
    )
    .map_err(|e| format!("portail Background : {e}"))?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn request_autostart(_enable: bool) -> Result<(), String> {
    Err("portail Background : Linux seulement".into())
}
