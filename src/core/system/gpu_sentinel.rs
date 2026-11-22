//! Témoin des sessions dessinées au GPU.
//!
//! Un pilote graphique qui plante (Vulkan, Mesa, pilote propriétaire) tue le processus
//! sans laisser l'utilisateur atteindre le réglage du mode de rendu. On le détecte :
//!
//! 1. Juste avant d'ouvrir la fenêtre au GPU, [`arm`] écrit un fichier témoin.
//! 2. Après [`STABLE_AFTER`] de fonctionnement, ou à la fermeture normale, il est effacé.
//! 3. Encore présent au démarrage suivant : la session GPU s'est arrêtée net. C'est
//!    retenu en base ([`GPU_BOOT_FAILED_KEY`]) et le mode automatique dessine désormais
//!    au processeur. Choisir « GPU » dans les réglages force un nouvel essai ; une
//!    session GPU qui tient efface le souvenir de l'échec.
//!
//! Le témoin porte le numéro du processus : celui d'une instance encore en vie (seconde
//! instance, redémarrage) n'est pas un plantage. Unix seulement ; sous Windows, Direct2D
//! ne pose pas ce problème.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::repository::settings::settings_repository::SettingsRepository;

/// Clé du réglage qui retient qu'une session GPU a planté (lue par le mode automatique).
pub const GPU_BOOT_FAILED_KEY: &str = "gpu_boot_failed";

/// Durée de fonctionnement au-delà de laquelle la session GPU est jugée saine.
const STABLE_AFTER: Duration = Duration::from_secs(30);

/// La session en cours dessine-t-elle au GPU ?
static BOOTED_GPU: AtomicBool = AtomicBool::new(false);

fn sentinel_path() -> PathBuf {
    crate::app::platform::paths::app_data_dir().join("gpu_boot_attempt")
}

/// Au démarrage : la session GPU précédente s'est arrêtée net, ou un échec est déjà
/// retenu. Un témoin trouvé est consigné en base puis effacé.
pub fn gpu_failed_before(pool: &sqlx::SqlitePool) -> bool {
    use crate::app::platform::runtime::block_on;
    // Témoin d'un processus disparu (ou illisible) : la session s'est arrêtée net.
    let owner = std::fs::read_to_string(sentinel_path()).ok().map(|s| s.trim().parse::<u32>().ok());
    if owner.is_some_and(|pid| !pid.is_some_and(is_alive)) {
        log::warn!("🖥  La session précédente, dessinée au GPU, s'est arrêtée net : le mode automatique passe au rendu logiciel");
        if let Err(e) = block_on(SettingsRepository::set(pool, GPU_BOOT_FAILED_KEY, "1")) {
            log::warn!("🖥  Échec GPU non retenu : {e}");
        }
        let _ = std::fs::remove_file(sentinel_path());
        return true;
    }
    block_on(SettingsRepository::get(pool, GPU_BOOT_FAILED_KEY)).ok().flatten().as_deref() == Some("1")
}

/// Le processus `pid` tourne-t-il encore ?
fn is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // Signal 0 : rien n'est envoyé, seule l'existence est vérifiée.
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// Pose le témoin juste avant d'ouvrir la fenêtre au GPU.
pub fn arm() {
    if !cfg!(unix) {
        return;
    }
    BOOTED_GPU.store(true, Ordering::Relaxed);
    let path = sentinel_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&path, std::process::id().to_string()) {
        log::warn!("🖥  Témoin GPU non posé ({}) : {e}", path.display());
    }
}

/// Efface le témoin de cette instance : la session a tenu, ou se ferme normalement.
pub fn disarm() {
    if !BOOTED_GPU.load(Ordering::Relaxed) {
        return;
    }
    let path = sentinel_path();
    let ours = std::fs::read_to_string(&path).is_ok_and(|s| s.trim() == std::process::id().to_string());
    if ours {
        let _ = std::fs::remove_file(path);
    }
}

/// Une fois l'interface affichée : la session GPU qui tient [`STABLE_AFTER`] efface son
/// témoin et le souvenir d'un échec passé.
pub fn confirm_when_stable(pool: sqlx::SqlitePool) {
    if !BOOTED_GPU.load(Ordering::Relaxed) {
        return;
    }
    crate::app::platform::runtime::spawn(async move {
        tokio::time::sleep(STABLE_AFTER).await;
        disarm();
        let failed = SettingsRepository::get(&pool, GPU_BOOT_FAILED_KEY).await.ok().flatten();
        if failed.as_deref() == Some("1") && SettingsRepository::set(&pool, GPU_BOOT_FAILED_KEY, "0").await.is_ok() {
            log::info!("🖥  Le rendu GPU tient de nouveau : échec passé oublié");
        } else {
            log::info!("🖥  Session GPU stable");
        }
    });
}
