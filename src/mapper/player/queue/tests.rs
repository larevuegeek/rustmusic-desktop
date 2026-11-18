use super::panel;
use crate::controller::shared::list_window::CARDS;
use crate::models::shared::ListWindow;
use crate::entity::queue::queue_track::QueueTrack;
use crate::models::app::Status;
use crate::query::queue::queue_state_row::QueueStateRow;

fn queue(len: usize, current: i32) -> QueueStateRow {
    QueueStateRow {
        profile_id: 1,
        current_index: current,
        is_shuffled: false,
        repeat_mode: "off".into(),
        tracks: (0..len)
            .map(|i| QueueTrack {
                queue_id: format!("q{i}"),
                profile_id: 1,
                position: i as i32,
                path: format!("/music/{i}.flac"),
                title: format!("Titre {i}"),
                artist: (i % 2 == 0).then(|| "Nirvana".to_string()),
                duration: Some(222.),
                cover: None,
            })
            .collect(),
    }
}

#[test]
fn rows_follow_the_current_track_and_are_numbered_from_one() {
    let q = queue(5, 1);
    let view = panel(&q, q.tracks.get(1), Status::Playing, &ListWindow::default());
    assert!(view.playing && !view.paused && view.has_tracks);
    assert_eq!(view.now_playing.as_ref().map(|n| n.title.as_str()), Some("Titre 1"));
    let rows: Vec<_> = view.upcoming.iter().map(|r| (r.index, r.rank, r.queue_id.as_str())).collect();
    assert_eq!(rows, [(2, 1, "q2"), (3, 2, "q3"), (4, 3, "q4")]);
    assert_eq!(view.upcoming[0].artist, "Nirvana");
    assert!(view.upcoming_count.is_some());
}

#[test]
fn an_empty_queue_has_nothing_to_play_and_long_queues_are_windowed() {
    let empty = queue(0, -1);
    let view = panel(&empty, None, Status::Ready, &ListWindow::default());
    assert!(!view.has_tracks && view.now_playing.is_none() && view.upcoming_count.is_none());

    let long = queue(1000, -1);
    let mut window = ListWindow::default();
    let view = panel(&long, None, Status::Paused, &window);
    assert!(view.paused);
    assert_eq!(view.upcoming.len(), CARDS);

    // Plus bas dans la file : les rangs restent ceux de la file entière.
    window.jump(500);
    let view = panel(&long, None, Status::Paused, &window);
    let first = &view.upcoming[0];
    assert_eq!((first.index, first.rank), (window.start, window.start + 1));
}

#[test]
fn a_shorter_queue_under_a_scrolled_window_still_shows_its_end() {
    let mut window = ListWindow::default();
    window.jump(800);
    let view = panel(&queue(30, -1), None, Status::Paused, &window);
    assert_eq!(view.upcoming.len(), 30);
}
