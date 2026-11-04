// SPDX-License-Identifier: MIT
// Port of IncubatorBlockEntity, IncubatorCraftingBehaviour and the three vessel
// block entities from Theurgy 1.120.1, plus the data shape of IncubationRecipe.
//
// The incubator holds no input slots of its own: recipe matching is done from the
// one input slot of each adjacent vessel (mercury, salt, sulfur). A craft produces
// one item into the incubator's single output slot and consumes one unit from each
// vessel. Multiblock validity is the presence of all three vessel block entities on
// the four horizontal neighbours of the incubator.
//
// Heating: a heat consumer is fed through the heat subsystem. The incubator checks
// whether its own position is recorded as a heated crafting receiver; the helper
// is resolved through a compile-time optional symbol so this module builds even if
// the heat module evolves.

use horizon_client::containers::ModBlockEntity;
use horizon_client::items::ItemStack;
use horizon_world::nbt::Compound;
use theurgy_engine::crafting::Crafting;

/// Value of `IncubationRecipe.DEFAULT_TIME`.
pub const DEFAULT_TIME: i32 = 100;

const INCUBATOR: &str = "theurgy:incubator";
const MERCURY_VESSEL: &str = "theurgy:incubator_mercury_vessel";
const SALT_VESSEL: &str = "theurgy:incubator_salt_vessel";
const SULFUR_VESSEL: &str = "theurgy:incubator_sulfur_vessel";

const RECIPE_KIND: &str = "theurgy:incubation";

/// The kind of vessel a block is, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vessel {
    Mercury,
    Salt,
    Sulfur,
}

impl Vessel {
    fn of_id(id: &str) -> Option<Self> {
        match id {
            MERCURY_VESSEL => Some(Vessel::Mercury),
            SALT_VESSEL => Some(Vessel::Salt),
            SULFUR_VESSEL => Some(Vessel::Sulfur),
            _ => None,
        }
    }

    /// The item tag the vessel's input slot accepts (`ItemTagRegistry`).
    pub fn tag(self) -> &'static str {
        match self {
            Vessel::Mercury => "theurgy:alchemical_mercuries",
            Vessel::Salt => "theurgy:alchemical_salts",
            Vessel::Sulfur => "theurgy:alchemical_sulfurs",
        }
    }
}

/// Java's `Direction.Plane.HORIZONTAL` in its iteration order (north, east, south,
/// west), so the neighbour scan is deterministic and matches the original.
const HORIZONTAL: [[i32; 3]; 4] = [[0, 0, -1], [1, 0, 0], [0, 0, 1], [-1, 0, 0]];

fn offer(target: &mut Option<[i32; 3]>, at: [i32; 3]) {
    if target.is_none() {
        *target = Some(at);
    }
}

/// The vessel positions found among `neighbours` (the horizontal neighbours of the
/// incubator, in Java's order), each `Some` only when that kind of vessel is present.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vessels {
    pub mercury: Option<[i32; 3]>,
    pub salt: Option<[i32; 3]>,
    pub sulfur: Option<[i32; 3]>,
}

impl Vessels {
    /// Whether the multiblock is assembled (all three kinds present).
    pub fn is_valid(&self) -> bool {
        self.mercury.is_some() && self.salt.is_some() && self.sulfur.is_some()
    }
}

