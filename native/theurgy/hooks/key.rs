fn hook(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    crate::game::theurgy_client::key(game, key, pressed);
}
