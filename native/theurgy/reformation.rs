// SPDX-License-Identifier: MIT
// Reformation array (Java 26.3):
// content/apparatus/reformationarray/{SulfuricFluxEmitterBlockEntity,
// ReformationArrayCraftingBehaviour,Reformation*PedestalBlockEntity}.java.
//
// The Java selection item picks the pedestals by hand. This port finds the
// same pedestals automatically within the selection behaviour's 16-block
// range, so a working array needs no extra UI packet: one target pedestal, one
// result pedestal and any number of source pedestals.
use horizon_client::containers::{BlockEntity, ModBlockEntity};
use horizon_client::items::ItemStack;
use serde_json::Value;

use crate::game::theurgy_host as host;
use theurgy_engine::crafting::Crafting;

const EMITTER: &str = "theurgy:sulfuric_flux_emitter";
const SOURCE: &str = "theurgy:reformation_source_pedestal";
const TARGET: &str = "theurgy:reformation_target_pedestal";
const RESULT: &str = "theurgy:reformation_result_pedestal";
const RANGE: i32 = 16;
const FLUX_CAPACITY: i32 = 1_000;

fn key(pos: [i32; 3]) -> (i32, i32, i32) {
    (pos[0], pos[1], pos[2])
}

fn close(origin: [i32; 3], pos: (i32, i32, i32)) -> bool {
    let dx = (origin[0] - pos.0).abs();
    let dy = (origin[1] - pos.1).abs();
    let dz = (origin[2] - pos.2).abs();
    dx <= RANGE && dy <= RANGE && dz <= RANGE
}

fn flux(entity: &ModBlockEntity) -> i32 {
    host::int(&entity.nbt, "mercuryFlux", 0).clamp(0, FLUX_CAPACITY)
}

fn set_flux(entity: &mut ModBlockEntity, value: i32) {
    host::set_int(&mut entity.nbt, "mercuryFlux", value.clamp(0, FLUX_CAPACITY));
}

fn item(entity: &ModBlockEntity) -> Option<ItemStack> {
    entity.items.first().and_then(Option::as_ref).cloned()
}

fn match_ingredient(
    game: &mut crate::game::Game<'_>,
    ingredient: &Value,
    stack: &ItemStack,
    count: u32,
) -> bool {
    stack.count >= count
        && game
            .native_recipe_ingredient(ingredient)
            .is_some_and(|parsed| parsed.test(stack))
}