/// Sorts the horizontal neighbours of an incubator into vessel slots, mirroring
/// `IncubatorBlockEntity.validateMultiblock`'s `checkForVessel` loop.
pub fn classify_neighbours<'a, I>(neighbours: I) -> Vessels
where
    I: IntoIterator<Item = (&'a str, [i32; 3])>,
{
    let mut out = Vessels::default();
    for (id, at) in neighbours {
        match Vessel::of_id(id) {
            Some(Vessel::Mercury) => offer(&mut out.mercury, at),
            Some(Vessel::Salt) => offer(&mut out.salt, at),
            Some(Vessel::Sulfur) => offer(&mut out.sulfur, at),
            None => {}
        }
    }
    out
}

/// The horizontal neighbour positions of `pos`, in Java's scan order.
pub fn horizontal_neighbours(pos: [i32; 3]) -> [[i32; 3]; 4] {
    HORIZONTAL.map(|[x, y, z]| [pos[0] + x, pos[1] + y, pos[2] + z])
}

/// Reads the crafting state stored on the incubator compound, using the same keys
/// as `theurgy_host::crafting` (progress / totalTime / isProcessing /
/// couldCraftLastTick).
fn read_network(nbt: &Compound) -> Crafting {
    Crafting {
        progress: crate::game::theurgy_host::int(nbt, "progress", 0),
        total_time: crate::game::theurgy_host::int(nbt, "totalTime", DEFAULT_TIME),
        is_processing: crate::game::theurgy_host::bool(nbt, "isProcessing"),
        could_craft_last_tick: crate::game::theurgy_host::bool(nbt, "couldCraftLastTick"),
    }
}

/// Writes the crafting state using the same keys as `theurgy_host::save_crafting`.
fn write_network(nbt: &mut Compound, state: &Crafting) {
    crate::game::theurgy_host::set_int(nbt, "progress", state.progress);
    crate::game::theurgy_host::set_int(nbt, "totalTime", state.total_time);
    crate::game::theurgy_host::set_int(nbt, "isProcessing", i32::from(state.is_processing));
    crate::game::theurgy_host::set_int(
        nbt,
        "couldCraftLastTick",
        i32::from(state.could_craft_last_tick),
    );
}

fn save_state(entity: &mut ModBlockEntity, state: &Crafting) {
    write_network(&mut entity.nbt, state);
}

/// The total crafting time the incubator would report now, mirroring
/// `IncubatorCraftingBehaviour.getTotalTime`: the recipe's own time when a recipe is
/// selected (which, once input has changed, is what is stored), else the default.
fn total_time_or_default(state: &Crafting) -> i32 {
    if state.total_time > 0 {
        state.total_time
    } else {
        DEFAULT_TIME
    }
}

/// The singular recipe description read from `IncubationRecipe`'s JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeShape {
    pub mercury: serde_json::Value,
    pub salt: serde_json::Value,
    pub sulfur: serde_json::Value,
    pub result: serde_json::Value,
    pub time: i32,
}

impl RecipeShape {
    /// Parses one recipe of type `theurgy:incubation` from its JSON, applying
    /// `IncubationRecipe.CODEC`'s field defaults.
    pub fn parse(json: &serde_json::Value) -> Option<Self> {
        Some(Self {
            mercury: json.get("mercury")?.clone(),
            salt: json.get("salt")?.clone(),
            sulfur: json.get("sulfur")?.clone(),
            result: json.get("result")?.clone(),
            time: json
                .get("time")
                .and_then(serde_json::Value::as_i64)
                .map_or(DEFAULT_TIME, |t| t.clamp(i32::MIN as i64, i32::MAX as i64) as i32),
        })
    }
}

/// Ticks a single vessel block entity: nothing but its persisted input slot.
pub fn tick_vessel(_entity: &mut ModBlockEntity) {
    // Vessel block entities have no active tick: their single input slot is driven by
    // the surrounding incubator.
}

/// Ticks the incubator block entity at `pos`.
///
/// In the original the incubator holds the crafting behaviour and the adjacent
/// vessels hold their slots; here the tick reads the three vessels through the game,
/// runs the shared crafting engine, and on a completed craft consumes one unit from
/// each vessel input and inserts the result into the incubator's output slot.
pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if entity.id == INCUBATOR {
        tick_incubator(game, pos, entity);
    } else {
        tick_vessel(entity);
    }
}

