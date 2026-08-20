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

use crate::commands::library_command::{add_directory, add_files, create_library, create_library_cache, resolve_cover_thumbnail, fetch_all_artist_images, fetch_artist_image, fetch_album_cover, fetch_all_album_covers, set_album_cover, search_deezer_covers, apply_deezer_cover, get_album, get_albums, get_albums_by_artist, get_artist, get_artists, get_similar_artists, get_file_tags, get_genres, get_libraries, get_library, get_library_cache_id_by_path, get_library_dirs, get_library_stats, get_track, set_track_rating, get_tracks, get_tracks_paginated, get_tracks_by_album, get_tracks_by_artist, get_tracks_by_artist_paginated, get_tracks_by_dir, get_tracks_by_genre, list_directory, remove_library, set_default_library, remove_library_dir, rescan_library, rescan_library_dir, save_thumbnail, read_cover_as_base64};
use crate::commands::player_command::{AUDIO_PLAYER, get_progress, open_file, open_files, pause_play, play_file, seek_to, stop_play};
use crate::commands::playlist_command::{add_track_liked, get_tracks_liked, remove_track_liked, get_playlists, get_playlist, create_playlist, update_playlist, delete_playlist, get_playlist_tracks, add_track_to_playlist, remove_track_from_playlist};
use crate::commands::profil_command::{get_profil, get_all_profils, create_profil, update_profil, delete_profil};
use crate::commands::queue_command::{add_queue_track, clear_queue, get_queue, remove_queue_track, replace_queue_tracks, update_queue_state_index, update_queue_state_repeat_mode, update_queue_state_shuffled};
use crate::commands::recent_command::{clear_recent_files, get_recent_files, insert_recent_file, remove_recent_file};
use crate::commands::volume_command::{get_devices, get_volume, set_volume, set_device, mute};
use crate::commands::settings_command::{get_setting, set_setting, get_all_settings, reset_application};
use crate::commands::search_command::search;
use crate::commands::lyrics_command::{get_lyrics, refresh_lyrics};
use crate::commands::dlna_command::{dlna_get_settings, dlna_status, dlna_start, dlna_stop, dlna_update_settings};
use crate::commands::audio_command::{
    get_audio_quality_status, get_replay_gain_settings, set_audio_quality_setting, set_gapless,
    set_next_track, set_replay_gain_settings,
};
use crate::commands::system_command::{get_render_mode, notify_ui_ready, set_render_mode};
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

fn init_logger() {
    let mut log_dir = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    log_dir.push("com.larevuegeek.rustmusic");
    log_dir.push("logs");
    fs::create_dir_all(&log_dir).ok();

    let log_file = log_dir.join("rustmusic.log");

    // Ouvrir le fichier en mode append (on ne perd pas les anciens logs)
    if let Ok(file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        let log_config = ConfigBuilder::new()
            .add_filter_ignore_str("zbus")
            .add_filter_ignore_str("zvariant")
            .add_filter_ignore_str("tracing")
            .add_filter_ignore_str("hyper")
            .add_filter_ignore_str("reqwest")
            .add_filter_ignore_str("h2")
            .add_filter_ignore_str("rustls")
            .build();

        CombinedLogger::init(vec![
            // Terminal : tout à partir de Info (visible dans `npm run tauri dev`)
            TermLogger::new(
                LevelFilter::Info,
                log_config.clone(),
                TerminalMode::Mixed,
                ColorChoice::Auto,
            ),
            // Fichier : erreurs uniquement
            WriteLogger::new(LevelFilter::Error, log_config, file),
        ]).ok();

        log::info!("Logger initialisé → {}", log_file.display());
    } else {
        eprintln!("⚠️ Impossible d'initialiser le logger dans {}", log_file.display());
    }
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

    apply_linux_render_env(force_software, &reason);
}

