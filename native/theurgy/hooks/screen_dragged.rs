fn hook(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    button: i32,
    delta: [f32; 2],
    _modifiers: u32,
    canceled: &mut bool,
) {
    crate::game::theurgy_client::mouse_dragged(game, screen, button, delta, canceled);
}
