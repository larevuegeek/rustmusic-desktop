//! Aperçu hors écran (`--export`) et banc d'essai (`--bench`).

use crate::app::*;
use crate::mapper::app::app_data::page_route;
use std::sync::mpsc::Receiver;
use crate::app::{AppHandle, Event};

/// Aperçu hors écran : charge les vraies données comme au lancement, puis rend une image
/// PNG sans ouvrir de fenêtre (`--export x.png [--size 1280x900] [--scale 1.5]`).
pub fn export(
    app: AppHandle,
    events: Receiver<Event>,
    path: &std::path::Path,
    size: (u32, u32),
    scale: f32,
    renderer: karbon_ui::window::RenderMode,
) -> karbon_ui::Result<()> {
    use karbon_ui::diagnostics::{self, BenchmarkOptions};
    let args: Vec<String> = std::env::args().collect();
    let value = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let mut music = MusicApp::new(app, events);
    music.export = true;
    music.export_open = value("--open").and_then(|m| match m.as_str() {
        "sleep" => Some(crate::models::app::Menu::Timer),
        "profile" => Some(crate::models::app::Menu::Profile),
        "source" => Some(crate::models::app::Menu::Source),
        "open" => Some(crate::models::app::Menu::Open),
        "library" => Some(crate::models::app::Menu::Library),
        "display" => Some(crate::models::app::Menu::Display),
        "mix" => Some(crate::models::app::Menu::Mix(0)),
        "output" => Some(crate::models::app::Menu::Output),
        "queue" => Some(crate::models::app::Menu::File),
        "network" => Some(crate::models::app::Menu::Network),
        _ => None,
    });
    // Mini-lecteur (« --open mini-player[:queue|:lyrics|:volume] ») et micro-lecteur
    // (« --open micro-player ») : la vue du mode, à la taille donnée par --size.
    if let Some(o) = value("--open").filter(|o| o.starts_with("mini-player") || o == "micro-player") {
        use crate::models::player::MiniMsg;
        use crate::models::player::MiniTab;
        music.export_msgs.push(crate::models::app::Msg::Mini(MiniMsg::Toggle));
        match o.as_str() {
            "micro-player" => music.export_msgs.push(crate::models::app::Msg::Mini(MiniMsg::Micro)),
            "mini-player:queue" => music.export_msgs.push(crate::models::app::Msg::Mini(MiniMsg::Tab(MiniTab::Queue))),
            "mini-player:lyrics" => music.export_msgs.push(crate::models::app::Msg::Mini(MiniMsg::Tab(MiniTab::Lyrics))),
            "mini-player:volume" => music.export_msgs.push(crate::models::app::Msg::Mini(MiniMsg::VolumeMenu(true))),
            _ => {}
        }
    }
    // Paroles : ouvertes par un message, qui lance aussi leur chargement.
    if value("--open").as_deref() == Some("lyrics") {
        music.export_msgs.push(crate::models::app::Msg::Menu(crate::models::app::Menu::Lyrics));
    }
    // Nouveautés (« --open whats-new[:diapo] »).
    if let Some(i) = value("--open").and_then(|v| v.strip_prefix("whats-new").map(|r| r.trim_start_matches(':').parse().unwrap_or(0))) {
        music.export_msgs.push(crate::models::app::Msg::WhatsNew(crate::models::updates::WhatsNewMsg::Open));
        music.export_msgs.push(crate::models::app::Msg::WhatsNew(crate::models::updates::WhatsNewMsg::Go(i)));
    }
    // Playlist auto ou mix (« --open smart », « --open new-mix », « --open smart:<id> »).
    if let Some(o) = value("--open").filter(|o| o.starts_with("smart") || o == "new-mix") {
        let id = o.strip_prefix("smart:").and_then(|i| i.parse().ok());
        music.export_msgs.push(crate::models::app::Msg::Smart(crate::models::playlists::SmartMsg::Open(id, o == "new-mix")));
    }
    // Écran des profils (« --open profiles », « --open profile-new », « --open profile:<id> »).
    if let Some(o) = value("--open").filter(|o| o.starts_with("profile")) {
        if o != "profile" {
            music.export_msgs.push(crate::models::app::Msg::Profiles(crate::models::profiles::ProfileMsg::Open));
        }
        if o == "profile-new" {
            music.export_msgs.push(crate::models::app::Msg::Profiles(crate::models::profiles::ProfileMsg::Create));
        } else if let Some(id) = o.strip_prefix("profile:").and_then(|i| i.parse().ok()) {
            music.export_msgs.push(crate::models::app::Msg::Profiles(crate::models::profiles::ProfileMsg::Edit(id)));
        }
    }
    // Import en cours (« --open import-progress », « --open import-listing ») : chargeur de
    // la bibliothèque et pastille de la barre d'état.
    if let Some(o) = value("--open").filter(|o| o.starts_with("import-")) {
        let listing = o == "import-listing";
        music.data.scan = Some(crate::models::app::ScanProgress {
            label: "Import".into(),
            import: true,
            listing,
            found: 9440,
            current: if listing { 0 } else { 3120 },
            total: if listing { 0 } else { 9440 },
            directory: "/home/david/Musique".into(),
            file_name: "03 Nirvana - Been a Son.flac".into(),
            started: std::time::Instant::now().checked_sub(std::time::Duration::from_secs(95)),
            ..Default::default()
        });
    }
    // Confirmation au mot tapé (« --open reset », « --open remove-server:<id> »).
    if value("--open").as_deref() == Some("reset") {
        music.export_msgs.push(crate::models::app::Msg::Typed(crate::models::layout::TypedMsg::Reset));
    }
    if let Some(id) = value("--open").and_then(|o| o.strip_prefix("remove-server:").and_then(|i| i.parse().ok())) {
        music.export_msgs.push(crate::models::app::Msg::Typed(crate::models::layout::TypedMsg::RemoveServer(id)));
    }
    // Fenêtres de serveur (« --open add-server », « --open server-folders:<id> »).
    if value("--open").as_deref() == Some("add-server") {
        music.export_msgs.push(crate::models::app::Msg::ServerForm(crate::models::servers::ServerFormMsg::OpenAdd));
    }
    if let Some(id) = value("--open").and_then(|o| o.strip_prefix("server-folders:").and_then(|i| i.parse().ok())) {
        music.export_msgs.push(crate::models::app::Msg::ServerForm(crate::models::servers::ServerFormMsg::OpenFolders(id)));
    }
    // Aperçu d'un podcast suivi (« --open pod-preview:<id> »).
    if let Some(id) = value("--open").and_then(|o| o.strip_prefix("pod-preview:").and_then(|i| i.parse().ok())) {
        music.export_msgs.push(crate::models::app::Msg::PodcastMenu(crate::models::podcasts::PodcastMenuMsg::OpenFollowed(crate::models::tracks::TrackMenuAt::Point(400., 300.), id)));
        music.export_msgs.push(crate::models::app::Msg::PodcastPreview(crate::models::podcasts::PodcastPreviewMsg::Open));
    }
    // Fiche d'une sortie audio (« --page settings-audio --open device:<n> »).
    if let Some(i) = value("--open").and_then(|o| o.strip_prefix("device:").and_then(|i| i.parse().ok())) {
        music.export_msgs.push(crate::models::app::Msg::Device(crate::models::settings::DeviceMsg::Open(i)));
    }
    // Éditeur de tags d'un fichier (« --open tags-edit:<chemin> »).
    if let Some(p) = value("--open").and_then(|o| o.strip_prefix("tags-edit:").map(str::to_string)) {
        music.export_msgs.push(crate::models::app::Msg::TagEditor(crate::models::tags::TagEditorMsg::Open(p)));
    }
    // Recherche de pochette (« --open cover-search:<album> »).
    if let Some(id) = value("--open").and_then(|o| o.strip_prefix("cover-search:").map(str::to_string)) {
        music.export_msgs.push(crate::models::app::Msg::Cover(crate::models::albums::CoverMsg::Open(id, "Stone Temple Pilots Core".into())));
    }
    // Bibliothèque (« --open new-library », « --open rename-library »).
    match value("--open").as_deref() {
        Some("new-library") => music.export_msgs.push(crate::models::app::Msg::LibraryForm(crate::models::library::LibraryFormMsg::Create)),
        Some("rename-library") => music.export_msgs.push(crate::models::app::Msg::LibraryForm(crate::models::library::LibraryFormMsg::Rename)),
        _ => {}
    }
    // Mode sélection, lignes cochées (« --open select:0,2 »).
    if let Some(rows) = value("--open").and_then(|o| o.strip_prefix("select:").map(str::to_string)) {
        music.export_msgs.push(crate::models::app::Msg::Selection(crate::models::tracks::SelectionMsg::Toggle));
        for n in rows.split(',').filter_map(|n| n.parse().ok()) {
            music.export_msgs.push(crate::models::app::Msg::Row(crate::models::tracks::RowMsg::Click(n)));
        }
    }
    // Cartes (« --open support », « --open update:available|downloading|error »).
    match value("--open").as_deref() {
        Some("support") => music.views.banners.support = true,
        Some("update:available") => music.views.banners.update = crate::models::layout::Update::Available("0.4.4".into()),
        Some("update:downloading") => music.views.banners.update = crate::models::layout::Update::Downloading("0.4.4".into(), 12_582_912, Some(31_457_280)),
        Some("update:error") => music.views.banners.update = crate::models::layout::Update::Error("Signature invalide".into()),
        _ => {}
    }
    // Colonnes du tableau (« --open columns »).
    if value("--open").as_deref() == Some("columns") {
        music.export_msgs.push(crate::models::app::Msg::Columns(crate::models::tracks::ColumnsMsg::Open));
    }
    // Colonne en cours de redimensionnement (« --open column-resize:<clé>:<largeur> »).
    if let Some((key, width)) = value("--open").and_then(|o| {
        let (key, width) = o.strip_prefix("column-resize:")?.rsplit_once(':')?;
        Some((key.to_string(), width.parse::<f32>().ok()?))
    }) {
        music.column_resize = Some(crate::models::tracks::ColumnResize { key, start: width, width });
    }
    // Menu du clic droit sur l'en-tête d'une colonne (« --open column-menu:<clé> »).
    if let Some(key) = value("--open").and_then(|o| o.strip_prefix("column-menu:").map(str::to_string)) {
        let at = crate::models::tracks::TrackMenuAt::Point(560., 300.);
        music.export_msgs.push(crate::models::app::Msg::Columns(crate::models::tracks::ColumnsMsg::Menu(key, at)));
    }
    // Menu « Gérer » d'une bibliothèque de la barre latérale (« --open lib-manage:<id> »).
    if let Some(id) = value("--open").and_then(|o| o.strip_prefix("lib-manage:").and_then(|i| i.parse().ok())) {
        music.export_msgs.push(crate::models::app::Msg::Library(crate::models::library::LibraryMsg::Manage(id)));
    }
    // Télécommande d'ampli avec un ampli fictif (« --open amp-remote[:mini|:sound|:nav|:standby] »).
    if let Some(variant) = value("--open").and_then(|o| o.strip_prefix("amp-remote").map(|v| v.trim_start_matches(':').to_string())) {
        music.views.amp_remote.view = Some(preview_amp(&variant));
        music.views.amp_remote.tab = match variant.as_str() {
            "sound" => crate::models::amp_remote::AmpTab::Sound,
            "nav" => crate::models::amp_remote::AmpTab::Nav,
            _ => crate::models::amp_remote::AmpTab::Sources,
        };
        // « left », « left-closed » : collée au bord gauche, plus bas.
        if variant.starts_with("left") {
            music.views.amp_remote.dock = Some(crate::models::amp_remote::AmpDock { left: true, inset: 20., top: 160. });
        }
        music.export_amp = Some(Box::leak(variant.into_boxed_str()));
    }
    // Accueil en cours de chargement (« --open home-loading »).
    music.export_skeleton = value("--open").as_deref() == Some("home-loading");
    // Écran de chargement (« --open boot »).
    music.export_boot = value("--open").as_deref() == Some("boot");
    // Dossiers de la bibliothèque (« --open lib-folders »).
    if value("--open").as_deref() == Some("lib-folders") {
        music.export_msgs.push(crate::models::app::Msg::LibraryFolders(crate::models::library::LibraryFoldersMsg::Open));
    }
    // Bilan de la vérification des artistes (« --open artist-report »).
    if value("--open").as_deref() == Some("artist-report") {
        music.export_msgs.push(crate::models::app::Msg::ArtistCheck(crate::models::artists::ArtistCheckMsg::Report));
    }
    // Atelier : nettoyage, historique (« --open clean », « --open history »).
    match value("--open").as_deref() {
        Some("clean") => music.export_msgs.push(crate::models::app::Msg::Clean(crate::models::tags::CleanMsg::Open)),
        Some("history") => music.export_msgs.push(crate::models::app::Msg::History(crate::models::tags::HistoryMsg::Open)),
        Some("rename") => music.export_msgs.push(crate::models::app::Msg::Rename(crate::models::tags::RenameMsg::Open)),
        Some("tag-source") => music.export_msgs.push(crate::models::app::Msg::Source(crate::models::tags::SourceMsg::Open)),
        Some("tag-source:2") => {
            music.export_msgs.push(crate::models::app::Msg::Source(crate::models::tags::SourceMsg::Open));
            music.export_msgs.push(crate::models::app::Msg::Source(crate::models::tags::SourceMsg::Auto));
        }
        Some("batch-tags") => {
            music.export_msgs.push(crate::models::app::Msg::Selection(crate::models::tracks::SelectionMsg::Toggle));
            for n in 0..3 {
                music.export_msgs.push(crate::models::app::Msg::Row(crate::models::tracks::RowMsg::Click(n)));
            }
            music.export_msgs.push(crate::models::app::Msg::Selection(crate::models::tracks::SelectionMsg::EditTags));
        }
        _ => {}
    }
    // Page Morceaux : pages suivantes chargées (« --open tracks-next:3 »).
    if let Some(n) = value("--open").and_then(|o| o.strip_prefix("tracks-next:").and_then(|n| n.parse::<usize>().ok())) {
        for _ in 0..n {
            music.export_msgs.push(crate::models::app::Msg::Tracks(crate::models::tracks::TracksMsg::LoadNext));
        }
    }
    // Chaîne audio d'exemple, DSD64 converti puis rééchantillonné (« --open pipeline ») :
    // pastilles sans lecture en cours ; son détail ouvert (« --open pipeline:menu »).
    if let Some(o) = value("--open").filter(|o| o.starts_with("pipeline")) {
        if o.ends_with(":menu") {
            music.export_msgs.push(crate::models::app::Msg::ChainMenu);
        }
        music.data.player.pipeline = Some(serde_json::json!({
            "source_format": "DSD64", "source_sample_rate": 2_822_400, "source_bits": 1, "source_channels": 2,
            "intermediate_pcm_rate": 88_200, "dsd_filter_taps": 2048, "dsd_decimation": 32,
            "output_sample_rate": 48_000, "output_channels": 2, "device_name": "Haut-parleurs (Realtek(R) Audio)",
            "resampler_active": true, "quality_profile": "high", "backend": "CPAL shared", "bit_perfect": false,
        }));
    }
    // Pochette du lecteur agrandie (« --open cover-zoom »), avec son menu (« cover-zoom:menu »).
    if let Some(o) = value("--open").filter(|o| o.starts_with("cover-zoom")) {
        music.export_msgs.push(crate::models::app::Msg::CoverZoom(true));
        if o.ends_with(":menu") {
            let at = crate::models::tracks::TrackMenuAt::Point(1000., 500.);
            music.export_msgs.push(crate::models::app::Msg::CoverMenu(Some(at)));
        }
    }
    // Saut à une lettre de l'index (« --open tracks-letter:M », albums-, artists-, genres-).
    if let Some((list, l)) = value("--open").and_then(|o| o.split_once("-letter:").map(|(a, b)| (a.to_string(), b.to_string()))) {
        use crate::models::app::Msg;
        match list.as_str() {
            "tracks" => music.export_msgs.push(Msg::Tracks(crate::models::tracks::TracksMsg::Letter(l))),
            "albums" => music.export_msgs.push(Msg::Albums(crate::models::albums::AlbumsMsg::Letter(l))),
            "artists" => music.export_msgs.push(Msg::Artists(crate::models::artists::ArtistsMsg::Letter(l))),
            "genres" => music.export_msgs.push(Msg::Genres(crate::models::genres::GenresMsg::Letter(l))),
            _ => {}
        }
    }
    // Barre latérale : menus des lignes et des sections, « Organiser » (voir crate::ui::pages::sidebar).
    if let Some(o) = value("--open") {
        music.export_msgs.extend(crate::mapper::sidebar::export_msgs(&o));
    }
        // Formulaire d'une nouvelle playlist (« --open new-playlist »).
    if value("--open").as_deref() == Some("new-playlist") {
        music.export_msgs.push(crate::models::app::Msg::PlaylistForm(crate::models::playlists::PlaylistFormMsg::New(None)));
    }
    music.export_search = value("--search");
    music.export_offset = value("--offset").and_then(|v| v.parse().ok()).unwrap_or(0.);
    // Menu `Menu::Popup(id)` ouvert dans l'aperçu (« tracks.sort »…).
    if let Some(id) = value("--popup") {
        music.export_popup = Some(Box::leak(id.into_boxed_str()));
    }
    // Menu contextuel d'un morceau ouvert sur la ligne n (« --track-menu 2 »), sous-liste
    // des playlists dépliée si « --track-menu 2+ ».
    if let Some(n) = value("--track-menu") {
        let sub = n.ends_with('+');
        let n = n.trim_end_matches('+').parse().unwrap_or(0);
        music.export_msgs.push(crate::models::app::Msg::Row(crate::models::tracks::RowMsg::Menu(n, crate::models::tracks::TrackMenuAt::Point(620., 300.))));
        if sub {
            music.export_msgs.push(crate::models::app::Msg::TrackMenu(crate::models::tracks::TrackMenuMsg::Sub));
        }
    }
    // Menu d'un album ouvert (« --album-menu <id> »), sous-listes dépliées.
    if let Some(id) = value("--album-menu") {
        music.export_msgs.push(crate::models::app::Msg::CollOpen(
            crate::models::tracks::TrackMenuAt::Point(620., 260.),
            "Core (2017 Remaster)".into(),
            crate::models::library::CollectionSource::Album { id, artist: None },
        ));
        music.export_msgs.push(crate::models::app::Msg::CollectionMenu(crate::models::library::CollectionMenuMsg::SubPlaylists));
        music.export_msgs.push(crate::models::app::Msg::CollectionMenu(crate::models::library::CollectionMenuMsg::SubCover));
    }
    // Menu de la barre de l'atelier de tags (« --open workshop:tags|files|issues|columns »).
    if let Some(m) = value("--open").and_then(|o| o.strip_prefix("workshop:").map(str::to_string)) {
        music.export_msgs.push(crate::models::app::Msg::Tags(crate::models::tags::TagsMsg::Menu(Some(m))));
    }
    // « … » d'une fiche (« --open detail-more:track|artist:<id>:<nom>|genre:<nom>|composer:<nom> »).
    if let Some(o) = value("--open").and_then(|o| o.strip_prefix("detail-more:").map(str::to_string)) {
        use crate::models::app::Msg;
        use crate::models::library::CollectionSource;
        let at = crate::models::tracks::TrackMenuAt::Named(crate::models::tracks::DETAIL_MORE.into());
        let (kind, rest) = o.split_once(':').unwrap_or((o.as_str(), ""));
        music.export_msgs.push(match kind {
            "artist" => {
                let (id, name) = rest.split_once(':').unwrap_or((rest, rest));
                Msg::Sidebar(crate::models::sidebar::SidebarMsg::ItemOpen(at, crate::models::sidebar::PinKind::Artist, id.into(), name.into()))
            }
            "genre" => Msg::CollOpen(at, rest.into(), CollectionSource::Genre(rest.into())),
            "composer" => Msg::CollOpen(at, rest.into(), CollectionSource::Composer(rest.into())),
            _ => Msg::Track(crate::models::tracks::TrackMsg::Menu),
        });
    }
    if let Some(path) = value("--folder") {
        let name = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_string();
        music.export_msgs.push(crate::models::app::Msg::Folders(crate::models::folders::FoldersMsg::Open(path, name)));
    }
    // Vue en liste des pages qui ont le choix grille / liste.
    if value("--view").as_deref() == Some("list") {
        music.views.view_mode = crate::models::albums::ViewMode::List;
    }
    music.export_route = value("--page").map(|p| page_route(&p));
    // Banc d'essai (« --bench <images> ») : repeinture statique, défilement à la molette
    // au milieu de la page, reconstruction, survol, puis animation (horloge simulée à
    // 60 Hz, après un clic en « --click x,y » : « Suivant » des nouveautés…), rapport JSON
    // de karbon-ui sur la sortie standard.
    if let Some(frames) = value("--bench").and_then(|v| v.parse().ok()) {
        let click = value("--click").and_then(|v| {
            let (x, y) = v.split_once(',')?;
            Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
        });
        let report = diagnostics::render_benchmark(
            BenchmarkOptions { width: size.0, height: size.1, scale, title_bar: false, renderer, frames, click, ..Default::default() },
            music,
        )?;
        println!("{}", report.to_json());
        return Ok(());
    }
    // Le petit lecteur de la zone de notification (« --open tray-player »), seul, avec un
    // morceau d'exemple : sa fenêtre n'existe qu'à côté de la principale.
    if value("--open").as_deref() == Some("tray-player") {
        let state = crate::models::tray::TraySnapshot {
            has_track: true,
            title: "Been a Son - solo acoustic, 1990".into(),
            artist: "Nirvana".into(),
            cover: value("--cover"),
            playing: true,
            progress: 0.42,
        };
        let player = crate::app::tray_player::TrayPlayer::new(state, music.inbox.sender(), karbon_ui::task::Inbox::new());
        let image = diagnostics::render_preview(
            BenchmarkOptions { width: size.0, height: size.1, scale, title_bar: false, reduced_motion: true, renderer, ..Default::default() },
            player,
        )?;
        diagnostics::write_png(&image, path)?;
        println!("Aperçu : {} ({} × {})", path.display(), image.width, image.height);
        return Ok(());
    }
    let image = diagnostics::render_preview(
        BenchmarkOptions {
            width: size.0,
            height: size.1,
            scale,
            title_bar: false,
            reduced_motion: true,
            renderer,
            pointer: value("--hover").and_then(|v| {
                let (x, y) = v.split_once(',')?;
                Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
            }),
            ..Default::default()
        },
        music,
    )?;
    diagnostics::write_png(&image, path)?;
    println!("Aperçu : {} ({} × {})", path.display(), image.width, image.height);
    Ok(())
}

