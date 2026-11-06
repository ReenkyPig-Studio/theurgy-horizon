// SPDX-License-Identifier: MIT
use horizon_client::containers::ModBlockEntity;
use horizon_world::nbt::{Compound, JavaString, Tag};

pub fn int(nbt: &Compound, key: &str, default: i32) -> i32 {
    match nbt.get(&JavaString::from(key)) {
        Some(Tag::Int(v)) => *v,
        Some(Tag::Short(v)) => i32::from(*v),
        Some(Tag::Byte(v)) => i32::from(*v),
        Some(Tag::Long(v)) => {
            if *v > i64::from(i32::MAX) {
                i32::MAX
            } else if *v < i64::from(i32::MIN) {
                i32::MIN
            } else {
                *v as i32
            }
        }
        _ => default,
    }
}

pub fn set_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(JavaString::from(key), Tag::Int(value));
}

pub fn bool(nbt: &Compound, key: &str) -> bool {
    int(nbt, key, 0) != 0
}

fn clamp_non_negative(value: i32) -> i32 {
    if value < 0 {
        0
    } else {
        value
    }
}

pub fn crafting(entity: &ModBlockEntity) -> theurgy_engine::crafting::Crafting {
    theurgy_engine::crafting::Crafting {
        progress: clamp_non_negative(int(&entity.nbt, "progress", 0)),
        total_time: clamp_non_negative(int(&entity.nbt, "totalTime", 0)),
        // Java's saveAdditional stores only the two shorts. These flags are
        // network/runtime state, so keep them in non-persistent menu data.
        is_processing: entity.data.get(4).copied().unwrap_or(0) != 0,
        could_craft_last_tick: entity.data.get(5).copied().unwrap_or(0) != 0,
    }
}

pub fn save_crafting(entity: &mut ModBlockEntity, state: &theurgy_engine::crafting::Crafting) {
    // CraftingBehaviour.saveAdditional uses putShort for both values.
    entity.nbt.insert(JavaString::from("progress"), Tag::Short(state.progress as i16));
    entity.nbt.insert(JavaString::from("totalTime"), Tag::Short(state.total_time as i16));
    entity.nbt.remove(&JavaString::from("isProcessing"));
    entity.nbt.remove(&JavaString::from("couldCraftLastTick"));
    if entity.data.len() < 6 {
        entity.data.resize(6, 0);
    }
    entity.data[4] = i32::from(state.is_processing);
    entity.data[5] = i32::from(state.could_craft_last_tick);
}

pub fn tank(entity: &ModBlockEntity, index: usize, capacity: i32) -> theurgy_engine::recipes::Tank {
    let capacity = clamp_non_negative(capacity);
    let empty = || theurgy_engine::recipes::Tank {
        fluid: None,
        amount: 0,
        capacity,
    };
    let Some(Tag::List(list)) = entity.nbt.get(&JavaString::from("fluids")) else {
        return empty();
    };
    let Some(Tag::Compound(map)) = list.get(index) else {
        return empty();
    };
    let fluid = match map.get(&JavaString::from("Fluid")) {
        Some(Tag::String(v)) => {
            let v = v.to_string_lossy();
            if v.is_empty() || v == "minecraft:empty" {
                None
            } else {
                Some(v)
            }
        }
        _ => None,
    };
    let amount = int(map, "Amount", 0).clamp(0, capacity);
    if fluid.is_none() || amount == 0 {
        return empty();
    }
    theurgy_engine::recipes::Tank {
        fluid,
        amount,
        capacity,
    }
}

pub fn save_tanks(entity: &mut ModBlockEntity, tanks: &[theurgy_engine::recipes::Tank]) {
    let list = tanks
        .iter()
        .map(|t| {
            let mut map = Compound::new();
            let fluid = match t.fluid.as_deref() {
                Some(id) if !id.is_empty() && id != "minecraft:empty" && t.amount > 0 => id,
                _ => "minecraft:empty",
            };
            let amount = if fluid == "minecraft:empty" {
                0
            } else {
                t.amount.clamp(0, clamp_non_negative(t.capacity))
            };
            map.insert(
                JavaString::from("Fluid"),
                Tag::String(JavaString::from(fluid)),
            );
            map.insert(JavaString::from("Amount"), Tag::Int(amount));
            Tag::Compound(map)
        })
        .collect();
    entity.nbt.insert(JavaString::from("fluids"), Tag::List(list));
}
