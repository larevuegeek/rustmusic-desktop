use super::podcast_list::sort_podcasts;
use crate::entity::podcast::podcast::PodcastView;

fn podcast(title: &str, pinned: bool, latest: &str, added: &str, new: i64) -> PodcastView {
    PodcastView { title: title.into(), pinned, latest_at: Some(latest.into()), created_at: added.into(), new_count: new, ..Default::default() }
}

fn titles(sort: &str, list: &[PodcastView]) -> Vec<String> {
    sort_podcasts(list, sort).iter().map(|p| p.title.clone()).collect()
}

#[test]
fn favourites_come_first_whatever_the_sort() {
    let list = [
        podcast("Zeta", false, "2026-03-01", "2025-01-01", 0),
        podcast("Alpha", false, "2026-01-01", "2025-06-01", 4),
        podcast("Mid", true, "2025-01-01", "2024-01-01", 0),
    ];
    assert_eq!(titles("name", &list), ["Mid", "Alpha", "Zeta"]);
    assert_eq!(titles("latest", &list), ["Mid", "Zeta", "Alpha"]);
    assert_eq!(titles("added", &list), ["Mid", "Alpha", "Zeta"]);
    assert_eq!(titles("new", &list), ["Mid", "Alpha", "Zeta"]);
}
