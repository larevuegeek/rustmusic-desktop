//! Les années : ce qu'affichent ses gabarits, bâti depuis l'état de la page.

use crate::models::years::*;
use crate::models::app::Choice;
use crate::models::genres::PeriodHead;
use std::collections::HashSet;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Msg;
use crate::mapper::albums::album_detail::album_item;
use crate::mapper::albums::natural_cmp;
use crate::mapper::genres::genre_detail::decade_hue;
use crate::mapper::genres::genre_detail::short_decade;
use crate::query::album::album_list_row::AlbumListRow;

/// Cartes rendues à la fois dans un carrousel (≈ la largeur visible).
pub(crate) const PAGE: usize = 6;

/// Au-delà, le carrousel « Tout » s'arrête : une année montre le reste.
pub(crate) const CEILING: usize = 40;

/// Avant 1900, une année est absente ou fantaisiste.
pub(crate) fn dated(y: Option<i32>) -> Option<i64> {
    y.filter(|y| *y >= 1900).map(i64::from)
}

/// Les albums datés regroupés par décennie, les plus récentes d'abord si `recent`.
pub(crate) fn albums_by_decade(albums: &[AlbumListRow], recent: bool) -> Vec<(i64, Vec<&AlbumListRow>)> {
    let mut by: Vec<(i64, Vec<&AlbumListRow>)> = Vec::new();
    for a in albums {
        let Some(y) = dated(a.year) else { continue };
        let d = y / 10 * 10;
        match by.iter_mut().find(|(k, _)| *k == d) {
            Some((_, l)) => l.push(a),
            None => by.push((d, vec![a])),
        }
    }
    by.sort_by(|x, y| if recent { y.0.cmp(&x.0) } else { x.0.cmp(&y.0) });
    by
}

impl MusicApp {
    pub fn years_view(&self) -> Option<YearsView> {
        let page = self.views.years.as_ref()?;
        let by = albums_by_decade(&self.data.albums, page.recent_items);
        let pl = |n: usize, one: &str, many: &str| (intl::number(n as i64), t(if n == 1 { one } else { many }));
        let index = by
            .iter()
            .map(|(d, _)| (*d, t_with("home.decade_short", &[("d", &short_decade(*d))]), decade_hue(*d), t_with("home.decade_name", &[("d", &short_decade(*d))])))
            .collect();
        let decades = by
            .into_iter()
            .skip_while(|(d, _)| page.since.is_some_and(|x| x != *d))
            .map(|(d, albums)| {
                let year = page.choice.get(&d).copied().flatten();
                let mut list: Vec<&AlbumListRow> = albums.iter().copied().filter(|a| year.is_none_or(|y| dated(a.year) == Some(y))).collect();
                list.sort_by(|x, y| {
                    let o = x.year.cmp(&y.year);
                    (if page.recent_items { o.reverse() } else { o }).then(natural_cmp(&x.artist, &y.artist)).then(natural_cmp(&x.title, &y.title))
                });
                let list_total = list.len();
                let ceiling = if year.is_none() { CEILING.min(list_total) } else { list_total };
                let start = page.start.get(&d).copied().unwrap_or(0).min(ceiling.saturating_sub(1));
                let cards = list.iter().take(ceiling).skip(start).take(PAGE + 1).map(|a| album_item(a)).collect();
                let tracks_count: i64 = albums.iter().map(|a| a.total_tracks).sum();
                let artists: HashSet<&str> = albums.iter().map(|a| a.artist_id.as_deref().unwrap_or(a.artist.as_str())).filter(|s| !s.is_empty()).collect();
                let name = t_with("home.decade_name", &[("d", &short_decade(d))]);
                DecadeView {
                    d,
                    short: short_decade(d),
                    hue: decade_hue(d),
                    albums_n: pl(albums.len(), "library_head.albums_one", "library_head.albums_n"),
                    tracks_n: (intl::number(tracks_count), t(if tracks_count == 1 { "library_head.tracks_one" } else { "library_head.tracks_n" })),
                    artists_n: pl(artists.len(), "library_head.artists_one", "library_head.artists_n"),
                    head: PeriodHead {
                        big: short_decade(d),
                        decade: d,
                        year,
                        total: intl::number(albums.len() as i64),
                        label: name.clone(),
                        chips: (d..d + 10)
                            .map(|yy| {
                                let n = albums.iter().filter(|a| dated(a.year) == Some(yy)).count();
                                (yy, n, match n {
                                    0 => t("years_view.year_empty"),
                                    1 => t("years_view.year_album"),
                                    n => t_with("years_view.year_albums", &[("n", &intl::number(n as i64))]),
                                })
                            })
                            .collect(),
                    },
                    mix: match year {
                        Some(y) => t_with("years_view.mix_year", &[("y", &y.to_string())]),
                        None => t_with("years_view.mix_decade", &[("d", &short_decade(d))]),
                    },
                    cards,
                    start,
                    total: ceiling,
                    see: match year {
                        Some(y) => t_with("years_view.see_year", &[("y", &y.to_string())]),
                        None => t_with("period_view.see_albums", &[("n", &intl::number(list_total as i64))]),
                    },
                    name,
                }
            })
            .collect();
        let orders = [(true, "years_view.newest_first", "material-symbols:arrow-downward-rounded"), (false, "years_view.oldest_first", "material-symbols:arrow-upward-rounded")]
            .into_iter()
            .map(|(r, l, i)| Choice { label: t(l), icon: i.into(), active: page.recent_items == r, msg: Msg::Years(YearsMsg::Order(r)), icon_only: false })
            .collect();
        Some(YearsView {
            decades,
            index,
            order_label: t(if page.recent_items { "years_view.newest_first" } else { "years_view.oldest_first" }),
            order_icon: if page.recent_items { "material-symbols:arrow-downward-rounded" } else { "material-symbols:arrow-upward-rounded" },
            orders,
        })
    }

    pub(crate) fn years_covers(&self) -> Vec<String> {
        self.years_view()
            .map(|v| v.decades.into_iter().flat_map(|d| d.cards.into_iter().filter_map(|a| a.cover)).collect())
            .unwrap_or_default()
    }
}
