use super::year_list::{albums_by_decade, dated};
use crate::query::album::album_list_row::AlbumListRow;

fn album(year: Option<i32>) -> AlbumListRow {
    AlbumListRow { year, ..Default::default() }
}

#[test]
fn years_before_1900_are_not_dates() {
    assert_eq!(dated(Some(1899)), None);
    assert_eq!(dated(Some(1900)), Some(1900));
    assert_eq!(dated(None), None);
}

#[test]
fn albums_are_grouped_by_decade() {
    let albums = [album(Some(1994)), album(Some(1971)), album(Some(1999)), album(None), album(Some(12))];
    let decades = |recent| albums_by_decade(&albums, recent).iter().map(|(d, l)| (*d, l.len())).collect::<Vec<_>>();
    assert_eq!(decades(false), [(1970, 1), (1990, 2)]);
    assert_eq!(decades(true), [(1990, 2), (1970, 1)]);
}
