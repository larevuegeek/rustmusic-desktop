use super::sidebar_update::moved;

#[test]
fn moved_follows_the_drop_side() {
    let l = ["a", "b", "c", "d"];
    assert_eq!(moved(&l, 0, 2, true), ["b", "c", "a", "d"]);
    assert_eq!(moved(&l, 0, 2, false), ["b", "a", "c", "d"]);
    assert_eq!(moved(&l, 3, 1, false), ["a", "d", "b", "c"]);
    assert_eq!(moved(&l, 3, 1, true), ["a", "b", "d", "c"]);
    assert_eq!(moved(&l, 1, 0, false), ["b", "a", "c", "d"]);
}
