fn hook(
    game: &mut crate::game::Game<'_>,
    button: i32,
    _key_mapping: &str,
    hand: horizon_client::first_person::Hand,
    _swing_hand: &mut bool,
    canceled: &mut bool,
) {
    crate::game::theurgy_client::interaction_key(game, button, hand, canceled);
}
