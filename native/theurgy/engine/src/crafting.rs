// SPDX-FileCopyrightText: 2023 klikli-dev
// SPDX-License-Identifier: MIT
// Port of tickServer/stopProcessing/progressPercent from CraftingBehaviour.java.
// Inventory transactions, recipe lookup, persistence and networking remain host work.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Crafting {
    pub progress: i32,
    pub total_time: i32,
    pub is_processing: bool,
    pub could_craft_last_tick: bool,
}
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TickEffects {
    pub craft_attempt: bool,
    pub block_updates: u8,
}
impl Crafting {
    pub fn progress_percent(&self) -> i32 {
        if self.total_time <= 0 { return 0; }
        ((self.progress as i64 * 100) / self.total_time as i64).clamp(0, 100) as i32
    }
    pub fn input_changed(&mut self, total_time: i32) {
        self.total_time = total_time;
        self.progress = 0;
    }
    pub fn stop(&mut self) -> u8 {
        if self.progress != 0 || self.is_processing {
            self.is_processing = false;
            self.could_craft_last_tick = false;
            self.progress = 0;
            1
        } else { 0 }
    }
    // can_craft is the host's recipe + simulated-output-insertion result.
    // next_total_time is getTotalTime() before craft mutates input, as in Java.
    pub fn tick(&mut self, can_process: bool, has_input: bool,
                can_craft: bool, next_total_time: i32) -> TickEffects {
        let mut effects = TickEffects::default();
        if !has_input {
            effects.block_updates += self.stop();
            return effects;
        }
        self.could_craft_last_tick = can_craft;
        if can_process && can_craft {
            if self.progress == 0 {
                self.is_processing = true;
                effects.block_updates += 1;
            }
            self.progress = self.progress.wrapping_add(1);
            if self.progress >= self.total_time {
                self.progress = 0;
                self.total_time = next_total_time;
                effects.craft_attempt = true;
                effects.block_updates += 1;
            }
        } else {
            effects.block_updates += self.stop();
        }
        effects
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completes_on_exact_tick_not_earlier() {
        let mut c = Crafting { total_time: 3, ..Default::default() };
        assert_eq!(c.tick(true, true, true, 3), TickEffects { craft_attempt: false, block_updates: 1 });
        assert!(!c.tick(true, true, true, 3).craft_attempt);
        assert_eq!(c.tick(true, true, true, 3), TickEffects { craft_attempt: true, block_updates: 1 });
        assert_eq!(c.progress, 0);
        assert!(c.is_processing); // Java leaves this true until stopped.
    }
    #[test]
    fn loss_of_heat_resets_not_pauses() {
        let mut c = Crafting { total_time: 10, ..Default::default() };
        c.tick(true, true, true, 10);
        assert_eq!(c.tick(false, true, true, 10).block_updates, 1);
        assert_eq!(c.progress, 0);
        assert!(!c.is_processing);
        assert!(!c.could_craft_last_tick);
    }
    #[test]
    fn blocked_output_stops_processing() {
        let mut c = Crafting { total_time: 10, ..Default::default() };
        c.tick(true, true, true, 10);
        assert!(!c.tick(true, true, false, 10).craft_attempt);
        assert_eq!(c.progress, 0);
    }
    #[test]
    fn idle_stop_does_not_clear_could_craft_java_quirk() {
        let mut c = Crafting::default();
        c.tick(false, true, true, 20);
        assert!(c.could_craft_last_tick);
        assert_eq!(c.stop(), 0);
    }
    #[test]
    fn zero_duration_crafts_on_first_tick_and_percent_is_safe() {
        let mut c = Crafting::default();
        assert_eq!(c.progress_percent(), 0);
        assert!(c.tick(true, true, true, 0).craft_attempt);
        c.total_time = 10;
        c.progress = i32::MAX;
        assert_eq!(c.progress_percent(), 100);
        c.progress = -1;
        assert_eq!(c.progress_percent(), 0);
    }
    #[test]
    fn missing_input_stops_once() {
        let mut c = Crafting { total_time: 10, ..Default::default() };
        c.tick(true, true, true, 10);
        assert_eq!(c.tick(true, false, true, 10).block_updates, 1);
        assert_eq!(c.tick(true, false, true, 10).block_updates, 0);
    }
}
