fn hook(
    game: &mut crate::game::Game<'_>,
    player: &horizon_client::mod_api::NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    crate::game::theurgy_probe::perform(game, player, command, ran);
    crate::game::theurgy_filter::perform(game, command, ran);
}
