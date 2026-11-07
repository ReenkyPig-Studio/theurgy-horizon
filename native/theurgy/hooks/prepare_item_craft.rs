fn hook(
    game: &mut crate::game::Game<'_>,
    _player: u128,
    _name: &str,
    _pos: Option<[i32; 3]>,
    result: &mut Option<horizon_client::items::ItemStack>,
) {
    crate::game::theurgy_items::prepare_item_craft(game, result);
}
