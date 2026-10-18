mod commands;
mod core;
mod entity;
mod mapper;
mod repository;
mod helper;
mod service;
mod state;

use std::sync::{Arc, Mutex};
use std::fs;
use tauri::tray::TrayIconBuilder;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::Manager;
use simplelog::{CombinedLogger, ColorChoice, TermLogger, TerminalMode, WriteLogger, LevelFilter, ConfigBuilder};

use crate::commands::smart_playlist_command::{count_smart_playlist, create_smart_playlist, get_rule_vocabulary, get_smart_playlist_rules, preview_smart_playlist, update_smart_playlist};
use crate::commands::playlist_view_command::{get_playlist_tracks_view, get_tracks_view_by_paths};
use crate::commands::export_command::{export_settings_and_playlists, import_settings_and_playlists, preview_import};
use crate::commands::pin_command::{get_library_pins, pin_album, pin_artist, unpin_album, unpin_artist};
use crate::commands::library_command::{add_directory, add_files, create_library, create_library_cache, resolve_cover_thumbnail, resolve_blurred_cover, fetch_all_artist_images, fetch_artist_image, fetch_album_cover, fetch_all_album_covers, cancel_task, clean_image_cache, set_album_cover, search_deezer_covers, apply_deezer_cover, get_album, get_albums, get_albums_by_artist, get_artist, get_artists, get_similar_artists, get_file_tags, get_genres, get_libraries, get_library, get_library_cache_id_by_path, get_library_dirs, get_library_stats, get_library_tag_keys, get_track, mark_track_played, set_track_rating, get_tracks, get_tracks_paginated, get_tracks_by_album, get_tracks_by_artist, get_tracks_by_artist_paginated, get_tracks_by_dir, get_tracks_under_path, get_tracks_by_genre, get_tracks_by_years, get_mix_tracks, get_mix_genres, get_track_letter_offsets, list_directory, remove_library, rename_library, set_default_library, remove_library_dir, rescan_library, rescan_library_dir, relocate_library_root, repair_artist_links, get_track_artists, get_track_locations, save_thumbnail, save_thumbnail_from_file, read_cover_as_base64};
use crate::commands::player_command::{AUDIO_PLAYER, get_progress, open_file, open_files, pause_play, play_file, seek_to, set_sleep_blocked, stop_play};
use crate::commands::playlist_command::{add_track_liked, get_tracks_liked, remove_track_liked, get_playlists, get_playlist, create_playlist, update_playlist, set_playlist_pinned, delete_playlist, get_playlist_tracks, add_track_to_playlist, remove_track_from_playlist};
use crate::commands::profil_command::{get_profile, get_all_profiles, create_profile, update_profile, delete_profile};
use crate::commands::queue_command::{add_queue_track, clear_queue, get_queue, remove_queue_track, replace_queue_tracks, update_queue_state_index, update_queue_state_repeat_mode, update_queue_state_shuffled};
use crate::commands::recent_command::{clear_recent_files, get_recent_files, insert_recent_file, remove_recent_file, save_playback_position};
use crate::commands::volume_command::{get_devices, get_volume, set_volume, set_device, mute};
use crate::commands::settings_command::{get_setting, set_setting, get_all_settings, reset_application};
use crate::commands::search_command::search;
use crate::commands::lyrics_command::{get_lyrics, refresh_lyrics};
use crate::commands::dlna_command::{dlna_get_settings, dlna_status, dlna_start, dlna_stop, dlna_update_settings};
use crate::commands::platform_command::{get_platform_features, set_autostart_portal};
use crate::commands::amp_remote_command::{amp_remote_cover, amp_remote_discover, amp_remote_select, amp_remote_send, amp_remote_status, set_amp_remote_enabled};
use crate::commands::network_player_command::{network_player_select, network_player_selected, network_players_discover, set_airplay_enabled, set_network_players_enabled};
use crate::commands::audio_command::{
    get_audio_quality_status, get_replay_gain_settings, set_audio_quality_setting, set_gapless, log_player_event,
    set_next_track, set_replay_gain_settings,
};
use crate::commands::system_command::{get_light_effects_auto, get_render_mode, notify_ui_ready, set_compact_mode, set_compact_window_size, set_dmabuf_mode, set_render_mode};
use crate::commands::audit_command::audit_library;
use crate::commands::rename_command::{
    apply_rename, check_pattern, clean_tags, list_batch_journal, order_moves, preview_rename,
    undo_batch,
};
use crate::commands::batch_command::cancel_batch;
use crate::commands::metadata_command::{
    metadata_match_album, metadata_search_albums, metadata_search_tracks, metadata_track_values,
};
use crate::commands::tag_command::{
    can_write_tags, prepare_image, prepare_image_from_url, read_track_images, read_track_tags, read_workshop_tracks, write_tags_batch, write_tags_each,
    write_track_tags,
};
use crate::commands::media_controls_command::{
    disable_media_controls, enable_media_controls, is_media_controls_active,
    update_media_metadata, update_media_playback,
};
use crate::commands::wasapi_command::{
    set_dop_preference, set_wasapi_exclusive_preference, wasapi_default_device_name,
    wasapi_list_output_devices, wasapi_probe_device_capabilities, wasapi_test_format_negotiation,
};
use crate::core::audio_player::audio_player::AudioPlayer;
use crate::helper::database::sqlite::{get_database_url};
use crate::state::AppState;

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
    let mut log_dir = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
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

