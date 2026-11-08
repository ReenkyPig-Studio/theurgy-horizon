fn hook(game: &mut crate::game::Game<'_>, dimension: &str, chunk: [i32; 2], new: bool) {
    let _ = crate::game::theurgy_worldgen::chunk_load(game, dimension, chunk, new);
}
