fn hook(
    level: &crate::game::GameLevel<'_>,
    _dimension: crate::game::LevelDimension,
    pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId,
    _face: horizon_core::geometry::Direction,
    output: &mut i32,
) {
    let _ = (_dimension, _face);
    crate::game::theurgy_machines::analog_output(level, pos, state, output);
}
