use crate::models::tracks::*;
use crate::query::track::track_list_row::TrackListRow;
use karbon_ui::intl::t;
use crate::mapper::tracks::format_bitrate;
use crate::mapper::tracks::short_format;
use crate::mapper::tracks::tier;
use crate::models::tracks::Tier;
use karbon_ui::ui::layout::{ComputedLength, Expression};
use karbon_ui::ui::prelude::Length;

/// Variables des largeurs de colonnes, posées sur la liste des lignes (`style:vars`) : une
/// largeur qui change ne reconstruit pas les lignes mémorisées, seule leur disposition
/// est refaite. Au-delà, une colonne garde une largeur fixe dans la clé de sa ligne.
const COLUMN_VARS: [&str; 24] = [
    "--cw-0", "--cw-1", "--cw-2", "--cw-3", "--cw-4", "--cw-5", "--cw-6", "--cw-7", "--cw-8", "--cw-9", "--cw-10", "--cw-11",
    "--cw-12", "--cw-13", "--cw-14", "--cw-15", "--cw-16", "--cw-17", "--cw-18", "--cw-19", "--cw-20", "--cw-21", "--cw-22",
    "--cw-23",
];
static COLUMN_WIDTHS: [Expression; COLUMN_VARS.len()] = {
    let mut all = [Expression::Number(0.); COLUMN_VARS.len()];
    let mut i = 0;
    while i < all.len() {
        all[i] = Expression::Variable { name: COLUMN_VARS[i], fallback: &Expression::Px(0.), length: true };
        i += 1;
    }
    all
};

/// Largeur de la cellule de la colonne `i` : la variable de sa colonne, ou `px`.
pub fn column_width(i: usize, px: f32) -> Length {
    COLUMN_WIDTHS.get(i).map_or(Length::Px(px), |e| Length::Computed(ComputedLength::new(e)))
}

/// Nombre de colonnes dont la largeur passe par une variable.
pub fn variable_columns() -> usize {
    COLUMN_VARS.len()
}

/// Les variables de largeur d'une table (`style:vars`).
pub fn column_vars(widths: impl Iterator<Item = f32>) -> impl Iterator<Item = (&'static str, Length)> {
    COLUMN_VARS.iter().copied().zip(widths.map(Length::Px))
}

/// Largeur minimale d'une colonne réglée à la main.
pub const WIDTH_MIN: f32 = 32.;
/// Largeur maximale d'une colonne réglée à la main.
pub const WIDTH_MAX: f32 = 600.;
/// Le titre peut aller bien plus loin que les autres colonnes.
pub const WIDTH_MAX_FLEXIBLE: f32 = 1600.;

/// Largeur estimée d'un texte de cellule (13 px) : pas de mesure de police côté
/// application, une moyenne par caractère suffit pour ajuster une colonne.
const CELL_CHAR: f32 = 7.;
/// Intitulé d'en-tête : 11 px gras, en capitales espacées.
const LABEL_CHAR: f32 = 8.;
/// Marge intérieure et flèche de tri.
const FIT_PADDING: f32 = 28.;

/// Largeur d'une colonne ajustée à son contenu (intitulé compris). Une colonne rendue par
/// un composant garde sa largeur d'origine, déjà celle de son contenu.
pub fn fitted_width(col: &TableColumn, texts: impl Iterator<Item = String>) -> f32 {
    if !matches!(col.widget, Widget::Text | Widget::Album) {
        return col.width;
    }
    let label = col.label().chars().count() as f32 * LABEL_CHAR;
    let widest = texts.map(|t| t.chars().count() as f32 * CELL_CHAR).fold(label, f32::max);
    (widest.ceil() + FIT_PADDING).clamp(WIDTH_MIN, WIDTH_MAX)
}

/// Largeur d'une colonne tirée de `dx` depuis `start`, au pixel près et dans ses bornes
/// (le titre va plus loin et garde sa largeur minimale).
pub fn resized_width(col: &TableColumn, start: f32, dx: f32) -> f32 {
    let (min, max) = if col.flexible {
        (WIDTH_MIN.max(col.min_width), WIDTH_MAX_FLEXIBLE)
    } else {
        (WIDTH_MIN, WIDTH_MAX)
    };
    (start + dx).round().clamp(min, max)
}

pub(crate) fn none(_: &TrackListRow) -> Option<String> {
    None
}

