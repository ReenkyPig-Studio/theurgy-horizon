fn hook(
    level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId,
    output: &mut bool,
) {
    crate::game::theurgy_machines::has_analog_output(level, state, output);
}
