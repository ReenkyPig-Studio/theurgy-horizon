fn hook(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    entity: Option<i32>,
    time_left: i32,
    used: &mut bool,
) {
    crate::game::theurgy_items::release_using(game, stack, entity, time_left, used);
}