/// Configure the WebKitGTK / GTK rendering pipeline before Tauri starts.
///
/// On Linux without a real GPU (VM, container) or with finicky stacks (old
/// WebKitGTK 2.40 on Debian 12, KDE Wayland + AMD Mesa, SteamOS), the default
/// GPU path can freeze the window, emit `gtk_widget_get_scale_factor failed`
/// floods, or abort with `EGL_BAD_PARAMETER`. We fall back to software
/// rendering in those cases so the app stays usable.
///
/// Resolution order :
///   1. `RUSTMUSIC_RENDER` env var (gpu | software | auto) — escape hatch
///      usable even when the app can't boot far enough to show the UI.
///   2. Crash sentinel : a previous GPU boot that never painted the UI
///      switches us to software and persists `gpu_boot_failed` (see
///      [`crate::core::gpu_sentinel`]).
///   3. The persisted `render_mode` setting (force-gpu / force-software).
///   4. Auto : software on VMs, SteamOS, or remembered GPU failure ;
///      GPU everywhere else.
///
/// IMPORTANT: must be called before the Tauri builder runs, otherwise the
/// env vars are read too late by WebKitGTK / GTK.
#[cfg(target_os = "linux")]
async fn configure_render_pipeline(app_state: &AppState, db_mode: crate::core::render_mode::RenderMode) {
    use crate::core::gpu_sentinel;
    use crate::core::render_mode::RenderMode;
    use crate::repository::settings::settings_repository::SettingsRepository;

    // Indépendant du mode de rendu : un cache de polices d'une autre version
    // bloque WebKit en GPU comme en logiciel.
    crate::core::fontconfig_cache::remove_mismatched_caches();

    let env_mode = RenderMode::from_env();
    if let Some(m) = env_mode {
        log::info!("🖥  RUSTMUSIC_RENDER override : {}", m.as_str());
    }
    let mode = env_mode.unwrap_or(db_mode);
    let src = if env_mode.is_some() { "env" } else { "user" };

    // Consume the crash sentinel : if it's still armed here, the previous GPU
    // boot aborted (EGL crash) or never painted the UI (white window killed
    // by the user). Record the failure so Auto stays on software afterwards.
    let previous_gpu_boot_failed = gpu_sentinel::is_armed();
    gpu_sentinel::disarm();

    let remembered_failure = SettingsRepository::get(&app_state.pool, gpu_sentinel::GPU_BOOT_FAILED_KEY)
        .await
        .ok()
        .flatten()
        .as_deref()
        == Some("1");

    let (force_software, reason): (bool, String) = if env_mode == Some(RenderMode::ForceGpu) {
        // Explicit per-launch GPU test : bypass sentinel and remembered flag.
        (false, "env override (force GPU)".to_string())
    } else if mode == RenderMode::ForceSoftware {
        (true, format!("{} override (force software)", src))
    } else if previous_gpu_boot_failed {
        let _ = SettingsRepository::set(&app_state.pool, gpu_sentinel::GPU_BOOT_FAILED_KEY, "1").await;
        if mode == RenderMode::ForceGpu {
            // A forced-GPU config that crashes would loop forever — demote it
            // back to Auto so the remembered failure takes effect.
            let _ = SettingsRepository::set(&app_state.pool, "render_mode", RenderMode::Auto.as_str()).await;
            log::warn!("🖥  Forced GPU mode crashed at last boot → resetting render_mode to auto");
        }
        (true, "previous GPU boot never displayed the UI".to_string())
    } else if mode == RenderMode::ForceGpu {
        (false, "user override (force GPU)".to_string())
    } else if remembered_failure {
        (true, "auto : remembered GPU boot failure (pick a render mode in Settings to retry)".to_string())
    } else if let Some(virt) = crate::core::system_detect::detect_linux_virt() {
        (true, format!("auto : virt detected ({})", virt))
    } else if crate::core::system_detect::detect_steamos() {
        (true, "auto : SteamOS detected".to_string())
    } else {
        (false, "auto : native machine".to_string())
    };

    if !force_software {
        // Arm the crash sentinel. It is disarmed by the frontend once real
        // frames have been painted (`notify_ui_ready`) or on graceful close.
        gpu_sentinel::arm();
        gpu_sentinel::set_booted_gpu(true);
    }

    // Flatpak : le WebKitGTK et le Mesa du runtime font clignoter des éléments en DMA-BUF
    // et en dessin GPU (Radeon 760M, runtime GNOME 50). Auto coupe les deux ; le GPU garde
    // la composition, et l'interface sa netteté HiDPI (le mode logiciel force GDK_SCALE=1).
    let flatpak_safe = !force_software
        && mode == RenderMode::Auto
        && env_mode.is_none()
        && crate::core::sandbox::flatpak_id().is_some();

    let dmabuf = crate::core::render_mode::dmabuf_mode(
        SettingsRepository::get(&app_state.pool, crate::core::render_mode::DMABUF_KEY)
            .await
            .ok()
            .flatten()
            .as_deref(),
    );
    apply_linux_render_env(force_software, &reason, dmabuf, flatpak_safe);
}

