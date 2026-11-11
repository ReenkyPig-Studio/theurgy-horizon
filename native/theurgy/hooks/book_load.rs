fn hook(_game: &mut crate::game::Game<'_>, player: u128, _name: &str, directory: &std::path::Path) {
    crate::game::theurgy_client::book_load(player, directory);
}
