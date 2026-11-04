// SPDX-License-Identifier: MIT
// Sal ammoniac accumulator and tank (Java 26.3):
// content/apparatus/salammoniacaccumulator/*, content/apparatus/salammoniactank/*.
use horizon_client::containers::{BlockEntity, ModBlockEntity};
use horizon_client::items::ItemStack;
use horizon_world::nbt::{Compound, JavaString, Tag};
use serde_json::Value;

use crate::game::theurgy_host as host;
use theurgy_engine::recipes::Tank;

const ACCUMULATOR: &str = "theurgy:sal_ammoniac_accumulator";
const TANK: &str = "theurgy:sal_ammoniac_tank";
const WATER_CAPACITY: i32 = 10_000;
const TANK_CAPACITY: i32 = 2_000;

fn key(pos: [i32; 3]) -> (i32, i32, i32) {
    (pos[0], pos[1], pos[2])
}

fn read_tank(entity: &ModBlockEntity, index: usize, capacity: i32) -> Tank {
    let Some(Tag::List(list)) = entity.nbt.get(&JavaString::from("fluids")) else {
        return Tank { fluid: None, amount: 0, capacity };
    };
    let Some(Tag::Compound(map)) = list.get(index) else {
        return Tank { fluid: None, amount: 0, capacity };
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
    Tank { fluid, amount, capacity }
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
        Tag::String(JavaString::from(tank.fluid.as_deref().unwrap_or("minecraft:empty"))),
    );
    map.insert(JavaString::from("Amount"), Tag::Int(tank.amount));
    list[index] = Tag::Compound(map);
    entity.nbt.insert(JavaString::from("fluids"), Tag::List(list));
}

fn fluid_field(value: Option<&Value>) -> Option<(String, i32)> {
    let value = value?;
    let id = value.get("ingredient").or_else(|| value.get("id"))?.as_str()?;
    let amount = value.get("amount")?.as_i64()?.clamp(0, i32::MAX as i64) as i32;
    Some((id.to_owned(), amount))
}

fn fluid_matches(wanted: &str, actual: Option<&str>) -> bool {
    let Some(actual) = actual else { return false };
    if let Some(tag) = wanted.strip_prefix('#') {
        return match tag {
            "minecraft:water" => actual == "minecraft:water",
            "theurgy:sal_ammoniac" | "theurgy:solvent" => actual == "theurgy:sal_ammoniac",
            _ => actual == tag,
        };
    }
    actual == wanted
}

fn fits(output: &Tank, fluid: &str, amount: i32) -> bool {
    amount > 0
        && amount <= output.capacity
        && output.fluid.as_deref().is_none_or(|f| f == fluid)
        && output.amount.saturating_add(amount) <= output.capacity
}

fn output_tank(game: &crate::game::Game<'_>, pos: [i32; 3]) -> Option<Tank> {
    let below = key([pos[0], pos[1] - 1, pos[2]]);
    match game.world.block_entities.get(&below) {
        Some(BlockEntity::Mod(entity)) if entity.id == TANK => {
            Some(read_tank(entity, 0, TANK_CAPACITY))
        }
        _ => None,
    }
}

fn fill_below(game: &mut crate::game::Game<'_>, pos: [i32; 3], fluid: &str, amount: i32) -> bool {
    if game.native_block([pos[0], pos[1] - 1, pos[2]]) != TANK {
        return false;
    }
    let below = [pos[0], pos[1] - 1, pos[2]];
    let mut filled = false;
    game.native_with_mod_block_entity(below, |entity| {
        let mut tank = read_tank(entity, 0, TANK_CAPACITY);
        if !fits(&tank, fluid, amount) {
            return;
        }
        tank.fluid = Some(fluid.to_owned());
        tank.amount += amount;
        write_tank(entity, 0, &tank);
        filled = true;
    });
    if filled {
        game.native_block_entity_sync(below);
    }
    filled
}

#[derive(Clone)]
struct Accumulation {
    solute: Option<String>,
    evaporant: Option<(String, i32)>,
    result: (String, i32),
    time: i32,
}

fn recipes(game: &crate::game::Game<'_>) -> Vec<Accumulation> {
    game.native_recipes_of_type("theurgy:accumulation")
        .iter()
        .filter_map(|r| {
            Some(Accumulation {
                solute: r.json.get("solute").and_then(Value::as_str).map(str::to_owned),
                evaporant: fluid_field(r.json.get("evaporant")),
                result: fluid_field(r.json.get("result"))?,
                time: r.json.get("time").and_then(Value::as_i64).unwrap_or(100)
                    .clamp(0, i32::MAX as i64) as i32,
            })
        })
        .collect()
}

