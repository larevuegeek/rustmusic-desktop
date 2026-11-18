//! Les radios : ce qu'affichent ses gabarits, bâti depuis l'état de la page,
//! et les pochettes à charger.

use crate::models::radios::*;
use crate::mapper::radios::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use crate::helper::date::elapsed_seconds;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Choice;
use crate::models::app::Msg;
use crate::core::radio::directory::RadioResult;
use crate::entity::radio::radio::{RadioRecent, RadioView};

/// Les plus écoutées montrées avant « Afficher plus ».
pub(crate) const VISIBLE: usize = 18;

/// Titres entendus avant « Afficher plus ».
pub(crate) const TRACKS: usize = 12;

pub const GENRES: [Option<&str>; 12] = [None, Some("news"), Some("pop"), Some("rock"), Some("jazz"), Some("classical"), Some("electronic"), Some("dance"), Some("hiphop"), Some("chillout"), Some("talk"), Some("oldies")];

pub(crate) fn from_view(r: &RadioView) -> RadioResult {
    RadioResult {
        uuid: r.uuid.clone(),
        name: r.name.clone(),
        stream_url: r.stream_url.clone(),
        homepage: r.homepage.clone(),
        favicon: r.favicon_url.clone(),
        country_code: r.country_code.clone(),
        tags: r.tags.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
        codec: r.codec.clone(),
        bitrate: r.bitrate.and_then(|b| u32::try_from(b).ok()),
        reduced_quality: r.reduced_quality,
        variants: Vec::new(),
    }
}

pub(crate) fn from_recent(r: &RadioRecent) -> RadioResult {
    RadioResult {
        uuid: r.uuid.clone(),
        name: r.name.clone(),
        stream_url: r.stream_url.clone(),
        homepage: r.homepage.clone(),
        favicon: r.favicon_url.clone(),
        country_code: r.country_code.clone(),
        tags: r.tags.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
        codec: r.codec.clone(),
        bitrate: r.bitrate.and_then(|b| u32::try_from(b).ok()),
        reduced_quality: r.reduced_quality,
        variants: Vec::new(),
    }
}

/// Une adresse collée plutôt que des mots-clés.
pub(crate) fn is_address(s: &str) -> bool {
    let s = s.trim();
    s.starts_with("http://") || s.starts_with("https://")
}

impl crate::models::radios::RadioItem {
    /// Ancre du menu « … » de la carte.
    pub fn more_id(&self) -> String {
        format!("radio.more.{}.{}", self.list, self.index)
    }
}

impl MusicApp {
    /// Pays des stations populaires : celui choisi, sinon celui de la langue de l'appli.
    pub(crate) fn radio_country(&self) -> String {
        match self.data.setting("radio_country") {
            Some(c) if COUNTRY_CODES.contains(&c) => c.to_string(),
            _ => match karbon_ui::intl::locale().tag() {
                "en" => "US",
                "de" => "DE",
                "es" => "ES",
                "it" => "IT",
                _ => "FR",
            }
            .to_string(),
        }
    }

    pub(crate) fn radio_item(&self, r: &RadioResult, logo_path: Option<&str>, rank: Option<usize>, play: Msg, fav: Msg) -> RadioItem {
        let genres: Vec<String> = r
            .tags
            .iter()
            .take(2)
            .map(|g| {
                let mut c = g.chars();
                c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
            })
            .collect();
        let format = [r.codec.clone(), r.bitrate.filter(|b| *b > 0).map(|b| format!("{b} kbit/s"))].into_iter().flatten().collect::<Vec<_>>().join(" ");
        let favorite = self.data.radios.iter().any(|f| (r.uuid.is_some() && f.uuid == r.uuid) || f.stream_url == r.stream_url);
        let logo = logo_path
            .map(String::from)
            .or_else(|| r.favicon.as_ref().and_then(|f| self.views.radios.logos.get(f).cloned().flatten()));
        RadioItem {
            key: r.uuid.clone().unwrap_or_else(|| r.stream_url.clone()),
            name: r.name.clone(),
            sub: if genres.is_empty() { if format.is_empty() { "\u{a0}".into() } else { format.clone() } } else { genres.join(" · ") },
            format,
            logo,
            rank,
            reduced: r.reduced_quality,
            favorite,
            play,
            fav,
            list: 'r',
            index: 0,
        }
    }