/// Apply the WebKitGTK / GDK env vars for the chosen rendering path.
#[cfg(target_os = "linux")]
fn apply_linux_render_env(force_software: bool, reason: &str) {
    if force_software {
        log::info!("🖥  Render mode : software ({})", reason);
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        // WebKitGTK ≥ 2.46 (Skia) ignore WEBKIT_DISABLE_COMPOSITING_MODE (le
        // compositing n'est plus désactivable) — cette variable est le « rendu
        // CPU » moderne. Inconnue des vieux WebKit = ignorée, donc sans risque.
        std::env::set_var("WEBKIT_SKIA_ENABLE_CPU_RENDERING", "1");
        std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
        std::env::set_var("GDK_BACKEND", "x11");
        // Fix recurring `gtk_widget_get_scale_factor failed` warning loop
        // when GTK can't query the scale from a non-existent compositor.
        std::env::set_var("GDK_SCALE", "1");
    } else {
        // Native / forced-GPU path. DMABUF is known to break on AMD/Mesa
        // stacks (Tauri/wry issue) so we still neutralise it unless the user
        // explicitly forces GPU mode AND we're on a native machine — in which
        // case we set it too since most modern Mesa builds still have issues.
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        log::info!("🖥  Render mode : GPU ({}, DMABUF off)", reason);
    }
}

#[cfg(not(target_os = "linux"))]
async fn configure_render_pipeline(_app_state: &AppState, _db_mode: crate::core::render_mode::RenderMode) {
    // No-op on Windows / macOS — Tauri's webview stack works fine out of the box.
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

    // Auto-start DLNA server if it was enabled at last shutdown.
    // (Errors are logged inside; never blocks startup.)
    crate::commands::dlna_command::auto_start_if_enabled(&app_state).await;

        tauri::Builder::default()
        // ⚠️ IMPORTANT : single-instance DOIT être le premier plugin enregistré
        // (recommandation officielle Tauri pour que la détection fonctionne).
        // Si une 2e instance tente de se lancer, on focus la fenêtre existante
        // et on ferme le doublon (la callback s'exécute dans la 1re instance).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
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
        }))
        .manage(app_state)
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

            let _tray = TrayIconBuilder::new()
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

            Ok(())
        })
        .on_window_event(|_window, event| {
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
            }
        })
        .invoke_handler(tauri::generate_handler![
            open_file,
            open_files,
            play_file,
            pause_play,
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
            add_directory,
            add_files,
            rescan_library,
            rescan_library_dir,
            get_library_dirs,
            get_tracks_by_dir,
            get_tracks_by_genre,
            get_tracks_by_artist,
            get_tracks_by_artist_paginated,
            get_albums_by_artist,
            get_similar_artists,
            fetch_artist_image,
            fetch_all_artist_images,
            fetch_album_cover,
            fetch_all_album_covers,
            set_album_cover,
            search_deezer_covers,
            apply_deezer_cover,
            list_directory,
            get_file_tags,
            remove_library_dir,
            get_profil,
            get_all_profils,
            create_profil,
            update_profil,
            delete_profil,
            get_recent_files,
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
            delete_playlist,
            get_playlist_tracks,
            add_track_to_playlist,
            remove_track_from_playlist,
            get_track,
            set_track_rating,
            get_tracks,
            get_tracks_paginated,
            get_tracks_by_album,
            get_album,
            get_albums,
            get_artist,
            get_artists,
            get_genres,
            get_library_stats,
            get_setting,
            set_setting,
            get_all_settings,
            reset_application,
            search,
            read_cover_as_base64,
            resolve_cover_thumbnail,
            get_lyrics,
            refresh_lyrics,
            dlna_get_settings,
            dlna_status,
            dlna_start,
            dlna_stop,
            dlna_update_settings,
            get_audio_quality_status,
            set_audio_quality_setting,
            get_replay_gain_settings,
            set_replay_gain_settings,
            set_next_track,
            set_gapless,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
