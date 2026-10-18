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

/// Cookie PulseAudio de secours. Sans cookie, le client PulseAudio de CPAL en envoie
/// un vide et PipeWire ferme la connexion : CPAL retombait sur ALSA « default », qui
/// échoue en Flatpak (`snd_pcm_avail_delay` : I/O error) — pas de son. Le bac à sable
/// ne voit pas `~/.config/pulse/cookie` (et PipeWire seul n'en crée pas toujours).
/// libpulse envoie alors 256 octets aléatoires, que le serveur accepte : l'identité
/// vient du socket. À appeler au démarrage, avant tout appel audio.
#[cfg(target_os = "linux")]
pub fn ensure_pulse_cookie() {
    use std::io::{Read, Write};
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::PathBuf;

    let maison = dirs::home_dir();
    let connu = std::env::var_os("PULSE_COOKIE")
        .map(PathBuf::from)
        .into_iter()
        .chain(maison.iter().flat_map(|h| [h.join(".config/pulse/cookie"), h.join(".pulse-cookie")]))
        .any(|p| p.exists());
    if connu {
        return;
    }
    let Some(dossier) = dirs::cache_dir() else { return };
    let chemin = dossier.join("pulse-cookie");
    if !chemin.exists() {
        let mut octets = [0u8; 256];
        let ecrit = std::fs::File::open("/dev/urandom")
            .and_then(|mut f| f.read_exact(&mut octets))
            .and_then(|_| std::fs::create_dir_all(&dossier))
            .and_then(|_| std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&chemin))
            .and_then(|mut f| f.write_all(&octets));
        if let Err(e) = ecrit {
            log::warn!("🔊 Cookie PulseAudio de secours impossible ({}) : {e}", chemin.display());
            return;
        }
    }
    std::env::set_var("PULSE_COOKIE", &chemin);
    log::info!("🔊 Cookie PulseAudio de secours : {}", chemin.display());
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
