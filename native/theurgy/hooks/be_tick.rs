fn hook(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut horizon_client::containers::ModBlockEntity) {
 crate::game::theurgy_registry::tick(game, pos, entity);
}
