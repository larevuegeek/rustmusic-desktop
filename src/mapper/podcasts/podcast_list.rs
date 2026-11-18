//! Les podcasts : ce qu'affichent ses gabarits, bâti depuis l'état de la page.

use crate::models::podcasts::*;
use std::collections::HashMap;
use crate::models::app::Choice;
use crate::models::app::Msg;
use crate::core::podcast::directory::DirectoryResult;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use crate::helper::date::parse_date;
use karbon_ui::intl::{t, t_with};
use crate::mapper::albums::natural_cmp;
use crate::mapper::albums::sort_key;
use crate::mapper::radios::{country_name, COUNTRY_CODES};
use crate::entity::podcast::podcast::PodcastView;

impl Default for PodcastsPage {
    fn default() -> Self {
        Self {
            input: String::new(),
            results: None,
            search_term: false,
            added: None,
            latest: Vec::new(),
            genre: None,
            popular: None,
            popular_error: false,
            popular_for: (String::new(), None),
            all: false,
            sort: "recent",
            sort_popular: "rank",
            filtre: String::new(),
            verification: false,
            images: HashMap::new(),
        }
    }
}

pub(crate) const POPULAR_VISIBLE: usize = 15;

pub const GENRES: [(Option<u32>, &str); 12] = [
    (None, "all"), (Some(1489), "news"), (Some(1324), "society"), (Some(1487), "history"), (Some(1488), "true_crime"),
    (Some(1303), "comedy"), (Some(1533), "science"), (Some(1304), "education"), (Some(1321), "business"), (Some(1545), "sports"),
    (Some(1301), "arts"), (Some(1318), "technology"),
];

pub const SORTS: [(&str, &str, &str); 4] = [
    ("recent", "podcasts.sort_recent", "material-symbols:schedule-outline-rounded"),
    ("name", "podcasts.sort_name", "material-symbols:sort-by-alpha-rounded"),
    ("added", "podcasts.sort_added", "material-symbols:bookmark-added-outline-rounded"),
    ("new", "podcasts.sort_new", "material-symbols:fiber-new-outline-rounded"),
];

pub const POPULAR_SORTS: [(&str, &str, &str); 3] = [
    ("rank", "podcasts.sort_rank", "material-symbols:trophy-outline-rounded"),
    ("name", "podcasts.sort_name", "material-symbols:sort-by-alpha-rounded"),
    ("date", "podcasts.sort_recent", "material-symbols:schedule-outline-rounded"),
];

/// « 12 mars », « 12 mars 2024 » (autre année).
pub fn episode_date(published: Option<&str>) -> String {
    use crate::helper::date::{day_month, day_month_year};
    let Some(p) = published else { return String::new() };
    let Some(d) = parse_date(p) else { return String::new() };
    let local = d.with_timezone(&chrono::Local);
    let now = chrono::Local::now();
    use chrono::Datelike;
    if local.year() == now.year() {
        day_month(local.day(), local.month())
    } else {
        day_month_year(local.day(), local.month(), local.year())
    }
}

/// « 1 h 02 », « 34 min », « 45 s ».
pub fn episode_duration(seconds: f64) -> String {
    if seconds <= 0. {
        return String::new();
    }
    let s = seconds.round() as i64;
    let (h, m) = (s / 3600, (s % 3600) / 60);
    if h > 0 {
        format!("{h} h {m:02}")
    } else if m > 0 {
        format!("{m} min")
    } else {
        format!("{s} s")
    }
}

pub(crate) fn is_address(s: &str) -> bool {
    let s = s.trim();
    s.starts_with("http://") || s.starts_with("https://")
}

impl crate::models::podcasts::DirectoryItem {
    /// Ancre du menu « … » de la carte.
    pub fn more_id(&self) -> String {
        format!("dir.more.{}.{}", self.list, self.index)
    }
}

impl crate::models::podcasts::PodcastItem {
    /// Ancre du menu « … » de la carte.
    pub fn more_id(&self) -> String {
        format!("podcast.more.{}", self.id)
    }
}

impl crate::models::podcasts::EpisodeRow {
    /// Ancre du menu de l'épisode.
    pub fn menu_id(&self) -> String {
        format!("episode.{}", self.id)
    }
}

impl crate::entity::podcast::podcast::PodcastPreview {
    /// « 42 épisodes ».
    pub fn episodes_label(&self) -> String {
        let key = if self.episode_count == 1 { "podcasts.episodes_one" } else { "podcasts.episodes_n" };
        format!("{} {}", karbon_ui::intl::number(self.episode_count as i64), karbon_ui::intl::t(key))
    }
}

