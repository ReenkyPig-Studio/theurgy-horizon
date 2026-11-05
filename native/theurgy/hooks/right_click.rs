fn hook(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face: [i32; 3],
    canceled: &mut bool,
) {
    let stack = game
        .native_carried_stack(game.selected as i32)
        .cloned();
    if crate::game::theurgy_logistics::filter_use_on(game, pos) {
        *canceled = true;
        return;
    }
    if let Some(stack) = stack {
        crate::game::theurgy_items::right_click_block(game, &stack, pos, face, canceled);
    }
}
