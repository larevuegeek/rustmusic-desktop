use super::mix_list::{mix_decades, MIN_TRACKS};
use crate::query::album::album_list_row::AlbumListRow;

fn album(year: Option<i32>, tracks: i64) -> AlbumListRow {
    AlbumListRow { year, total_tracks: tracks, ..Default::default() }
}

#[test]
fn a_decade_needs_enough_tracks_for_a_mix() {
    let albums = [album(Some(1971), MIN_TRACKS - 1), album(Some(1994), 6), album(Some(1999), 6), album(Some(2003), 30), album(None, 50)];
    assert_eq!(mix_decades(&albums), [2000, 1990]);
}
