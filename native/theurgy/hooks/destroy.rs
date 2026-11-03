fn hook(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
    canceled: &mut bool,
) {
    crate::game::theurgy_items::destroyed(game, pos, state, canceled);
}
