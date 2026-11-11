fn hook(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    c: char,
    _modifiers: u32,
    canceled: &mut bool,
) {
    if !crate::game::theurgy_filter_client::char_typed(game, screen, c, canceled) {
        crate::game::theurgy_client::screen_char(game, screen, c, canceled);
    }
}
