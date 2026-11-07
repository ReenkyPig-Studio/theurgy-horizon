fn hook(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    c: char,
    _modifiers: u32,
    canceled: &mut bool,
) {
    crate::game::theurgy_filter_client::char_typed(game, screen, c, canceled);
}
