use super::artist_detail::album_groups;
use crate::query::album::album_list_row::AlbumListRow;
use crate::query::track::track_list_row::TrackListRow;

fn track(album: &str, n: i32) -> TrackListRow {
    TrackListRow { album_id: Some(album.into()), album: Some(album.into()), track_number: Some(n), disc_number: 1, ..Default::default() }
}

fn album(id: &str, title: &str, year: i32) -> AlbumListRow {
    AlbumListRow { id: id.into(), title: title.into(), year: Some(year), ..Default::default() }
}

#[test]
fn tracks_are_grouped_by_album_in_disc_order() {
    let tracks = [track("b", 2), track("a", 1), track("b", 1)];
    let albums = [album("a", "Animals", 1977), album("b", "Meddle", 1971)];
    let groups = album_groups(&tracks, &albums, "album", true);
    assert_eq!(groups.iter().map(|g| g.title.as_str()).collect::<Vec<_>>(), ["Animals", "Meddle"]);
    assert_eq!(groups[1].tracks_list.iter().map(|x| x.track_number).collect::<Vec<_>>(), [Some(1), Some(2)]);
}

#[test]
fn the_year_sort_follows_the_chosen_direction() {
    let tracks = [track("a", 1), track("b", 1), track("c", 1)];
    let albums = [album("a", "Animals", 1977), album("b", "Meddle", 1971), album("c", "The Wall", 1979)];
    let years = |asc| album_groups(&tracks, &albums, "year", asc).iter().map(|g| g.year).collect::<Vec<_>>();
    assert_eq!(years(true), [Some(1971), Some(1977), Some(1979)]);
    assert_eq!(years(false), [Some(1979), Some(1977), Some(1971)]);
}

#[test]
fn a_track_without_album_id_keeps_its_album_name() {
    let mut loose = track("x", 1);
    loose.album_id = None;
    loose.album = Some("Bootleg".into());
    loose.year = Some("1994-05-01".into());
    let groups = album_groups(&[loose], &[], "album", true);
    assert_eq!(groups[0].key, "?Bootleg");
    assert_eq!(groups[0].title, "Bootleg");
    assert_eq!(groups[0].year, Some(1994));
}