fn tick_incubator(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    // Rediscover the vessels from the world each tick so a broken/replaced vessel is
    // noticed immediately (Java defers this to the next query, which a tick matches).
    let mut neighbours: Vec<(String, [i32; 3])> = Vec::new();
    for at in horizontal_neighbours(pos) {
        if let Some(id) = game_vessel_at(game, at) {
            neighbours.push((id, at));
        }
    }
    let vessels = classify_neighbours(neighbours.iter().map(|(id, at)| (id.as_str(), *at)));

    let mut state = read_network(&entity.nbt);

    let (Some(mercury), Some(salt), Some(sulfur)) =
        (vessels.mercury, vessels.salt, vessels.sulfur)
    else {
        // No valid multiblock: no recipe, so a running craft is stopped.
        if state.stop() > 0 {
            save_state(entity, &state);
            game.native_block_entity_sync(pos);
        }
        return;
    };

    let mercury_stack = vessel_input(game, mercury);
    let salt_stack = vessel_input(game, salt);
    let sulfur_stack = vessel_input(game, sulfur);

    let has_input = mercury_stack.is_some() && salt_stack.is_some() && sulfur_stack.is_some();
    let is_heated = is_heated(game, pos);

    if !has_input {
        if state.stop() > 0 {
            save_state(entity, &state);
            game.native_block_entity_sync(pos);
        }
        return;
    }

    // Match against the recipe table from the host.
    let recipes = game.native_recipes_of_type(RECIPE_KIND);
    let mut matched: Option<RecipeShape> = None;
    for rec in recipes {
        let Some(shape) = RecipeShape::parse(&rec.json) else {
            continue;
        };
        let ok = matches(game, &shape.mercury, mercury_stack.as_ref())
            && matches(game, &shape.salt, salt_stack.as_ref())
            && matches(game, &shape.sulfur, sulfur_stack.as_ref());
        if ok {
            matched = Some(shape);
            break;
        }
    }

    let Some(shape) = matched else {
        if state.stop() > 0 {
            save_state(entity, &state);
            game.native_block_entity_sync(pos);
        }
        return;
    };

    // Whether the output accepts the result right now. Java's craft() performs this
    // at the moment the timer elapses; we compute it up front to gate the timer.
    let result = game.native_recipe_result(&shape.result);
    let can_craft = result.as_ref().is_some_and(|out| output_can_fit(entity, out));

    let time = shape.time.max(0);
    if state.total_time != time {
        state.input_changed(time);
    }
    // The engine's CraftingBehaviour measures heat through `can_process`; the
    // incubator requires heat to progress, exactly like `HeatConsumerBehaviour`.
    let effects = state.tick(is_heated, true, can_craft, time);

    let mut crafted = false;
    if effects.craft_attempt {
        if let Some(out) = result {
            if perform_craft(game, mercury, salt, sulfur, entity, &out) {
                crafted = true;
                state.progress = 0;
                state.total_time = total_time_or_default(&state);
            }
        }
    }
    save_state(entity, &state);
    if effects.block_updates > 0 || crafted {
        game.native_block_entity_sync(pos);
    }
}

/// Gets the id of the mod block entity at `at` when it is a vessel; none otherwise.
fn game_vessel_at(game: &mut crate::game::Game<'_>, at: [i32; 3]) -> Option<String> {
    game.native_with_mod_block_entity(at, |be| be.id.clone())
        .filter(|id| Vessel::of_id(id).is_some())
}

/// Reads the single input slot of a vessel, as `ItemStack` when present.
fn vessel_input(game: &mut crate::game::Game<'_>, at: [i32; 3]) -> Option<ItemStack> {
    let inner: Option<Option<ItemStack>> =
        game.native_with_mod_block_entity(at, |be| be.items.first().cloned().flatten());
    inner
        .flatten()
        .filter(|s: &ItemStack| s.count > 0)
}

/// Whether the incubator is currently heated (`HeatConsumerBehaviour.isHeated`).
/// The brazier below and caloric-flux `hotUntil` state are both authoritative.
fn is_heated(game: &crate::game::Game<'_>, pos: [i32; 3]) -> bool {
    crate::game::theurgy_heat::heated(game, pos)
}

