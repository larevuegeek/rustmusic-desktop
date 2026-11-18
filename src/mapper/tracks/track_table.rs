//! Le tableau des morceaux : ce qu'affichent ses gabarits, bâti depuis l'état de la page.

use crate::models::tracks::*;
use crate::mapper::tracks::*;
use crate::models::app::Route;
use std::collections::HashMap;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::t;
use crate::query::track::track_list_row::TrackListRow;

/// Clé de mémoire d'une cellule (`memo` des lignes) : tout ce qu'elle affiche, sauf sa
/// largeur, portée par une variable de la liste (`column_width`).
impl std::hash::Hash for Cell {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        (&self.key, self.widget, self.flexible, self.min_width.to_bits()).hash(h);
        (&self.text, self.right, self.numeric).hash(h);
    }
}

/// Clé de mémoire d'une ligne (`memo={…}` de TrackRows.ui) : tout ce qu'elle affiche.
impl Row {
    /// Ancre du menu « … » de la ligne.
    pub fn more_id(&self) -> String {
        format!("row.more.{}", self.n)
    }

    /// « Album · 1994 », ou l'album seul sans année.
    pub fn album_line(&self) -> String {
        if self.year.is_empty() {
            self.album.clone()
        } else {
            format!("{} · {}", self.album, self.year)
        }
    }
}

impl std::hash::Hash for Row {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        (self.n, self.first, &self.id, &self.title, &self.artist, &self.artist_route, &self.album, &self.album_route).hash(h);
        (&self.year, &self.cover, &self.number, self.in_progress, self.played, self.liked, self.tier).hash(h);
        (&self.tier_label, &self.details, &self.short, self.rating.to_bits(), self.rated, &self.cells).hash(h);
        for cell in self.cells.iter().skip(variable_columns()) {
            cell.width.to_bits().hash(h);
        }
    }
}

