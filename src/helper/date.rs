//! Dates de la base : lecture des textes SQLite ou ISO 8601, et temps écoulé, que
//! `karbon_ui::intl` met en mots dans la langue de l'utilisateur.

use chrono::{DateTime, Utc};
use karbon_ui::intl::{t, t_with};

/// Une date SQLite (sans fuseau, en UTC) ou ISO 8601, en instant.
pub fn parse_date(text: &str) -> Option<DateTime<Utc>> {
    let iso = if text.contains('T') {
        text.to_string()
    } else {
        format!("{}Z", text.replacen(' ', "T", 1))
    };
    DateTime::parse_from_rfc3339(&iso)
        .map(|d| d.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|d| d.and_utc())
        })
}

/// L'année à quatre chiffres d'une date libre (tag `date`), sinon vide.
pub fn date_to_year(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        let run = bytes[i..].iter().take_while(|b| b.is_ascii_digit()).count();
        if run == 4 {
            let year: u32 = text[i..i + 4].parse().unwrap_or(0);
            if (1000..=2999).contains(&year) {
                return year.to_string();
            }
        }
        i += run.max(1);
    }
    String::new()
}

/// Secondes écoulées entre la date `text` et `now` (`intl::relative_time`), ou `None`
/// si le texte n'est pas une date.
pub fn elapsed_seconds(text: &str, now: DateTime<Utc>) -> Option<i64> {
    parse_date(text).map(|instant| (now - instant).num_seconds())
}

/// Nom du mois (1 à 12) dans la langue de l'utilisateur : « mars », « March ».
pub fn month_name(month: u32) -> String {
    t(&format!("dates.month_{month}"))
}

/// Nom court du mois : « févr. », « Feb ».
pub fn month_short(month: u32) -> String {
    t(&format!("dates.month_short_{month}"))
}

/// « 12 mars 1994 », « March 12, 1994 ».
pub fn long_date(day: u32, month: u32, year: i32) -> String {
    t_with("dates.long", &[("d", &day.to_string()), ("month", &month_name(month)), ("y", &year.to_string())])
}

/// « 12 mars », « Mar 12 » (mois court).
pub fn day_month(day: u32, month: u32) -> String {
    t_with("dates.day_month", &[("d", &day.to_string()), ("month", &month_short(month))])
}

/// « 12 mars 2024 », « Mar 12, 2024 » (mois court).
pub fn day_month_year(day: u32, month: u32, year: i32) -> String {
    t_with("dates.day_month_year", &[("d", &day.to_string()), ("month", &month_short(month)), ("y", &year.to_string())])
}

/// « mars 2024 », « March 2024 ».
pub fn month_year(month: u32, year: i32) -> String {
    t_with("dates.month_year", &[("month", &month_name(month)), ("y", &year.to_string())])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_dates_are_read_as_utc() {
        let now = parse_date("2026-11-06 20:00:00").unwrap();
        assert_eq!(elapsed_seconds("2026-11-06 19:57:00", now), Some(180));
        assert_eq!(elapsed_seconds("2026-11-02T20:00:00Z", now), Some(4 * 86_400));
        assert_eq!(elapsed_seconds("jamais", now), None);
    }
}
