mod commands;
mod core;
mod entity;
mod component;
mod controller;
mod mapper;
mod models;
mod query;
mod repository;
mod helper;
mod service;
pub mod app;

use std::sync::{Arc, Mutex};
use std::fs;
use simplelog::{CombinedLogger, ColorChoice, TermLogger, TerminalMode, WriteLogger, LevelFilter, ConfigBuilder};

use crate::commands::playback::player_command::AUDIO_PLAYER;
use crate::core::audio_player::audio_player::AudioPlayer;
use crate::helper::database::sqlite::{get_database_url};
use crate::app::platform::app_state::AppState;

/// Taille au-delà de laquelle le journal est archivé.
///
/// Sans borne, un fichier ouvert en ajout grossit indéfiniment. Cinq mégaoctets
/// représentent des dizaines de milliers de lignes : de quoi couvrir plusieurs
/// semaines d'usage, tout en restant ouvrable dans un éditeur.
const LOG_MAX_BYTES: u64 = 5 * 1024 * 1024;

/// Archive le journal s'il a dépassé sa taille, en n'en gardant qu'un.
///
/// Deux fichiers suffisent : l'actuel et le précédent. Au-delà, on accumule
/// des archives que personne ne relit — et le but est de borner, pas de
/// constituer un historique.
fn rotate_log(path: &std::path::Path) {
    let too_big = fs::metadata(path).map(|m| m.len() > LOG_MAX_BYTES).unwrap_or(false);
    if !too_big {
        return;
    }
    let archive = path.with_extension("log.1");
    let _ = fs::remove_file(&archive);
    let _ = fs::rename(path, &archive);
}

fn init_logger() {
    let mut log_dir = crate::app::platform::paths::data_root().unwrap_or_else(|| std::path::PathBuf::from("."));
    log_dir.push("com.larevuegeek.rustmusic");
    log_dir.push("logs");
    fs::create_dir_all(&log_dir).ok();

    let log_file = log_dir.join("rustmusic.log");
    rotate_log(&log_file);

    // Ouvrir le fichier en mode append (on ne perd pas les anciens logs)
    if let Ok(file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        // Date **et** heure. Sans la date, un journal qui couvre plusieurs
        // jours est illisible : « 12:02:55 » ne dit pas de quel jour, et deux
        // lignes voisines peuvent être séparées d'une semaine.
        //
        // La macro vérifie le format à la compilation : une faute de frappe
        // devient une erreur de build et non un horodatage muet à l'exécution.
        let time_format = time::macros::format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        );

        let mut builder = ConfigBuilder::new();
        builder
            .set_time_format_custom(time_format)
            .add_filter_ignore_str("zbus")
            .add_filter_ignore_str("zvariant")
            .add_filter_ignore_str("tracing")
            .add_filter_ignore_str("hyper")
            .add_filter_ignore_str("reqwest")
            .add_filter_ignore_str("h2")
            .add_filter_ignore_str("rustls")
            // Le client PulseAudio de CPAL écrit « Reactor error: Client
            // disconnected » en Error à chaque connexion refermée normalement.
            .add_filter_ignore_str("pulseaudio");

        // Heure locale plutôt qu'UTC : on relit un journal en le comparant à
        // « ça a planté vers 18 h », pas à un décalage qu'il faut calculer.
        //
        // L'appel échoue sur certains systèmes — lire le fuseau local n'est pas
        // sûr en présence de plusieurs fils d'exécution. On garde alors UTC,
        // qui reste exploitable, plutôt que de renoncer au journal.
        if builder.set_time_offset_to_local().is_err() {
            eprintln!("Fuseau local indisponible : les horodatages seront en UTC.");
        }

        // Symphonia décrit chaque MP3 ouvert en Info (« xing header… ») : des
        // milliers de lignes par scan. On n'en garde que les alertes.
        let symphonia_config = builder.clone().add_filter_allow_str("symphonia").build();
        // En développement, l'appli seule descend au détail (enchaînements, seeks) : sqlx & co en noieraient le fichier.
        let app_debug_config = builder.clone().add_filter_allow_str("rustmusic_lib").build();
        let others_config = builder.clone().add_filter_ignore_str("symphonia").add_filter_ignore_str("rustmusic_lib").build();
        let log_config = builder.add_filter_ignore_str("symphonia").build();
        let term_config = log_config.clone();

        let mut loggers: Vec<Box<dyn simplelog::SharedLogger>> = vec![
            // Terminal : tout à partir de Info (visible dans `npm run tauri dev`)
            TermLogger::new(
                LevelFilter::Info,
                term_config,
                TerminalMode::Mixed,
                ColorChoice::Auto,
            ),
        ];
        if let Ok(symphonia_file) = file.try_clone() {
            loggers.push(WriteLogger::new(LevelFilter::Warn, symphonia_config, symphonia_file));
        }
        // Fichier : dès Info, comme le terminal. Un morceau qui ne s'enchaîne
        // pas ou un DoP qui saute ne lève aucun avertissement : en Warn seul,
        // le journal d'un testeur restait vide.
        if cfg!(debug_assertions) {
            if let Ok(app_file) = file.try_clone() {
                loggers.push(WriteLogger::new(LevelFilter::Debug, app_debug_config, app_file));
            }
            loggers.push(WriteLogger::new(LevelFilter::Info, others_config, file));
        } else {
            loggers.push(WriteLogger::new(LevelFilter::Info, log_config, file));
        }
        CombinedLogger::init(loggers).ok();

        install_panic_hook();

        log::info!("Logger initialisé → {}", log_file.display());
    } else {
        eprintln!("⚠️ Impossible d'initialiser le logger dans {}", log_file.display());
    }
}