fn solute_matches(game: &mut crate::game::Game<'_>, solute: &str, stack: &ItemStack) -> bool {
    game.native_recipe_ingredient(&Value::String(solute.to_owned()))
        .is_some_and(|ingredient| ingredient.test(stack))
}

/// Accumulator server tick. Requires a sal ammoniac tank directly below; no
/// heat. Water tank is NBT `fluids[0]`, input slot is `entity.items[0]`.
pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if entity.id == TANK {
        let mut tank = read_tank(entity, 0, TANK_CAPACITY);
        tank.amount = tank.amount.clamp(0, TANK_CAPACITY);
        if tank.amount == 0 {
            tank.fluid = None;
        }
        write_tank(entity, 0, &tank);
        entity.data.clear();
        entity.data.extend([tank.amount, tank.capacity]);
        return;
    }
    if entity.id != ACCUMULATOR || game.native_block([pos[0], pos[1] - 1, pos[2]]) != TANK {
        return;
    }
    let mut water = read_tank(entity, 0, WATER_CAPACITY);
    let mut state = host::crafting(entity);
    let recipes = recipes(game);
    let mut selected: Option<Accumulation> = None;
    for recipe in &recipes {
        if let Some(solute) = &recipe.solute {
            let Some(stack) = entity.items.first().and_then(Option::as_ref) else { continue };
            if !solute_matches(game, solute, stack) {
                continue;
            }
        } else if entity.items.first().is_some_and(Option::is_some) {
            continue;
        }
        if let Some((wanted, amount)) = &recipe.evaporant
            && (!fluid_matches(wanted, water.fluid.as_deref()) || water.amount < *amount)
        {
            continue;
        }
        let Some(output) = output_tank(game, pos) else { continue };
        if fits(&output, &recipe.result.0, recipe.result.1) {
            selected = Some(recipe.clone());
            break;
        }
    }
    let time = selected.as_ref().map_or(0, |r| r.time);
    if state.total_time != time {
        state.input_changed(time);
    }
    let has_input = water.amount > 0 || entity.items.first().is_some_and(Option::is_some);
    let effects = state.tick(true, has_input, selected.is_some(), time);
    if effects.craft_attempt {
        if let Some(recipe) = selected {
            // Output capacity was checked this tick, but the commit is atomic:
            // fill first, then consume only when the tank accepted it.
            if fill_below(game, pos, &recipe.result.0, recipe.result.1) {
                if recipe.solute.is_some()
                    && let Some(slot) = entity.items.first_mut().and_then(Option::as_mut)
                {
                    if slot.count > 1 {
                        slot.count -= 1;
                    } else {
                        entity.items[0] = None;
                    }
                }
                if let Some((_, amount)) = recipe.evaporant {
                    water.amount = (water.amount - amount).max(0);
                    if water.amount == 0 {
                        water.fluid = None;
                    }
                }
            }
        }
    }
    write_tank(entity, 0, &water);
    host::save_crafting(entity, &state);
    entity.data.clear();
    entity.data.extend([state.progress, state.total_time, water.amount, water.capacity]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tank_holds_two_buckets_and_never_mixes() {
        let empty = Tank { fluid: None, amount: 0, capacity: 2_000 };
        assert!(fits(&empty, "minecraft:water", 2_000));
        assert!(!fits(&empty, "minecraft:water", 2_001));
        let water = Tank { fluid: Some("minecraft:water".into()), amount: 1_000, capacity: 2_000 };
        assert!(fits(&water, "minecraft:water", 1_000));
        assert!(!fits(&water, "theurgy:sal_ammoniac", 1_000));
    }

    #[test]
    fn water_tag_matches_only_water() {
        assert!(fluid_matches("#minecraft:water", Some("minecraft:water")));
        assert!(!fluid_matches("#minecraft:water", Some("theurgy:sal_ammoniac")));
        assert!(fluid_matches("#theurgy:sal_ammoniac", Some("theurgy:sal_ammoniac")));
    }

    #[test]
    fn result_and_evaporant_fields_parse() {
        use serde_json::json;
        assert_eq!(fluid_field(Some(&json!({"id":"minecraft:lava","amount":1000}))), Some(("minecraft:lava".into(), 1000)));
        assert_eq!(fluid_field(Some(&json!({"ingredient":"#minecraft:water","amount":1000}))), Some(("#minecraft:water".into(), 1000)));
    }
}
