fn hook(game: &crate::game::Game<'_>, block: &str, screen: &mut Option<NativeScreen>) {
    let _ = game;
    if screen.is_none() { *screen = crate::game::theurgy_screens::native_screen(block); }
}
