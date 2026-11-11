fn hook(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::theurgy_fluids::level_tick_post(game, dimension);
}
