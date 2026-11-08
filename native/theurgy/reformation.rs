// SPDX-License-Identifier: MIT
// Reformation array from Java 26.3:
// `SulfuricFluxEmitterBlockEntity`, `ReformationArrayCraftingBehaviour` and
// `ReformationRecipe`. Only pedestals explicitly selected by the player are
// part of an array; there is deliberately no world scan or nearest match.

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
const FLUX_CAPACITY: i32 = 1_000;
const DEFAULT_TIME: i32 = 100;

fn handler_key() -> JavaString {
    JavaString::from("mercuryFluxHandler")
}

fn flux(entity: &ModBlockEntity) -> i32 {
    let Some(Tag::Compound(handler)) = entity.nbt.get(&handler_key()) else {
        return 0;
    };
    host::int(handler, "energy", 0).clamp(0, FLUX_CAPACITY)
}

fn set_flux(entity: &mut ModBlockEntity, value: i32) {
    let clamped = value.clamp(0, FLUX_CAPACITY);
    let mut handler = match entity.nbt.get(&handler_key()) {
        Some(Tag::Compound(existing)) => existing.clone(),
        _ => Compound::new(),
    };
    host::set_int(&mut handler, "energy", clamped);
    entity.nbt.insert(handler_key(), Tag::Compound(handler));
}

fn same_resource(a: &ItemStack, b: &ItemStack) -> bool {
    a.item == b.item && a.damage == b.damage && a.extra == b.extra
}

fn pedestal_item(game: &crate::game::Game<'_>, at: [i32; 3], expected: &str) -> Option<ItemStack> {
    let key = (at[0], at[1], at[2]);
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get(&key) else {
        return None;
    };
    if entity.id != expected {
        return None;
    }
    entity
        .items
        .first()
        .and_then(Option::as_ref)
        .filter(|stack| stack.count > 0)
        .cloned()
}

fn pedestal_exists(game: &crate::game::Game<'_>, at: [i32; 3], expected: &str) -> bool {
    let key = (at[0], at[1], at[2]);
    matches!(
        game.world.block_entities.get(&key),
        Some(BlockEntity::Mod(entity)) if entity.id == expected
    )
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
        .unwrap_or(i64::from(fallback))
        .clamp(0, i64::from(i32::MAX)) as i32
}

fn as_u32(value: Option<&Value>, fallback: u32) -> u32 {
    value
        .and_then(Value::as_u64)
        .unwrap_or(u64::from(fallback))
        .min(u64::from(u32::MAX)) as u32
}

#[derive(Clone)]
struct SourceUse {
    pos: [i32; 3],
    count: u32,
    expected: ItemStack,
}

struct RecipeChoice {
    cost: i32,
    time: i32,
    sources: Vec<SourceUse>,
    result: ItemStack,
}

fn output_fits(existing: Option<ItemStack>, result: &ItemStack, max_stack: u32) -> bool {
    match existing {
        None => result.count <= max_stack,
        Some(old) => {
            same_resource(&old, result)
                && u64::from(old.count) + u64::from(result.count) <= u64::from(max_stack)
        }
    }
}

/// Reads only the three Java selection fields. Missing or malformed selections
/// mean an unassembled array, never permission to search nearby block entities.
fn selection(entity: &ModBlockEntity) -> (Vec<[i32; 3]>, Option<[i32; 3]>, Option<[i32; 3]>) {
    let mut sources = Vec::new();
    for pos in crate::game::theurgy_selection::read_points(&entity.nbt, "sourcePedestals") {
        if !sources.contains(&pos) {
            sources.push(pos);
        }
    }
    (
        sources,
        crate::game::theurgy_selection::read_point(&entity.nbt, "targetPedestal"),
        crate::game::theurgy_selection::read_point(&entity.nbt, "resultPedestal"),
    )
}

pub fn trail_color(start: [f32; 3], end: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|axis| start[axis] + (end[axis] - start[axis]) * t)
}

