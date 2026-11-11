fn hook(_game: &mut crate::game::Game<'_>, channel: &str, data: &[u8]) {
    crate::game::theurgy_client::book_from_server(channel, data);
}
