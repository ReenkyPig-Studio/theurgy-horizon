// SPDX-License-Identifier: MIT
// Direct item interactions from Theurgy 26.3's `ItemHandlerBehaviour` classes.
//
// The Java mod has no apparatus menus. `TwoSlotItemHandlerBehaviour`,
// `DynamicOneOutputSlotItemHandlerBehaviour` and `OneSlotItemHandlerBehaviour`
// insert the held stack on right click; an empty hand takes the output first,
// then an input. Horizon opens no screen for these blocks.

use horizon_client::items::ItemStack;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Interaction {
    TwoSlot,
    DynamicOutput,
    OneInput,
    OneOutput,
}

fn interaction(block: &str) -> Option<Interaction> {
    Some(match block {
        "theurgy:calcination_oven" | "theurgy:distiller" | "theurgy:liquefaction_cauldron" => {
            Interaction::TwoSlot
        }
        "theurgy:fermentation_vat" | "theurgy:digestion_vat" => Interaction::DynamicOutput,
        "theurgy:incubator" | "theurgy:reformation_result_pedestal" => Interaction::OneOutput,
        "theurgy:pyromantic_brazier"
        | "theurgy:incubator_mercury_vessel"
        | "theurgy:incubator_salt_vessel"
        | "theurgy:incubator_sulfur_vessel"
        | "theurgy:reformation_source_pedestal"
        | "theurgy:reformation_target_pedestal"
        | "theurgy:mercury_catalyst"
        | "theurgy:sal_ammoniac_accumulator" => Interaction::OneInput,
        _ => return None,
    })
}

fn slots(kind: Interaction) -> usize {
    match kind {
        Interaction::TwoSlot => 2,
        Interaction::DynamicOutput => 4,
        Interaction::OneInput | Interaction::OneOutput => 1,
    }
}

fn same_resource(a: &ItemStack, b: &ItemStack) -> bool {
    a.item == b.item && a.damage == b.damage && a.extra == b.extra
}

/// `ItemUtil.insertItemReturnRemaining` for one slot. No other slot is changed.
fn insert_one(slot: &mut Option<ItemStack>, held: &ItemStack, max_stack: u32) -> u32 {
    if held.count == 0 {
        return 0;
    }
    let max_stack = max_stack.max(1);
    match slot {
        Some(existing) if !same_resource(existing, held) || existing.count >= max_stack => 0,
        Some(existing) => {
            let moved = held.count.min(max_stack - existing.count);
            existing.count += moved;
            moved
        }
        None => {
            let moved = held.count.min(max_stack);
            *slot = Some(ItemStack {
                item: held.item.clone(),
                count: moved,
                damage: held.damage,
                extra: held.extra.clone(),
            });
            moved
        }
    }
}

fn recipe_kind(block: &str) -> Option<&'static str> {
    Some(match block {
        "theurgy:calcination_oven" => "theurgy:calcination",
        "theurgy:distiller" => "theurgy:distillation",
        "theurgy:liquefaction_cauldron" => "theurgy:liquefaction",
        "theurgy:fermentation_vat" => "theurgy:fermentation",
        "theurgy:digestion_vat" => "theurgy:digestion",
        "theurgy:mercury_catalyst" => "theurgy:catalysation",
        "theurgy:sal_ammoniac_accumulator" => "theurgy:accumulation",
        _ => return None,
    })
}

/// `MonitoredItemStackHandler.isValid` for direct insertion. This also keeps a
/// block item being placed from disappearing into the clicked machine first.
fn accepts_input(game: &mut crate::game::Game<'_>, block: &str, stack: &ItemStack) -> bool {
    let tag = match block {
        "theurgy:incubator_mercury_vessel" => Some("theurgy:alchemical_mercuries"),
        "theurgy:incubator_salt_vessel" => Some("theurgy:alchemical_salts"),
        "theurgy:incubator_sulfur_vessel" => Some("theurgy:alchemical_sulfurs"),
        "theurgy:reformation_source_pedestal" | "theurgy:reformation_target_pedestal" => {
            Some("theurgy:alchemical_sulfurs_and_niters")
        }
        _ => None,
    };
    if let Some(tag) = tag {
        return game.res.items().item_in_tag(tag, &stack.item);
    }
    if block == "theurgy:pyromantic_brazier" {
        return game
            .res
            .items()
            .get(&stack.item)
            .is_some_and(|info| info.fuel.is_some());
    }
    let Some(kind) = recipe_kind(block) else {
        return false;
    };
    let recipes: Vec<serde_json::Value> = game
        .native_recipes_of_type(kind)
        .iter()
        .map(|recipe| (*recipe.json).clone())
        .collect();
    recipes.iter().any(|recipe| {
        theurgy_engine::recipes::ingredient_parts(kind, recipe)
            .iter()
            .any(|(ingredient, _)| {
                game.native_recipe_ingredient(ingredient)
                    .is_some_and(|ingredient| ingredient.test(stack))
            })
    })
}

