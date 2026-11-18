//! Les résultats de la recherche rapide, en sections : le meilleur résultat, puis les
//! artistes, les albums, les morceaux et les compositeurs, chacun borné.

use crate::models::home::*;
use crate::models::app::Data;
use crate::models::app::Route;
use crate::entity::search::search_result::SearchResult;
use karbon_ui::intl::t;

/// Sections de la liste, dans l'ordre : type et nombre de lignes au plus.
const SECTIONS: [(ResultKind, usize); 4] = [
    (ResultKind::Artist, 3),
    (ResultKind::Album, 4),
    (ResultKind::Track, 5),
    (ResultKind::Composer, 2),
];
/// Pertinence au-delà de laquelle aucun résultat n'est mis en tête (le texte n'est qu'au
/// milieu d'un mot).
const TOP_MAX_MATCH: u8 = 2;

/// Lignes de la liste de suggestions, dans l'ordre affiché (celui des flèches et du clic).
pub(crate) fn results(data: &Data) -> Vec<SearchHit> {
    let query = data.search_query.trim().to_lowercase();
    let kind = |r: &SearchResult| ResultKind::parse(&r.result_type);
    // Meilleur résultat : la meilleure correspondance, un artiste avant un album, un
    // album avant un compositeur, un compositeur avant un morceau.
    let top = data
        .search
        .iter()
        .enumerate()
        .map(|(i, r)| (match_rank(&r.title, &query), kind_rank(kind(r)), i))
        .filter(|(rank, ..)| *rank <= TOP_MAX_MATCH)
        .min()
        .map(|(.., i)| i);
    let mut hits = Vec::new();
    if let Some(i) = top {
        hits.push(SearchHit { section: t("search.top_result"), top: true, ..result(&data.search[i]) });
    }
    for (section, max) in SECTIONS {
        // Déjà rangés par pertinence par la base ; le tri stable le garantit.
        let mut found: Vec<(usize, &SearchResult)> = data
            .search
            .iter()
            .enumerate()
            .filter(|(i, r)| kind(r) == section && Some(*i) != top)
            .collect();
        found.sort_by_key(|(_, r)| match_rank(&r.title, &query));
        for (n, (_, r)) in found.into_iter().take(max).enumerate() {
            let title = if n == 0 { section_title(section) } else { String::new() };
            hits.push(SearchHit { section: title, ..result(r) });
        }
    }
    hits
}

pub(crate) fn result(r: &SearchResult) -> SearchHit {
    let genre = ResultKind::parse(&r.result_type);
    let route = match genre {
        ResultKind::Album => Some(Route::Album(r.id.clone())),
        ResultKind::Artist => Some(Route::Artist(r.id.clone())),
        ResultKind::Composer => Some(Route::Composer(r.title.clone())),
        ResultKind::Track => None,
    };
    SearchHit {
        title: r.title.clone(),
        subtitle: r.subtitle.clone().unwrap_or_default(),
        genre,
        cover_path: r.thumbnail_path.clone(),
        route,
        path: r.path.clone(),
        section: String::new(),
        top: false,
    }
}

/// 0 : nom exact ; 1 : commence par le texte ; 2 : un mot commence par lui ; 3 : le
/// contient ; 4 : ne le contient pas (morceau trouvé par son artiste).
fn match_rank(name: &str, query: &str) -> u8 {
    let name = name.to_lowercase();
    if query.is_empty() || !name.contains(query) {
        4
    } else if name == query {
        0
    } else if name.starts_with(query) {
        1
    } else if name
        .match_indices(query)
        .any(|(i, _)| name[..i].ends_with(|c: char| !c.is_alphanumeric()))
    {
        2
    } else {
        3
    }
}

fn kind_rank(kind: ResultKind) -> u8 {
    match kind {
        ResultKind::Artist => 0,
        ResultKind::Album => 1,
        ResultKind::Composer => 2,
        ResultKind::Track => 3,
    }
}

fn section_title(kind: ResultKind) -> String {
    t(match kind {
        ResultKind::Artist => "library.artists",
        ResultKind::Album => "library.albums",
        ResultKind::Track => "library.tracks",
        ResultKind::Composer => "composers_view.mode_composers",
    })
}

impl ResultKind {
    pub fn label(self) -> String {
        match self {
            Self::Track => karbon_ui::intl::t("track_view.kicker"),
            Self::Album => karbon_ui::intl::t("album_view.type_album"),
            Self::Artist => karbon_ui::intl::t("artist_view.kicker"),
            Self::Composer => karbon_ui::intl::t("composers_view.kicker"),
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Track => "lucide:music",
            Self::Album => "lucide:disc-album",
            Self::Artist => "lucide:mic-2",
            Self::Composer => "material-symbols:stylus-note-rounded",
        }
    }

    pub(crate) fn parse(kind: &str) -> Self {
        match kind {
            "album" => Self::Album,
            "artist" | "artiste" => Self::Artist,
            "composer" | "compositeur" => Self::Composer,
            _ => Self::Track,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::match_rank;

    #[test]
    fn exact_then_prefix_then_word_then_inside() {
        assert_eq!(match_rank("Nirv", "nirv"), 0);
        assert_eq!(match_rank("Nirvana", "nirv"), 1);
        assert_eq!(match_rank("The Nirvana Tribute", "nirv"), 2);
        assert_eq!(match_rank("Le (nirvana)", "nirv"), 2);
        assert_eq!(match_rank("Anirvana", "nirv"), 3);
        assert_eq!(match_rank("Lithium", "nirv"), 4);
    }
}
