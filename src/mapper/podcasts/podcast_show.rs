use crate::models::podcasts::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Menu;
use crate::mapper::podcasts::episode_date;
use crate::mapper::podcasts::episode_duration;
use crate::entity::podcast::podcast::EpisodeView;

/// Épisodes gardés au plus dans la fenêtre (longues archives).
pub(crate) const EPISODES: usize = 60;

/// Le texte d'une description HTML : balises retirées, entités courantes décodées.
pub fn html_text(html: &str) -> String {
    let mut out = String::new();
    let mut inside_tag = false;
    for c in html.chars() {
        match c {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                out.push(' ');
            }
            _ if !inside_tag => out.push(c),
            _ => {}
        }
    }
    let out = out.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&rsquo;", "’");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// « mars 2024 » (mois long et année).
pub(crate) fn month_title(key: &str) -> String {
    let year = key.get(..4).and_then(|y| y.parse::<i32>().ok());
    let month = key.get(5..7).and_then(|m| m.parse::<u32>().ok()).filter(|m| (1..=12).contains(m));
    let (Some(y), Some(m)) = (year, month) else { return t("podcasts.undated") };
    crate::helper::date::month_year(m, y)
}

impl MusicApp {
    pub(crate) fn show_visible(&self) -> Vec<&EpisodeView> {
        let Some(page) = self.views.show.as_ref() else { return Vec::new() };
        let mut l: Vec<&EpisodeView> = page.episodes.iter().filter(|e| !page.unplayed_only || !e.played).collect();
        if page.older {
            l.reverse();
        }
        l
    }


    pub(crate) fn show_to_listen(&self) -> Option<&EpisodeView> {
        let page = self.views.show.as_ref()?;
        page.episodes
            .iter()
            .find(|e| !e.played && e.position > 0.)
            .or_else(|| page.episodes.iter().find(|e| !e.played))
            .or(page.episodes.first())
    }

    pub(crate) fn episode_state(&self, e: &EpisodeView) -> Option<bool> {
        (self.data.player.path.as_deref() == Some(e.path.as_str())).then(|| self.playing())
    }

    pub fn show_view(&self) -> Option<ShowView> {
        let page = self.views.show.as_ref()?;
        let p = page.podcast.as_ref()?;
        let unplayed = page.episodes.iter().filter(|e| !e.played).count();
        let to_listen = self.show_to_listen();
        let resume = to_listen.filter(|e| !e.played && e.position > 0. && e.duration.is_none_or(|d| e.position < d as f64)).map(|e| {
            let d = e.duration.unwrap_or(0) as f64;
            let progress_pct = (e.position / if d > 0. { d } else { e.position }).min(1.) as f32 * 100.;
            (e.title.clone(), progress_pct, t_with("podcasts.remaining", &[("t", &episode_duration(d - e.position))]), self.episode_state(e) == Some(true))
        });
        let visible = self.show_visible();
        let mut months: Vec<(String, String, Vec<EpisodeRow>)> = Vec::new();
        for e in &visible[page.window.range(visible.len())] {
            let key = e.published_at.as_deref().and_then(|d| d.get(..7)).unwrap_or("").to_string();
            if months.last().is_none_or(|(c, ..)| *c != key) {
                months.push((key.clone(), if key.is_empty() { t("podcasts.undated") } else { month_title(&key) }, Vec::new()));
            }
            let d = e.duration.unwrap_or(0) as f64;
            let in_progress = !e.played && e.position > 0. && e.position < d;
            months.last_mut().unwrap().2.push(EpisodeRow {
                id: e.id,
                new_one: e.is_new && !e.played,
                date: episode_date(e.published_at.as_deref()),
                season: e.season.zip(e.episode).map(|(s, n)| t_with("podcasts.season_episode", &[("s", &s.to_string()), ("e", &n.to_string())])),
                progress: in_progress.then(|| (((e.position / d) * 100.).min(100.) as f32, t_with("podcasts.remaining", &[("t", &episode_duration(d - e.position))]))),
                duration: episode_duration(d),
                played: e.played,
                downloaded: e.file_path.is_some(),
                title: e.title.clone(),
                summary: e.summary.clone().filter(|s| !s.trim().is_empty()).map(|s| html_text(&s)),
                open: page.open_items.iter().find(|(i, _)| *i == e.id).map(|(_, d)| d.as_deref().map(html_text)),
                playback: self.episode_state(e),
                download: page.downloads.contains(&e.id),
                menu: page.menu_episode == Some(e.id) && self.menu == Some(Menu::Popup("podcast.episode")),
            });
        }
        Some(ShowView {
            title: p.title.clone(),
            title_long: p.title.chars().count() > 32,
            author: p.author.clone().filter(|a| !a.is_empty()),
            cover: p.cover_path.clone(),
            episodes_n: format!("{} {}", intl::number(p.episode_count), t(if p.episode_count == 1 { "podcasts.episodes_one" } else { "podcasts.episodes_n" })),
            unplayed: (unplayed > 0).then(|| t_with(if unplayed == 1 { "podcasts.unplayed_one" } else { "podcasts.unplayed_n" }, &[("n", &intl::number(unplayed as i64))])),
            latest: p.latest_at.as_deref().map(|d| t_with("podcasts.latest_on", &[("date", &episode_date(Some(d)))])),
            to_listen: to_listen.map(|e| e.id),
            resume,
            pinned: p.pinned,
            auto_download: p.auto_download,
            has_unplayed: unplayed > 0,
            description: p.description.clone().filter(|d| !d.trim().is_empty()).map(|d| html_text(&d)),
            description_open: page.description,
            months: months.into_iter().map(|(_, title, l)| (title, l)).collect(),
            empty: visible.is_empty(),
            older: page.older,
            unplayed_only: page.unplayed_only,
            verification: page.verification,
            menu: self.menu == Some(Menu::Popup("podcast.subscription")),
            confirm: page.confirm,
        })
    }
}