/// Fait passer les paniques par le journal.
///
/// Par défaut un `panic!` écrit sur la sortie d'erreur, qui n'existe pas pour
/// une application graphique lancée depuis l'explorateur : le fichier de
/// journal reste muet, et la panne devient une énigme. C'est exactement ce qui
/// s'est produit avec la base migrée par une version plus récente — aucune
/// trace, pendant des jours.
///
/// Le gestionnaire d'origine est conservé et rappelé : on ajoute une écriture,
/// on ne change pas le comportement du programme.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "emplacement inconnu".to_string());

        // La charge utile est une chaîne dans l'immense majorité des cas.
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "panique sans message".to_string());

        log::error!("PANIQUE à {location} — {message}");
        previous(info);
    }));
}

/// Builds de développement, Linux : GNOME relie la fenêtre au `.desktop` du nom de
/// l'exécutable (`rustmusic` en dev, sans entrée installée : icône générique).
#[cfg(all(target_os = "linux", debug_assertions))]
fn register_dev_desktop_entry() {
    let Some(folder) = crate::app::platform::paths::data_root().map(|d| d.join("applications")) else { return };
    let Some(exe) = std::env::current_exe().ok() else { return };
    let name = exe.file_name().and_then(|n| n.to_str()).unwrap_or("rustmusic").to_string();
    // Vectorielle : nette à toutes les échelles (200 %, grille des applications).
    let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/icons/icon-source.svg");
    let content = format!(
        "[Desktop Entry]\nType=Application\nName=RustMusic (dev)\nExec={}\nIcon={icon}\nStartupWMClass={name}\nNoDisplay=true\n",
        exe.display()
    );
    let file = folder.join(format!("{name}.desktop"));
    if std::fs::read_to_string(&file).ok().as_deref() == Some(content.as_str()) {
        return;
    }
    match std::fs::create_dir_all(&folder).and_then(|_| std::fs::write(&file, content)) {
        Ok(()) => log::info!("🖼  Entrée de bureau de développement : {}", file.display()),
        Err(e) => log::warn!("Entrée de bureau de développement non écrite ({}) : {e}", file.display()),
    }
}

