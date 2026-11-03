fn hook(
    game: &crate::game::Game<'_>,
    block: &str,
    data: &[i32],
    batch: &mut HudBatch,
    origin: [f32; 2],
) {
    crate::game::theurgy_screens::draw(game, block, data, batch, origin);
}
