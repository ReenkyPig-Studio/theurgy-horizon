fn hook(
    game: &mut crate::game::Game<'_>,
    scroll: [f64; 2],
    accumulated: [i32; 2],
    canceled: &mut bool,
) {
    crate::game::theurgy_items::mouse_scroll(game, scroll, accumulated, canceled);
}