/// Apply the WebKitGTK / GDK env vars for the chosen rendering path.
#[cfg(target_os = "linux")]
fn apply_linux_render_env(force_software: bool, reason: &str, dmabuf: &str, flatpak_safe: bool) {
    if force_software {
        log::info!("🖥  Render mode : software ({})", reason);
        crate::core::render_mode::set_software_active(true);
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        crate::core::render_mode::set_dmabuf_active(false);
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        // WebKitGTK ≥ 2.46 (Skia) ignore WEBKIT_DISABLE_COMPOSITING_MODE (le
        // compositing n'est plus désactivable) — cette variable est le « rendu
        // CPU » moderne. Inconnue des vieux WebKit = ignorée, donc sans risque.
        std::env::set_var("WEBKIT_SKIA_ENABLE_CPU_RENDERING", "1");
        std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
        // X11 seulement s'il est joignable : en Flatpak sous Wayland (`fallback-x11`),
        // ou sur une session Wayland sans XWayland, `DISPLAY` est vide et forcer
        // X11 faisait échouer l'initialisation de GTK. Un choix explicite prime.
        let x11_joignable = std::env::var("DISPLAY").is_ok_and(|d| !d.is_empty());
        if x11_joignable && std::env::var_os("GDK_BACKEND").is_none() {
            std::env::set_var("GDK_BACKEND", "x11");
        }
        // Fix recurring `gtk_widget_get_scale_factor failed` warning loop
        // when GTK can't query the scale from a non-existent compositor.
        std::env::set_var("GDK_SCALE", "1");
    } else {
        // DMA-BUF : sans lui, chaque image est recopiée par le processeur (17 au
        // lieu de 37 images/s au défilement en 4K). Coupé là où le pilote le gère
        // mal (NVIDIA propriétaire, GPU virtuel, Radeon ancienne, pas de GPU
        // accéléré : fenêtre vide ou image qui clignote au défilement) ; la
        // variable d'environnement et `dmabuf_mode` priment.
        let mut raison_coupure = None;
        let actif = match std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER") {
            // WebKit ne le coupe que pour une valeur autre que « 0 ».
            Ok(valeur) => valeur == "0",
            Err(_) => {
                let actif = match dmabuf {
                    "on" => true,
                    "off" => false,
                    _ => {
                        raison_coupure = if flatpak_safe {
                            Some("Flatpak".into())
                        } else {
                            crate::core::system_detect::dmabuf_unsuitable_gpu()
                        };
                        raison_coupure.is_none()
                    }
                };
                if !actif {
                    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
                }
                actif
            }
        };
        crate::core::render_mode::set_dmabuf_active(actif);
        if flatpak_safe && std::env::var_os("WEBKIT_SKIA_ENABLE_CPU_RENDERING").is_none() {
            std::env::set_var("WEBKIT_SKIA_ENABLE_CPU_RENDERING", "1");
            log::info!("🖥  Flatpak : pages dessinées par le processeur, composition par le GPU");
        }
        log::info!(
            "🖥  Render mode : GPU ({}, DMABUF {} — réglage {}{})",
            reason,
            if actif { "on" } else { "off" },
            dmabuf,
            raison_coupure.map(|r| format!(" : {r}")).unwrap_or_default()
        );
    }
}

