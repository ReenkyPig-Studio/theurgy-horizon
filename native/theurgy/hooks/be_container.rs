fn hook(
    game: &crate::game::Game<'_>,
    block: &str,
    container: &mut Option<horizon_client::containers::ModContainer>,
) {
    let _ = game;
    if let Some(spec) = crate::game::theurgy_registry::container(block) {
        *container = Some(spec);
    }
}