pub(crate) fn date(d: &Option<chrono::DateTime<chrono::Utc>>) -> Option<String> {
    d.map(|d| d.with_timezone(&chrono::Local).format("%d/%m/%Y").to_string())
}

pub(crate) fn size(bytes: i64) -> Option<String> {
    (bytes > 0).then(|| karbon_ui::intl::bytes(bytes as u64))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn col(
    key: &str,
    width: f32,
    right: bool,
    numeric: bool,
    widget: Widget,
    value: fn(&TrackListRow) -> Option<String>,
) -> TableColumn {
    TableColumn {
        key: key.into(),
        fallback: key.into(),
        width,
        flexible: false,
        min_width: 0.,
        right,
        numeric,
        widget,
        value,
    }
}

pub fn fixed() -> Vec<TableColumn> {
    use crate::models::tracks::Widget::*;
    let mut title = col("title", 240., false, false, Title, |t| Some(t.title.clone()));
    title.flexible = true;
    title.min_width = 180.;
    vec![
        col("index", 20., true, true, Index, none),
        col("cover", 32., false, false, Cover, none),
        title,
        col("artist", 160., false, false, Text, |t| t.artist.clone()),
        col("album", 176., false, false, Album, |t| t.album.clone()),
        col("album_artist", 160., false, false, Text, |t| t.album_artist.clone()),
        col("year", 56., false, true, Text, |t| t.year.clone()),
        col("genre", 112., false, false, Text, |t| t.genre.clone()),
        col("rating", 80., false, false, Rating, none),
        col("duration", 48., true, true, Text, none),
        col("track_number", 48., true, true, Text, |t| t.track_number.filter(|n| *n > 0).map(|n| n.to_string())),
        col("disc_number", 48., true, true, Text, |t| (t.disc_number > 0).then(|| t.disc_number.to_string())),
        col("play_count", 56., true, true, Text, |t| (t.play_count > 0).then(|| t.play_count.to_string())),
        col("last_played_at", 96., false, true, Text, |t| date(&t.last_played_at)),
        col("created_at", 96., false, true, Text, |t| date(&Some(t.created_at))),
        col("quality", 140., false, false, Quality, |t| {
            let label = t_tier(tier(t.audio_format.as_deref(), t.bits_per_sample, t.sample_rate));
            Some(format!("{label} · {}", short_format(t.audio_format.as_deref(), t.bits_per_sample, t.sample_rate)))
        }),
        col("audio_format", 64., false, false, Text, |t| t.audio_format.clone()),
        col("extension", 64., false, false, Text, |t| Some(t.extension.clone())),
        col("bitrate", 80., true, true, Text, |t| t.bitrate.filter(|b| *b > 0).map(|_| format_bitrate(t.bitrate))),
        col("sample_rate", 80., true, true, Text, |t| {
            t.sample_rate
                .filter(|r| *r > 0)
                .map(|r| format!("{} kHz", format!("{:.1}", r as f64 / 1000.).trim_end_matches(".0").replace('.', ",")))
        }),
        col("bits_per_sample", 48., true, true, Text, |t| t.bits_per_sample.filter(|b| *b > 0).map(|b| format!("{b} bit"))),
        col("channels", 64., false, false, Text, |t| {
            t.channels.map(|c| match c {
                1 => t_unit("units.mono"),
                2 => t_unit("units.stereo"),
                6 => "5.1".into(),
                8 => "7.1".into(),
                n => format!("{n} ch"),
            })
        }),
        col("file_size", 80., true, true, Text, |t| size(t.file_size.unwrap_or(t.size))),
        col("filename", 208., false, false, Text, |t| Some(t.filename.clone())),
        col("path", 288., false, false, Text, |t| Some(t.path.clone())),
    ]
}

pub(crate) fn t_tier(tier: Tier) -> String {
    t(match tier {
        Tier::Hires => "tracks_view.hires",
        Tier::Lossless => "tracks_view.lossless",
        Tier::Lossy => "tracks_view.lossy",
    })
}

pub(crate) fn t_unit(key: &str) -> String {
    t(key)
}

/// L'intitulé d'une colonne de tag, traduit par `columns.tag.*`.
pub fn tag_title(key: &str) -> String {
    if let Some(custom) = key.strip_prefix("custom:") {
        return custom.to_string();
    }
    let translated = t(&format!("columns.tag.{key}"));
    if !translated.starts_with("columns.tag.") {
        return translated;
    }
    key.replace('_', " ")
}

/// La valeur d'un tag : champ nommé du jeu de tags, ou tag libre.
pub(crate) fn tag_value(track: &TrackListRow, key: &str) -> Option<String> {
    let raw = track.tags.as_deref()?;
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    if let Some(custom) = key.strip_prefix("custom:") {
        return value.get("custom_tags")?.as_array()?.iter().find_map(|pair| {
            let pair = pair.as_array()?;
            (pair.first()?.as_str()? == custom).then(|| pair.get(1)?.as_str().map(str::to_string))?
        });
    }
    match value.get(key)? {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

pub(crate) const DEFAULT: [&str; 7] = ["index", "cover", "title", "artist", "album", "rating", "duration"];

pub(crate) const MOCKUP: [&str; 7] = ["index", "cover", "title", "album", "quality", "rating", "duration"];

/// La liste enregistrée, la mise en page de la maquette si c'est le défaut.
pub fn shown_keys(raw: Option<&str>) -> Vec<String> {
    let list: Vec<String> = raw
        .and_then(|r| serde_json::from_str::<Vec<String>>(r).ok())
        .map(|l| {
            if l.iter().any(|c| ["index", "cover", "title"].contains(&c.as_str())) {
                l
            } else {
                ["index", "cover", "title"].iter().map(|s| s.to_string()).chain(l).collect()
            }
        })
        .unwrap_or_else(|| DEFAULT.iter().map(|s| s.to_string()).collect());
    if list.iter().map(String::as_str).eq(DEFAULT.iter().copied()) {
        MOCKUP.iter().map(|s| s.to_string()).collect()
    } else {
        list
    }
}

/// Les colonnes connues ; `tag:<clé>` devient une colonne de tag.
pub fn resolve(keys: &[String]) -> Vec<TableColumn> {
    let all = fixed();
    keys.iter()
        .filter_map(|k| {
            if let Some(tag) = k.strip_prefix("tag:") {
                let mut c = col(k, 128., false, false, Widget::Text, none);
                c.fallback = tag_title(tag);
                // La valeur d'un tag dépend de sa clé : lue à part (`tag_cell`).
                c.value = none;
                return Some(c);
            }
            all.iter().find(|c| &c.key == k).cloned()
        })
        .collect()
}

/// La cellule d'une colonne de tag.
pub fn tag_cell(track: &TrackListRow, key: &str) -> Option<String> {
    key.strip_prefix("tag:").and_then(|k| tag_value(track, k))
}

impl TableColumn {
    /// Traduit, sinon l'intitulé de repli ; les tags par leur nom.
    pub fn label(&self) -> String {
        if let Some(tag) = self.key.strip_prefix("tag:") {
            return tag_title(tag);
        }
        let key = format!("columns.{}", self.key);
        let text = t(&key);
        if text == key {
            self.fallback.clone()
        } else {
            text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(key: &str) -> TableColumn {
        resolve(&[key.to_string()]).into_iter().next().unwrap()
    }

    #[test]
    fn a_dragged_column_stays_within_its_bounds() {
        let year = column("year");
        assert_eq!(resized_width(&year, 80., 20.4), 100.);
        assert_eq!(resized_width(&year, 80., -200.), WIDTH_MIN);
        assert_eq!(resized_width(&year, 500., 400.), WIDTH_MAX);
        let title = column("title");
        assert_eq!(resized_width(&title, 900., 400.), 1300.);
        assert_eq!(resized_width(&title, 120., -200.), WIDTH_MIN.max(title.min_width));
    }

    #[test]
    fn a_fitted_column_follows_its_longest_text_and_its_label() {
        let tag = column("tag:mood");
        let short = fitted_width(&tag, ["ok".to_string()].into_iter());
        let long = fitted_width(&tag, ["un texte bien plus long que les autres".to_string()].into_iter());
        assert!(short >= WIDTH_MIN && long > short);
        assert_eq!(fitted_width(&tag, std::iter::repeat_n("x".repeat(500), 1)), WIDTH_MAX);
        // Une colonne dessinée par un composant garde sa largeur.
        let rating = column("rating");
        assert_eq!(fitted_width(&rating, ["x".repeat(80)].into_iter()), rating.width);
    }
}
