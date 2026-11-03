fn hook(
    game: &crate::game::Game<'_>,
    block: &str,
    fluids: &mut Option<horizon_client::containers::ModFluidTanks>,
) {
    let _ = game;
    if let Some(spec) = crate::game::theurgy_registry::fluids(block) {
        *fluids = Some(spec);
    }
}
