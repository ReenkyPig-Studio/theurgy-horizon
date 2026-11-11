fn hook(
    level: &crate::game::GameLevel<'_>,
    dimension: crate::game::LevelDimension,
    kind: &str,
    from: horizon_core::position::BlockPos,
    to: horizon_core::position::BlockPos,
    _canceled: &mut bool,
) {
    crate::game::theurgy_fluids::block_spread(level, dimension, kind, from, to);
}
