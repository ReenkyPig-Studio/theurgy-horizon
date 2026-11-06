fn hook(
    pos: [i32; 3],
    block: &str,
    state: horizon_world::registry::BlockStateId,
    entity: &horizon_client::containers::ModBlockEntity,
    bounds: &mut [f32; 6],
) {
    crate::game::theurgy_render::bounds(pos, block, state, entity, bounds);
}
