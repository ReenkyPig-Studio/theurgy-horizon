fn hook(game: &mut crate::game::Game<'_>, slot: i32, selected: bool) {
    crate::game::theurgy_selection::inventory_tick(game, slot, selected);
    crate::game::theurgy_items::inventory_tick(game, slot);
}
