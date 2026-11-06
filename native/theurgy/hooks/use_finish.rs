fn hook(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    entity: Option<i32>,
    hand: horizon_client::first_person::Hand,
    duration: i32,
    result: &mut Option<horizon_client::items::ItemStack>,
) {
    crate::game::theurgy_items::finish_using(game, stack, entity, hand, duration, result);
}
