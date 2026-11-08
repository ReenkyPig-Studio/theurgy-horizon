// SPDX-License-Identifier: MIT
use horizon_client::containers::{BlockEntity, ModBlockEntity};
use horizon_client::items::ItemStack;
use horizon_world::nbt::{Compound, JavaString, Tag};
use serde_json::Value;
use std::sync::Arc;

use theurgy_engine::machines::{commit_plan, layout, CraftPlan, MachineLayout, ProcessResult};
use theurgy_engine::recipes::{fluid_part, ingredient_parts, Item, Recipe, Tank};

use crate::game::theurgy_host;

/// Machine kinds shared with `engine::machines::layout`.
const MACHINE_KINDS: [(&str, &str); 5] = [
    ("theurgy:calcination_oven", "theurgy:calcination"),
    ("theurgy:distiller", "theurgy:distillation"),
    ("theurgy:liquefaction_cauldron", "theurgy:liquefaction"),
    ("theurgy:fermentation_vat", "theurgy:fermentation"),
    ("theurgy:digestion_vat", "theurgy:digestion"),
];

/// Tank capacity used by `registry::fluids` for fluid-capable machines.
const MACHINE_TANK_CAPACITY: i32 = 10_000;

/// Fallback max stack, used only when the item table cannot resolve an item.
const FALLBACK_MAX_STACK: u32 = 64;

/// Recipe kind for the block id, if the block is an item/fluid machine.
fn kind_for_block(block: &str) -> Option<&'static str> {
    MACHINE_KINDS
        .iter()
        .find(|(id, _)| *id == block)
        .map(|(_, kind)| *kind)
}

/// Number of item slots the machine's container exposes (input slots then
/// output slot), matching `registry::container`.
fn slot_count(kind: &str) -> usize {
    match kind {
        "theurgy:calcination" | "theurgy:distillation" | "theurgy:liquefaction" => 2,
        "theurgy:fermentation" | "theurgy:digestion" => 4,
        _ => 0,
    }
}

/// Machines whose Java block entity uses `HeatConsumerBehaviour` and gates its
/// crafting tick on `isHeated()`: calcination oven, distiller, liquefaction
/// cauldron. Fermentation and digestion run while not `OPEN` (the vat's open
/// state is the block property), and are not heat-gated.
fn is_heat_consumer(kind: &str) -> bool {
    matches!(
        kind,
        "theurgy:calcination" | "theurgy:distillation" | "theurgy:liquefaction"
    )
}

/// Reads the item slot at `index` from the authoritative `entity.items`,
/// preserving the stack's components and its actual max stack.
fn read_slot(entity: &ModBlockEntity, index: usize) -> Option<&ItemStack> {
    entity.items.get(index).and_then(Option::as_ref)
}

/// The real max stack of a live stack, resolved from the host's item table.
/// Falls back to the recipe-independent default when the item is unknown.
fn slot_max_stack(game: &crate::game::Game<'_>, stack: &ItemStack) -> u32 {
    game.native_max_stack_size(&stack.item).unwrap_or(FALLBACK_MAX_STACK)
}

fn same_resource(a: &ItemStack, b: &ItemStack) -> bool {
    a.item == b.item && a.damage == b.damage && a.extra == b.extra
}

/// Snapshot of the item slots as the engine sees them (id + count + real max
/// stack). Components are not needed by the engine's counting; the source
/// slots are read straight from `entity.items` when a craft commits.
fn read_items(
    game: &crate::game::Game<'_>,
    entity: &ModBlockEntity,
    count: usize,
) -> Vec<Option<Item>> {
    (0..count)
        .map(|i| {
            read_slot(entity, i).map(|stack| Item {
                id: stack.item.to_string(),
                count: stack.count,
                max_stack: slot_max_stack(game, stack),
            })
        })
        .collect()
}

