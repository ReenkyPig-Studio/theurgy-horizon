fn hook(game: &mut crate::game::Game<'_>, block: &str, entity: &mut Option<(String, horizon_world::nbt::Compound)>) {
 crate::game::theurgy_registry::create(block, entity);
}
