fn hook(
    level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId,
    source: &mut bool,
) {
    crate::game::theurgy_machines::is_signal_source(level, state, source);
}