fn input_signature(entity: &ModBlockEntity, kind: &str, recipe_id: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in kind.bytes().chain(recipe_id.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    // CraftingBehaviour.onInputItemChanged is wired to the input inventory
    // only (*StorageBehaviour): output-slot changes never reset totalTime.
    let inputs = layout(kind).map(|value| value.inputs).unwrap_or(&[]);
    for slot in inputs.iter().map(|index| entity.items.get(*index).and_then(Option::as_ref)) {
        if let Some(stack) = slot {
            for byte in format!("{:?}|{:?}|{:?}", stack.item, stack.damage, stack.extra).bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Applies the craft plan mutations back to `entity.items`. Only counts are
/// changed on existing stacks (a decrement or an output + count); the
/// components of consumed/kept stacks are never touched, and the output stack
/// is assembled with the recipe's cloned result.
///
/// The `plan` has been validated against `items` (the snapshot of the same
/// slots) by `commit_plan`; a stale plan (slot already changed by other code
/// inside the same tick) leaves the entity untouched by returning false.
fn apply_plan_to_items(
    game: &crate::game::Game<'_>,
    entity: &mut ModBlockEntity,
    plan: &CraftPlan,
    assembled: Option<&ItemStack>,
) -> bool {
    // Re-check fit against the live stacks (component-aware max stack).
    if let Some((slot, result)) = &plan.output {
        let fits = match entity.items.get(*slot).and_then(Option::as_ref) {
            None => result.count <= result.max_stack,
            Some(old) => {
                old.item.as_ref() == result.id.as_str()
                    && assembled.is_none_or(|result_stack| same_resource(old, result_stack))
                    && u64::from(old.count) + u64::from(result.count)
                        <= u64::from(slot_max_stack(game, old).min(result.max_stack))
            }
        };
        if !fits {
            return false;
        }
    }
    for &(slot, count) in &plan.inputs {
        match entity.items.get(slot).and_then(Option::as_ref) {
            Some(stack) if stack.count >= count => {}
            _ => return false,
        }
    }
    for &(slot, count) in &plan.inputs {
        if let Some(Some(stack)) = entity.items.get_mut(slot) {
            stack.count -= count;
            if stack.count == 0 {
                entity.items[slot] = None;
            }
        }
    }
    if let Some((slot, result)) = &plan.output {
        if *slot >= entity.items.len() {
            return false;
        }
        match entity.items.get_mut(*slot) {
            Some(existing @ Some(_)) => {
                if let Some(stack) = existing.as_mut() {
                    stack.count = stack.count.saturating_add(result.count);
                }
            }
            Some(slot_ref) => {
                *slot_ref = Some(assembled.cloned().unwrap_or_else(|| ItemStack::new(&result.id, result.count)));
            }
            None => return false,
        }
    }
    true
}

fn read_tank(entity: &ModBlockEntity, index: usize) -> Tank {
    let capacity = if entity.id == "theurgy:liquefaction_cauldron" {
        2_000
    } else {
        MACHINE_TANK_CAPACITY
    };
    let empty = Tank {
        fluid: None,
        amount: 0,
        capacity,
    };
    let Some(Tag::List(list)) = entity.nbt.get(&JavaString::from("fluids")) else {
        return empty;
    };
    let Some(Tag::Compound(map)) = list.get(index) else {
        return empty;
    };
    let fluid = match map.get(&JavaString::from("Fluid")) {
        Some(Tag::String(v)) => {
            let v = v.to_string_lossy();
            (v != "minecraft:empty").then_some(v)
        }
        _ => None,
    };
    let amount = match map.get(&JavaString::from("Amount")) {
        Some(Tag::Int(v)) => (*v).clamp(0, capacity),
        _ => 0,
    };
    Tank {
        fluid,
        amount,
        capacity,
    }
}

fn write_tank(entity: &mut ModBlockEntity, index: usize, tank: &Tank) {
    let mut list = match entity.nbt.get(&JavaString::from("fluids")) {
        Some(Tag::List(l)) => l.clone(),
        _ => Vec::new(),
    };
    while list.len() <= index {
        list.push(Tag::End);
    }
    let mut map = Compound::new();
    map.insert(
        JavaString::from("Fluid"),
        Tag::String(JavaString::from(
            tank.fluid.as_deref().unwrap_or("minecraft:empty"),
        )),
    );
    map.insert(JavaString::from("Amount"), Tag::Int(tank.amount));
    list[index] = Tag::Compound(map);
    entity
        .nbt
        .insert(JavaString::from("fluids"), Tag::List(list));
}

/// Build a plan from a freshly matched recipe: consume the sized count per
/// ingredient from distinct input slots, output to the output slot, and drain
/// the recipe fluid from the machine tank.
fn plan_for(
    kind: &str,
    recipe: &Recipe,
    items: &[Option<Item>],
    tanks: &[Tank],
    matches: &mut dyn FnMut(&Value, &Item) -> bool,
) -> Option<CraftPlan> {
    let layout = layout(kind)?;
    let parts = ingredient_parts(kind, recipe.json);
    if parts.is_empty() {
        return None;
    }
    let mut inputs: Vec<(usize, u32)> = Vec::new();
    let mut used: Vec<bool> = vec![false; items.len()];
    for (ingredient, count) in &parts {
        let mut chosen = None;
        for slot in layout.inputs {
            if used.get(*slot).copied().unwrap_or(true) {
                continue;
            }
            if let Some(item) = items.get(*slot).and_then(Option::as_ref) {
                if item.count >= *count && matches(ingredient, item) {
                    chosen = Some(*slot);
                    break;
                }
            }
        }
        let slot = chosen?;
        used[slot] = true;
        inputs.push((slot, *count));
    }
    let fluid = match fluid_part(kind, recipe.json) {
        Some((fluid_id, amount)) => {
            let tank_slot = layout.tank?;
            let tank = tanks.get(tank_slot)?;
            if tank.amount < amount || tank.fluid.as_deref() != Some(fluid_id) {
                return None;
            }
            Some((tank_slot, amount))
        }
        None => None,
    };
    let result = recipe.json.get("result")?;
    let id = if let Some(s) = result.get("id").and_then(Value::as_str) {
        s.to_owned()
    } else if let Some(s) = result.as_str() {
        s.to_owned()
    } else {
        return None;
    };
    let count = result
        .get("count")
        .and_then(Value::as_u64)
        .unwrap_or(1) as u32;
    if count == 0 {
        return None;
    }
    let max_stack = recipe
        .json
        .get("result")
        .and_then(|r| r.get("max_stack"))
        .and_then(Value::as_u64)
        .map(|v| v.min(99) as u32)
        .unwrap_or(FALLBACK_MAX_STACK)
        .max(1);
    Some(CraftPlan {
        inputs,
        output: Some((
            layout.output,
            Item {
                id,
                count,
                max_stack,
            },
        )),
        fluid,
    })
}

fn recipe_time(json: &Value) -> i32 {
    json.get("time")
        .and_then(Value::as_i64)
        .unwrap_or(100)
        .clamp(0, i64::from(i32::MAX)) as i32
}

fn default_crafting_time(kind: &str) -> i32 {
    if matches!(kind, "theurgy:fermentation" | "theurgy:digestion") { 200 } else { 100 }
}

fn matches_ingredient(
    game: &mut crate::game::Game<'_>,
    ingredient: &Value,
    item: &Item,
) -> bool {
    match game.native_recipe_ingredient(ingredient) {
        Some(parsed) => parsed.test(&ItemStack::new(&item.id, item.count)),
        None => false,
    }
}

/// A recipe snapshot taken out of `native_recipes_of_type`: the recipe parts
/// are cloned/`Arc`'d so the borrow on `game` ends before ingredient matching
/// (which needs `&mut game`).
struct RecipeSnapshot {
    id: String,
    json: Arc<Value>,
    time: i32,
}

pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let Some(kind) = kind_for_block(&entity.id) else {
        return;
    };
    let Some(MachineLayout {
        tank: tank_slot, ..
    }) = layout(kind)
    else {
        return;
    };
    let input_slots: &[usize] = layout(kind).map(|l| l.inputs).unwrap_or(&[]);
    let slots = slot_count(kind);
    let mut items = read_items(game, entity, slots);
    let mut tanks: Vec<Tank> = match tank_slot {
        Some(i) => vec![read_tank(entity, i)],
        None => Vec::new(),
    };
    let mut state = theurgy_host::crafting(entity);
    let old_state = state.clone();
    let mut current_signature = 0u64;

    // Heat consumers gate on the actual heat source at the machine's position;
    // fermentation/digestion run while the vat is not OPEN.
    let can_process = if is_heat_consumer(kind) {
        crate::game::theurgy_heat::heated(game, pos, entity)
    } else {
        !vat_is_open(game, pos)
    };
    let was_open = !is_heat_consumer(kind) && vat_is_open(game, pos);
    let mut selected_can_craft = false;

    let recipe_snapshot: Vec<RecipeSnapshot> = game
        .native_recipes_of_type(kind)
        .iter()
        .map(|r| RecipeSnapshot {
            id: r.id.clone(),
            json: Arc::clone(&r.json),
            time: recipe_time(&r.json),
        })
        .collect();
    // Pre-select the first recipe satisfied by the current item/fluid slots.
    let tank_for_match = tanks.first().cloned();
    let mut selected: Option<usize> = None;
    for (idx, snap) in recipe_snapshot.iter().enumerate() {
        let parts = ingredient_parts(kind, &snap.json);
        if parts.is_empty() {
            continue;
        }
        if matches!(kind, "theurgy:fermentation" | "theurgy:digestion") {
            let filled = input_slots.iter().filter(|slot| {
                items.get(**slot).is_some_and(Option::is_some)
            }).count();
            if filled != parts.len() {
                continue;
            }
        }
        let mut ok = true;
        let mut used: Vec<bool> = vec![false; items.len()];
        for (ingredient, count) in &parts {
            let mut found = false;
            for slot in input_slots {
                if used.get(*slot).copied().unwrap_or(true) {
                    continue;
                }
                let Some(item) = items.get(*slot).and_then(Option::as_ref) else {
                    continue;
                };
                if item.count < *count {
                    continue;
                }
                if matches_ingredient(game, ingredient, item) {
                    used[*slot] = true;
                    found = true;
                    break;
                }
            }
            if !found {
                ok = false;
                break;
            }
        }
        if !ok {
            continue;
        }
        if let Some((fluid_id, amount)) = fluid_part(kind, &snap.json) {
            let matches_fluid = tank_for_match
                .as_ref()
                .map(|t| t.amount >= amount && t.fluid.as_deref() == Some(fluid_id))
                .unwrap_or(false);
            if !matches_fluid {
                continue;
            }
        }
        selected = Some(idx);
        break;
    }

    let result: ProcessResult = match selected {
        None => {
            let old = state.clone();
            let previous = ((entity.data.get(8).copied().unwrap_or(0) as u32 as u64) << 32)
                | entity.data.get(7).copied().unwrap_or(0) as u32 as u64;
            let signature = input_signature(entity, kind, "");
            current_signature = signature;
            let has_items = layout(kind).map(|value| value.inputs).unwrap_or(&[]).iter()
                .any(|index| entity.items.get(*index).is_some_and(Option::is_some));
            if previous != signature && (previous != 0 || has_items) {
                // CraftingBehaviour.onInputItemChanged stores the current
                // recipe time, or the type's default even with no recipe.
                state.total_time = default_crafting_time(kind);
                state.progress = 0;
            }
            let updates = state.stop();
            ProcessResult {
                selected_recipe: None,
                crafted: false,
                state_changed: old != state || updates != 0,
            }
        }
        Some(idx) => {
            let snap = &recipe_snapshot[idx];
            let recipe = Recipe {
                id: &snap.id,
                json: &snap.json,
                time: snap.time,
            };
            let signature = input_signature(entity, kind, &snap.id);
            current_signature = signature;
            let previous = ((entity.data.get(8).copied().unwrap_or(0) as u32 as u64) << 32)
                | entity.data.get(7).copied().unwrap_or(0) as u32 as u64;
            if previous != 0 && previous != signature {
                state.input_changed(recipe.time);
            } else if state.total_time != recipe.time {
                state.input_changed(recipe.time);
            }
            let mut matcher =
                |ingredient: &Value, item: &Item| -> bool { matches_ingredient(game, ingredient, item) };
            let plan = plan_for(kind, &recipe, &items, &tanks, &mut matcher);
            let mut can_craft = false;
            if let Some(plan_ref) = plan.as_ref() {
                // Simulate the commit against cloned state, matching canCraft.
                let mut sim_items = items.clone();
                let mut sim_tanks = tanks.clone();
                can_craft = commit_plan(&mut sim_items, &mut sim_tanks, plan_ref);
            selected_can_craft = can_craft;
            }
            let result_json = recipe.json.get("result");
            let assembled = result_json.and_then(|json| game.native_recipe_result(json));
            let output_index = layout(kind).map(|value| value.output).unwrap_or(0);
            if let Some(result_stack) = assembled.as_ref() {
                if let Some(old) = read_slot(entity, output_index) {
                    if !same_resource(old, result_stack) {
                        can_craft = false;
                        selected_can_craft = false;
                    }
                }
            }
            let effects = state.tick(can_process, true, can_craft, recipe.time);
            let did_craft = if effects.craft_attempt && can_craft {
                plan.as_ref()
                    .map(|plan| {
                        if commit_plan(&mut items, &mut tanks, plan)
                            && apply_plan_to_items(game, entity, plan, assembled.as_ref())
                        {
                            true
                        } else {
                            false
                        }
                    })
                    .unwrap_or(false)
            } else {
                false
            };
            ProcessResult {
                selected_recipe: Some(snap.id.clone()),
                crafted: did_craft,
                state_changed: old_state != state || effects.block_updates != 0,
            }
        }
    };

    // Persist tank changes (only progress/tanks live in NBT; items are kept in
    // `entity.items` and were applied above).
    for (i, tank) in tanks.iter().enumerate() {
        write_tank(entity, i, tank);
    }
    theurgy_host::save_crafting(entity, &state);
    // Menu data slots: progress, total time, fluid amount, capacity. The
    // screen reads these as i32 and clamps them itself.
    let heat_cache = entity.data.get(6).copied().unwrap_or(0);
    entity.data.clear();
    entity.data.extend([
        state.progress.max(0),
        state.total_time.max(0),
        tanks.first().map_or(0, |tank| tank.amount.max(0)),
        tanks.first().map_or(0, |tank| tank.capacity.max(0)),
        i32::from(state.is_processing),
        i32::from(state.could_craft_last_tick),
        heat_cache,
        current_signature as u32 as i32,
        (current_signature >> 32) as u32 as i32,
    ]);
    if !is_heat_consumer(kind) {
        let facing = game.native_block_property(pos, "facing").unwrap_or_else(|| "north".into());
        let _ = facing;
        let offset_key = JavaString::from("randomSlowTickOffset");
        if !entity.nbt.contains_key(&offset_key) {
            use horizon_core::random::RandomSource;
            let offset = game.world.random.next_int_bound(20).unwrap_or(0);
            entity.nbt.insert(offset_key, Tag::Int(offset));
        }
        let offset = theurgy_host::int(&entity.nbt, "randomSlowTickOffset", 0);
        if was_open && selected_can_craft
            && (game.world.game_time + i64::from(offset)) % 20 == 0
            && game.native_neighbor_signal(game.dim.id(), pos) > 0
        {
            game.native_set_block_property_with(pos, "open", "false", 2);
        }
        let redstone_signal = game.native_neighbor_signal(game.dim.id(), pos);
        let previous_signal = theurgy_host::int(&entity.nbt, "lastRedstoneSignal", redstone_signal);
        if previous_signal != redstone_signal {
            // VatRedstoneChangeOpenCloseLidBehaviour opens only on a
            // high-to-low transition, not whenever processing is idle.
            if redstone_signal == 0 && !was_open {
                game.native_set_block_property_with(pos, "open", "true", 2);
            }
            theurgy_host::set_int(&mut entity.nbt, "lastRedstoneSignal", redstone_signal);
        }
        if kind == "theurgy:fermentation" {
            let has_output = read_slot(entity, 3).is_some_and(|stack| stack.count > 0);
            if game.native_block_property(pos, "has_output").as_deref() != Some(if has_output { "true" } else { "false" }) {
                game.native_set_block_property_with(pos, "has_output", if has_output { "true" } else { "false" }, 3);
            }
        }
    }
    if result.crafted {
        let mut options = horizon_client::particles::ParticleOptions::named("theurgy:colored_bubble");
        options.color = [0.35, 0.8, 1.0, 1.0];
        options.scale = 0.75;
        let at = [pos[0] as f64 + 0.5, pos[1] as f64 + 3.0, pos[2] as f64 + 0.5];
        let dust = horizon_client::particles::ParticleOptions::dust([0.35, 0.8, 1.0], 0.75);
        for _ in 0..16 {
            game.particles.add(&dust, at, [0.0, 0.03, 0.0]);
            game.particles.add(&options, at, [0.0, 0.03, 0.0]);
        }
        game.native_flame_particle(at, false);
        game.with_level(|_, level, _| level.native_send_particles(&dust, None, at, [0.25, 0.35, 0.25], 0.02, 4));
        game.with_level(|_, level, _| level.native_send_particles(&options, None, at, [0.25, 0.35, 0.25], 0.02, 4));
    }
    if result.state_changed || result.crafted {
        game.native_block_entity_sync(pos);
    }
}

/// `GenericVatInteractionBehaviour.useItemOn`: closing is allowed only when
/// the current items/fluid form a complete recipe whose whole output fits.
pub(crate) fn vat_can_craft(game: &mut crate::game::Game<'_>, pos: [i32; 3]) -> bool {
    let Some(entity) = game.world.block_entities.get(&(pos[0], pos[1], pos[2])).and_then(|block| match block {
        BlockEntity::Mod(entity) => Some(entity.as_ref().clone()),
        _ => None,
    }) else { return false; };
    let Some(kind) = kind_for_block(&entity.id)
        .filter(|kind| matches!(*kind, "theurgy:fermentation" | "theurgy:digestion")) else { return false; };
    let items = read_items(game, &entity, 4);
    let tanks = vec![read_tank(&entity, 0)];
    let snapshots: Vec<RecipeSnapshot> = game.native_recipes_of_type(kind).iter()
        .map(|recipe| RecipeSnapshot { id: recipe.id.clone(), json: Arc::clone(&recipe.json), time: recipe_time(&recipe.json) })
        .collect();
    for snap in snapshots {
        let parts = ingredient_parts(kind, &snap.json);
        if parts.is_empty() || (0..3).filter(|slot| items.get(*slot).is_some_and(Option::is_some)).count() != parts.len() {
            continue;
        }
        let mut used = [false; 4];
        let mut matched = true;
        for (ingredient, count) in &parts {
            let mut found = false;
            for slot in 0..3 {
                if used[slot] { continue; }
                let Some(item) = items.get(slot).and_then(Option::as_ref) else { continue; };
                if item.count >= *count && matches_ingredient(game, ingredient, item) {
                    used[slot] = true;
                    found = true;
                    break;
                }
            }
            if !found { matched = false; break; }
        }
        if !matched { continue; }
        if let Some((fluid, amount)) = fluid_part(kind, &snap.json)
            && !tanks.first().is_some_and(|tank| tank.amount >= amount && tank.fluid.as_deref() == Some(fluid))
        {
            continue;
        }
        let recipe = Recipe { id: &snap.id, json: &snap.json, time: snap.time };
        let mut matcher = |ingredient: &Value, item: &Item| matches_ingredient(game, ingredient, item);
        let Some(plan) = plan_for(kind, &recipe, &items, &tanks, &mut matcher) else { continue; };
        let mut sim_items = items.clone();
        let mut sim_tanks = tanks.clone();
        if !commit_plan(&mut sim_items, &mut sim_tanks, &plan) { continue; }
        if let Some(result) = snap.json.get("result").and_then(|json| game.native_recipe_result(json))
            && read_slot(&entity, 3).is_some_and(|old| !same_resource(old, &result))
        {
            continue;
        }
        return true;
    }
    false
}

/// Whether a fermentation/digestion vat's `open` block property is set. Both
/// vats gate crafting on `!OPEN`; the open state is exposed through the block
/// state of the vat at `pos`.
fn vat_is_open(game: &mut crate::game::Game<'_>, pos: [i32; 3]) -> bool {
    match game.native_block_property(pos, "open") {
        Some(value) => value == "true",
        None => false,
    }
}

fn vat_block(state: horizon_world::registry::BlockStateId, level: &crate::game::GameLevel<'_>) -> Option<&'static str> {
    let state = level.registry.state(state)?;
    match state.name.as_str() {
        "theurgy:fermentation_vat" => Some("theurgy:fermentation_vat"),
        "theurgy:digestion_vat" => Some("theurgy:digestion_vat"),
        _ => None,
    }
}

/// `VatRedstoneHasOutputBehaviour`: only the block's front emits 15.
pub fn block_signal(
    level: &crate::game::GameLevel<'_>,
    pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId,
    face: horizon_core::geometry::Direction,
    signal: &mut i32,
) {
    let Some(_) = vat_block(state, level) else { return; };
    let facing = level.registry.state(state)
        .and_then(|descriptor| descriptor.properties.get("facing"))
        .and_then(|value| crate::game::theurgy_logistics::facing_name_to_dir(value))
        .unwrap_or(2);
    let face_id = match face {
        horizon_core::geometry::Direction::Down => 0,
        horizon_core::geometry::Direction::Up => 1,
        horizon_core::geometry::Direction::North => 2,
        horizon_core::geometry::Direction::South => 3,
        horizon_core::geometry::Direction::West => 4,
        horizon_core::geometry::Direction::East => 5,
    };
    if face_id != facing {
        *signal = 0;
        return;
    }
    let has_output = level.world.block_entities.get(&(pos.x, pos.y, pos.z)).is_some_and(|entity| {
        let horizon_client::containers::BlockEntity::Mod(entity) = entity else { return false; };
        entity.items.get(3).is_some_and(Option::is_some)
    });
    *signal = if has_output { 15 } else { 0 };
}

pub fn is_signal_source(
    level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId,
    source: &mut bool,
) {
    if vat_block(state, level).is_some() {
        *source = true;
    }
}

pub fn has_analog_output(
    level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId,
    output: &mut bool,
) {
    if vat_block(state, level).is_some() {
        *output = true;
    }
}

/// `VatAnalogSignalOutputBehaviour`: one output slot, vanilla container
/// formula (nonempty = 1 + floor(fill * 14)).
pub fn analog_output(
    level: &crate::game::GameLevel<'_>,
    pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId,
    output: &mut i32,
) {
    if vat_block(state, level).is_none() {
        return;
    }
    *output = level.world.block_entities.get(&(pos.x, pos.y, pos.z)).map_or(0, |entity| {
        let horizon_client::containers::BlockEntity::Mod(entity) = entity else { return 0; };
        entity.items.get(3).and_then(Option::as_ref).map_or(0, |stack| {
            if stack.count == 0 {
                return 0;
            }
            let max = level.items.get(&stack.item).map_or(64, |info| info.max_stack).max(1);
            1 + ((u64::from(stack.count) * 14) / u64::from(max)) as i32
        })
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_machine_block_maps_to_a_recipe_kind() {
        assert_eq!(
            kind_for_block("theurgy:calcination_oven"),
            Some("theurgy:calcination")
        );
        assert_eq!(
            kind_for_block("theurgy:distiller"),
            Some("theurgy:distillation")
        );
        assert_eq!(
            kind_for_block("theurgy:liquefaction_cauldron"),
            Some("theurgy:liquefaction")
        );
        assert_eq!(
            kind_for_block("theurgy:fermentation_vat"),
            Some("theurgy:fermentation")
        );
        assert_eq!(
            kind_for_block("theurgy:digestion_vat"),
            Some("theurgy:digestion")
        );
    }

    #[test]
    fn unrelated_blocks_are_ignored() {
        assert_eq!(kind_for_block("theurgy:pyromantic_brazier"), None);
        assert_eq!(kind_for_block("minecraft:furnace"), None);
    }

    #[test]
    fn slot_counts_match_container_shape() {
        assert_eq!(slot_count("theurgy:calcination"), 2);
        assert_eq!(slot_count("theurgy:distillation"), 2);
        assert_eq!(slot_count("theurgy:liquefaction"), 2);
        assert_eq!(slot_count("theurgy:fermentation"), 4);
        assert_eq!(slot_count("theurgy:digestion"), 4);
    }

    #[test]
    fn heat_consumer_classification_matches_source() {
        // The three heat-consumer machines.
        assert!(is_heat_consumer("theurgy:calcination"));
        assert!(is_heat_consumer("theurgy:distillation"));
        assert!(is_heat_consumer("theurgy:liquefaction"));
        // The vats gate on OPEN, not heat.
        assert!(!is_heat_consumer("theurgy:fermentation"));
        assert!(!is_heat_consumer("theurgy:digestion"));
    }

    #[test]
    fn recipe_time_defaults_and_clamps() {
        use serde_json::json;
        assert_eq!(recipe_time(&json!({})), 100);
        assert_eq!(recipe_time(&json!({"time": 50})), 50);
        assert_eq!(recipe_time(&json!({"time": -5})), 0);
    }

    #[test]
    fn plan_uses_declared_max_stack_from_result() {
        use serde_json::json;
        let json = json!({
            "ingredient": "minecraft:oak_log",
            "result": {"id": "theurgy:x", "count": 1, "max_stack": 16},
            "time": 100
        });
        let recipe = Recipe {
            id: "test",
            json: &json,
            time: 100,
        };
        let items = vec![
            Some(Item {
                id: "minecraft:oak_log".into(),
                count: 1,
                max_stack: 64,
            }),
            None,
        ];
        let mut matcher = |_: &Value, _item: &Item| true;
        let plan = plan_for("theurgy:distillation", &recipe, &items, &[], &mut matcher)
            .expect("plan");
        assert_eq!(plan.output.as_ref().unwrap().1.max_stack, 16);
    }

    #[test]
    fn plan_rejects_shorted_fluid() {
        use serde_json::json;
        let json = json!({
            "ingredient": "minecraft:oak_log",
            "solvent": {"ingredient": "theurgy:sal_ammoniac", "amount": 10},
            "result": {"id": "theurgy:x", "count": 1},
            "time": 100
        });
        let recipe = Recipe {
            id: "test",
            json: &json,
            time: 100,
        };
        let items = vec![
            Some(Item {
                id: "minecraft:oak_log".into(),
                count: 1,
                max_stack: 64,
            }),
            None,
        ];
        let tanks = vec![Tank {
            fluid: Some("theurgy:sal_ammoniac".into()),
            amount: 9,
            capacity: MACHINE_TANK_CAPACITY,
        }];
        let mut matcher = |_: &Value, _item: &Item| true;
        assert!(plan_for("theurgy:liquefaction", &recipe, &items, &tanks, &mut matcher).is_none());
    }

    #[test]
    fn plan_uses_only_input_slots_and_distinct_slots() {
        use serde_json::json;
        // Fermentation: two ingredients must come from distinct input slots.
        let json = json!({
            "ingredients": [
                {"ingredient": "minecraft:a", "count": 1},
                {"ingredient": "minecraft:a", "count": 1}
            ],
            "fluid": {"ingredient": "minecraft:water", "amount": 100},
            "result": {"id": "theurgy:out", "count": 1},
            "time": 100
        });
        let recipe = Recipe {
            id: "t",
            json: &json,
            time: 100,
        };
        let items = vec![
            Some(Item {
                id: "minecraft:a".into(),
                count: 2,
                max_stack: 64,
            }),
            None,
            None,
            None,
        ];
        let tanks = vec![Tank {
            fluid: Some("minecraft:water".into()),
            amount: 100,
            capacity: MACHINE_TANK_CAPACITY,
        }];
        let mut matcher = |_: &Value, _item: &Item| true;
        // Only one stack, cannot satisfy two separate ingredient matches from
        // distinct slots -> plan fails.
        assert!(plan_for("theurgy:fermentation", &recipe, &items, &tanks, &mut matcher).is_none());
    }

    #[test]
    fn tank_roundtrip_preserves_fluid_and_amount() {
        let mut entity = ModBlockEntity {
            id: "theurgy:liquefaction_cauldron".into(),
            nbt: Compound::new(),
            items: Vec::new(),
            data: Vec::new(),
        };
        let tank = Tank {
            fluid: Some("theurgy:sal_ammoniac".into()),
            amount: 500,
            capacity: MACHINE_TANK_CAPACITY,
        };
        write_tank(&mut entity, 0, &tank);
        assert_eq!(read_tank(&entity, 0), tank);
        // Absent tank reads as empty.
        assert_eq!(read_tank(&entity, 1).fluid, None);
        assert_eq!(read_tank(&entity, 1).amount, 0);
    }
}
