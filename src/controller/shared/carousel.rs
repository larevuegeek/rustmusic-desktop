//! Les carrousels : une zone qui défile à l'horizontale ; les flèches la font glisser d'une
//! page de cartes, calée sur le bord d'une carte.

use karbon_ui::ui::scroll::{ScrollMetrics, ScrollRequest};
use crate::app::music_app::MusicApp;
use crate::models::shared::{CarouselGeometry, CarouselScroll};

/// Écart entre deux cartes (`gap-4.5`).
const GAP: f32 = 18.;

/// Décalage visé par une flèche : une page de cartes entières en avant ou en arrière,
/// depuis `base` calé sur une carte. `count` : nombre de cartes.
pub(crate) fn slide_target(g: CarouselGeometry, base: f32, forward: bool, count: usize) -> f32 {
    if count == 0 || g.extent <= 0. {
        return base;
    }
    // Pas d'une carte, écart compris.
    let pitch = (g.extent + GAP) / count as f32;
    let page = ((g.viewport + GAP) / pitch).floor().max(1.) * pitch;
    let snapped = (base / pitch).round() * pitch;
    let target = if forward { snapped + page } else { snapped - page };
    target.clamp(0., g.max)
}

impl CarouselGeometry {
    /// La géométrie d'un événement de la zone.
    pub fn of(s: ScrollMetrics) -> Self {
        Self { offset: s.offset, max: s.max_offset(), viewport: s.viewport_size(), extent: s.extent }
    }
}

impl CarouselScroll {
    /// Décalage qui décide des flèches : la cible d'un glissement en cours, sinon la position.
    fn settled_offset(&self) -> f32 {
        match self.request {
            Some(r) if self.gliding => r.offset,
            _ => self.geometry.offset,
        }
    }

    /// La zone n'est pas au début.
    pub fn can_go_back(&self) -> bool {
        self.settled_offset() > 1.
    }

    /// Des cartes dépassent à droite.
    pub fn can_go_forward(&self) -> bool {
        self.geometry.max - self.settled_offset() > 1.
    }
}

impl MusicApp {
    /// Une flèche : glisse d'une page. Un clic pendant un glissement part de sa cible.
    pub(crate) fn carousel_slide(&mut self, id: &'static str, forward: bool, count: usize) -> bool {
        self.views.carousel_token += 1;
        let token = self.views.carousel_token;
        let state = self.views.carousels.entry(id).or_default();
        let base = state.settled_offset();
        let offset = slide_target(state.geometry, base, forward, count);
        state.glide(token, offset);
        true
    }

    /// Ramène un carrousel au début, d'un coup (liste retriée).
    pub(crate) fn carousel_rewind(&mut self, id: &'static str) {
        self.views.carousel_token += 1;
        let token = self.views.carousel_token;
        let state = self.views.carousels.entry(id).or_default();
        state.request = Some(ScrollRequest { token, offset: 0., smooth: false });
        state.gliding = false;
    }

    /// Géométrie signalée par la zone ; la vue n'est rebâtie que si une flèche change d'état,
    /// jamais pendant le glissement d'une flèche (son état est déjà celui de l'arrivée).
    pub(crate) fn carousel_scrolled(&mut self, id: &'static str, geometry: CarouselGeometry) -> bool {
        self.views.carousels.entry(id).or_default().scrolled(geometry)
    }
}

impl CarouselScroll {
    /// Une flèche lance un glissement vers `offset` (jeton `token`).
    pub(crate) fn glide(&mut self, token: u64, offset: f32) {
        self.request = Some(ScrollRequest { token, offset, smooth: true });
        self.gliding = (offset - self.geometry.offset).abs() > 1.;
    }

    /// Nouvelle géométrie ; `true` si une flèche change d'état.
    pub(crate) fn scrolled(&mut self, geometry: CarouselGeometry) -> bool {
        let before = (self.can_go_back(), self.can_go_forward());
        if let (true, Some(r)) = (self.gliding, self.request) {
            let arrived = (geometry.offset - r.offset).abs() <= 1.;
            // La molette pendant le glissement le détourne : la position reprend la main.
            let away = (geometry.offset - self.geometry.offset) * (r.offset - self.geometry.offset) < 0.;
            self.gliding = !(arrived || away);
        }
        self.geometry = geometry;
        before != (self.can_go_back(), self.can_go_forward())
    }
}
