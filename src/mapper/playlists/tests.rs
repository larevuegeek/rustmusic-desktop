use super::playlist_list::sort_playlists;
use crate::entity::playlist::playlist::Playlist;

fn playlist(id: i64, name: &str, tracks: i64, position: i64) -> Playlist {
    Playlist { id, name: name.into(), track_count: tracks, position, ..Default::default() }
}

#[test]
fn playlists_follow_the_chosen_sort() {
    let list = [playlist(1, "Running", 40, 2), playlist(2, "Apéro", 12, 0), playlist(3, "Le Calme", 40, 1)];
    let names = |sort| {
        let mut l: Vec<&Playlist> = list.iter().collect();
        sort_playlists(&mut l, sort);
        l.iter().map(|p| p.name.clone()).collect::<Vec<_>>()
    };
    assert_eq!(names("name"), ["Apéro", "Le Calme", "Running"]);
    assert_eq!(names("size"), ["Le Calme", "Running", "Apéro"]);
    assert_eq!(names("sidebar"), ["Apéro", "Le Calme", "Running"]);
}