/// `Ingredient.test` against `native_recipe_ingredient`.
fn matches(
    game: &crate::game::Game<'_>,
    ingredient_json: &serde_json::Value,
    stack: Option<&ItemStack>,
) -> bool {
    let Some(stack) = stack else { return false };
    match game.native_recipe_ingredient(ingredient_json) {
        Some(ingredient) => ingredient.test(stack),
        None => false,
    }
}

/// The maximum stack size of a single output slot.
const MAX_STACK: u32 = 64;

/// Whether the incubator's single output slot can take `result` without overflowing.
fn output_can_fit(entity: &ModBlockEntity, result: &ItemStack) -> bool {
    if result.count == 0 {
        return false;
    }
    match entity.items.first().and_then(|s| s.as_ref()) {
        None => result.count <= MAX_STACK,
        Some(old) if old.count == 0 => result.count <= MAX_STACK,
        Some(old) => {
            old.item == result.item
                && old.count.saturating_add(result.count) <= MAX_STACK
        }
    }
}

/// Performs the atomic craft across the four block entities: consume one from each
/// vessel, then add one result to the incubator's output slot. Returns whether it
/// committed (all parts, in order, without partial writes when a vessel is short).
fn perform_craft(
    game: &mut crate::game::Game<'_>,
    mercury: [i32; 3],
    salt: [i32; 3],
    sulfur: [i32; 3],
    incubator: &mut ModBlockEntity,
    result: &ItemStack,
) -> bool {
    // Simulate the consumptions first; if any input is shorter than one, abort.
    let can_take = |game: &mut crate::game::Game<'_>, at: [i32; 3]| -> bool {
        game.native_with_mod_block_entity(at, |be| {
            be.items
                .first()
                .and_then(|s| s.as_ref())
                .is_some_and(|s| s.count >= 1)
        })
        .unwrap_or(false)
    };
    if !can_take(game, mercury) || !can_take(game, salt) || !can_take(game, sulfur) {
        return false;
    }
    if !output_can_fit(incubator, result) {
        return false;
    }

    // Consume.
    for at in [mercury, salt, sulfur] {
        let _ = game.native_with_mod_block_entity(at, |be| {
            if let Some(Some(slot)) = be.items.first_mut() {
                slot.count = slot.count.saturating_sub(1);
                if slot.count == 0 {
                    be.items[0] = None;
                }
            }
        });
    }

    // Insert into incubator output slot.
    if incubator.items.is_empty() {
        incubator.items.push(Some(result.clone()));
    } else if let Some(slot) = incubator.items[0].as_mut() {
        if slot.count == 0 || slot.item == result.item {
            slot.item = result.item.clone();
            slot.count = slot.count.saturating_add(result.count);
        } else {
            return false;
        }
    } else {
        incubator.items[0] = Some(result.clone());
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn stack(item: &str, count: u32) -> ItemStack {
        ItemStack {
            item: item.into(),
            count,
            ..Default::default()
        }
    }

    #[test]
    fn horizontal_neighbours_match_java_order() {
        let got = horizontal_neighbours([10, 5, -3]);
        assert_eq!(got, [[10, 5, -4], [11, 5, -3], [10, 5, -2], [9, 5, -3]]);
    }

    #[test]
    fn classify_needs_all_three_kinds() {
        let only = classify_neighbours([(MERCURY_VESSEL, [1, 0, 0]), (SALT_VESSEL, [0, 0, 1])]);
        assert!(!only.is_valid());
        assert_eq!(only.mercury, Some([1, 0, 0]));
        assert_eq!(only.salt, Some([0, 0, 1]));
        assert_eq!(only.sulfur, None);

        let all = classify_neighbours([
            (SULFUR_VESSEL, [0, 0, -1]),
            (MERCURY_VESSEL, [1, 0, 0]),
            (SALT_VESSEL, [0, 0, 1]),
        ]);
        assert!(all.is_valid());
        assert_eq!(all.mercury, Some([1, 0, 0]));
        assert_eq!(all.salt, Some([0, 0, 1]));
        assert_eq!(all.sulfur, Some([0, 0, -1]));
    }

    #[test]
    fn classify_ignores_unknown_blocks() {
        let only = classify_neighbours([
            ("theurgy:calcination_oven", [1, 0, 0]),
            (MERCURY_VESSEL, [0, 0, 1]),
        ]);
        assert_eq!(only.mercury, Some([0, 0, 1]));
        assert!(only.salt.is_none());
    }

    #[test]
    fn classify_keeps_first_of_each_kind() {
        let v = classify_neighbours([(SALT_VESSEL, [1, 0, 0]), (SALT_VESSEL, [0, 0, 1])]);
        assert_eq!(v.salt, Some([1, 0, 0]));
    }

    #[test]
    fn recipe_shape_defaults_time_and_requires_fields() {
        let recipe = json!({
            "mercury": "theurgy:alchemical_mercury",
            "salt": "#theurgy:alchemical_salts",
            "sulfur": "theurgy:alchemical_sulfur",
            "result": {"id": "minecraft:gold_nugget"}
        });
        let shape = RecipeShape::parse(&recipe).unwrap();
        assert_eq!(shape.time, DEFAULT_TIME);

        let bad = json!({
            "mercury": "theurgy:alchemical_mercury",
            "salt": "theurgy:alchemical_salt",
            "result": {"id": "minecraft:gold_nugget"}
        });
        assert!(RecipeShape::parse(&bad).is_none());
    }

    #[test]
    fn recipe_shape_parses_explicit_time() {
        let recipe = json!({
            "mercury": "a", "salt": "b", "sulfur": "c",
            "result": {"id": "minecraft:dirt"}, "time": 40
        });
        assert_eq!(RecipeShape::parse(&recipe).unwrap().time, 40);
    }

    #[test]
    fn default_total_time_used_when_unset() {
        let state = Crafting::default();
        assert_eq!(total_time_or_default(&state), DEFAULT_TIME);
        let state = Crafting { total_time: 30, ..Default::default() };
        assert_eq!(total_time_or_default(&state), 30);
    }

    #[test]
    fn output_fit_with_empty_slot() {
        let be = ModBlockEntity {
            id: INCUBATOR.into(),
            nbt: Compound::new(),
            items: Vec::new(),
            data: Vec::new(),
        };
        assert!(output_can_fit(&be, &stack("theurgy:result", 1)));
    }

    #[test]
    fn output_fit_rejects_wrong_item_and_overflow() {
        let mut be = ModBlockEntity {
            id: INCUBATOR.into(),
            nbt: Compound::new(),
            items: vec![Some(stack("minecraft:stone", 64))],
            data: Vec::new(),
        };
        assert!(!output_can_fit(&be, &stack("minecraft:stone", 1)));
        assert!(!output_can_fit(&be, &stack("minecraft:dirt", 1)));

        be.items[0] = Some(stack("minecraft:stone", 63));
        assert!(output_can_fit(&be, &stack("minecraft:stone", 1)));
        assert!(!output_can_fit(&be, &stack("minecraft:stone", 2)));
    }

    #[test]
    fn output_fit_treats_zero_count_slot_as_empty() {
        let be = ModBlockEntity {
            id: INCUBATOR.into(),
            nbt: Compound::new(),
            items: vec![Some(stack("minecraft:stone", 0))],
            data: Vec::new(),
        };
        assert!(output_can_fit(&be, &stack("minecraft:dirt", 1)));
    }

    #[test]
    fn state_round_trips_through_compound() {
        let state = Crafting {
            progress: 7,
            total_time: 42,
            is_processing: true,
            could_craft_last_tick: false,
        };
        let mut nbt = Compound::new();
        write_network(&mut nbt, &state);
        assert_eq!(read_network(&nbt), state);
    }
}