/// Emitter tick from `SulfuricFluxEmitterBlockEntity.tickServer` and the
/// transaction in `ReformationArrayCraftingBehaviour.craft`.
pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if entity.id != EMITTER {
        return;
    }
    let (selected_sources, selected_target, selected_result) = selection(entity);
    let (Some(target_pos), Some(result_pos)) = (selected_target, selected_result) else {
        stop_idle(entity);
        return;
    };
    if !pedestal_exists(game, result_pos, RESULT) {
        stop_idle(entity);
        return;
    }
    let Some(target_stack) = pedestal_item(game, target_pos, TARGET) else {
        stop_idle(entity);
        return;
    };
    let sources: Vec<[i32; 3]> = selected_sources
        .into_iter()
        .filter(|source| pedestal_item(game, *source, SOURCE).is_some())
        .collect();
    if sources.is_empty() {
        stop_idle(entity);
        return;
    }

    let available_flux = flux(entity);
    let snapshots: Vec<(String, Value)> = game
        .native_recipes_of_type("theurgy:reformation")
        .iter()
        .map(|recipe| (recipe.id.clone(), (*recipe.json).clone()))
        .collect();
    let mut chosen: Option<RecipeChoice> = None;
    for (_id, json) in &snapshots {
        let cost = as_i32(json.get("mercuryFlux"), 0);
        if available_flux < cost {
            continue;
        }
        let Some(target_ingredient) = json.get("target") else {
            continue;
        };
        if !match_ingredient(game, target_ingredient, &target_stack, 1) {
            continue;
        }
        let Some(required) = json.get("sources").and_then(Value::as_array) else {
            continue;
        };
        let mut used: Vec<SourceUse> = Vec::new();
        let mut complete = true;
        for entry in required {
            let ingredient = entry.get("ingredient").unwrap_or(entry);
            let count = as_u32(entry.get("count"), 1);
            let found = sources.iter().find_map(|source| {
                if used.iter().any(|previous| previous.pos == *source) {
                    return None;
                }
                let stack = pedestal_item(game, *source, SOURCE)?;
                match_ingredient(game, ingredient, &stack, count).then(|| SourceUse {
                    pos: *source,
                    count,
                    expected: stack,
                })
            });
            if let Some(found) = found {
                used.push(found);
            } else {
                complete = false;
                break;
            }
        }
        if !complete {
            continue;
        }
        let Some(result_json) = json.get("result") else {
            continue;
        };
        let Some(mut result_stack) = game.native_recipe_result(result_json) else {
            continue;
        };
        // ReformationRecipe.assemble applies all target components to the result.
        if target_stack.damage > 0 {
            result_stack.damage = target_stack.damage;
        }
        if target_stack.extra.is_some() {
            result_stack.extra = target_stack.extra.clone();
        }
        let max_stack = game
            .res
            .items()
            .get(&result_stack.item)
            .map_or(64, |info| info.max_stack)
            .max(1);
        let existing = pedestal_item(game, result_pos, RESULT);
        if !output_fits(existing, &result_stack, max_stack) {
            continue;
        }
        chosen = Some(RecipeChoice {
            cost,
            time: as_i32(json.get("time"), DEFAULT_TIME),
            sources: used,
            result: result_stack,
        });
        break;
    }

    let mut state = host::crafting(entity);
    let total = chosen.as_ref().map_or(DEFAULT_TIME, |choice| choice.time);
    if state.total_time != total {
        state.input_changed(total);
    }
    // The selected multiblock has input even when no recipe currently matches.
    // CraftingBehaviour uses canCraft separately for progress and rollback.
    let effects = state.tick(true, true, chosen.is_some(), total);
    let mut current_flux = available_flux;
    let mut crafted = false;

    if effects.craft_attempt && let Some(choice) = chosen {
        // Revalidate the complete transaction before the first write. The game
        // is single-threaded inside this hook, so all subsequent writes are the
        // commit of this checked plan, not a best-effort partial craft.
        let sources_ok = choice.sources.iter().all(|source| {
            pedestal_item(game, source.pos, SOURCE)
                .is_some_and(|stack| same_resource(&stack, &source.expected) && stack.count >= source.count)
        });
        let max_stack = game
            .res
            .items()
            .get(&choice.result.item)
            .map_or(64, |info| info.max_stack)
            .max(1);
        let output_ok = output_fits(
            pedestal_item(game, result_pos, RESULT),
            &choice.result,
            max_stack,
        );
        if sources_ok && output_ok && current_flux >= choice.cost {
            for source in &choice.sources {
                let committed = game
                    .native_with_mod_block_entity(source.pos, |pedestal| {
                        let Some(Some(stack)) = pedestal.items.first_mut() else {
                            return false;
                        };
                        if !same_resource(stack, &source.expected) || stack.count < source.count {
                            return false;
                        }
                        stack.count -= source.count;
                        if stack.count == 0 {
                            pedestal.items[0] = None;
                        }
                        true
                    })
                    .unwrap_or(false);
                debug_assert!(committed, "revalidated source changed during reformation commit");
                game.native_block_entity_sync(source.pos);
            }
            let inserted = game
                .native_with_mod_block_entity(result_pos, |pedestal| {
                    if pedestal.items.is_empty() {
                        pedestal.items.push(None);
                    }
                    match &mut pedestal.items[0] {
                        Some(existing) if same_resource(existing, &choice.result) => {
                            existing.count += choice.result.count;
                            true
                        }
                        Some(existing) => {
                            *existing = choice.result.clone();
                            true
                        }
                        slot @ None => {
                            *slot = Some(choice.result.clone());
                            true
                        }
                    }
                })
                .unwrap_or(false);
            debug_assert!(inserted, "revalidated output changed during reformation commit");
            current_flux -= choice.cost;
            crafted = true;
            state.progress = 0;
            game.native_block_entity_sync(result_pos);
        }
    }

    set_flux(entity, current_flux);
    host::save_crafting(entity, &state);
    entity.data.clear();
    entity
        .data
        .extend([state.progress, state.total_time, current_flux, FLUX_CAPACITY,
            i32::from(state.is_processing), i32::from(state.could_craft_last_tick)]);
    if effects.block_updates > 0 || crafted {
        game.native_block_entity_sync(pos);
    }
}

