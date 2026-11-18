//! Les mix : ce qu'affichent ses gabarits, bâti depuis l'état de la page,
//! et les pochettes à charger.

use crate::models::mixes::*;
use chrono::Utc;
use crate::app::music_app::MusicApp;
use crate::helper::string::string::hue;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Menu;
use crate::mapper::home::preview;
use crate::mapper::genres::genre_detail::decade_hue;
use crate::mapper::genres::genre_detail::short_decade;

/// En dessous, un genre ou une décennie ne fait pas un mix.
pub(crate) const MIN_TRACKS: i64 = 10;

/// Les décennies qui ont assez de titres pour un mix, les plus récentes d'abord.
pub(crate) fn mix_decades(albums: &[crate::query::album::album_list_row::AlbumListRow]) -> Vec<i64> {
    let mut tracks: Vec<(i64, i64)> = Vec::new();
    for a in albums {
        let Some(y) = a.year.filter(|y| *y >= 1900) else { continue };
        let d = i64::from(y / 10 * 10);
        match tracks.iter_mut().find(|(k, _)| *k == d) {
            Some((_, n)) => *n += a.total_tracks,
            None => tracks.push((d, a.total_tracks)),
        }
    }
    tracks.retain(|(_, n)| *n >= MIN_TRACKS);
    tracks.sort_by(|a, b| b.0.cmp(&a.0));
    tracks.into_iter().map(|(d, _)| d).collect()
}

impl crate::models::mixes::MixTile {
    /// Ancre du menu de la tuile.
    pub fn menu_id(&self) -> String {
        format!("mix.{}", self.key)
    }
}

impl MusicApp {
    /// Les mix de la page, sans leur tirage.
    pub(crate) fn mix_specs(&self) -> (Vec<MixTile>, Vec<MixTile>, Vec<MixTile>, Vec<MixTile>) {
        let tile = |key: String, kind: String, name: String, help: String, h: f32, source: MixSource| MixTile {
            key,
            kind,
            name,
            help,
            h,
            source,
            pre: String::new(),
            short: String::new(),
            playlist: None,
            covers_list: Vec::new(),
            description: String::new(),
            meta: String::new(),
            ready: false,
            menu: false,
            decade: false,
        };
        let genre = |g: &crate::commands::library::library_command::GenreMix, kind: String| {
            tile(
                format!("genre:{}", g.name.to_lowercase()),
                kind,
                g.name.clone(),
                t_with("mix_view.genre_desc", &[("n", &g.artists.to_string())]),
                hue(&g.name.to_lowercase()),
                MixSource::Kind { kind: "genre", genre: Some(g.name.clone()), decade: None },
            )
        };
        let genres = &self.data.mix_genres;
        // « Pour vous » : deux genres du jour parmi les six plus fournis, les oubliés, le Hi-Res.
        let mut for_you = Vec::new();
        if !genres.is_empty() {
            let rotating: Vec<_> = genres.iter().take(6).collect();
            let day = (Utc::now().timestamp() / 86_400) as usize;
            let chosen_items: Vec<_> = if rotating.len() <= 2 {
                rotating.clone()
            } else {
                vec![rotating[day % rotating.len()], rotating[(day + 1) % rotating.len()]]
            };
            for_you.extend(chosen_items.into_iter().map(|g| genre(g, t("home.mix_of_day"))));
            for_you.push(tile("oublies".into(), t("home.rediscover"), t("home.forgotten"), t("home.forgotten_kind"), 200., MixSource::Kind { kind: "oublies", genre: None, decade: None }));
            if self.data.stats.as_ref().is_some_and(|s| s.quality_hires > 0) {
                for_you.push(tile("hires".into(), t("home.studio_quality"), "Hi-Res".into(), t("home.hires_kind"), 150., MixSource::Kind { kind: "hires", genre: None, decade: None }));
            }
        }
        let custom = self
            .data
            .playlists
            .iter()
            .filter(|p| p.is_mix)
            .map(|p| MixTile { playlist: Some(p.id), ..tile(format!("perso:{}", p.id), t("mix_view.custom_kind"), p.name.clone(), p.name.clone(), hue(&p.name), MixSource::Playlist(p.id)) })
            .collect();
        let by_genre = genres
            .iter()
            .map(|g| genre(g, t("mix_view.genre_kind")))
            .filter(|m| !for_you.iter().any(|p| p.key == m.key))
            .collect();
        let decades = mix_decades(&self.data.albums)
            .into_iter()
            .map(|d| {
                let short = short_decade(d);
                MixTile {
                    decade: true,
                    pre: t("home.decade_pre"),
                    short: t_with("home.decade_short", &[("d", &short)]),
                    ..tile(
                        format!("decennie:{d}"),
                        t("home.decade_pre"),
                        t_with("home.decade_name", &[("d", &short)]),
                        t_with("home.decade_kind", &[("a", &d.to_string()), ("b", &(d + 9).to_string())]),
                        decade_hue(d),
                        MixSource::Kind { kind: "decennie", genre: None, decade: Some(d) },
                    )
                }
            })
            .collect();
        (for_you, custom, by_genre, decades)
    }

    pub fn mixes_view(&self) -> MixesView {
        // Un seul menu « mix » ouvert à la fois : la carte qui le porte.
        let menu = match self.menu {
            Some(Menu::Popup("mix")) => self.views.mix_menu.clone(),
            _ => None,
        };
        let fill = |mut l: Vec<MixTile>| {
            for m in l.iter_mut() {
                if let Some(draw) = self.data.mixes.get(&m.key).filter(|d| d.loaded) {
                    let (covers_list, description, meta) = preview(&draw.tracks);
                    m.covers_list = covers_list;
                    m.description = description;
                    m.meta = meta;
                    m.ready = true;
                }
                m.menu = menu.as_deref() == Some(m.key.as_str());
            }
            l
        };
        let (a, b, c, d) = self.mix_specs();
        MixesView { loading: !self.data.mix_genres.is_empty() || self.data.stats.is_some(), for_you: fill(a), custom: fill(b), genres: fill(c), decades: fill(d) }
    }

    pub(crate) fn mixes_covers(&self) -> Vec<String> {
        let v = self.mixes_view();
        let small = crate::app::resources::assets::small_key;
        v.for_you
            .iter()
            .chain(&v.custom)
            .chain(&v.genres)
            .chain(&v.decades)
            .flat_map(|m| if m.covers_list.len() >= 4 { m.covers_list.iter().map(|p| small(p)).collect::<Vec<_>>() } else { m.covers_list.iter().take(1).cloned().collect() })
            .collect()
    }
}

/// Les actions du menu « … » d'un mix de « Pour vous » (rang `index`) : lire, ajouter à
/// la file, nouveau tirage, enregistrer en playlist.
pub fn mix_actions(index: usize) -> Vec<crate::models::app::Choice> {
    use crate::models::app::Msg;
    [
        ("material-symbols-light:play-arrow-rounded", "albums_view.play", Msg::PlayMix(index)),
        ("material-symbols-light:playlist-add-rounded", "albums_view.enqueue", Msg::CloseMenu),
        ("material-symbols-light:refresh-rounded", "mix_view.redraw", Msg::RenewMixes),
        ("material-symbols-light:playlist-add-check-rounded", "mix_view.save_as_playlist", Msg::CloseMenu),
    ]
    .into_iter()
    .map(|(icon, key, msg)| crate::models::app::Choice {
        label: karbon_ui::intl::t(key),
        icon: icon.into(),
        active: false,
        msg,
        icon_only: false,
    })
    .collect()
}
