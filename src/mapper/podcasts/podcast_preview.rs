use crate::models::podcasts::*;
use crate::helper::date::parse_date;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;

/// Jour et mois courts d'un épisode (« 12 », « MARS »), sinon rien.
pub fn day_month(published: Option<&str>) -> Option<(String, String)> {
    let d = parse_date(published?)?;
    use chrono::Datelike;
    let month = crate::helper::date::month_short(d.month());
    Some((d.day().to_string(), month.trim_end_matches('.').to_uppercase()))
}

impl PodcastPreviewState {
    /// L'abonnement correspondant, s'il existe : (id).
    pub fn followed(&self, app: &MusicApp) -> Option<i64> {
        app.data.podcasts.iter().find(|p| p.feed_url == self.feed_url).map(|p| p.id)
    }

    /// `[kicker, genre]`.
    pub fn kicker(&self) -> String {
        [Some(t("podcasts.kicker_podcast")), self.genre.clone()].into_iter().flatten().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")
    }

    /// Date du dernier épisode.
    pub fn latest(&self) -> Option<String> {
        self.preview.as_ref().and_then(|p| p.episodes.first()).and_then(|e| e.published_at.clone()).or_else(|| self.released_at.clone())
    }

    /// Description en texte simple.
    pub fn description(&self) -> Option<String> {
        self.preview.as_ref()?.description.as_deref().map(crate::mapper::podcasts::html_text).filter(|d| !d.trim().is_empty())
    }

    pub fn site(&self) -> Option<String> {
        self.preview.as_ref()?.link.clone().filter(|l| l.starts_with("http://") || l.starts_with("https://"))
    }
}
