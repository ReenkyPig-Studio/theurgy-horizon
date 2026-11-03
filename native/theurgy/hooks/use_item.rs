fn hook(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, claimed: &mut bool) {
 crate::game::theurgy_items::use_item(game, stack, claimed);
}
