fn hook(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face: [i32; 3],
    canceled: &mut bool,
) {
    let stack = game
        .native_carried_stack(game.selected as i32)
        .cloned();
    crate::game::theurgy_selection::right_click(game, stack.as_ref(), pos, canceled);
    if *canceled {
        return;
    }
    crate::game::theurgy_interactions::right_click(game, stack.as_ref(), pos, canceled);
    if *canceled {
        return;
    }
    if crate::game::theurgy_logistics::filter_use_on(game, pos) {
        *canceled = true;
        return;
    }
    if let Some(stack) = stack {
        crate::game::theurgy_items::right_click_block(game, &stack, pos, face, canceled);
    }
}
