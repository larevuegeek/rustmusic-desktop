//! La page des morceaux : chargement par fenêtres, lettres, tri, recherche et réponse aux messages.

use crate::models::tracks::*;
use std::time::Duration;
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use crate::controller::tracks::track_rows::to_queue;
use crate::commands::library::library_command;

/// Au-delà, on lâche l'autre bout de la fenêtre chargée. À
/// 500 lignes (≈ 33 000 éléments), les passes de karbon-ui qui parcourent tout l'arbre à
/// chaque image (peinture des éléments `fixed`, ombres, faits du pointeur) coûtent ≈ 17 ms
/// par image de défilement ; 200 gardent le défilement sous 4 ms.
pub(crate) const WINDOW: usize = 200;

/// Taille d'une page.
pub(crate) const PAGE: i64 = 100;

/// Lignes d'un lot ajoutées par image : un lot de 100 d'un seul coup dépassait le temps
/// d'une image (~25 ms) et l'écran sautait un affichage.
pub(crate) const SLICE: usize = 10;

/// À moins de 400 px d'un bord de la zone, la page voisine se charge.
pub(crate) const NEAR: f32 = 400.;

/// « Tout lire » : la liste telle qu'affichée, plafonnée.
pub(crate) const PLAYBACK_CEILING: i64 = 1000;

impl TracksPage {
    /// Fait entrer au plus `how_many` lignes du lot reçu dans la fenêtre ; `true` si la
    /// vue change.
    pub(crate) fn absorber(&mut self, how_many: usize) -> bool {
        if self.a_venir.is_empty() {
            return false;
        }
        let n = how_many.min(self.a_venir.len());
        match self.upcoming_start {
            // Vers le haut : la fin du lot touche la fenêtre.
            Some(start_point) => {
                let start = self.a_venir.len() - n;
                let slice = self.a_venir.split_off(start);
                self.tracks.splice(0..0, slice);
                self.tracks.truncate(WINDOW);
                self.start = if self.a_venir.is_empty() { start_point } else { self.start - n as i64 };
            }
            None => {
                self.tracks.extend(self.a_venir.drain(..n));
                let excess = self.tracks.len().saturating_sub(WINDOW);
                self.tracks.drain(..excess);
                self.start += excess as i64;
            }
        }
        if self.a_venir.is_empty() {
            self.following = false;
        }
        true
    }
}

impl MusicApp {
    /// Ouvre la page (ou la retrouve) et charge sa liste.
    pub(crate) fn tracks_open(&mut self) {
        let Some(library) = self.data.library_id else { return };
        let keep = self.views.tracks.as_ref().is_some_and(|p| p.library_id == library);
        if !keep {
            self.views.tracks = Some(TracksPage::new(library));
        }
        self.tracks_load(0, None);
        self.tracks_load_letters();
    }