/// Builds de développement, Linux : GNOME relie la fenêtre au `.desktop` du nom de
/// l'exécutable (`rustmusic` en dev, sans entrée installée : icône générique).
#[cfg(all(target_os = "linux", debug_assertions))]
fn register_dev_desktop_entry() {
    let Some(dossier) = dirs::data_dir().map(|d| d.join("applications")) else { return };
    let Some(exe) = std::env::current_exe().ok() else { return };
    let nom = exe.file_name().and_then(|n| n.to_str()).unwrap_or("rustmusic").to_string();
    // Vectorielle : nette à toutes les échelles (200 %, grille des applications).
    let icone = concat!(env!("CARGO_MANIFEST_DIR"), "/icons/icon-source.svg");
    let contenu = format!(
        "[Desktop Entry]\nType=Application\nName=RustMusic (dev)\nExec={}\nIcon={icone}\nStartupWMClass={nom}\nNoDisplay=true\n",
        exe.display()
    );
    let fichier = dossier.join(format!("{nom}.desktop"));
    if std::fs::read_to_string(&fichier).ok().as_deref() == Some(contenu.as_str()) {
        return;
    }
    match std::fs::create_dir_all(&dossier).and_then(|_| std::fs::write(&fichier, contenu)) {
        Ok(()) => log::info!("🖼  Entrée de bureau de développement : {}", fichier.display()),
        Err(e) => log::warn!("Entrée de bureau de développement non écrite ({}) : {e}", fichier.display()),
    }
}

#[cfg(not(target_os = "linux"))]
async fn configure_render_pipeline(_app_state: &AppState, _db_mode: crate::core::render_mode::RenderMode) {
    // No-op on Windows / macOS — Tauri's webview stack works fine out of the box.
}

/// Instance unique : une 2e instance réveille la première puis se ferme. En Flatpak, le nom
/// D-Bus reprend l'identifiant de l'appli, le seul que le bac à sable autorise d'office.
fn single_instance() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let builder = tauri_plugin_single_instance::Builder::new().callback(focus_existing_window);
    match crate::core::sandbox::flatpak_id() {
        Some(id) => builder.dbus_id(id).build(),
        None => builder.build(),
    }
}

