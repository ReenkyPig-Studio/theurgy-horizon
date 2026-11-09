fn hook(
    level: &crate::game::GameLevel<'_>,
    pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId,
    survives: &mut Option<bool>,
) {
    crate::game::theurgy_items::can_survive(level, pos, state, survives);
}