/// Les podcasts, favoris d'abord, puis par nom, date d'ajout, nouveautés ou dernier
/// épisode (par défaut).
pub(crate) fn sort_podcasts<'a>(podcasts: &'a [PodcastView], sort: &str) -> Vec<&'a PodcastView> {
    let mut l: Vec<&PodcastView> = podcasts.iter().collect();
    let by_name = |x: &&PodcastView, y: &&PodcastView| natural_cmp(&sort_key(&x.title), &sort_key(&y.title));
    let by_latest = |x: &&PodcastView, y: &&PodcastView| y.latest_at.cmp(&x.latest_at);
    l.sort_by(|x, y| {
        y.pinned.cmp(&x.pinned).then_with(|| match sort {
            "name" => by_name(x, y),
            "added" => y.created_at.cmp(&x.created_at).then(by_name(x, y)),
            "new" => y.new_count.cmp(&x.new_count).then(by_latest(x, y)),
            _ => by_latest(x, y).then(by_name(x, y)),
        })
    });
    l
}

impl MusicApp {
    /// Le nombre d'abonnements et son libellé : (« 3 », « émissions »).
    pub fn podcasts_count(&self) -> (String, String) {
        let n = self.data.podcasts.len();
        let key = if n == 1 { "podcasts.count_one" } else { "podcasts.count_n" };
        (karbon_ui::intl::number(n as i64), karbon_ui::intl::t(key))
    }

