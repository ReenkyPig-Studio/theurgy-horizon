fn hook(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    key: &str,
    _repeat: bool,
    _modifiers: u32,
    canceled: &mut bool,
) {
    crate::game::theurgy_filter_client::key_pressed(game, screen, key, canceled);
}
