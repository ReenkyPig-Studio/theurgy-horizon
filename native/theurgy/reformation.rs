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
use horizon_world::nbt::{Compound, JavaString, Tag};
use serde_json::Value;

use crate::game::theurgy_host as host;
use theurgy_engine::crafting::Crafting;

const EMITTER: &str = "theurgy:sulfuric_flux_emitter";
const SOURCE: &str = "theurgy:reformation_source_pedestal";
const TARGET: &str = "theurgy:reformation_target_pedestal";
const RESULT: &str = "theurgy:reformation_result_pedestal";
const RANGE: i32 = 16;
const FLUX_CAPACITY: i32 = 1_000;
const DEFAULT_TIME: i32 = 100;
const MAX_STACK: u32 = 64;

fn key(pos: [i32; 3]) -> (i32, i32, i32) {
    (pos[0], pos[1], pos[2])
}

fn close(origin: [i32; 3], pos: (i32, i32, i32)) -> bool {
    let dx = (origin[0] - pos.0).abs();
    let dy = (origin[1] - pos.1).abs();
    let dz = (origin[2] - pos.2).abs();
    dx <= RANGE && dy <= RANGE && dz <= RANGE
}

fn handler_key() -> JavaString {
    JavaString::from("mercuryFluxHandler")
}

fn energy_key() -> JavaString {
    JavaString::from("energy")
}

/// Read the Java `mercuryFluxHandler` child compound's `energy` int. An absent
/// child (or absent/malformed energy tag) means zero.
fn flux(entity: &ModBlockEntity) -> i32 {
    let Some(Tag::Compound(handler)) = entity.nbt.get(&handler_key()) else {
        return 0;
    };
    host::int(handler, "energy", 0).clamp(0, FLUX_CAPACITY)
}

/// Write the Java `mercuryFluxHandler` child compound with an `energy` int,
/// preserving any other children of the handler.
fn set_flux(entity: &mut ModBlockEntity, value: i32) {
    let clamped = value.clamp(0, FLUX_CAPACITY);
    let mut handler = match entity.nbt.get(&handler_key()) {
        Some(Tag::Compound(existing)) => existing.clone(),
        _ => Compound::new(),
    };
    host::set_int(&mut handler, "energy", clamped);
    entity.nbt.insert(handler_key(), Tag::Compound(handler));
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

fn as_i32(value: Option<&Value>, fallback: i32) -> i32 {
    value
        .and_then(Value::as_i64)
        .unwrap_or(fallback as i64)
        .clamp(0, i32::MAX as i64) as i32
}

fn as_u32(value: Option<&Value>, fallback: u32) -> u32 {
    value
        .and_then(Value::as_u64)
        .unwrap_or(fallback as u64)
        .min(u32::MAX as u64) as u32
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
        let cost = as_i32(json.get("mercuryFlux"), 0);
        if available_flux < cost { continue; }
        let needed = json.get("sources").and_then(Value::as_array).cloned().unwrap_or_default();
        let mut used: Vec<(usize, u32)> = Vec::new();
        let mut ok = true;
        for entry in &needed {
            let ingredient = entry.get("ingredient").unwrap_or(entry);
            let count = as_u32(entry.get("count"), 1);
            let found = sources.iter().enumerate().find(|(i, at)| {
                if used.iter().any(|(u, _)| u == i) { return false; }
                let Some(BlockEntity::Mod(pedestal)) = game.world.block_entities.get(at) else { return false; };
                let Some(stack) = item(pedestal) else { return false; };
                match_ingredient(game, ingredient, &stack, count)
            }).map(|(i, _)| i);
            if let Some(i) = found { used.push((i, count)); } else { ok = false; break; }
        }
        if !ok { continue; }
        let Some(res_json) = json.get("result") else { continue };
        let Some(res_id) = res_json.get("id").and_then(Value::as_str) else { continue };
        let res_count = as_u32(res_json.get("count"), 1);
        let Some(BlockEntity::Mod(result_entity)) = game.world.block_entities.get(&result) else { continue };
        let fits = match item(result_entity) {
            None => true,
            Some(existing) => existing.item.as_ref() == res_id
                && existing.count.saturating_add(res_count) <= MAX_STACK,
        };
        if !fits { continue; }
        chosen = Some((id.clone(), cost, used, res_id.to_owned(), res_count));
        break;
    }

    let mut state = host::crafting(entity);
    let total = chosen.as_ref().map_or(DEFAULT_TIME, |c| {
        snapshot.iter().find(|(id, _)| *id == c.0)
            .map(|(_, json)| as_i32(json.get("time"), DEFAULT_TIME))
            .unwrap_or(DEFAULT_TIME)
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
    #[test]
    fn absent_handler_means_zero_flux() {
        let e = ModBlockEntity::default();
        assert_eq!(flux(&e), 0);
    }
    #[test]
    fn malformed_handler_is_safe() {
        let mut e = ModBlockEntity::default();
        e.nbt.insert(handler_key(), Tag::Int(7));
        assert_eq!(flux(&e), 0);
        set_flux(&mut e, 42);
        assert_eq!(flux(&e), 42);
    }
}
