fn hook(
    level: &crate::game::GameLevel<'_>,
    dimension: crate::game::LevelDimension,
    pos: horizon_core::position::BlockPos,
    old: horizon_world::registry::BlockStateId,
    new: horizon_world::registry::BlockStateId,
) {
    crate::game::theurgy_fluids::block_changed(level, dimension, pos, old, new);
}