/// Démarre le moteur puis ouvre l'interface native ; rend la main à la fermeture.
///
/// Appelé depuis le thread principal, le runtime tokio déjà installé
/// (`app::platform::runtime::install`) : les initialisations asynchrones sont attendues ici, les
/// services de fond sont lancés sur le runtime, puis la boucle de fenêtres prend le
/// thread.
pub fn run() -> karbon_ui::Result<()> {
    use crate::app::platform::runtime;
    use crate::app::AppHandle;

    // ─── INITIALISATION DU LOGGER ────────────────────────────────
    init_logger();
    // `RustMusic /play` : gardé jusqu'à ce que l'interface soit prête.
    crate::core::system::launch_args::remember_startup();

    let database_url: String = get_database_url("com.larevuegeek.rustmusic", "rustmusic.db".into());
    log::info!("SQLite DB = {}", database_url);

    // Pas d'`expect` : c'est le seul point de démarrage qui échoue pour une
    // raison que l'utilisateur peut corriger, et il échoue avant qu'une fenêtre
    // existe. Un panic ici ne laisse aucune trace — ni message, ni journal.
    let app_state: AppState = match runtime::block_on(AppState::new(&database_url)) {
        Ok(state) => state,
        Err(error) => crate::helper::fatal::fatal(
            "RustMusic — impossible de démarrer",
            &crate::helper::fatal::explain_migration(&error),
        ),
    };

    // Avant tout appel audio : sans cookie, CPAL perd PulseAudio et retombe sur ALSA.
    #[cfg(target_os = "linux")]
    crate::core::system::sandbox::ensure_pulse_cookie();

    #[cfg(all(target_os = "linux", debug_assertions))]
    register_dev_desktop_entry();

    // Une carte laissée hors service par un plantage en pleine lecture exclusive revient à PipeWire.
    #[cfg(target_os = "linux")]
    crate::core::audio_player::output::pipewire_profile::restore_pending();

    let audio_player = AudioPlayer::new();

    // Restaurer le volume ET le périphérique sélectionné depuis la config.
    // Le device DOIT être restauré pour que le DoP retrouve le bon `hw:` ALSA
    // au 1er morceau (sinon fallback DSD2PCM).
    if let Ok(config) = crate::core::settings_manager::settings_manager::SettingsManager::load_config() {
        audio_player.set_volume(config.volume_initial);
        audio_player.set_device(config.device_default);
    }

    AUDIO_PLAYER
        .set(Arc::new(Mutex::new(audio_player)))
        .unwrap();

    // Initialise le profil de qualité audio depuis les settings (auto par défaut).
    // Doit être appelé avant le premier play, sinon le 1er morceau utilise
    // les valeurs par défaut (High) au lieu du choix utilisateur.
    runtime::block_on(crate::commands::audio::audio_command::init_from_settings(&app_state));

    // La poignée partagée par les commandes et les services de fond ; ses événements
    // sont lus par l'interface.
    let (app, events) = AppHandle::new();
    app.manage(app_state.clone());

    // Veille : pause avant la mise en veille (Linux), blocage pendant la lecture.
    crate::core::system::sleep::start(app.clone());

    // ─── SERVEUR DLNA ──────────────────────────────────
    // Détaché, et les erreurs restent internes : un port occupé par un
    // autre serveur multimédia est un désagrément, pas une raison
    // d'empêcher l'écoute.
    let dlna_state = app_state.clone();
    runtime::spawn(async move {
        crate::commands::network::dlna_command::auto_start_if_enabled(&dlna_state).await;
    });

    // ─── LECTEUR RÉSEAU ────────────────────────────────
    // Désactivé par défaut ; activé, le lecteur mémorisé est relu depuis sa description.
    let np_state = app_state.clone();
    crate::core::network_player::airplay::sender::set_app(app.clone());
    runtime::spawn(async move {
        crate::commands::network::network_player_command::init(&np_state).await;
    });

    // Sortie audio perdue en pleine lecture : l'interface est prévenue.
    crate::core::audio_player::output::shared_engine::set_app(app.clone());

    // ─── TÉLÉCOMMANDE D'AMPLI ──────────────────────────
    let remote_state = app_state.clone();
    crate::core::amp_remote::session::set_app(app.clone());
    runtime::spawn(async move {
        crate::commands::network::amp_remote_command::init(&remote_state).await;
    });

    // ─── PODCASTS ──────────────────────────────────────
    let podcast_state = app_state.clone();
    let podcast_app = app.clone();
    runtime::spawn(async move {
        crate::commands::online::podcast_command::init(&podcast_state, podcast_app).await;
    });

    // ─── SUIVI D'ÉCOUTE ──────────────────────────────
    let scrobble_state = app_state.clone();
    runtime::spawn(async move {
        crate::commands::online::scrobble_command::init(&scrobble_state).await;
    });

    // ─── ARTISTES MULTIPLES ──────────────────────────
    let split_state = app_state.clone();
    runtime::spawn(async move {
        crate::commands::library::artist_split_command::init(&split_state).await;
    });

    // ─── ALBUMS DISTANTS ─────────────────────────────
    let regroup_pool = app_state.pool.clone();
    runtime::spawn(async move {
        crate::service::external_server::album_regroup::regroup_once(&regroup_pool).await;
    });

    // ─── RADIOS ──────────────────────────────────────
    let radio_state = app_state.clone();
    runtime::spawn(async move {
        crate::commands::online::radio_command::init(&radio_state).await;
    });

    // ─── INTERFACE ───────────────────────────────────
    let result = match export_args() {
        Some((path, size, scale, renderer)) => crate::app::export(app, events, &path, size, scale, renderer),
        None => crate::app::run(app, events),
    };

    crate::core::system::window_geometry::on_exit();
    // Quitter pendant une lecture exclusive, ou dans les 5 s qui suivent :
    // la carte mise hors service (profil « off ») revient à PipeWire.
    #[cfg(target_os = "linux")]
    crate::core::audio_player::output::pipewire_profile::restore_all();
    result
}

