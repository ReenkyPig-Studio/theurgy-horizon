fn hook(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
) {
    crate::game::theurgy_items::placed(game, stack, pos, state);
}
