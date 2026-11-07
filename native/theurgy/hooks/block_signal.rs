fn hook(
    level: &crate::game::GameLevel<'_>,
    _dimension: crate::game::LevelDimension,
    pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId,
    face: horizon_core::geometry::Direction,
    _direct: bool,
    signal: &mut i32,
) {
    let _ = _dimension;
    crate::game::theurgy_machines::block_signal(level, pos, state, face, signal);
}