#[cfg(test)]
mod logger_tests {
    use super::*;

    /// Dossier temporaire propre à ce test.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rustmusic-log-test-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_short_log_is_not_archived() {
        let dir = scratch("court");
        let log = dir.join("rustmusic.log");
        fs::write(&log, b"une ligne").unwrap();

        rotate_log(&log);

        assert!(log.exists(), "le journal courant doit rester en place");
        assert!(!dir.join("rustmusic.log.1").exists());
    }

    #[test]
    fn an_oversized_log_is_archived() {
        let dir = scratch("gros");
        let log = dir.join("rustmusic.log");
        fs::write(&log, vec![b'x'; (LOG_MAX_BYTES + 1) as usize]).unwrap();

        rotate_log(&log);

        // L'ancien contenu est préservé sous un autre nom, pas supprimé : c'est
        // souvent là que se trouve la trace de la panne qu'on cherche.
        assert!(dir.join("rustmusic.log.1").exists());
        assert!(!log.exists(), "le nouveau journal sera recréé à l'ouverture");
    }

    #[test]
    fn only_one_archive_is_kept() {
        let dir = scratch("archive");
        let log = dir.join("rustmusic.log");
        fs::write(dir.join("rustmusic.log.1"), b"archive precedente").unwrap();
        fs::write(&log, vec![b'x'; (LOG_MAX_BYTES + 1) as usize]).unwrap();

        rotate_log(&log);

        // La précédente est remplacée, pas accumulée : le but est de borner
        // l'espace occupé, pas de constituer un historique.
        let archive = fs::read(dir.join("rustmusic.log.1")).unwrap();
        assert_ne!(archive, b"archive precedente");
        assert_eq!(archive.len() as u64, LOG_MAX_BYTES + 1);
    }

    #[test]
    fn a_missing_log_does_nothing() {
        let dir = scratch("absent");
        rotate_log(&dir.join("rustmusic.log"));
        assert!(!dir.join("rustmusic.log.1").exists());
    }
}

/// `--export x.png [--size LxH] [--scale s] [--renderer=…]` : aperçu hors écran.
fn export_args() -> Option<(std::path::PathBuf, (u32, u32), f32, karbon_ui::window::RenderMode)> {
    let args: Vec<String> = std::env::args().collect();
    let value = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let path = std::path::PathBuf::from(value("--export")?);
    let size = value("--size")
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or((1280, 900));
    let scale = value("--scale").and_then(|s| s.parse().ok()).unwrap_or(1.5);
    let renderer = karbon_ui::window::WindowOptions::from_args(args.iter().filter(|a| a.starts_with("--renderer")).cloned())
        .map(|o| o.renderer)
        .unwrap_or(karbon_ui::window::RenderMode::Auto);
    Some((path, size, scale, renderer))
}