fn focus_existing_window(app: &tauri::AppHandle, _args: Vec<String>, _cwd: String) {
    eprintln!("📌 Single-instance callback fired");

    // On itère TOUTES les fenêtres au lieu de chercher "main" en dur
    let windows = app.webview_windows();
    eprintln!("   Windows trouvées : {}", windows.len());

    for (label, window) in windows {
        let is_min = window.is_minimized().unwrap_or(false);
        let is_vis = window.is_visible().unwrap_or(false);
        eprintln!("   → '{}': minimized={}, visible={}", label, is_min, is_vis);

        // Restauration progressive avec log à chaque étape
        if is_min {
            match window.unminimize() {
                Ok(_) => eprintln!("   ✓ unminimize OK"),
                Err(e) => eprintln!("   ✗ unminimize ERR: {}", e),
            }
        }
        if !is_vis {
            match window.show() {
                Ok(_) => eprintln!("   ✓ show OK"),
                Err(e) => eprintln!("   ✗ show ERR: {}", e),
            }
        }

        // Bypass focus-stealing Windows
        let _ = window.set_always_on_top(true);
        std::thread::sleep(std::time::Duration::from_millis(100));
        match window.set_focus() {
            Ok(_) => eprintln!("   ✓ set_focus OK"),
            Err(e) => eprintln!("   ✗ set_focus ERR: {}", e),
        }
        let _ = window.set_always_on_top(false);

        // Flash icône taskbar comme dernier recours
        let _ = window.request_user_attention(Some(
            tauri::UserAttentionType::Informational
        ));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {

    // ─── INITIALISATION DU LOGGER ────────────────────────────────
    // Écrit les erreurs dans un fichier logs/rustmusic.log
    // dans le même répertoire que la DB (AppData)
    init_logger();

    let database_url: String = get_database_url("com.larevuegeek.rustmusic", "rustmusic.db".into());
    log::info!("SQLite DB = {}", database_url);

    // Pas d'`expect` : c'est le seul point de démarrage qui échoue pour une
    // raison que l'utilisateur peut corriger, et il échoue avant qu'une fenêtre
    // existe. Un panic ici ne laisse aucune trace — ni message, ni journal.
    let app_state: AppState = match AppState::new(&database_url).await {
        Ok(state) => state,
        Err(error) => crate::helper::fatal::fatal(
            "RustMusic — impossible de démarrer",
            &crate::helper::fatal::explain_migration(&error),
        ),
    };

    // ─── CONFIGURATION ENV LINUX ─────────────────────────────────
    // Lit le setting `render_mode` (auto par défaut), puis applique les env
    // vars WebKitGTK / GDK correspondantes. DOIT être appelé avant
    // tauri::Builder::default().run(), sinon WebKit lit les vars trop tard.
    let render_mode = crate::core::render_mode::RenderMode::parse_or_auto(
        crate::repository::settings::settings_repository::SettingsRepository::get(
            &app_state.pool,
            "render_mode",
        )
        .await
        .ok()
        .flatten()
        .as_deref(),
    );
    configure_render_pipeline(&app_state, render_mode).await;
    // Avant tout appel audio : sans cookie, CPAL perd PulseAudio et retombe sur ALSA.
    #[cfg(target_os = "linux")]
    crate::core::sandbox::ensure_pulse_cookie();

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
    crate::commands::audio_command::init_from_settings(&app_state).await;

    // Le démarrage automatique du serveur DLNA **n'est pas ici** : il attend
    // le `setup`, après que le plugin d'instance unique a tranché.
    //
    // Avant, il ouvrait le port depuis ce point : chaque double-clic sur une
    // application déjà lancée produisait une seconde instance qui tentait de
    // s'y lier, échouait, journalisait « adresse déjà utilisée », puis se
    // refermait une ligne plus loin en découvrant l'instance existante. Cinq
    // occurrences dans le journal, toutes fausses.

        tauri::Builder::default()
        // ⚠️ IMPORTANT : single-instance DOIT être le premier plugin enregistré
        // (recommandation officielle Tauri pour que la détection fonctionne).
        // Si une 2e instance tente de se lancer, on focus la fenêtre existante
        // et on ferme le doublon (la callback s'exécute dans la 1re instance).
        .plugin(single_instance())
        .manage(app_state)
        // ─── Mémoire de la fenêtre ───
        //
        // Taille, position, maximisée, plein écran : sans ça l'application
        // rouvrait invariablement en 1280×900, quel que soit l'état dans lequel
        // on l'avait laissée.
        //
        // Les drapeaux sont choisis, pas laissés par défaut. `DECORATIONS` en
        // particulier est écarté : la fenêtre est déclarée `decorations: false`
        // et porte sa propre barre de titre. Le plugin la restaurerait avec les
        // décorations du système par-dessus.
        //
        // Le plugin plutôt qu'une sauvegarde maison dans la table `settings` :
        // ce n'est pas la sauvegarde qui est difficile, c'est la restauration.
        // Une position enregistrée sur un écran débranché depuis rouvre la
        // fenêtre hors du bureau, injoignable. Le plugin ramène ce cas dans les
        // limites visibles ; l'écrire soi-même, c'est le découvrir en
        // production.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED
                        | tauri_plugin_window_state::StateFlags::FULLSCREEN,
                )
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,  // pas d'arguments supplémentaires
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        // Auto-updater : interroge rustmusic.dev/api/releases pour vérifier
        // s'il y a une version plus récente que la version locale. Côté
        // frontend, la vérification est déclenchée au démarrage dans
        // +layout.svelte → bannière de mise à jour si dispo.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Veille : pause avant la mise en veille (Linux), blocage pendant la lecture.
            crate::core::system_sleep::start(app.handle().clone());

            // ─── SYSTEM TRAY ───────────────────────────────────
            // Crée une icône dans la zone de notification (systray)
            // avec un menu "Afficher" et "Quitter"
            // Clic gauche sur l'icône = afficher la fenêtre

            let show_item = MenuItemBuilder::with_id("show", "Afficher RustMusic")
                .build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quitter")
                .build(app)?;

            let tray_menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let tray = TrayIconBuilder::new();
            #[cfg(target_os = "linux")]
            let tray = match crate::core::sandbox::tray_icon_dir() {
                Some(dir) => tray.temp_dir_path(dir),
                None => tray,
            };
            let _tray = tray
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("RustMusic")
                .menu(&tray_menu)
                .on_menu_event(|app: &tauri::AppHandle, event: tauri::menu::MenuEvent| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                window.show().unwrap_or_default();
                                window.set_focus().unwrap_or_default();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|_tray, event: tauri::tray::TrayIconEvent| {
                    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        // Le tray icon event ne donne pas facilement accès au window
                        // On utilise le menu "Afficher" pour ça
                    }
                })
                .build(app)?;

            // Linux : WebKitGTK ne voit pas toujours le pointeur quitter la fenêtre et garde
            // le survol ; GTK reçoit l'entrée et la sortie, relayées au micro-lecteur.
            #[cfg(target_os = "linux")]
            {
                if let Some(gtk_window) = app.get_webview_window("main").and_then(|w| w.gtk_window().ok()) {
                    use gtk::gdk::{EventMask, NotifyType};
                    use gtk::prelude::{WidgetExt, WidgetExtManual};
                    use tauri::Emitter;
                    gtk_window.add_events(EventMask::ENTER_NOTIFY_MASK | EventMask::LEAVE_NOTIFY_MASK);
                    let handle = app.handle().clone();
                    gtk_window.connect_enter_notify_event(move |_, event| {
                        if event.detail() != NotifyType::Inferior {
                            let _ = handle.emit("window-pointer", true);
                        }
                        gtk::glib::Propagation::Proceed
                    });
                    let handle = app.handle().clone();
                    gtk_window.connect_leave_notify_event(move |_, event| {
                        if event.detail() != NotifyType::Inferior {
                            let _ = handle.emit("window-pointer", false);
                        }
                        gtk::glib::Propagation::Proceed
                    });
                }
            }

            // ─── DÉTECTION CRASH DU WEB PROCESS (Linux) ───────────
            // Si le processus de rendu WebKit meurt (SIGABRT EGL/DMABUF, mix
            // AppImage/libs hôte…), la fenêtre reste blanche mais le process
            // principal survit : notify_ui_ready n'arrivera jamais, et une
            // fermeture « propre » (Alt+F4) désarmerait la sentinelle. On
            // écoute donc le signal WebKit `web-process-terminated` : échec
            // GPU persisté immédiatement, puis redémarrage automatique →
            // l'app revient d'elle-même en rendu logiciel.
            #[cfg(target_os = "linux")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let app_handle = app.handle().clone();
                    let _ = window.with_webview(move |platform_webview| {
                        use webkit2gtk::WebViewExt;
                        platform_webview.inner().connect_web_process_terminated(move |_wv, reason| {
                            if reason != webkit2gtk::WebProcessTerminationReason::Crashed {
                                log::error!("🖥  WebKit web process terminated ({:?}) — not a crash, ignoring", reason);
                                return;
                            }
                            // Crash en mode software = pas un problème GPU :
                            // on log seulement (et surtout pas de boucle de
                            // redémarrages infinis).
                            if !crate::core::gpu_sentinel::booted_gpu() {
                                log::error!("🖥  WebKit web process crashed while already in software mode — not restarting");
                                return;
                            }
                            log::error!("🖥  WebKit web process crashed (GPU mode) → recording GPU failure, restarting in software mode");
                            crate::core::gpu_sentinel::mark_web_process_crashed();
                            // Ceinture + bretelles : la sentinelle force le
                            // fallback au prochain boot même si l'écriture DB
                            // ci-dessous échoue.
                            crate::core::gpu_sentinel::arm();
                            let app_handle = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                let state = app_handle.state::<AppState>();
                                let _ = crate::repository::settings::settings_repository::SettingsRepository::set(
                                    &state.pool,
                                    crate::core::gpu_sentinel::GPU_BOOT_FAILED_KEY,
                                    "1",
                                )
                                .await;
                                app_handle.restart();
                            });
                        });
                    });
                }
            }

            // ─── SERVEUR DLNA ──────────────────────────────────
            // Ici et pas avant : à ce point, le plugin d'instance unique a
            // laissé passer ce processus, donc c'est bien lui qui doit tenir
            // le port. Une seconde instance se sera refermée sans y toucher.
            //
            // Détaché, et les erreurs restent internes : un port occupé par un
            // autre serveur multimédia est un désagrément, pas une raison
            // d'empêcher l'écoute.
            let dlna_state = app.state::<AppState>().inner().clone();
            tauri::async_runtime::spawn(async move {
                crate::commands::dlna_command::auto_start_if_enabled(&dlna_state).await;
            });

            // ─── LECTEUR RÉSEAU ────────────────────────────────
            // Désactivé par défaut ; activé, le lecteur mémorisé est relu depuis sa description.
            let np_state = app.state::<AppState>().inner().clone();
            crate::core::network_player::airplay::sender::set_app(app.handle().clone());
            tauri::async_runtime::spawn(async move {
                crate::commands::network_player_command::init(&np_state).await;
            });

            // Sortie audio perdue en pleine lecture : l'interface est prévenue.
            crate::core::audio_player::output::shared_engine::set_app(app.handle().clone());

            // ─── TÉLÉCOMMANDE D'AMPLI ──────────────────────────
            // Désactivée par défaut ; activée, l'ampli mémorisé est rejoint.
            let remote_state = app.state::<AppState>().inner().clone();
            crate::core::amp_remote::session::set_app(app.handle().clone());
            tauri::async_runtime::spawn(async move {
                crate::commands::amp_remote_command::init(&remote_state).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Fermeture propre de la fenêtre = le boot était OK même si le
            // frontend n'a pas eu le temps d'appeler notify_ui_ready
            // (utilisateur qui ferme l'app immédiatement). Sans ça, un
            // fast-close serait compté comme un crash GPU au prochain boot.
            // Exception : si le web process WebKit a crashé (fenêtre blanche),
            // un Alt+F4 ne doit PAS effacer l'échec enregistré.
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if !crate::core::gpu_sentinel::web_process_crashed() {
                    crate::core::gpu_sentinel::disarm();
                }

                // Sauvegarde explicite de l'état de la fenêtre.
                //
                // Le plugin le fait déjà à la sortie du processus, mais cette
                // sortie n'a pas toujours lieu : l'application garde une icône
                // dans la zone de notification, et peut donc survivre à la
                // fermeture de sa fenêtre. Le moment sûr, c'est **maintenant** —
                // la fenêtre existe encore, ses dimensions sont lisibles.
                crate::core::window_geometry::save_now(window.app_handle());
            }

            // Et au fil des redimensionnements : l'appli finit souvent sans sortie propre.
            if matches!(event, tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_)) {
                crate::core::window_geometry::schedule_save(window.app_handle().clone());
            }
        })
        .invoke_handler(tauri::generate_handler![
            open_file,
            open_files,
            play_file,
            pause_play,
            set_sleep_blocked,
            stop_play,
            seek_to,
            get_progress,
            mute,
            get_volume,
            set_volume,
            get_devices,
            crate::commands::device_command::get_output_devices,
            set_device,
            save_thumbnail,
            save_thumbnail_from_file,
            add_directory,
            add_files,
            rescan_library,
            repair_artist_links,
            get_track_artists,
            get_track_locations,
            rescan_library_dir,
            relocate_library_root,
            get_library_dirs,
            get_tracks_by_dir,
            get_tracks_under_path,
            get_tracks_by_genre,
            get_tracks_by_years,
            get_mix_genres,
            get_mix_tracks,
            get_track_letter_offsets,
            get_tracks_by_artist,
            get_tracks_by_artist_paginated,
            get_albums_by_artist,
            get_similar_artists,
            fetch_artist_image,
            fetch_all_artist_images,
            fetch_album_cover,
            fetch_all_album_covers,
            cancel_task,
            clean_image_cache,
            set_album_cover,
            search_deezer_covers,
            apply_deezer_cover,
            list_directory,
            get_file_tags,
            remove_library_dir,
            get_profile,
            get_all_profiles,
            create_profile,
            update_profile,
            delete_profile,
            get_recent_files,
            save_playback_position,
            insert_recent_file,
            remove_recent_file,
            clear_recent_files,
            get_queue,
            update_queue_state_index,
            update_queue_state_shuffled,
            update_queue_state_repeat_mode,
            clear_queue,
            add_queue_track,
            remove_queue_track,
            replace_queue_tracks,
            create_library,
            remove_library,
            set_default_library,
            rename_library,
            get_library,
            get_libraries,
            create_library_cache,
            get_library_cache_id_by_path,
            add_track_liked,
            remove_track_liked,
            get_tracks_liked,
            get_playlists,
            get_playlist,
            create_playlist,
            update_playlist,
            set_playlist_pinned,
            delete_playlist,
            get_playlist_tracks,
            add_track_to_playlist,
            remove_track_from_playlist,
            get_track,
            set_track_rating,
            get_tracks,
            get_tracks_paginated,
            get_library_tag_keys,
            get_playlist_tracks_view,
            get_tracks_view_by_paths,
            mark_track_played,
            export_settings_and_playlists,
            preview_import,
            get_rule_vocabulary,
            count_smart_playlist,
            preview_smart_playlist,
            create_smart_playlist,
            update_smart_playlist,
            get_smart_playlist_rules,
            import_settings_and_playlists,
            get_tracks_by_album,
            get_album,
            get_albums,
            get_artist,
            get_artists,
            get_genres,
            get_library_stats,
            crate::commands::library_command::get_library_last_scan,
            get_library_pins,
            crate::commands::sidebar_command::get_sidebar_covers,
            pin_album,
            unpin_album,
            pin_artist,
            unpin_artist,
            get_setting,
            set_setting,
            get_all_settings,
            reset_application,
            search,
            read_cover_as_base64,
            resolve_cover_thumbnail,
            resolve_blurred_cover,
            get_lyrics,
            refresh_lyrics,
            dlna_get_settings,
            dlna_status,
            network_players_discover,
            network_player_selected,
            network_player_select,
            set_network_players_enabled,
            set_airplay_enabled,
            amp_remote_discover,
            amp_remote_select,
            amp_remote_status,
            amp_remote_send,
            get_platform_features,
            set_autostart_portal,
            amp_remote_cover,
            set_amp_remote_enabled,
            dlna_start,
            dlna_stop,
            dlna_update_settings,
            get_audio_quality_status,
            set_audio_quality_setting,
            get_replay_gain_settings,
            set_replay_gain_settings,
            set_next_track,
            set_gapless,
            log_player_event,
            can_write_tags,
            read_track_tags,
            prepare_image,
            prepare_image_from_url,
            read_track_images,
            read_workshop_tracks,
            write_track_tags,
            write_tags_batch,
            write_tags_each,
            cancel_batch,
            audit_library,
            check_pattern,
            preview_rename,
            apply_rename,
            order_moves,
            clean_tags,
            list_batch_journal,
            undo_batch,
            metadata_search_albums,
            metadata_match_album,
            metadata_search_tracks,
            metadata_track_values,
            get_render_mode,
            get_light_effects_auto,
            set_render_mode,
            notify_ui_ready,
            enable_media_controls,
            disable_media_controls,
            is_media_controls_active,
            update_media_metadata,
            update_media_playback,
            wasapi_list_output_devices,
            wasapi_default_device_name,
            wasapi_test_format_negotiation,
            wasapi_probe_device_capabilities,
            set_wasapi_exclusive_preference,
            set_dop_preference,
            set_dmabuf_mode,
            set_compact_window_size,
            set_compact_mode,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                crate::core::window_geometry::on_exit(app);
                // Quitter pendant une lecture exclusive, ou dans les 5 s qui suivent :
                // la carte mise hors service (profil « off ») revient à PipeWire.
                #[cfg(target_os = "linux")]
                crate::core::audio_player::output::pipewire_profile::restore_all();
            }
        });
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
