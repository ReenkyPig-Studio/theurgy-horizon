fn hook(game: &crate::game::Game<'_>, block: &str, fluids: &mut Option<horizon_client::containers::ModFluidTanks>) {
 if fluids.is_none() { *fluids = crate::game::theurgy_registry::fluids(block); }
}
