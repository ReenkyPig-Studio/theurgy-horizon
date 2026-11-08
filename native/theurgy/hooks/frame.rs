fn hook(game: &mut crate::game::Game<'_>, _elapsed: std::time::Duration) {
    crate::game::theurgy_filter_client::frame(game);
    crate::game::theurgy_client::frame(game);
}
