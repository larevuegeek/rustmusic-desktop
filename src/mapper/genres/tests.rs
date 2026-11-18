use super::genre_detail::{artist_counts, decade_hue, genre_albums, short_decade, year_span};
use crate::models::genres::GenrePage;
use crate::query::album::album_list_row::AlbumListRow;
use crate::query::track::track_list_row::TrackListRow;

fn album(id: &str, title: &str, year: Option<i32>, genre: Option<&str>) -> AlbumListRow {
    AlbumListRow { id: id.into(), title: title.into(), year, genre: genre.map(String::from), ..Default::default() }
}

fn titles(list: &[&AlbumListRow]) -> Vec<String> {
    list.iter().map(|a| a.title.clone()).collect()
}

#[test]
fn a_genre_keeps_its_albums_whatever_the_case() {
    let albums = [album("1", "Nevermind", Some(1991), Some("Rock")), album("2", "Kind of Blue", Some(1959), Some("Jazz")), album("3", "In Utero", Some(1993), Some("ROCK"))];
    let page = GenrePage::new("rock".into(), None);
    assert_eq!(titles(&genre_albums(&page, &albums)), ["In Utero", "Nevermind"]);
}

#[test]
fn a_decade_keeps_the_albums_of_its_ten_years() {
    let albums = [album("1", "A", Some(1989), None), album("2", "B", Some(1990), None), album("3", "C", Some(1999), None), album("4", "D", Some(2000), None)];
    let decade = GenrePage::new("x".into(), Some((1990, None)));
    assert_eq!(titles(&genre_albums(&decade, &albums)), ["C", "B"]);
    let year = GenrePage::new("x".into(), Some((1990, Some(1999))));
    assert_eq!(titles(&genre_albums(&year, &albums)), ["C"]);
}

#[test]
fn a_composer_keeps_the_albums_of_his_tracks() {
    let albums = [album("1", "A", None, None), album("2", "B", None, None)];
    let mut page = GenrePage::new("Bach".into(), None);
    page.composer = true;
    page.tracks = vec![TrackListRow { album_id: Some("2".into()), ..Default::default() }];
    assert_eq!(titles(&genre_albums(&page, &albums)), ["B"]);
}

#[test]
fn artists_are_counted_most_present_first() {
    let t = |id: &str| TrackListRow { artist_id: Some(id.into()), ..Default::default() };
    let counts = artist_counts(&[t("a"), t("b"), t("b"), TrackListRow::default(), t("b"), t("a"), t("c")]);
    assert_eq!(counts, [("b".to_string(), 3), ("a".to_string(), 2), ("c".to_string(), 1)]);
}

#[test]
fn the_year_span_ignores_undated_albums() {
    let a = [album("1", "A", Some(1999), None), album("2", "B", Some(1971), None), album("3", "C", None, None)];
    assert_eq!(year_span(&a.iter().collect::<Vec<_>>()), Some("1971 – 1999".into()));
    assert_eq!(year_span(&[&a[0]]), Some("1999".into()));
    assert_eq!(year_span(&[&a[2]]), None);
}

#[test]
fn decades_are_written_short_before_2000() {
    assert_eq!(short_decade(1980), "80");
    assert_eq!(short_decade(2010), "2010");
    assert_eq!(decade_hue(1950), 0.);
    assert!((0. ..360.).contains(&decade_hue(1900)));
}

#[test]
fn genres_follow_the_chosen_sort() {
    use super::genre_list::sort_genres;
    use crate::entity::library::genre_view::GenreView;
    let g = |name: &str, albums: i64, tracks: i64| GenreView { name: name.into(), total_albums: albums, total_tracks: tracks, ..Default::default() };
    let list = [g("Rock", 3, 90), g("Jazz", 9, 40), g("Blues", 3, 90)];
    let names = |sort| {
        let mut l: Vec<&GenreView> = list.iter().collect();
        sort_genres(&mut l, sort);
        l.iter().map(|x| x.name.clone()).collect::<Vec<_>>()
    };
    assert_eq!(names("name"), ["Blues", "Jazz", "Rock"]);
    assert_eq!(names("albums"), ["Jazz", "Blues", "Rock"]);
    assert_eq!(names("tracks"), ["Blues", "Rock", "Jazz"]);
}
