//! Les résultats de la recherche rapide.

use crate::models::home::*;
use crate::models::app::Data;
use crate::models::app::Route;
use crate::entity::search::search_result::SearchResult;

pub(crate) fn results(data: &Data) -> Vec<SearchHit> {
    data.search.iter().map(result).collect()
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
    }
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