/// Un ampli Onkyo fictif pour l'aperçu de la télécommande : allumé sur CD (ou sur NET avec
/// une liste et une lecture), Stéréo, zone 2 allumée ; en veille pour « standby ».
fn preview_amp(variant: &str) -> crate::core::amp_remote::session::View {
    use crate::core::amp_remote::model::*;
    let mut caps = crate::core::amp_remote::onkyo::default_caps();
    caps.zone2 = true;
    // « v100 » : volume sur 100, trois chiffres à l'afficheur (TX-RZ900…).
    if variant == "v100" {
        caps.volume_max = 100.;
    }
    let input = |name: &str| caps.inputs.iter().find(|i| i.name.eq_ignore_ascii_case(name)).or(caps.inputs.first()).map(|i| i.id.clone());
    let main_input = if variant == "nav" { input("NET") } else { input("CD") };
    let mut net = Net::default();
    if variant == "nav" {
        net.title = "Spotify".into();
        net.count = 24;
        net.cursor = Some(1);
        let lines = [("Mes titres", "folder"), ("Daft Punk", "artist"), ("Random Access Memories", "album"), ("Get Lucky", "track"), ("Playlists", "playlist")];
        net.lines = lines.iter().map(|(text, kind)| NetLine { text: (*text).into(), kind: (*kind).into() }).collect();
        net.track = "Get Lucky".into();
        net.artist = "Daft Punk".into();
        net.album = "Random Access Memories".into();
        net.time = "02:41 / 06:09".into();
        net.status = "play".into();
    }
    let on = variant != "standby";
    let connected = variant != "offline";
    let state = State {
        connected,
        main: ZoneState { power: on, volume: Some(if variant == "v100" { 36. } else { 42. }), muted: false, input: main_input },
        zone2: Some(ZoneState { power: true, volume: Some(30.), muted: false, input: input("TUNER") }),
        mode: caps.modes.first().cloned(),
        bass: Some(2),
        treble: Some(0),
        net,
    };
    let amp = Amp { id: "onkyo:preview".into(), name: "TX-NR686".into(), brand: Brand::Onkyo, model: "TX-NR686".into(), ip: "192.168.1.20".into(), port: 60128 };
    crate::core::amp_remote::session::View { amp, status: Status { caps, state, cover: None } }
}