    pub(crate) fn podcast_country(&self) -> String {
        match self.data.setting("podcast_country") {
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

    pub(crate) fn podcast_item(&self, p: &PodcastView) -> PodcastItem {
        PodcastItem {
            id: p.id,
            title: p.title.clone(),
            author: p.author.clone().unwrap_or_default(),
            cover: p.cover_path.clone(),
            badge: (p.new_count > 0)
                .then(|| format!("{} {}", intl::number(p.new_count), t(if p.new_count == 1 { "podcasts.new_short_one" } else { "podcasts.new_short_n" }))),
            pinned: p.pinned,
        }
    }

    /// Les favoris d'abord, puis l'ordre choisi.
    pub(crate) fn podcasts_sorted(&self) -> Vec<&PodcastView> {
        sort_podcasts(&self.data.podcasts, self.views.podcasts.sort)
    }

    pub(crate) fn directory_item(&self, l: char, i: usize, r: &DirectoryResult, rank: Option<usize>) -> DirectoryItem {
        let page = &self.views.podcasts;
        DirectoryItem {
            key: r.feed_url.clone(),
            title: r.title.clone(),
            sub: r.author.clone().or_else(|| r.genre.clone()).unwrap_or_default(),
            image: r.image_url.as_ref().and_then(|u| page.images.get(u).cloned().flatten()),
            rank,
            tracked: self.data.podcasts.iter().any(|p| p.feed_url == r.feed_url),
            busy: page.added.as_deref() == Some(r.feed_url.as_str()),
            open: Msg::Podcasts(PodcastsMsg::Open(l, i)),
            follow: Msg::Podcasts(PodcastsMsg::Follow(l, i)),
            list: l,
            index: i,
        }
    }

    pub fn podcasts_view(&self) -> PodcastsView {
        let page = &self.views.podcasts;
        let podcasts = &self.data.podcasts;
        let new_items: i64 = podcasts.iter().map(|p| p.new_count).sum();
        let country = self.podcast_country();
        let name = country_name;
        let setting = self.data.setting("podcast_country").unwrap_or("");
        let mut choice = vec![Choice {
            label: t_with("podcasts.country_auto", &[("country", &name(&country))]),
            icon: crate::mapper::radios::flag(&country).to_string(),
            active: setting.is_empty(),
            msg: Msg::Podcasts(PodcastsMsg::Country("")),
            icon_only: false,
        }];
        choice.extend(COUNTRY_CODES.iter().map(|c| Choice {
            label: country_name(c),
            icon: crate::mapper::radios::flag(c).to_string(),
            active: setting == *c,
            msg: Msg::Podcasts(PodcastsMsg::Country(c)),
            icon_only: false,
        }));
        let country_label = choice.iter().find(|c| c.active).map(|c| c.label.clone()).unwrap_or_default();
        let (sort_label, sort_icon) = SORTS.iter().find(|x| x.0 == page.sort).map(|x| (t(x.1), x.2)).unwrap_or((t(SORTS[0].1), SORTS[0].2));
        let sorts = SORTS
            .iter()
            .map(|x| Choice { label: t(x.1), icon: x.2.into(), active: x.0 == page.sort, msg: Msg::Podcasts(PodcastsMsg::Sort(x.0)), icon_only: false })
            .collect();
        let (pop_label, pop_icon) = POPULAR_SORTS.iter().find(|x| x.0 == page.sort_popular).map(|x| (t(x.1), x.2)).unwrap_or((t(POPULAR_SORTS[0].1), POPULAR_SORTS[0].2));
        let pop_sorts = POPULAR_SORTS
            .iter()
            .map(|x| Choice { label: t(x.1), icon: x.2.into(), active: x.0 == page.sort_popular, msg: Msg::Podcasts(PodcastsMsg::PopularSort(x.0)), icon_only: false })
            .collect();
        let popular = page.popular.as_ref().map(|l| {
            let mut idx: Vec<usize> = (0..l.len()).collect();
            match page.sort_popular {
                "name" => idx.sort_by(|a, b| natural_cmp(&sort_key(&l[*a].title), &sort_key(&l[*b].title))),
                "date" => idx.sort_by(|a, b| l[*b].released_at.cmp(&l[*a].released_at).then(natural_cmp(&sort_key(&l[*a].title), &sort_key(&l[*b].title)))),
                _ => {}
            }
            idx.into_iter()
                .take(if page.all { usize::MAX } else { POPULAR_VISIBLE })
                .map(|i| self.directory_item('p', i, &l[i], Some(i + 1)))
                .collect()
        });
        let playing = self.data.player.path.as_deref();
        let latest = page
            .latest
            .iter()
            .map(|l| {
                let e = &l.episode;
                let duration = e.duration.unwrap_or(0) as f64;
                let starts = e.position > 0. && e.position < duration;
                let detail = if starts { t_with("podcasts.remaining", &[("t", &episode_duration(duration - e.position))]) } else { episode_duration(duration) };
                EpisodeCardItem {
                    id: e.id,
                    podcast_id: e.podcast_id,
                    podcast_title: l.podcast_title.clone(),
                    title: e.title.clone(),
                    cover: l.cover_path.clone(),
                    badge: (l.new_count > 0).then(|| if l.new_count > 1 { format!("{} {}", l.new_count, t("podcasts.new_short_n")) } else { t("podcasts.new_badge") }),
                    progress: starts.then(|| ((e.position / duration.max(1.)) * 100.).min(100.) as f32),
                    meta: [episode_date(e.published_at.as_deref()), detail].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · "),
                    playing: playing == Some(e.path.as_str()) && self.playing(),
                }
            })
            .collect();
        let q = page.filtre.trim().to_lowercase();
        let subscriptions = self
            .podcasts_sorted()
            .into_iter()
            .filter(|p| q.is_empty() || format!("{} {}", p.title, p.author.clone().unwrap_or_default()).to_lowercase().contains(&q))
            .map(|p| self.podcast_item(p))
            .collect();
        PodcastsView {
            active: self.data.setting("podcasts") == Some("true"),
            count: (!podcasts.is_empty()).then(|| {
                let n = podcasts.len();
                (
                    intl::number(n as i64),
                    t(if n == 1 { "podcasts.count_one" } else { "podcasts.count_n" }),
                    (new_items > 0).then(|| format!("{} {}", intl::number(new_items), t(if new_items == 1 { "podcasts.new_one" } else { "podcasts.new_n" }))),
                )
            }),
            input: page.input.clone(),
            address: is_address(&page.input),
            busy: page.search_term || page.added.as_deref() == Some(page.input.trim()),
            results: page.results.as_ref().map(|l| l.iter().enumerate().map(|(i, r)| self.directory_item('r', i, r, None)).collect()),
            latest,
            podcasts: self.podcasts_sorted().into_iter().map(|p| self.podcast_item(p)).collect(),
            loaded: true,
            sort_label,
            sort_icon,
            sorts,
            pop_label,
            pop_icon,
            pop_sorts,
            country_hint: t_with("podcasts.popular_hint", &[("country", &name(&country))]),
            country_label,
            country: choice,
            genres: GENRES.iter().map(|(id, k)| (*id, t(&format!("podcasts.genre_{k}")), page.genre == *id)).collect(),
            popular,
            popular_error: page.popular_error,
            more: page
                .popular
                .as_ref()
                .filter(|l| !page.all && l.len() > POPULAR_VISIBLE)
                .map(|l| t_with("podcasts.show_more_popular", &[("n", &(l.len() - POPULAR_VISIBLE).to_string())])),
            verification: page.verification,
            filtre: page.filtre.clone(),
            filtre_placeholder: t_with("podcasts.filter", &[("n", &intl::number(podcasts.len() as i64))]),
            subscriptions,
        }
    }

    pub(crate) fn directory_at(&self, l: char, i: usize) -> Option<DirectoryResult> {
        let page = &self.views.podcasts;
        match l {
            'r' => page.results.as_ref()?.get(i).cloned(),
            _ => page.popular.as_ref()?.get(i).cloned(),
        }
    }

    pub(crate) fn podcasts_covers(&self) -> Vec<String> {
        let v = self.podcasts_view();
        let mut paths: Vec<String> = Vec::new();
        for it in v.results.iter().flatten().chain(v.popular.iter().flatten()) {
            paths.extend(it.image.clone());
        }
        paths.extend(v.latest.iter().filter_map(|e| e.cover.clone()));
        paths.extend(v.podcasts.iter().filter_map(|p| p.cover.clone()));
        paths
    }
}
