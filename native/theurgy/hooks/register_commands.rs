fn hook(commands: &mut Vec<horizon_client::mod_api::NativeCommand>) {
    crate::game::theurgy_probe::register(commands);
    crate::game::theurgy_filter::register(commands);
}
