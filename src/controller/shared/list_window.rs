//! La fenêtre glissante des listes en mémoire : elle grandit par pas à l'approche du bas,
//! recule d'une tranche vers le haut et saute à un élément (lettre).

use std::ops::Range;
use crate::app::music_app::MusicApp;
use crate::models::shared::ListWindow;

/// Éléments ajoutés quand on approche du bas, ou rendus quand on remonte.
pub(crate) const SLICE: usize = 48;

/// Éléments ajoutés par image quand la fenêtre grandit : une tranche entière d'un coup
/// dépasse le temps d'une image.
pub(crate) const STEP: usize = 12;

/// Éléments gardés au-dessus de l'élément visé (une rangée ou deux de grille).
pub(crate) const CONTEXT: usize = 12;

/// À moins de 600 px d'un bord de la zone, la tranche voisine se charge.
pub(crate) const NEAR: f32 = 600.;

/// Capacité des listes de cartes : au-delà, les passes sur tout l'arbre coûtent une image.
pub(crate) const CARDS: usize = 96;

impl Default for ListWindow {
    fn default() -> Self {
        Self::new(CARDS)
    }
}

impl ListWindow {
    pub(crate) fn new(capacity: usize) -> Self {
        Self { capacity, start: 0, end: capacity, target_end: 0, view: 0, target: None }
    }

    /// Une nouvelle zone qui repart du haut (filtres, tri, autre liste).
    pub(crate) fn reset(&mut self) {
        self.start = 0;
        self.end = self.capacity;
        self.target_end = 0;
        self.target = None;
        self.view += 1;
    }

    /// La fenêtre est en train de grandir (`Application::has_frame_work`).
    pub(crate) fn growing(&self) -> bool {
        self.end < self.target_end
    }

    /// Agrandit la fenêtre d'au plus `step` éléments vers `target_end` ; `true` si la vue
    /// change. Le haut ne recule qu'à la fin et d'une tranche entière : une grille garde
    /// ses colonnes.
    pub(crate) fn advance(&mut self, step: usize) -> bool {
        if self.end >= self.target_end {
            return false;
        }
        self.end = (self.end + step).min(self.target_end);
        if self.end == self.target_end {
            self.target_end = 0;
            if self.end - self.start > self.capacity {
                self.start += SLICE;
            }
        }
        true
    }

    /// Une tranche de plus en bas, par pas (`at_once` : toute la tranche, pour l'export).
    pub(crate) fn more(&mut self, len: usize, at_once: bool) -> bool {
        if self.end >= len || self.target_end > 0 {
            return false;
        }
        self.target_end = (self.end + SLICE).min(len);
        self.advance(if at_once { SLICE } else { STEP });
        true
    }

    /// Rend la tranche d'avant, en haut.
    pub(crate) fn previous(&mut self) -> bool {
        if self.start == 0 {
            return false;
        }
        self.start = self.start.saturating_sub(SLICE);
        self.end = self.end.min(self.start + self.capacity);
        self.target_end = 0;
        true
    }

    /// Une nouvelle zone qui commence un peu avant `index` et l'amène en haut.
    pub(crate) fn jump(&mut self, index: usize) {
        self.start = index.saturating_sub(CONTEXT);
        self.end = self.start + self.capacity;
        self.target_end = 0;
        self.target = Some(index);
        self.view += 1;
    }

    /// Défilement de la zone (`offset` sur `max`) : la tranche voisine à l'approche d'un bord.
    pub(crate) fn scrolled(&mut self, offset: f32, max: f32, len: usize, at_once: bool) -> bool {
        if max - offset < NEAR && self.end < len {
            return self.more(len, at_once);
        }
        if self.start > 0 && offset < NEAR {
            return self.previous();
        }
        false
    }

    /// Les éléments à bâtir d'une liste de `len` éléments. Une liste devenue plus courte
    /// que le haut de la fenêtre montre sa fin.
    pub(crate) fn range(&self, len: usize) -> Range<usize> {
        if self.start >= len {
            return len.saturating_sub(self.capacity)..len;
        }
        self.start..self.end.clamp(self.start, len)
    }

    /// Rang de l'élément visé dans la fenêtre.
    pub(crate) fn target_slot(&self) -> Option<usize> {
        self.target.and_then(|i| i.checked_sub(self.start))
    }
}

impl MusicApp {
    /// Les fenêtres des listes ouvertes : une page de plus, une ligne de plus.
    fn list_windows(&self) -> impl Iterator<Item = &ListWindow> {
        let v = &self.views;
        [
            v.albums.as_ref().map(|p| &p.window),
            v.artists.as_ref().map(|p| &p.window),
            v.genres.as_ref().map(|p| &p.window),
            v.genre.as_ref().map(|p| &p.window),
            v.folders.as_ref().map(|p| &p.window),
            v.show.as_ref().map(|p| &p.window),
            Some(&v.queue_window),
        ]
        .into_iter()
        .flatten()
    }

    /// Une fenêtre grandit encore (`Application::has_frame_work`).
    pub(crate) fn lists_growing(&self) -> bool {
        self.list_windows().any(ListWindow::growing)
    }

    /// Un pas pour chaque fenêtre qui grandit (`Application::frame_work`) ; `true` si une
    /// vue change.
    pub(crate) fn lists_frame_work(&mut self) -> bool {
        let v = &mut self.views;
        [
            v.albums.as_mut().map(|p| &mut p.window),
            v.artists.as_mut().map(|p| &mut p.window),
            v.genres.as_mut().map(|p| &mut p.window),
            v.genre.as_mut().map(|p| &mut p.window),
            v.folders.as_mut().map(|p| &mut p.window),
            v.show.as_mut().map(|p| &mut p.window),
            Some(&mut v.queue_window),
        ]
        .into_iter()
        .flatten()
        .fold(false, |changed, w| w.advance(STEP) | changed)
    }
}
