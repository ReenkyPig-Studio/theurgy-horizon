fn hook(mods: &mut Vec<horizon_client::mod_api::NativeMod>) {
    crate::game::theurgy_client::mods(mods);
}
