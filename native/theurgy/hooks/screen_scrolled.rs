fn hook(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    scroll: [f64; 2],
    _modifiers: u32,
    canceled: &mut bool,
) {
    crate::game::theurgy_client::mouse_scrolled(game, screen, scroll, canceled);
}
