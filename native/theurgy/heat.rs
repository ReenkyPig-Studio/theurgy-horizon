// SPDX-License-Identifier: MIT
use horizon_client::containers::{BlockEntity, ModBlockEntity};
use horizon_client::items::ItemStack;
use horizon_world::nbt::JavaString;
use theurgy_engine::crafting::Crafting;

use crate::game::theurgy_host as host;

const BRAZIER: &str = "theurgy:pyromantic_brazier";
const HEAT_CHECK_INTERVAL: i64 = 20;
const HEAT_TARGET_TICKS: i32 = 20 * 6;

fn key(pos: [i32; 3]) -> (i32, i32, i32) {
    (pos[0], pos[1], pos[2])
}

fn stack_burn_time(game: &crate::game::Game<'_>, stack: &ItemStack) -> i32 {
    game.res
        .providers()
        .burn_time(game.res.items(), stack, false)
        .max(0)
}

fn consume_fuel(game: &crate::game::Game<'_>, entity: &mut ModBlockEntity) -> bool {
    let Some(Some(stack)) = entity.items.first_mut() else {
        return false;
    };
    if stack.count > 1 {
        stack.count -= 1;
        return true;
    }
    let remainder = game
        .res
        .items()
        .get(&stack.item)
        .and_then(|info| info.crafting_remainder.clone());
    entity.items[0] = remainder.map(|id| ItemStack::new(&id, 1));
    true
}

fn decrement_lit_time(current: i32) -> i32 {
    // PyromanticBrazierBlockEntity.tickServer decrements only positive values.
    // With no fuel a malformed/legacy negative Java short remains untouched.
    if current > 0 { current - 1 } else { current }
}

/// The pyromantic brazier's server tick: refuel from slot 0, decrement the
/// remaining lit time, and mirror `lit` into the block state. It is the only
/// heat provider in the ported apparatus; the crafting machines read it
/// through `heated`.
pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if entity.id != BRAZIER {
        return;
    }
    let mut remaining = host::int(&entity.nbt, "remainingLitTime", 0);
    let was_lit = remaining > 0;
    remaining = decrement_lit_time(remaining);
    let has_fuel = entity.items.first().and_then(Option::as_ref).is_some();
    let mut turned_on = false;
    if remaining <= 0 && has_fuel {
        let burn = entity
            .items
            .first()
            .and_then(Option::as_ref)
            .map_or(0, |stack| stack_burn_time(game, stack));
        if burn > 0 {
            remaining = burn;
            if consume_fuel(game, entity) {
                turned_on = true;
            } else {
                remaining = 0;
            }
        }
    }
    let lit = remaining > 0;
    entity.nbt.insert(JavaString::from("remainingLitTime"), horizon_world::nbt::Tag::Short(remaining as i16));
    if was_lit != lit || turned_on {
        game.native_set_block_property_with(pos, "lit", if lit { "true" } else { "false" }, 3);
        game.native_block_entity_sync(pos);
    }
}

/// Whether a heat consumer at `pos` is heated, as Java's
/// `HeatConsumerBehaviour.isHeated`:
/// - the block directly below is a lit pyromantic brazier (HEAT_PROVIDER), or
/// - its own heat receiver was filled by a caloric flux emitter and has not
///   expired (`getIsHotUntil() > level.getGameTime`).
///
/// Like Java, re-checks only when `gameTime % 20 == 0` and otherwise returns
/// the cached value (`data[6]`), so LIT lags heat by up to 20 ticks.
pub fn heated(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) -> bool {
    let now = game.world.game_time;
    if entity.data.len() < 7 {
        entity.data.resize(7, 0);
    }
    if now % HEAT_CHECK_INTERVAL != 0 {
        return entity.data[6] != 0;
    }

    let below = [pos[0], pos[1] - 1, pos[2]];
    let below_hot = game.native_block(below) == BRAZIER
        && (game.native_block_property(below, "lit").as_deref() == Some("true")
            || game.world.block_entities.get(&key(below)).is_some_and(|block| {
                matches!(block, BlockEntity::Mod(brazier) if host::int(&brazier.nbt, "remainingLitTime", 0) > 0)
            }));
    let hot = below_hot || i64::from(host::int(&entity.nbt, "hotUntil", 0)) > now;
    let was_hot = entity.data[6] != 0;
    entity.data[6] = i32::from(hot);
    if was_hot != hot && game.native_block_property(pos, "lit").is_some() {
        game.native_set_block_property_with(pos, "lit", if hot { "true" } else { "false" }, 3);
    }
    hot
}

/// The value a caloric flux emitter should write after a successful transfer.
pub fn hot_until_for(game: &crate::game::Game<'_>) -> i32 {
    (game.world.game_time.saturating_add(i64::from(HEAT_TARGET_TICKS))).min(i64::from(i32::MAX)) as i32
}

/// The crafting state's `lit` block property, when the machine has one. Kept
/// here so machine ticks don't each implement the rule.
pub fn mirror_lit(game: &mut crate::game::Game<'_>, pos: [i32; 3], state: &Crafting) {
    if game.native_block_property(pos, "lit").is_some() {
        game.native_set_block_property_with(
            pos,
            "lit",
            if state.is_processing { "true" } else { "false" },
            3,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_java_short_is_preserved_without_fuel() {
        assert_eq!(decrement_lit_time(-123), -123);
        assert_eq!(decrement_lit_time(0), 0);
        assert_eq!(decrement_lit_time(1), 0);
        assert_eq!(decrement_lit_time(32_767), 32_766);
    }

    #[test]
    fn hot_until_is_six_seconds_ahead() {
        assert_eq!(HEAT_TARGET_TICKS, 120);
    }
    #[test]
    fn brazier_key_is_position() {
        assert_eq!(key([1, 2, 3]), (1, 2, 3));
    }
}
