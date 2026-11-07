fn hook(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    x: f32,
    y: f32,
    batch: &mut horizon_client::hud::HudBatch,
) {
    crate::game::theurgy_client::item_decorations(game, stack, x, y, batch);
}
