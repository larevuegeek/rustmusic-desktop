//! Les genres : icône d'un genre, tuiles de l'accueil.

use crate::models::home::*;
use crate::models::app::Data;
use crate::models::app::Route;
use crate::models::app::Section;

/// Une icône qui évoque le genre, du plus précis au plus général.
pub fn genre_icon(name: &str) -> &'static str {
    let n: String = name
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            _ => c,
        })
        .collect();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    if has(&["comedie musicale", "musical", "stage & screen"]) {
        "ph:mask-happy-duotone"
    } else if has(&["film", "soundtrack", "bande originale", "bande-originale", "score", "ost", "games"]) {
        "ph:film-slate-duotone"
    } else if has(&["humour", "humor", "comedy"]) {
        "ph:mask-happy-duotone"
    } else if has(&["spoken word", "poesie"]) {
        "ph:megaphone-duotone"
    } else if has(&["metal", "gothi", "stoner", "doom"]) {
        "ph:skull-duotone"
    } else if has(&["hard rock", "hardrock", "hark rock"]) {
        "ph:lightning-duotone"
    } else if has(&["grunge"]) {
        "ph:drop-duotone"
    } else if has(&["punk", "ska"]) {
        "ph:flame-duotone"
    } else if has(&["psyche"]) {
        "ph:rainbow-duotone"
    } else if has(&["progressi"]) {
        "ph:planet-duotone"
    } else if has(&["new wave", "synthwave", "80s", "annees 80"]) {
        "ph:cassette-tape-duotone"
    } else if has(&["rap", "trap"]) {
        "ph:microphone-stage-duotone"
    } else if has(&["hip hop", "hip-hop", "hiphop"]) {
        "ph:headphones-duotone"
    } else if has(&["jazz", "swing", "bebop", "lounge"]) {
        "ph:martini-duotone"
    } else if has(&["disco", "funk"]) {
        "ph:disco-ball-duotone"
    } else if has(&["r&b", "r & b", "rnb", "rhythm and blues", "soul"]) {
        "ph:heart-duotone"
    } else if has(&["blues"]) {
        "ph:guitar-duotone"
    } else if has(&["downtempo", "chill", "ambient", "trip hop", "trip-hop", "new age", "relax"]) {
        "ph:cloud-moon-duotone"
    } else if has(&["house"]) {
        "ph:speaker-hifi-duotone"
    } else if has(&["dance", "club"]) {
        "ph:disco-ball-duotone"
    } else if has(&["electr", "techno", "trance", "edm", "synth"]) {
        "ph:waveform-duotone"
    } else if has(&["reggae", "dub"]) {
        "ph:leaf-duotone"
    } else if has(&["country", "western", "bluegrass"]) {
        "ph:cactus-duotone"
    } else if has(&["folk", "acoustic", "singer-songwriter", "singer songwriter"]) {
        "ph:campfire-duotone"
    } else if has(&["classi", "opera", "baroque", "orchest", "symphon", "instrumental", "piano"]) {
        "ph:piano-keys-duotone"
    } else if has(&["chanson", "variete", "french", "francais", "franzosisch"]) {
        "ph:microphone-duotone"
    } else if has(&["latin", "salsa", "bossa", "tango"]) {
        "ph:pepper-duotone"
    } else if has(&["monde", "world", "afro"]) {
        "ph:globe-hemisphere-west-duotone"
    } else if has(&["gospel", "religi", "christian", "spiritu"]) {
        "ph:church-duotone"
    } else if has(&["enfant", "child", "kids", "comptine"]) {
        "ph:balloon-duotone"
    } else if has(&["noel", "christmas", "xmas"]) {
        "ph:tree-evergreen-duotone"
    } else if has(&["indie", "inde", "alternati", "alt ", "college"]) {
        "ph:vinyl-record-duotone"
    } else if has(&["top ", "hits"]) {
        "ph:shooting-star-duotone"
    } else if has(&["pop"]) {
        "ph:sparkle-duotone"
    } else if has(&["rock", "garage"]) {
        "ph:guitar-duotone"
    } else {
        "ph:music-notes-duotone"
    }
}

pub(crate) fn genres(data: &Data) -> Vec<GenreTile> {
    let mut genres: Vec<_> = data.genres.iter().collect();
    genres.sort_by(|a, b| b.total_tracks.cmp(&a.total_tracks));
    genres
        .into_iter()
        .take(6)
        .map(|g| GenreTile {
            name: g.name.clone(),
            total: g.total_tracks,
            icon: genre_icon(&g.name),
        })
        .collect()
}

/// Les onglets affichés, et pour le menu d'affichage l'état de chacun.
pub(crate) fn tiles(data: &Data, route: &Route) -> (Vec<Tile>, [bool; 7]) {
    let shown = data.library_tabs();
    let mut flags = [false; 7];
    for (i, (s, ..)) in Section::ALL.iter().enumerate() {
        flags[i] = shown.contains(s);
    }
    let current = route.section();
    let mut tiles: Vec<Tile> = Section::ALL
        .iter()
        .filter(|(s, ..)| shown.contains(s) || current == Some(*s))
        .map(|(s, key, _, icon)| Tile {
            key,
            label: s.label(),
            icon,
            route: Route::Library(*s),
            active: current == Some(*s),
        })
        .collect();
    if data.playlists_tab() {
        tiles.push(Tile {
            key: "playlists",
            label: karbon_ui::intl::t("nav.playlists"),
            icon: "material-symbols:queue-music-rounded",
            route: Route::Playlists,
            active: matches!(route, Route::Playlists | Route::Playlist(_)),
        });
    }
    (tiles, flags)
}