    pub fn radios_view(&self) -> RadiosView {
        let page = &self.views.radios;
        let active = self.data.setting("radios") == Some("true");
        let item = |l: char, i: usize, r: &RadioResult, logo: Option<&str>, rank: Option<usize>| {
            let mut it = self.radio_item(r, logo, rank, Msg::Radios(RadiosMsg::Play(l, i)), Msg::Radios(RadiosMsg::Favorite(l, i)));
            it.list = l;
            it.index = i;
            it
        };
        let favorites: Vec<RadioItem> = self.data.radios.iter().enumerate().map(|(i, r)| item('f', i, &from_view(r), r.logo_path.as_deref(), None)).collect();
        let recent_items = page.recent_items.iter().enumerate().map(|(i, r)| item('h', i, &from_recent(r), r.logo_path.as_deref(), None)).collect();
        let country = self.radio_country();
        let name = country_name;
        let setting = self.data.setting("radio_country").unwrap_or("");
        let mut choice = vec![Choice {
            label: t_with("podcasts.country_auto", &[("country", &name(&country))]),
            icon: flag(&country).to_string(),
            active: setting.is_empty(),
            msg: Msg::Radios(RadiosMsg::Country("")),
            icon_only: false,
        }];
        choice.extend(COUNTRY_CODES.iter().map(|c| Choice {
            label: country_name(c),
            icon: flag(c).to_string(),
            active: setting == *c,
            msg: Msg::Radios(RadiosMsg::Country(c)),
            icon_only: false,
        }));
        let country_label = choice.iter().find(|c| c.active).map(|c| c.label.clone()).unwrap_or_default();
        let tracks_count: Vec<TitleItem> = page
            .tracks_count
            .iter()
            .take(if page.all_tracks { usize::MAX } else { TRACKS })
            .map(|r| TitleItem {
                id: r.id,
                title: r.title.clone(),
                sub: format!("{} · {}", r.station, intl::relative_time_short(elapsed_seconds(&r.played_at, chrono::Utc::now()).unwrap_or(0))),
                logo: r.logo_path.clone(),
            })
            .collect();
        RadiosView {
            active,
            count: (!self.data.radios.is_empty()).then(|| {
                let n = self.data.radios.len();
                (intl::number(n as i64), t(if n == 1 { "radios.favorites_one" } else { "radios.favorites_n" }))
            }),
            input: page.input.clone(),
            address: is_address(&page.input),
            search_term: page.search_term,
            results: page.results.as_ref().map(|l| l.iter().enumerate().map(|(i, r)| item('r', i, r, None, None)).collect()),
            favorites,
            loaded: true,
            recent_items,
            tracks_count,
            tracks_more: (page.tracks_count.len() > TRACKS).then(|| {
                if page.all_tracks { t("radios.titles_less") } else { t_with("radios.titles_more", &[("n", &(page.tracks_count.len() - TRACKS).to_string())]) }
            }),
            all_tracks: page.all_tracks,
            country_hint: t_with("radios.popular_hint", &[("country", &name(&country))]),
            country_label,
            country: choice,
            genres: GENRES.iter().map(|g| (*g, t(&format!("radios.genre_{}", g.unwrap_or("all"))), page.genre == *g)).collect(),
            popular: page.popular.as_ref().map(|l| {
                l.iter().take(if page.all { usize::MAX } else { VISIBLE }).enumerate().map(|(i, r)| item('p', i, r, None, Some(i + 1))).collect()
            }),
            popular_error: page.popular_error,
            more: page
                .popular
                .as_ref()
                .filter(|l| !page.all && l.len() > VISIBLE)
                .map(|l| t_with("radios.show_more", &[("n", &intl::number((l.len() - VISIBLE) as i64))])),
        }
    }

    /// La station d'une liste de la page.
    pub(crate) fn radio_at(&self, list: char, i: usize) -> Option<(RadioResult, Option<String>, Option<i64>)> {
        let page = &self.views.radios;
        match list {
            'r' => page.results.as_ref()?.get(i).map(|r| (r.clone(), None, None)),
            'p' => page.popular.as_ref()?.get(i).map(|r| (r.clone(), None, None)),
            'f' => self.data.radios.get(i).map(|r| (from_view(r), r.logo_path.clone(), Some(r.id))),
            'h' => page.recent_items.get(i).map(|r| (from_recent(r), r.logo_path.clone(), None)),
            _ => None,
        }
    }

    pub(crate) fn radios_covers(&self) -> Vec<String> {
        let v = self.radios_view();
        let small = crate::app::resources::assets::small_key;
        let mut paths: Vec<String> = Vec::new();
        for it in v.favorites.iter().chain(&v.recent_items).chain(v.results.iter().flatten()).chain(v.popular.iter().flatten()) {
            paths.extend(it.logo.clone());
        }
        paths.extend(v.tracks_count.iter().filter_map(|x| x.logo.as_deref().map(small)));
        paths
    }
}
