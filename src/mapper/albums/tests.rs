use super::album_detail::{disc_order, sort_tracks};
use super::album_list::{filter_albums, NO_GENRE};
use super::album_sort::{natural_cmp, sort_key, sort_letter};
use crate::models::albums::AlbumsPage;
use crate::query::album::album_list_row::AlbumListRow;
use crate::query::track::track_list_row::TrackListRow;
use std::cmp::Ordering;

fn album(title: &str, artist: &str, year: Option<i32>, genre: Option<&str>) -> AlbumListRow {
    AlbumListRow {
        id: title.to_lowercase(),
        title: title.into(),
        artist: artist.into(),
        year,
        genre: genre.map(String::from),
        album_type: "album".into(),
        ..Default::default()
    }
}

fn titles(list: &[(&AlbumListRow, &str)]) -> Vec<String> {
    list.iter().map(|(a, _)| a.title.clone()).collect()
}

#[test]
fn the_sort_key_drops_accents_and_leading_articles() {
    assert_eq!(sort_key("The Wall"), "wall");
    assert_eq!(sort_key("L'Été indien"), "ete indien");
    assert_eq!(sort_key("  Écho "), "echo");
    assert_eq!(sort_letter("The Wall"), "W");
    assert_eq!(sort_letter("1999"), "#");
}

#[test]
fn natural_order_reads_numbers_as_numbers() {
    assert_eq!(natural_cmp("disc 2", "disc 10"), Ordering::Less);
    assert_eq!(natural_cmp("abc", "abd"), Ordering::Less);
    assert_eq!(natural_cmp("a", "a"), Ordering::Equal);
}

#[test]
fn albums_are_sorted_by_title_without_articles() {
    let albums = [album("The Wall", "Pink Floyd", Some(1979), None), album("Animals", "Pink Floyd", Some(1977), None), album("Meddle", "Pink Floyd", Some(1971), None)];
    let page = AlbumsPage::new(1);
    assert_eq!(titles(&filter_albums(&page, &albums)), ["Animals", "Meddle", "The Wall"]);
}

#[test]
fn the_year_sort_puts_recent_albums_first() {
    let albums = [album("A", "x", Some(1971), None), album("B", "x", Some(1999), None), album("C", "x", None, None)];
    let mut page = AlbumsPage::new(1);
    page.sort = "year".into();
    assert_eq!(titles(&filter_albums(&page, &albums)), ["B", "A", "C"]);
}

#[test]
fn search_and_genre_filters_combine() {
    let albums = [
        album("Nevermind", "Nirvana", Some(1991), Some("Rock")),
        album("Blue Train", "John Coltrane", Some(1957), Some("Jazz")),
        album("Bleach", "Nirvana", Some(1989), None),
    ];
    let mut page = AlbumsPage::new(1);
    page.search_term = "nirvana".into();
    assert_eq!(titles(&filter_albums(&page, &albums)), ["Bleach", "Nevermind"]);
    page.genre = Some("rock".into());
    assert_eq!(titles(&filter_albums(&page, &albums)), ["Nevermind"]);
    page.genre = Some(NO_GENRE.into());
    assert_eq!(titles(&filter_albums(&page, &albums)), ["Bleach"]);
}

#[test]
fn tracks_follow_disc_then_track_number() {
    let t = |disc: i32, n: i32, name: &str| TrackListRow { disc_number: disc, track_number: Some(n), filename: name.into(), ..Default::default() };
    let mut l = vec![t(2, 1, "c"), t(1, 2, "b"), t(1, 1, "a")];
    l.sort_by(disc_order);
    assert_eq!(l.iter().map(|x| x.filename.as_str()).collect::<Vec<_>>(), ["a", "b", "c"]);
    sort_tracks(&mut l, "index", false);
    assert_eq!(l.first().map(|x| x.filename.as_str()), Some("b"));
}

#[test]
fn an_album_is_split_by_disc_unless_sorted() {
    use super::album_detail::disc_groups;
    let t = |disc: i32| TrackListRow { disc_number: disc, ..Default::default() };
    let tracks = [t(1), t(1), t(2), t(0)];
    assert_eq!(disc_groups(&tracks, true), [(1, vec![0, 1, 3]), (2, vec![2])]);
    assert_eq!(disc_groups(&tracks, false), [(0, vec![0, 1, 2, 3])]);
    assert_eq!(disc_groups(&tracks[..2], true), [(0, vec![0, 1])]);
}
