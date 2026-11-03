fn hook(game: &mut crate::game::Game<'_>, pos: [i32; 3], state: horizon_world::registry::BlockStateId, canceled: &mut bool) {
 crate::game::theurgy_items::destroy(game, pos, state, canceled);
}