    /// Recharge une fenêtre qui commence à `start_point` ; `anchor` = index absolu à amener en haut.
    pub(crate) fn tracks_load(&mut self, start_point: i64, anchor: Option<i64>) {
        let Some(page) = self.views.tracks.as_mut() else { return };
        page.token += 1;
        page.loading_state = true;
        if !page.a_venir.is_empty() {
            page.a_venir.clear();
            page.following = false;
        }
        let token = page.token;
        let p = page.params();
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                library_command::get_tracks_paginated(
                    app.state(), p.library, start_point, PAGE, p.sort_by, p.sort_dir, p.filter, p.missing_cover, p.quality,
                    p.favorites, p.genre,
                )
                .await
            },
            move |me, result| {
                let Some(page) = me.views.tracks.as_mut().filter(|p| p.token == token) else { return false };
                page.loading_state = false;
                match result {
                    Ok(r) => {
                        page.tracks = r.tracks;
                        page.total = r.total;
                        page.start = start_point;
                        page.anchor = anchor;
                        page.view += 1;
                    }
                    Err(e) => log::error!("[morceaux] chargement : {e}"),
                }
                true
            },
        );
    }

    pub(crate) fn tracks_load_letters(&mut self) {
        let Some(page) = self.views.tracks.as_mut() else { return };
        if page.sort != "title" {
            page.letters.clear();
            return;
        }
        let token = page.token;
        let p = page.params();
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                library_command::get_track_letter_offsets(
                    app.state(), p.library, p.sort_dir, p.filter, p.missing_cover, p.quality, p.favorites, p.genre,
                )
                .await
            },
            move |me, result| {
                let Some(page) = me.views.tracks.as_mut().filter(|p| p.token == token) else { return false };
                page.letters = result
                    .map(|l| l.into_iter().map(|x| (x.letter, x.offset)).collect())
                    .unwrap_or_default();
                true
            },
        );
    }

    pub(crate) fn tracks_reload(&mut self) {
        self.tracks_load(0, None);
        self.tracks_load_letters();
    }

    /// « Tout lire » de l'en-tête : la liste filtrée et triée, plafonnée.
    pub(crate) fn tracks_play_all(&mut self) {
        let Some(page) = self.views.tracks.as_ref() else { return };
        let p = page.params();
        let profile = self.data.profile_id().unwrap_or(0);
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                library_command::get_tracks_paginated(
                    app.state(), p.library, 0, PLAYBACK_CEILING, p.sort_by, p.sort_dir, p.filter, p.missing_cover,
                    p.quality, p.favorites, p.genre,
                )
                .await
            },
            move |me, result| {
                if let Ok(r) = result {
                    if let Some(first) = r.tracks.first().map(|t| t.path.clone()) {
                        me.play_list(to_queue(profile, &r.tracks), &first);
                    }
                }
                true
            },
        );
    }

    /// Saisie de la recherche : envoyée après 300 ms sans frappe, à partir de deux caractères.
    pub(crate) fn tracks_search(&mut self, value: String) -> bool {
        let Some(page) = self.views.tracks.as_mut() else { return false };
        page.search = value;
        page.search_gen += 1;
        let n = page.search_gen;
        self.backend.call(async { tokio::time::sleep(Duration::from_millis(300)).await }, move |me, ()| {
            me.handle(Msg::Tracks(TracksMsg::SearchSettled(n)))
        });
        true
    }

    /// Tri par une colonne de l'en-tête : le même inverse, un autre repart croissant.
    pub(crate) fn tracks_sort_column(&mut self, key: String) -> bool {
        let Some(page) = self.views.tracks.as_mut() else { return false };
        page.asc = !(page.sort == key && page.asc);
        page.sort = key;
        self.tracks_reload();
        true
    }

    pub(crate) fn tracks_update(&mut self, msg: TracksMsg) -> bool {
        let Some(page) = self.views.tracks.as_mut() else { return false };
        let mut reload = true;
        match msg {
            TracksMsg::Sort(key) => {
                page.sort = key;
                page.asc = true;
                self.menu = None;
            }
            TracksMsg::Reverse => {
                page.asc = !page.asc;
                self.menu = None;
            }
            TracksMsg::Detailed(on) => {
                page.detailed = on;
                reload = false;
            }
            TracksMsg::Hires => page.hires = !page.hires,
            TracksMsg::Lossless => page.lossless = !page.lossless,
            TracksMsg::Favorites => page.favorites = !page.favorites,
            TracksMsg::NoCover => page.without_cover = !page.without_cover,
            TracksMsg::GenreMenu => {
                reload = false;
                let need = page.genres.is_none();
                let id = page.library_id;
                let open = self.popup("tracks.genre");
                self.menu = if open { None } else { Some(crate::models::app::Menu::Popup("tracks.genre")) };
                if need {
                    let app = self.backend.app.clone();
                    self.backend.call(
                        async move { library_command::get_genres(app.state(), id).await },
                        |me, genres| {
                            if let Some(page) = me.views.tracks.as_mut() {
                                let mut g = genres.unwrap_or_default();
                                g.sort_by(|a, b| b.total_tracks.cmp(&a.total_tracks));
                                page.genres = Some(g);
                            }
                            true
                        },
                    );
                }
            }
            TracksMsg::Genre(g) => {
                page.genre = g;
                self.menu = None;
            }
            TracksMsg::ClearFilters => {
                page.search.clear();
                page.sent.clear();
                page.hires = false;
                page.lossless = false;
                page.favorites = false;
                page.without_cover = false;
                page.genre = None;
            }
            TracksMsg::SearchSettled(n) => {
                if n != page.search_gen {
                    return false;
                }
                let q = page.search.trim().to_string();
                let next = if q.chars().count() >= 2 { q } else { String::new() };
                if next == page.sent {
                    return false;
                }
                page.sent = next;
            }
            TracksMsg::LoadNext => {
                if page.following || page.start + page.tracks.len() as i64 >= page.total {
                    return false;
                }
                page.following = true;
                let token = page.token;
                let offset = page.start + page.tracks.len() as i64;
                let p = page.params();
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        library_command::get_tracks_paginated(
                            app.state(), p.library, offset, PAGE, p.sort_by, p.sort_dir, p.filter, p.missing_cover,
                            p.quality, p.favorites, p.genre,
                        )
                        .await
                    },
                    move |me, result| {
                        let spread = !me.export;
                        let Some(page) = me.views.tracks.as_mut().filter(|p| p.token == token) else { return false };
                        page.following = false;
                        if let Ok(r) = result {
                            // Deux pages qui se chevauchent (ex æquo du tri) : une ligne par id.
                            let seen: std::collections::HashSet<String> = page.tracks.iter().map(|t| t.id.clone()).collect();
                            page.a_venir = r.tracks.into_iter().filter(|t| !seen.contains(&t.id)).collect();
                            page.upcoming_start = None;
                            page.following = !page.a_venir.is_empty();
                            page.absorber(if spread { SLICE } else { usize::MAX });
                            return true;
                        }
                        true
                    },
                );
                return true;
            }
            TracksMsg::LoadPrevious => {
                if page.following || page.start == 0 {
                    return false;
                }
                let start_point = (page.start - PAGE).max(0);
                let limit = page.start - start_point;
                page.following = true;
                let token = page.token;
                let p = page.params();
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        library_command::get_tracks_paginated(
                            app.state(), p.library, start_point, limit, p.sort_by, p.sort_dir, p.filter, p.missing_cover,
                            p.quality, p.favorites, p.genre,
                        )
                        .await
                    },
                    move |me, result| {
                        let spread = !me.export;
                        let Some(page) = me.views.tracks.as_mut().filter(|p| p.token == token) else { return false };
                        page.following = false;
                        if let Ok(r) = result {
                            let seen: std::collections::HashSet<String> = page.tracks.iter().map(|t| t.id.clone()).collect();
                            page.a_venir = r.tracks.into_iter().filter(|t| !seen.contains(&t.id)).collect();
                            page.upcoming_start = Some(start_point);
                            page.following = !page.a_venir.is_empty();
                            page.absorber(if spread { SLICE } else { usize::MAX });
                            return true;
                        }
                        true
                    },
                );
                return true;
            }
            TracksMsg::Letter(l) => {
                let Some(i) = page.letters.iter().find(|(x, _)| *x == l).map(|(_, o)| *o) else { return false };
                if i >= page.start && i < page.start + page.tracks.len() as i64 {
                    page.anchor = Some(i);
                    page.view += 1;
                } else {
                    // Un peu de contexte au-dessus : remonter d'une ligne recharge la suite.
                    self.tracks_load((i - 20).max(0), Some(i));
                }
                return true;
            }
            TracksMsg::Scrolled(offset, max) => {
                if page.following || page.loading_state {
                    return false;
                }
                if page.start + (page.tracks.len() as i64) < page.total && max - offset < NEAR {
                    return self.tracks_update(TracksMsg::LoadNext);
                }
                if page.start > 0 && offset < NEAR {
                    return self.tracks_update(TracksMsg::LoadPrevious);
                }
                return false;
            }
        }
        if reload {
            self.tracks_reload();
        }
        true
    }

    /// Une tranche du lot reçu : `Application::frame_work`.
    pub fn tracks_frame_work(&mut self) -> bool {
        self.views.tracks.as_mut().is_some_and(|p| p.absorber(SLICE))
    }
}