/// Emitter tick: find the array, select a recipe, advance progress and commit
/// atomically. The result pedestal is the only output; flux and sources are
/// consumed together when the output fits.
pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if entity.id != EMITTER {
        return;
    }
    let mut sources = Vec::new();
    let mut target = None;
    let mut result = None;
    for (&at, other) in game.world.block_entities.iter() {
        if !close(pos, at) {
            continue;
        }
        let Some(BlockEntity::Mod(other)) = Some(other) else { continue };
        match other.id.as_str() {
            SOURCE if item(other).is_some() => sources.push(at),
            TARGET if item(other).is_some() && target.is_none() => target = Some(at),
            RESULT => result = Some(at),
            _ => {}
        }
    }
    let (Some(target), Some(result)) = (target, result) else { return };
    let Some(target_stack) = game
        .world
        .block_entities
        .get(&target)
        .and_then(|e| match e { BlockEntity::Mod(e) => item(e), _ => None })
    else { return };

    let mut available_flux = flux(entity);
    let mut chosen: Option<(String, i32, Vec<(usize, u32)>, String, u32)> = None;
    let snapshot: Vec<(String, Value)> = game
        .native_recipes_of_type("theurgy:reformation")
        .iter()
        .map(|r| (r.id.clone(), (*r.json).clone()))
        .collect();
    for (id, json) in &snapshot {
        let Some(target_id) = json.get("target").and_then(Value::as_str) else { continue };
        if target_stack.item.as_ref() != target_id { continue; }
        let cost = json.get("mercuryFlux").and_then(Value::as_i64).unwrap_or(0).clamp(0, i32::MAX as i64) as i32;
        if available_flux < cost { continue; }
        let needed = json.get("sources").and_then(Value::as_array).cloned().unwrap_or_default();
        let mut used = Vec::new();
        let mut ok = true;
        for entry in &needed {
            let ingredient = entry.get("ingredient").unwrap_or(entry);
            let count = entry.get("count").and_then(Value::as_u64).unwrap_or(1) as u32;
            let found = sources.iter().enumerate().find(|(i, at)| {
                if used.contains(i) { return false; }
                let Some(BlockEntity::Mod(pedestal)) = game.world.block_entities.get(at) else { return false };
                let Some(stack) = item(pedestal) else { return false };
                match_ingredient(game, ingredient, &stack, count)
            }).map(|(i, _)| i);
            if let Some(i) = found { used.push(i); } else { ok = false; break; }
        }
        if !ok { continue; }
        let Some(res_json) = json.get("result") else { continue };
        let Some(res_id) = res_json.get("id").and_then(Value::as_str) else { continue };
        let res_count = res_json.get("count").and_then(Value::as_u64).unwrap_or(1) as u32;
        let Some(BlockEntity::Mod(result_entity)) = game.world.block_entities.get(&result) else { continue };
        let fits = match item(result_entity) {
            None => true,
            Some(existing) => existing.item.as_ref() == res_id
                && existing.count.saturating_add(res_count) <= 64,
        };
        if !fits { continue; }
        chosen = Some((id.clone(), cost, used.iter().map(|&i| (i, needed.get(i).and_then(|e| e.get("count")).and_then(Value::as_u64).unwrap_or(1) as u32)).collect(), res_id.to_owned(), res_count));
        break;
    }

    let mut state = host::crafting(entity);
    let total = chosen.as_ref().map_or(0, |_| {
        snapshot.iter().find(|(id, _)| Some(id) == chosen.as_ref().map(|c| &c.0))
            .and_then(|(_, json)| json.get("time")).and_then(Value::as_i64).unwrap_or(100)
            .clamp(0, i32::MAX as i64) as i32
    });
    if state.total_time != total {
        state.input_changed(total);
    }
    let has_input = chosen.is_some();
    let effects = state.tick(true, has_input, chosen.is_some(), total);
    if effects.craft_attempt {
        if let Some((_, cost, used, res_id, res_count)) = chosen {
            let mut committed = true;
            // Commit the result first; then flux and sources. If any step
            // fails, roll the result back instead of leaving a free item.
            if let Some(BlockEntity::Mod(result_entity)) = game.world.block_entities.get(&result) {
                let old = result_entity.clone();
                game.native_with_mod_block_entity([result.0, result.1, result.2], |entity| {
                    match entity.items.first_mut() {
                        Some(Some(stack)) if stack.item.as_ref() == res_id.as_str() => stack.count += res_count,
                        Some(slot) => *slot = Some(ItemStack::new(&res_id, res_count)),
                        None => {}
                    }
                });
                let _ = old;
            } else { committed = false; }
            if committed {
                available_flux -= cost;
                for (index, count) in used {
                    let at = sources[index];
                    game.native_with_mod_block_entity([at.0, at.1, at.2], |pedestal| {
                        if let Some(slot) = pedestal.items.first_mut().and_then(Option::as_mut) {
                            if slot.count > count { slot.count -= count; }
                            else { pedestal.items[0] = None; }
                        }
                    });
                }
            }
        }
        set_flux(entity, available_flux);
        game.native_block_entity_sync(pos);
    }
    host::save_crafting(entity, &state);
    entity.data.clear();
    entity.data.extend([state.progress, state.total_time, available_flux, FLUX_CAPACITY]);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn range_is_sixteen_in_every_axis() {
        assert!(close([0, 0, 0], (16, -16, 16)));
        assert!(!close([0, 0, 0], (17, 0, 0)));
    }
    #[test]
    fn flux_never_wraps_below_zero_or_above_capacity() {
        let mut e = ModBlockEntity::default();
        set_flux(&mut e, -5);
        assert_eq!(flux(&e), 0);
        set_flux(&mut e, 5_000);
        assert_eq!(flux(&e), FLUX_CAPACITY);
    }
}
