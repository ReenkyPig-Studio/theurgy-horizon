fn hook(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face: [i32; 3],
    state: horizon_world::registry::BlockStateId,
    canceled: &mut bool,
) {
    let _ = (face, state);
    let stack = game.native_carried_stack(game.selected as i32).cloned();
    crate::game::theurgy_selection::left_click(game, stack.as_ref(), pos, canceled);
}