fn stop_idle(entity: &mut ModBlockEntity) {
    let mut state = host::crafting(entity);
    let effects = state.tick(true, false, false, DEFAULT_TIME);
    host::save_crafting(entity, &state);
    let current_flux = flux(entity);
    entity.data.clear();
    entity.data.extend([
        state.progress,
        state.total_time,
        current_flux,
        FLUX_CAPACITY,
        i32::from(state.is_processing),
        i32::from(state.could_craft_last_tick),
    ]);
    let _ = effects;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projectile_color_lerp_matches_java_endpoints() {
        let start = [2.0 / 3.0, 8.0 / 255.0, 2.0 / 3.0];
        let end = [0.0, 1.0, 0.0];
        for (step, expected) in [(0.0, start), (1.0, end)] {
            let color = trail_color(start, end, step);
            assert_eq!(color, expected);
        }
    }

    #[test]
    fn flux_never_wraps_below_zero_or_above_capacity() {
        let mut entity = ModBlockEntity::default();
        set_flux(&mut entity, -5);
        assert_eq!(flux(&entity), 0);
        set_flux(&mut entity, 5_000);
        assert_eq!(flux(&entity), FLUX_CAPACITY);
    }

    #[test]
    fn malformed_handler_is_safe() {
        let mut entity = ModBlockEntity::default();
        entity.nbt.insert(handler_key(), Tag::Int(7));
        assert_eq!(flux(&entity), 0);
        set_flux(&mut entity, 42);
        assert_eq!(flux(&entity), 42);
    }

    #[test]
    fn no_selection_means_no_sources_or_pedestals() {
        let entity = ModBlockEntity::default();
        assert_eq!(selection(&entity), (Vec::new(), None, None));
    }

    #[test]
    fn output_requires_the_same_components() {
        let result = ItemStack::new("theurgy:result", 1);
        let mut renamed = result.clone();
        renamed.data_mut().set_other(
            "minecraft:custom_name",
            Some(Tag::String(JavaString::from("different"))),
        );
        assert!(!output_fits(Some(renamed), &result, 64));
    }
}
