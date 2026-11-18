//! Les variables de thème calculées (teinte, accent, contraste).

use karbon_ui::ui::prelude::*;
use crate::app::resources::assets::Assets;
use crate::models::home::ViewModel;
use super::*;

impl MusicApp {
    /// Variables du thème pour cet état, plus la teinte de la pochette du morceau chargé
    /// (`--lh`, `--ls` appliqués aux jetons du lecteur, comme
    /// `app.css` : les dégradés et les ombres les lisent à la construction de la vue).
    pub(crate) fn theme_variables(&self, assets: &Assets, vm: &ViewModel, dark: bool, contrast: bool) -> CssVariables {
        let (h, s) = vm
            .details
            .cover_path
            .as_deref()
            .and_then(|p| assets.covers.get(p))
            .map(|image| image.dominant_hue())
            .unwrap_or((150., 0.));
        let accent = super::accent::custom(self.data.setting("accent_color"));
        let key = (dark, contrast, h.to_bits(), s.to_bits(), accent);
        let mut cache = self.theme_cache.borrow_mut();
        if let Some((k, variables)) = cache.as_ref() {
            if *k == key {
                return variables.clone();
            }
        }
        let mut v = crate::app::resources::css::root_variables(dark, contrast).clone();
        let mut set = |name: &str, c: Color| {
            v.set(name, VariableValue::Color(c));
        };
        let k = |l: f32, c: f32, a: f32| Color::oklch(l, c * s, h, a);
        if dark {
            set("--lc-acc", k(0.8, 0.12, 1.));
            set("--lc-play", k(0.8, 0.12, 1.));
            set("--lc-play-tx", k(0.18, 0.04, 1.));
            set("--lc-fmt-bg", k(0.32, 0.07, 1.));
            set("--lc-fmt-tx", k(0.92, 0.07, 1.));
            set("--lc-statut", k(0.1, 0.015, 0.45));
            set("--lc-statut-bd", k(0.4, 0.05, 0.25));
            set("--lc-fond-1", k(0.3, 0.07, 0.95));
            set("--lc-fond-2", k(0.18, 0.03, 0.96));
            set("--lc-fond-3", k(0.13, 0.012, 0.97));
            set("--lc-ombre", k(0.2, 0.06, 0.6));
            set("--lc-reflet", k(0.7, 0.08, 0.18));
            set("--lc-lisere", k(0.4, 0.06, 0.35));
            set("--lc-encours", k(0.35, 0.07, 0.35));
            set("--mf-1", k(0.3, 0.07, 0.97));
            set("--mf-2", k(0.17, 0.03, 0.98));
            set("--mf-3", k(0.12, 0.012, 1.));
        } else {
            set("--lc-acc", k(0.5, 0.14, 1.));
            set("--lc-play", k(0.55, 0.15, 1.));
            set("--lc-fmt-bg", k(0.87, 0.06, 1.));
            set("--lc-fmt-tx", k(0.36, 0.1, 1.));
            set("--lc-statut", k(0.93, 0.03, 0.7));
            set("--lc-statut-bd", k(0.8, 0.04, 0.5));
            set("--lc-fond-1", k(0.9, 0.06, 1.));
            set("--lc-fond-2", k(0.95, 0.03, 1.));
            set("--lc-fond-3", k(0.975, 0.012, 1.));
            set("--lc-ombre", k(0.45, 0.06, 0.16));
            set("--lc-lisere", k(0.78, 0.05, 0.6));
            set("--lc-encours", k(0.86, 0.06, 0.6));
            set("--mf-1", k(0.88, 0.07, 1.));
            set("--mf-2", k(0.95, 0.03, 1.));
            set("--mf-3", k(0.975, 0.012, 1.));
        }
        set("--lc-pochette-ombre", k(0.25, 0.08, 0.55));
        set("--lc-pochette-lisere", k(0.6, 0.08, 0.25));
        set("--lc-lecture-ombre", k(0.55, 0.13, 0.4));
        // Couleur d'accent choisie : sa palette et les verts du thème qui la suivent.
        if let Some(rgb) = accent {
            for (name, color) in super::accent::variables(rgb, dark) {
                set(name.as_str(), color);
            }
        }
        *cache = Some((key, v.clone()));
        v
    }
}
