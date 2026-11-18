use super::list_window::{CONTEXT, SLICE, STEP};
use crate::models::shared::ListWindow;

/// Fait grandir la fenêtre jusqu'au bout, comme les images successives.
fn settle(w: &mut ListWindow) {
    while w.advance(STEP) {}
}

#[test]
fn a_new_window_shows_the_first_items() {
    let w = ListWindow::new(96);
    assert_eq!(w.range(1000), 0..96);
    assert_eq!(w.range(30), 0..30);
}

#[test]
fn approaching_the_bottom_grows_by_steps_then_drops_a_slice_on_top() {
    let mut w = ListWindow::new(96);
    assert!(w.scrolled(1000., 1200., 1000, false));
    assert_eq!(w.range(1000), 0..96 + STEP);
    assert!(w.growing());
    settle(&mut w);
    assert!(!w.growing());
    assert_eq!(w.range(1000), SLICE..96 + SLICE);
}

#[test]
fn a_window_never_grows_past_the_list() {
    let mut w = ListWindow::new(96);
    assert!(!w.scrolled(1000., 1200., 96, false));
    assert!(w.more(100, true));
    // Au-delà de la capacité, la première tranche part en haut.
    assert_eq!(w.range(100), SLICE..100);
    assert!(!w.more(100, true));
}

#[test]
fn growing_does_not_stack_a_second_slice() {
    let mut w = ListWindow::new(96);
    assert!(w.more(1000, false));
    assert!(!w.more(1000, false));
}

#[test]
fn approaching_the_top_gives_back_the_slice_before() {
    let mut w = ListWindow::new(96);
    w.jump(500);
    assert_eq!(w.range(1000), 500 - CONTEXT..596 - CONTEXT);
    assert!(w.scrolled(100., 5000., 1000, false));
    assert_eq!(w.range(1000).start, 500 - CONTEXT - SLICE);
    assert_eq!(w.range(1000).len(), 96);
}

#[test]
fn a_jump_targets_the_item_inside_a_new_zone() {
    let mut w = ListWindow::new(96);
    let view = w.view;
    w.jump(5);
    assert_eq!(w.range(1000), 0..96);
    assert_eq!(w.target_slot(), Some(5));
    assert_eq!(w.view, view + 1);
    w.reset();
    assert_eq!(w.target_slot(), None);
    assert_eq!(w.range(1000), 0..96);
}

/// Onze cartes de 150 px séparées de 18 px dans une vue de 950 px.
fn eleven_cards(offset: f32) -> crate::models::shared::CarouselGeometry {
    let extent = 11. * 150. + 10. * 18.;
    crate::models::shared::CarouselGeometry { offset, max: extent - 950., viewport: 950., extent }
}

#[test]
fn an_arrow_slides_by_whole_cards() {
    use super::carousel::slide_target;
    // 950 px tiennent 5 cartes entières (5 × 168 = 840).
    assert_eq!(slide_target(eleven_cards(0.), 0., true, 11), 840.);
    assert_eq!(slide_target(eleven_cards(840.), 840., false, 11), 0.);
}

#[test]
fn an_arrow_stops_at_the_ends() {
    use super::carousel::slide_target;
    let g = eleven_cards(840.);
    assert_eq!(slide_target(g, 840., true, 11), g.max);
    assert_eq!(slide_target(eleven_cards(100.), 100., false, 11), 0.);
}

#[test]
fn an_arrow_snaps_a_free_scroll_to_a_card() {
    use super::carousel::slide_target;
    // Vingt cartes, défilées à la main à 180 px : repart du bord de la 2e carte (168).
    let extent = 20. * 150. + 19. * 18.;
    let g = crate::models::shared::CarouselGeometry { offset: 180., max: extent - 950., viewport: 950., extent };
    assert_eq!(slide_target(g, 180., true, 20), 168. + 840.);
}

#[test]
fn a_glide_sets_the_arrows_once_and_never_rebuilds_on_the_way() {
    let mut s = crate::models::shared::CarouselScroll::default();
    assert!(s.scrolled(eleven_cards(0.)));
    assert!(!s.can_go_back() && s.can_go_forward());
    // La flèche : l'état d'arrivée tout de suite (la vue est rebâtie par le clic).
    s.glide(1, 840.);
    assert!(s.can_go_back());
    // Chaque image du glissement : rien à rebâtir, arrivée comprise.
    for offset in [12., 200., 600., 820., 840.] {
        assert!(!s.scrolled(eleven_cards(offset)), "{offset}");
    }
    assert!(!s.gliding);
}

#[test]
fn the_wheel_during_a_glide_hands_the_arrows_back_to_the_position() {
    let mut s = crate::models::shared::CarouselScroll::default();
    s.scrolled(eleven_cards(400.));
    s.glide(1, 0.);
    s.scrolled(eleven_cards(300.));
    // Molette vers la droite pendant le retour au début.
    s.scrolled(eleven_cards(350.));
    assert!(!s.gliding);
    assert!(s.can_go_back());
}
