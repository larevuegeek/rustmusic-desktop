//! Les statistiques de la bibliothèque : ce qu'affichent ses gabarits, bâti depuis l'état de la page.

use crate::models::library::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use crate::helper::color::hex;
use karbon_ui::intl::{t, t_with};
use crate::mapper::tracks::format_bitrate;
use crate::models::app::{Msg, Route};

/// Formats listés ; les suivants sont réunis en « Autres ».
const FORMATS_SHOWN: usize = 5;

/// Une part sous laquelle le bloc de la barre segmentée ne porte pas son pourcentage.
const LABEL_MIN_PCT: f32 = 9.;

/// « 97 % » (« < 1 % » pour une part non nulle qui s'arrondit à zéro).
pub(crate) fn share_label(part: i64, total: i64) -> String {
    if total <= 0 || part <= 0 {
        return "0 %".into();
    }
    let pct = part as f64 / total as f64 * 100.;
    if pct < 0.5 {
        "< 1 %".into()
    } else {
        format!("{} %", pct.round())
    }
}

/// Jours d'écoute à raison de deux heures par jour.
pub(crate) fn listening_days(total_sec: f64) -> i64 {
    (total_sec / 7200.).ceil() as i64
}

/// Barres classées : longueur relative à la première ligne.
fn ranked(rows: Vec<(String, i64, Option<Msg>)>, label: &dyn Fn(i64) -> String) -> Vec<Bar> {
    let max = rows.first().map(|r| r.1).unwrap_or(1).max(1);
    rows.into_iter()
        .enumerate()
        .map(|(i, (name, count, open))| Bar { rank: i + 1, name, count: label(count), share: None, pct: count as f32 / max as f32 * 100., open })
        .collect()
}

impl MusicApp {
    pub fn stats_view(&self) -> Option<StatsView> {
        let s = self.data.stats.as_ref()?;
        let color = hex(self.data.profile.as_ref().map(|p| p.color.as_str()).unwrap_or("#22c55e"));
        let tracks_label = |n: i64| if n == 1 { t("home.track_one") } else { t_with("home.tracks_n", &[("n", &intl::number(n))]) };

        let kicker = match self.data.library() {
            Some(l) => format!("{} · {}", t("library_head.kicker"), l.name),
            None => t("library_head.kicker"),
        };
        let counts = vec![
            (intl::number(s.total_tracks), t("stats.tracks")),
            (intl::number(s.total_albums), t("stats.albums")),
            (intl::number(s.total_artists), t("stats.artists")),
        ];

        let average = (s.total_tracks > 0).then(|| intl::duration(s.total_duration_sec / s.total_tracks as f64));
        let per_album = (s.total_albums > 0).then(|| intl::decimal(s.total_tracks as f64 / s.total_albums as f64, 1));
        let tile = |icon, label: &str, value: String| StatTile { icon, label: t(label), value };
        let mut tiles = vec![tile("material-symbols:sell-outline-rounded", "stats.genres", intl::number(s.total_genres))];
        if s.total_size_bytes > 0 {
            tiles.push(tile("material-symbols:hard-drive-outline", "stats.size", intl::bytes(s.total_size_bytes as u64)));
        }
        tiles.push(tile("material-symbols:graphic-eq-rounded", "stats.avg_bitrate", format_bitrate(Some(s.avg_bitrate.round() as i32))));
        tiles.push(tile("material-symbols:play-circle-outline-rounded", "stats.plays", intl::number(s.total_play_count)));
        tiles.extend(average.map(|a| tile("material-symbols:timer-outline-rounded", "stats.avg_track", a)));
        tiles.extend(per_album.map(|n| tile("material-symbols:album-outline-rounded", "stats.per_album", n)));

        let quality_total = s.quality_hires + s.quality_lossless + s.quality_lossy;
        let quality = [("stats.quality_hires", s.quality_hires), ("stats.quality_lossless", s.quality_lossless), ("stats.quality_lossy", s.quality_lossy)]
            .into_iter()
            .enumerate()
            .filter(|(_, (_, n))| *n > 0)
            .map(|(tone, (key, n))| {
                let pct = n as f32 / quality_total.max(1) as f32 * 100.;
                Segment { tone, name: t(key), count: intl::number(n), share: share_label(n, quality_total), pct, labeled: pct >= LABEL_MIN_PCT }
            })
            .collect();

        let formats_total: i64 = s.formats.iter().map(|f| f.count).sum();
        let mut format_rows: Vec<(String, i64, Option<Msg>)> = s.formats.iter().take(FORMATS_SHOWN).map(|f| (f.name.clone(), f.count, None)).collect();
        let rest: i64 = s.formats.iter().skip(FORMATS_SHOWN).map(|f| f.count).sum();
        if rest > 0 {
            format_rows.push((t("stats.others"), rest, None));
        }
        let mut formats = ranked(format_rows, &|n| intl::number(n));
        for (bar, f) in formats.iter_mut().zip(s.formats.iter().take(FORMATS_SHOWN).map(|f| f.count).chain([rest])) {
            bar.share = Some(share_label(f, formats_total));
        }

        let genres = s.top_genres.iter().map(|g| (g.name.clone(), g.count, Some(Msg::Go(Route::Genre(g.name.clone()))))).collect();
        let artists = s
            .top_artists
            .iter()
            .map(|a| {
                let open = self.data.artists.iter().find(|x| x.name == a.name).map(|x| Msg::Go(Route::Artist(x.id.clone())));
                (a.name.clone(), a.count, open)
            })
            .collect();

        let max_played = s.top_played.first().map(|x| x.play_count).unwrap_or(1).max(1);
        Some(StatsView {
            color,
            kicker,
            duration: intl::duration(s.total_duration_sec),
            listen_plan: t("stats.listen_plan").replace("{n}", &intl::number(listening_days(s.total_duration_sec))),
            counts,
            tiles,
            quality,
            formats,
            top_genres: ranked(genres, &tracks_label),
            top_artists: ranked(artists, &tracks_label),
            top_played: s
                .top_played
                .iter()
                .enumerate()
                .map(|(i, p)| Played {
                    rank: i + 1,
                    title: p.title.clone(),
                    artist: p.artist.clone(),
                    cover: p.thumbnail_path.clone(),
                    count: t_with(if p.play_count > 1 { "stats.plays_n" } else { "stats.plays_one" }, &[("n", &p.play_count.to_string())]),
                    pct: p.play_count as f32 / max_played as f32 * 100.,
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shares_round_and_never_show_zero_for_a_present_part() {
        assert_eq!(share_label(9155, 9440), "97 %");
        assert_eq!(share_label(3, 9440), "< 1 %");
        assert_eq!(share_label(0, 9440), "0 %");
    }

    #[test]
    fn listening_days_count_two_hours_a_day() {
        assert_eq!(listening_days(7200.), 1);
        assert_eq!(listening_days(7201.), 2);
        assert_eq!(listening_days(25. * 86400. + 12. * 3600.), 306);
    }
}
