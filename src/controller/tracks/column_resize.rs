//! Largeur des colonnes du tableau des morceaux : poignée tirée dans l'en-tête, double-clic
//! pour ajuster au contenu.

use std::collections::HashMap;
use karbon_ui::intl;
use karbon_ui::ui::drag_drop::{PanEvent, PanPhase};
use crate::app::music_app::MusicApp;
use crate::mapper::tracks::{fitted_width, resized_width, resolve, tag_cell};
use crate::models::tracks::{ColumnMenu, ColumnResize, TableColumn, TrackMenuAt, Widget};

/// La colonne d'une clé (bâtie ou tag).
fn column(key: &str) -> Option<TableColumn> {
    resolve(&[key.to_string()]).into_iter().next()
}

impl MusicApp {
    /// La poignée d'une colonne est tirée : la colonne suit le pointeur et sa largeur est
    /// enregistrée au relâchement.
    pub(crate) fn column_resize(&mut self, key: String, event: PanEvent) -> bool {
        let Some(col) = column(&key) else {
            return false;
        };
        match event.phase {
            PanPhase::Start => {
                // La colonne part de sa largeur à l'écran (le titre automatique compris) : pas
                // d'à-coup au premier mouvement.
                let start = if event.parent_width > 0. {
                    event.parent_width
                } else {
                    self.column_widths().get(&key).copied().unwrap_or(col.width)
                };
                let width = resized_width(&col, start, event.dx);
                self.column_resize = Some(ColumnResize { key, start, width });
            }
            PanPhase::Move => {
                let Some(r) = self.column_resize.as_mut().filter(|r| r.key == key) else {
                    return false;
                };
                let width = resized_width(&col, r.start, event.dx);
                if width == r.width {
                    return false;
                }
                r.width = width;
            }
            PanPhase::End => {
                let Some(r) = self.column_resize.take() else {
                    return false;
                };
                let mut widths = self.saved_column_widths();
                widths.insert(r.key, r.width);
                self.save_column_widths(&widths);
            }
        }
        true
    }

    /// La colonne s'ajuste aux textes de la liste affichée ; le titre revient à la largeur
    /// automatique.
    pub(crate) fn column_fit(&mut self, key: &str) -> bool {
        self.views.column_menu = None;
        let Some(col) = column(key) else {
            return true;
        };
        let mut widths = self.saved_column_widths();
        if col.flexible {
            if widths.remove(key).is_none() {
                return true;
            }
        } else {
            let tracks = self.table_tracks().unwrap_or_default();
            let texts = tracks.iter().filter_map(|t| match key {
                "duration" => t.duration.map(intl::clock),
                k if k.starts_with("tag:") => tag_cell(t, k),
                _ => (col.value)(t),
            });
            widths.insert(key.to_string(), fitted_width(&col, texts));
        }
        self.save_column_widths(&widths);
        true
    }

    /// Double-clic sur la poignée, ou menu de l'en-tête : la colonne reprend sa largeur
    /// d'origine (automatique pour le titre).
    pub(crate) fn column_original(&mut self, key: &str) -> bool {
        self.views.column_menu = None;
        let mut widths = self.saved_column_widths();
        if widths.remove(key).is_some() {
            self.save_column_widths(&widths);
        }
        true
    }

    /// Clic droit sur l'en-tête d'une colonne.
    pub(crate) fn column_menu(&mut self, key: String, at: TrackMenuAt) -> bool {
        let Some(col) = column(&key) else {
            return false;
        };
        self.menu = None;
        self.views.column_menu = Some(ColumnMenu {
            flexible: col.flexible,
            fit: matches!(col.widget, Widget::Text | Widget::Album),
            resizable: !matches!(col.widget, Widget::Cover | Widget::Index),
            key,
            at,
        });
        true
    }

    fn save_column_widths(&mut self, widths: &HashMap<String, f32>) {
        let rounded: HashMap<&str, i64> = widths.iter().map(|(k, v)| (k.as_str(), v.round() as i64)).collect();
        self.set_setting("track_column_widths", serde_json::to_string(&rounded).unwrap_or_default());
    }
}
