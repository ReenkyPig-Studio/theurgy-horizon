fn hook(game: &crate::game::Game<'_>, block: &str, container: &mut Option<horizon_client::containers::ModContainer>) {
 if container.is_none() { *container = crate::game::theurgy_registry::container(block); }
}
