use crate::models::updates::*;
use crate::mapper::updates::update_whats_new::*;
use crate::models::app::Route;
use crate::models::app::SettingsPage;

pub(crate) const V040: [Slide; 7] = [
    slide(
        "servers",
        "material-symbols:cloud-outline-rounded",
        ["#0ea5e9", "#6366f1"],
        &["material-symbols:dns-outline-rounded", "material-symbols:cloud-download-outline-rounded", "material-symbols:favorite-rounded"],
        &[
            ("simple-icons:jellyfin", "Jellyfin"),
            ("material-symbols:cloud-outline-rounded", "Navidrome"),
            ("simple-icons:dlna", "DLNA"),
            ("material-symbols:cloud-outline-rounded", "Subsonic"),
            ("simple-icons:truenas", "TrueNAS"),
            ("simple-icons:synology", "Synology"),
        ],
        Some(SlideGo::Settings(SettingsPage::Storage)),
    ),
    slide(
        "sources",
        "material-symbols:stacks-outline-rounded",
        ["#8b5cf6", "#0ea5e9"],
        &["material-symbols:computer-outline-rounded", "material-symbols:cloud-outline-rounded", "material-symbols:swap-horiz-rounded"],
        &[],
        None,
    ),
    slide(
        "radios",
        "material-symbols:radio-outline-rounded",
        ["#f97316", "#e11d48"],
        &["material-symbols:history-rounded", "material-symbols:high-quality-outline-rounded", "material-symbols:cast-rounded"],
        &[
            ("material-symbols:public", "Radio Browser"),
            ("material-symbols:history-rounded", "Historique"),
            ("material-symbols:high-quality-outline-rounded", "AAC · MP3 · HLS"),
            ("material-symbols:cast-rounded", "UPnP"),
            ("simple-icons:sonos", "Sonos"),
            ("material-symbols:music-note-rounded", "Titre en direct"),
        ],
        Some(SlideGo::Enabled("radios", || Route::Radios)),
    ),
    slide(
        "podcasts",
        "material-symbols:podcasts-rounded",
        ["#8b5cf6", "#c026d3"],
        &["material-symbols:search-rounded", "material-symbols:rss-feed-rounded", "material-symbols:trophy-outline-rounded"],
        &[
            ("simple-icons:applepodcasts", "Apple Podcasts"),
            ("material-symbols:rss-feed-rounded", "RSS"),
            ("material-symbols:trophy-outline-rounded", "Top 100"),
            ("material-symbols:download-rounded", "Hors ligne"),
            ("material-symbols:upload-file-outline-rounded", "OPML"),
        ],
        Some(SlideGo::Enabled("podcasts", || Route::Podcasts)),
    ),
    slide(
        "podcast_listening",
        "material-symbols:download-for-offline-rounded",
        ["#6366f1", "#0ea5e9"],
        &["material-symbols:bolt-rounded", "material-symbols:replay-rounded", "material-symbols:queue-music-rounded"],
        &[],
        Some(SlideGo::Enabled("podcasts", || Route::PodcastDownloads)),
    ),
    slide(
        "podcast_preview",
        "material-symbols:star-rounded",
        ["#f59e0b", "#e11d48"],
        &["material-symbols:visibility-outline-rounded", "material-symbols:more-horiz", "material-symbols:keep-rounded"],
        &[],
        Some(SlideGo::Enabled("podcasts", || Route::Podcasts)),
    ),
    slide(
        "sidebar_organize",
        "material-symbols:swap-vert-rounded",
        ["#10b981", "#0d9488"],
        &["material-symbols:drag-indicator", "material-symbols:queue-music-rounded", "material-symbols:favorite-rounded"],
        &[],
        Some(SlideGo::SidebarOrganize),
    ),
];

pub(crate) const V042: [Slide; 4] = [
    slide(
        "folder_covers",
        "material-symbols:folder-rounded",
        ["#f59e0b", "#d97706"],
        &["material-symbols:album", "material-symbols:photo-library-rounded", "material-symbols:high-quality-outline-rounded"],
        &[],
        Some(SlideGo::Folders),
    ),
    slide(
        "lastfm",
        "simple-icons:lastdotfm",
        ["#e11d48", "#f97316"],
        &["material-symbols:graphic-eq-rounded", "material-symbols:history-rounded", "material-symbols:public"],
        &[("simple-icons:lastdotfm", "Last.fm"), ("material-symbols:graphic-eq-rounded", "ListenBrainz")],
        Some(SlideGo::Settings(SettingsPage::Online)),
    ),
    slide(
        "m4a_tags",
        "material-symbols:sell-outline-rounded",
        ["#10b981", "#0ea5e9"],
        &["material-symbols:music-note-rounded", "material-symbols:image-outline-rounded", "material-symbols:verified-outline-rounded"],
        &[],
        None,
    ),
    slide(
        "playback_start",
        "material-symbols:play-circle-outline-rounded",
        ["#8b5cf6", "#0ea5e9"],
        &["material-symbols:terminal-rounded", "material-symbols:percent-rounded", "material-symbols:counter-1-outline-rounded"],
        &[],
        Some(SlideGo::Settings(SettingsPage::General)),
    ),
];

pub(crate) const V043: [Slide; 3] = [
    slide(
        "artist_split",
        "material-symbols:group-outline-rounded",
        ["#8b5cf6", "#ec4899"],
        &["material-symbols:call-split-rounded", "simple-icons:musicbrainz", "material-symbols:undo-rounded"],
        &[],
        Some(SlideGo::Settings(SettingsPage::Storage)),
    ),
    slide(
        "composers",
        "material-symbols:stylus-note-rounded",
        ["#10b981", "#0ea5e9"],
        &["material-symbols:piano-rounded", "material-symbols:mic-external-on-outline-rounded", "material-symbols:search-rounded"],
        &[],
        Some(SlideGo::Composers),
    ),
    slide(
        "mac_polish",
        "simple-icons:apple",
        ["#94a3b8", "#64748b"],
        &["material-symbols:dark-mode-outline-rounded", "material-symbols:title-rounded", "material-symbols:apps-rounded"],
        &[],
        None,
    ),
];

/// 0.5.0 : l'interface native (karbon-ui) ; la 0.4.4 n'avait pas de diapos.
pub(crate) const V050: [Slide; 1] = [slide(
    "native_ui",
    "material-symbols:auto-awesome-rounded",
    ["#22c55e", "#0ea5e9"],
    &["material-symbols:bolt-rounded", "material-symbols:computer-outline-rounded", "material-symbols:palette-outline-rounded"],
    &[],
    None,
)];

/// Rappelées après celles de la version, avec le badge « Depuis la … » : les nouveautés de
/// la 0.4.3 (dernières présentées avant la 0.5.0), puis les grandes nouveautés de la 0.4.0.
pub(crate) const RECALLED: [&str; 2] = ["0.4.3", "0.4.0"];

pub(crate) fn release(version: &str) -> &'static [Slide] {
    match version {
        "0.4.0" => &V040,
        "0.4.2" => &V042,
        "0.4.3" => &V043,
        "0.5.0" => &V050,
        _ => &[],
    }
}

/// (diapo, version rappelée).
pub fn slides() -> Vec<(&'static Slide, Option<&'static str>)> {
    let own = release(VERSION);
    let mut out: Vec<_> = own.iter().map(|s| (s, None)).collect();
    if !own.is_empty() {
        for recalled in RECALLED.into_iter().filter(|v| *v != VERSION) {
            out.extend(release(recalled).iter().map(|s| (s, Some(recalled))));
        }
    }
    out
}
