// SPDX-License-Identifier: MIT
fn theurgy_bootstrap(game: &mut crate::game::Game<'_>) {
    let _ = game;
    use std::sync::atomic::{AtomicBool, Ordering};
    static DONE: AtomicBool = AtomicBool::new(false);
    if !DONE.swap(true, Ordering::Relaxed) {
        // [[crate]] id engine is exposed as theurgy_engine by horizon-mixins.
        let state = theurgy_engine::crafting::Crafting::default();
        eprintln!("[theurgy] bootstrap; engine={}%; gameplay port NOT complete",
                  state.progress_percent());
    }
}
