fn hook(game: &crate::game::Game<'_>, block: &str, ticks: &mut bool) {
 crate::game::theurgy_registry::ticks(block, ticks);
}
