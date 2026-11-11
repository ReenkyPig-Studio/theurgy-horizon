fn hook(game: &mut crate::game::Game<'_>, player: u128, _name: &str) {
    crate::game::theurgy_client::book_logged_in(game, player);
}
