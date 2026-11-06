fn hook(
    stack: &horizon_client::items::ItemStack,
    animation: &mut horizon_world::consumable::UseAnimation,
) {
    crate::game::theurgy_items::use_animation(stack, animation);
}
