fn hook(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    button: i32,
    _modifiers: u32,
    canceled: &mut bool,
) {
    crate::game::theurgy_client::mouse_pressed(game, screen, button, canceled);
}
