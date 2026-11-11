fn hook(_game: &mut crate::game::Game<'_>, player: u128, channel: &str, data: &[u8]) {
    crate::game::theurgy_client::book_from_client(player, channel, data);
}