fn entity_pos(game: &crate::game::Game<'_>, pos: [i32; 3]) -> ([i32; 3], String, Interaction) {
    let mut block = game.native_block(pos);
    let mut entity_pos = pos;
    if game.native_block_property(pos, "half").as_deref() == Some("upper") {
        entity_pos[1] -= 1;
        block = game.native_block(entity_pos);
    }
    let kind = interaction(&block).expect("caller checked the apparatus id");
    (entity_pos, block, kind)
}

/// Handles an apparatus click before Horizon's generic chest/barrel template.
/// Returns without claiming a click for which Java returns `PASS`.
pub fn right_click(
    game: &mut crate::game::Game<'_>,
    held: Option<&ItemStack>,
    pos: [i32; 3],
    canceled: &mut bool,
) {
    if *canceled || interaction(&game.native_block(pos)).is_none() {
        return;
    }
    let (entity_pos, block, kind) = entity_pos(game, pos);
    let slot_count = slots(kind);

    if let Some(held) = held.filter(|stack| stack.count > 0) {
        if kind == Interaction::OneOutput || !accepts_input(game, &block, held) {
            return;
        }
        let candidates: &[usize] = match kind {
            Interaction::TwoSlot => &[0],
            Interaction::DynamicOutput => &[0, 1, 2],
            Interaction::OneInput => &[0],
            Interaction::OneOutput => &[],
        };
        let max_stack = game
            .res
            .items()
            .get(&held.item)
            .map_or(64, |info| info.max_stack)
            .max(1);

        // DynamicOneOutputSlotItemHandlerBehaviour commits the first input slot
        // which accepts anything; it does not spread one click over all inputs.
        for &index in candidates {
            let unique_vat_resource = matches!(kind, Interaction::DynamicOutput)
                && game
                    .native_with_mod_block_entity(entity_pos, |entity| {
                        entity.items.iter().enumerate().any(|(other, stack)| {
                            other != index
                                && other < slot_count - 1
                                && stack.as_ref().is_some_and(|stack| same_resource(stack, held))
                        })
                    })
                    .unwrap_or(false);
            if unique_vat_resource {
                continue;
            }
            let moved = game
                .native_with_mod_block_entity(entity_pos, |entity| {
                    if entity.items.len() < slot_count {
                        entity.items.resize(slot_count, None);
                    }
                    insert_one(&mut entity.items[index], held, max_stack)
                })
                .unwrap_or(0);
            if moved > 0 {
                if let Some(carried) = game.native_carried_stack(game.selected as i32) {
                    carried.count = carried.count.saturating_sub(moved);
                }
                game.native_block_entity_sync(entity_pos);
                *canceled = true;
                return;
            }
        }
        return;
    }

    let output_first: &[usize] = match kind {
        Interaction::TwoSlot => &[1, 0],
        Interaction::DynamicOutput => &[3, 0, 1, 2],
        Interaction::OneInput | Interaction::OneOutput => &[0],
    };
    let player = game.native_acting_id();
    for &index in output_first {
        let Some(stack) = game.native_with_mod_block_entity(entity_pos, |entity| {
            entity.items.get(index).and_then(Option::as_ref).cloned()
        }) else {
            continue;
        };
        let Some(stack) = stack.filter(|stack| stack.count > 0) else {
            continue;
        };

        // Give first, then remove exactly the amount accepted by the inventory.
        // A full player inventory therefore cannot void the machine's stack.
        let remainder = game.native_give_stack(player, stack.clone());
        let left = remainder.as_ref().map_or(0, |stack| stack.count);
        let delivered = stack.count.saturating_sub(left);
        if delivered == 0 {
            continue;
        }
        let removed = game
            .native_with_mod_block_entity(entity_pos, |entity| {
                let Some(Some(slot)) = entity.items.get_mut(index) else {
                    return false;
                };
                if !same_resource(slot, &stack) || slot.count < delivered {
                    return false;
                }
                slot.count -= delivered;
                if slot.count == 0 {
                    entity.items[index] = None;
                }
                true
            })
            .unwrap_or(false);
        // There is no concurrent world mutation inside this hook. Keep a debug
        // assertion near the transaction boundary in case that ever changes.
        debug_assert!(removed, "machine stack changed during direct extraction");
        if removed {
            game.native_block_entity_sync(entity_pos);
            *canceled = true;
            return;
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_preserves_components_and_returns_remainder() {
        let mut held = ItemStack::new("theurgy:test", 20);
        held.damage = 3;
        let mut slot = None;
        assert_eq!(insert_one(&mut slot, &held, 16), 16);
        assert_eq!(slot.as_ref().unwrap().count, 16);
        assert_eq!(slot.as_ref().unwrap().damage, 3);
        assert_eq!(insert_one(&mut slot, &held, 16), 0);
        assert_eq!(insert_one(&mut slot, &ItemStack::new("minecraft:stone", 1), 64), 0);
    }

    #[test]
    fn output_only_never_accepts_a_held_stack() {
        assert_eq!(interaction("theurgy:incubator"), Some(Interaction::OneOutput));
        assert_eq!(interaction("theurgy:reformation_result_pedestal"), Some(Interaction::OneOutput));
        assert_eq!(interaction("theurgy:distiller"), Some(Interaction::TwoSlot));
        assert_eq!(interaction("minecraft:chest"), None);
    }
}
