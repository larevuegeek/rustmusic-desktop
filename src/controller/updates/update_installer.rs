use crate::models::updates::*;
use crate::mapper::updates::update_installer::*;
use std::time::{Duration, Instant};
use karbon_ui::window::WindowCommand;
use karbon_updater::Updater;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::layout::Update;

/// Adresse des versions ; `{{target}}-{{arch}}` (« windows-x86_64 ») : la clé sous laquelle le
/// serveur range les paquets.
pub(crate) const ENDPOINT: &str = "https://rustmusic.dev/api/releases/{{target}}-{{arch}}/{{current_version}}";

/// Clé publique minisign (`tauri signer`) : les paquets sont signés par sa clé secrète.
pub(crate) const PUBLIC_KEY: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/keys/rustmusic.key.pub"));

/// Nouvelles tentatives d'une vérification automatique échouée (serveur en plein
/// déploiement, réseau qui revient).
pub(crate) const RETRIES_MS: [u64; 2] = [60_000, 600_000];

pub(crate) fn updater() -> Updater {
    Updater::new(PUBLIC_KEY, current_version())
        .endpoint(ENDPOINT)
        .user_agent(concat!("RustMusic/", env!("CARGO_PKG_VERSION")))
}

/// Version de l'application. En développement, `RUSTMUSIC_UPDATE_FROM=0.4.3` la remplace
/// pour essayer la carte et le téléchargement (l'installation est refusée depuis un
/// dossier `target` de Cargo).
pub(crate) fn current_version() -> String {
    #[cfg(debug_assertions)]
    if let Ok(version) = std::env::var("RUSTMUSIC_UPDATE_FROM") {
        return version;
    }
    env!("CARGO_PKG_VERSION").to_owned()
}

impl MusicApp {
    /// Au lancement : vérification silencieuse 5 s plus tard, sauf sous Flatpak, en aperçu
    /// et dans une compilation de développement (la vérification manuelle reste possible).
    pub(crate) fn updater_start(&mut self) {
        if flathub() || self.export || cfg!(debug_assertions) {
            return;
        }
        self.update_check_after(5000, 0);
    }

    pub(crate) fn update_check_after(&mut self, ms: u64, attempt: usize) {
        self.backend.call(async move { tokio::time::sleep(Duration::from_millis(ms)).await }, move |me, ()| {
            me.update_check(true, attempt)
        });
    }

    /// `silent` (vérification automatique) n'affiche ni « à jour » ni les
    /// erreurs.
    pub(crate) fn update_check(&mut self, silent: bool, attempt: usize) -> bool {
        let banners = &mut self.views.banners;
        if banners.checking || matches!(banners.update, Update::Downloading(..) | Update::Installing | Update::Ready(_)) {
            return false;
        }
        banners.checking = true;
        self.backend.call(
            async move {
                tokio::task::spawn_blocking(|| updater().check())
                    .await
                    .map_err(|e| karbon_updater::Error::Install(e.to_string()))
                    .and_then(|result| result)
            },
            move |me, result| {
                let banners = &mut me.views.banners;
                banners.checking = false;
                match result {
                    Ok(Some(update)) => {
                        banners.last_check = LastCheck::Available;
                        banners.update = Update::Available(update.version.clone());
                        banners.pending = Some(update);
                    }
                    Ok(None) => banners.last_check = LastCheck::UpToDate,
                    Err(error) => {
                        log::warn!("Mises à jour : {error}");
                        banners.last_check = LastCheck::Failed;
                        if !silent {
                            banners.update = Update::Error(error.to_string());
                        } else if let Some(&wait) = RETRIES_MS.get(attempt) {
                            me.update_check_after(wait, attempt + 1);
                        }
                    }
                }
                true
            },
        );
        true
    }

    /// Téléchargement (avancement dans la carte), signature vérifiée,
    /// installation, puis sortie : l'installateur (Windows) ou le relanceur (macOS, Linux)
    /// démarre la nouvelle version une fois ce processus terminé.
    pub(crate) fn update_install(&mut self) -> bool {
        let Some(update) = self.views.banners.pending.clone() else {
            self.views.banners.update = Update::Error(t("updater.nothing_to_install"));
            return true;
        };
        let version = update.version.clone();
        self.views.banners.update = Update::Downloading(version.clone(), 0, None);
        let backend = self.backend.clone();
        let shown = version.clone();
        self.backend.call(
            async move {
                tokio::task::spawn_blocking(move || {
                    let mut last = Instant::now();
                    let package = update.download(|done, total| {
                        // Dix rafraîchissements par seconde au plus.
                        if last.elapsed() < Duration::from_millis(100) && Some(done) != total {
                            return;
                        }
                        last = Instant::now();
                        let shown = shown.clone();
                        backend.post(move |me| {
                            let downloading = matches!(me.views.banners.update, Update::Downloading(..));
                            if downloading {
                                me.views.banners.update = Update::Downloading(shown, done, total);
                            }
                            downloading
                        });
                    })?;
                    backend.post(|me| {
                        me.views.banners.update = Update::Installing;
                        true
                    });
                    package.install()
                })
                .await
                .map_err(|e| karbon_updater::Error::Install(e.to_string()))
                .and_then(|result| result)
            },
            move |me, result| {
                match result {
                    Ok(()) => {
                        me.views.banners.update = Update::Ready(version);
                        // `relaunch()` 1,5 s après : le temps de lire le message.
                        me.backend.call(async { tokio::time::sleep(Duration::from_millis(1500)).await }, |me, ()| {
                            me.window_commands.push(WindowCommand::Exit);
                            false
                        });
                    }
                    Err(error) => {
                        log::error!("Mise à jour : {error}");
                        me.views.banners.update = Update::Error(error.to_string());
                    }
                }
                true
            },
        );
        true
    }
}
