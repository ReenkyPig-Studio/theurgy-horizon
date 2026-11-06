fn hook(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    duration: &mut i32,
) {
    crate::game::theurgy_items::use_duration(game, stack, duration);
}