impl Table {
    /// Les largeurs des colonnes, en variables posées sur la liste des lignes.
    pub fn width_vars(&self) -> impl Iterator<Item = (&'static str, karbon_ui::ui::prelude::Length)> + '_ {
        column_vars(self.head.iter().map(|c| c.width))
    }
}

/// Clé de mémoire (`memo={…}`) d'une valeur sans `Hash` : sa forme `Debug`, qui couvre
/// tout ce qu'elle porte.
pub fn debug_key(value: &impl std::fmt::Debug) -> String {
    format!("{value:?}")
}

impl MusicApp {
    /// Les largeurs enregistrées (`track_column_widths`).
    pub fn saved_column_widths(&self) -> HashMap<String, f32> {
        self.data
            .setting("track_column_widths")
            .and_then(|raw| serde_json::from_str::<HashMap<String, f64>>(raw).ok())
            .map(|m| {
                m.into_iter()
                    .filter(|(_, v)| (f64::from(WIDTH_MIN)..=f64::from(WIDTH_MAX_FLEXIBLE)).contains(v))
                    .map(|(k, v)| (k, v as f32))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Les largeurs à dessiner : celles enregistrées, et la colonne qu'on tire en ce moment.
    pub fn column_widths(&self) -> HashMap<String, f32> {
        let mut widths = self.saved_column_widths();
        if let Some(r) = &self.column_resize {
            widths.insert(r.key.clone(), r.width);
        }
        widths
    }

    /// La table d'une liste : colonnes choisies (sans l'artiste, passé sous le titre ; en
    /// détaillé, album, année et qualité passent sur la ligne d'infos).
    pub fn track_table(
        &self,
        tracks: &[TrackListRow],
        detailed: bool,
        sort_key: Option<&str>,
        asc: bool,
        positions: bool,
    ) -> Table {
        self.track_table_at(tracks, 0, detailed, sort_key, asc, positions)
    }

    /// `track_table` d'une fenêtre qui commence au rang `base` de la liste entière.
    pub fn track_table_at(
        &self,
        tracks: &[TrackListRow],
        base: usize,
        detailed: bool,
        sort_key: Option<&str>,
        asc: bool,
        positions: bool,
    ) -> Table {
        const DETAIL: [&str; 6] = ["album", "quality", "year", "audio_format", "bits_per_sample", "sample_rate"];
        let keys = shown_keys(self.data.setting("track_columns"));
        let cols: Vec<TableColumn> = resolve(&keys)
            .into_iter()
            .filter(|c| c.key != "artist" && !(detailed && DETAIL.contains(&c.key.as_str())))
            .collect();
        let widths = self.column_widths();
        let width_of = |c: &TableColumn| -> f32 {
            match c.widget {
                Widget::Cover => {
                    if detailed {
                        68.
                    } else {
                        40.
                    }
                }
                Widget::Index => widths.get(&c.key).copied().unwrap_or(c.width).max(28.),
                _ if c.key == "duration" => widths.get(&c.key).copied().unwrap_or(c.width).max(44.),
                _ => widths.get(&c.key).copied().unwrap_or(c.width),
            }
        };
        let flexible = cols.iter().position(|c| c.flexible && !widths.contains_key(&c.key));
        let head = cols
            .iter()
            .enumerate()
            .map(|(i, c)| HeadCell {
                key: c.key.clone(),
                label: c.label(),
                widget: c.widget,
                width: width_of(c),
                flexible: Some(i) == flexible,
                min_width: c.min_width,
                right: c.right || c.key == "duration",
                sorted: sort_key == Some(c.key.as_str()),
                asc,
            })
            .collect();
        let library = self.data.library_id.unwrap_or(0);
        let playing_path = self.data.player.path.as_deref();
        let rows = tracks
            .iter()
            .enumerate()
            .map(|(i, track)| {
                let n = base + i;
                let tier_of = tier(track.audio_format.as_deref(), track.bits_per_sample, track.sample_rate);
                let f = track.audio_format.clone().unwrap_or_default().to_uppercase();
                let details = match (track.bits_per_sample, track.sample_rate) {
                    (Some(b), Some(r)) if b > 1 && r > 0 => {
                        let khz = format!("{:.1}", r as f64 / 1000.).trim_end_matches(".0").replace('.', ",");
                        format!("{f} · {b} bit · {khz} kHz")
                    }
                    _ if track.bitrate.unwrap_or(0) > 0 => format!("{f} · {}", format_bitrate(track.bitrate)),
                    _ => short_format(track.audio_format.as_deref(), track.bits_per_sample, track.sample_rate),
                };
                let in_progress = playing_path == Some(track.path.as_str());
                let cells = cols
                    .iter()
                    .enumerate()
                    .map(|(i, c)| Cell {
                        key: c.key.clone(),
                        widget: c.widget,
                        width: width_of(c),
                        flexible: Some(i) == flexible,
                        min_width: c.min_width,
                        text: match c.key.as_str() {
                            "duration" => track.duration.map(intl::clock).unwrap_or_default(),
                            k if k.starts_with("tag:") => crate::mapper::tracks::tag_cell(track, k).unwrap_or_default(),
                            _ => (c.value)(track).unwrap_or_default(),
                        },
                        right: c.right,
                        numeric: c.numeric,
                    })
                    .collect();
                Row {
                    n,
                    first: i == 0,
                    id: track.id.clone(),
                    title: track.title.clone(),
                    artist: track.artist.clone().unwrap_or_default(),
                    artist_route: track.library_artist_id.clone().map(Route::Artist),
                    album: track.album.clone().unwrap_or_default(),
                    album_route: track.album_id.clone().map(Route::Album),
                    year: track.year.clone().unwrap_or_default(),
                    cover: track.thumbnail_path.clone(),
                    number: track
                        .track_number
                        .map(|n| n.to_string())
                        .or_else(|| positions.then(|| (n + 1).to_string()))
                        .unwrap_or_else(|| "—".into()),
                    in_progress,
                    played: in_progress && self.playing(),
                    liked: self.data.liked.contains(&track.path),
                    tier: tier_of,
                    tier_label: t(match tier_of {
                        Tier::Hires => "tracks_view.hires",
                        Tier::Lossless => "tracks_view.lossless",
                        Tier::Lossy => "tracks_view.lossy",
                    }),
                    details,
                    short: short_format(track.audio_format.as_deref(), track.bits_per_sample, track.sample_rate),
                    rating: track.rating.unwrap_or(0.),
                    rated: track.rating.is_some(),
                    cells,
                }
            })
            .collect();
        let _ = library;
        Table {
            head,
            rows,
            detailed,
            filler: flexible.is_none(),
            target: None,
        }
    }

    /// Les morceaux de la table de la page ouverte.
    pub(crate) fn table_tracks(&self) -> Option<Vec<TrackListRow>> {
        match self.route() {
            Route::Library(crate::models::app::Section::Tracks) => self.views.tracks.as_ref().map(|p| p.tracks.clone()),
            Route::Album(_) => Some(self.album_sorted()),
            Route::Track(_) => self.views.track.as_ref().map(|p| p.album.clone()),
            Route::Artist(_) => Some(self.artist_order()),
            Route::Genre(_) | Route::Decade(_) | Route::Composer(_) => Some(self.genre_sorted()),
            Route::Playlist(_) | Route::Liked | Route::Recent => Some(self.playlist_sorted()),
            _ => None,
        }
    }

    /// Rang, dans la liste entière, de la première ligne de `table_tracks` (`Row::n`).
    pub(crate) fn table_base(&self) -> usize {
        match self.route() {
            Route::Library(crate::models::app::Section::Tracks) => {
                self.views.tracks.as_ref().map_or(0, |p| usize::try_from(p.start).unwrap_or(0))
            }
            _ => 0,
        }
    }
}
