// SPDX-License-Identifier: MIT
// Operator-only structured block-entity probe used by the Java↔Rust
// differential harness. It changes no state and has no filesystem/network IO.

use horizon_client::mod_api::{NativeCommand, NativeServerPlayer};
use horizon_client::text::Text;
use horizon_world::nbt::Tag;

fn tag_json(tag: &Tag) -> serde_json::Value {
    match tag {
        Tag::End => serde_json::Value::Null,
        Tag::Byte(value) => serde_json::json!(*value),
        Tag::Short(value) => serde_json::json!(*value),
        Tag::Int(value) => serde_json::json!(*value),
        Tag::Long(value) => serde_json::json!(*value),
        Tag::Float(bits) => serde_json::Number::from_f64(f32::from_bits(*bits) as f64)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        Tag::Double(bits) => serde_json::Number::from_f64(f64::from_bits(*bits))
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        Tag::ByteArray(values) => serde_json::json!(values),
        Tag::String(value) => serde_json::json!(value.to_string_lossy()),
        Tag::List(values) => serde_json::Value::Array(values.iter().map(tag_json).collect()),
        Tag::Compound(values) => serde_json::Value::Object(values.iter()
            .map(|(key, value)| (key.to_string_lossy(), tag_json(value))).collect()),
        Tag::IntArray(values) => serde_json::json!(values),
        Tag::LongArray(values) => serde_json::json!(values),
    }
}

fn probe_state(game: &mut crate::game::Game<'_>, pos: [i32; 3]) -> serde_json::Value {
    let state = game.world.block(pos[0], pos[1], pos[2]);
    let block = game.res.facts().get(state).name.to_string();
    let properties = game.pack.assets.registry.state(state)
        .map(|state| serde_json::Value::Object(state.properties.iter()
            .map(|(key, value)| (key.clone(), serde_json::json!(value))).collect()))
        .unwrap_or(serde_json::Value::Null);
    let vanilla_items = match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
        Some(horizon_client::containers::BlockEntity::Chest(stacks)) => stacks.clone(),
        _ => Vec::new(),
    };
    let entity = game.native_with_mod_block_entity(pos, |entity| {
        let nbt = entity.nbt.iter().map(|(key, value)| (key.to_string_lossy(), tag_json(value)))
            .collect::<serde_json::Map<_, _>>();
        let items = entity.items.iter().enumerate().filter_map(|(slot, stack)| {
            let stack = stack.as_ref()?;
            let Tag::Compound(record) = crate::game::theurgy_logistics::stack_tag(stack) else { return None; };
            let mut record = serde_json::Value::Object(record.iter()
                .map(|(key, value)| (key.to_string_lossy(), tag_json(value))).collect());
            record["Slot"] = serde_json::json!(slot);
            Some(record)
        }).collect::<Vec<_>>();
        (nbt, items, entity.data.clone())
    });
    let (nbt, mod_items, data) = entity.map_or((serde_json::Map::new(), Vec::new(), Vec::new()), |value| value);
    let items = if vanilla_items.is_empty() {
        mod_items
    } else {
        vanilla_items.iter().enumerate().filter_map(|(slot, stack)| {
            let stack = stack.as_ref()?;
            let Tag::Compound(record) = crate::game::theurgy_logistics::stack_tag(stack) else { return None; };
            let mut record = serde_json::Value::Object(record.iter()
                .map(|(key, value)| (key.to_string_lossy(), tag_json(value))).collect());
            record["Slot"] = serde_json::json!(slot);
            Some(record)
        }).collect()
    };
    let mercury_flux_capacity = match block.as_str() {
        "theurgy:mercury_catalyst" => Some(50_000),
        "theurgy:mercury_capacitor" => Some(500_000),
        "theurgy:caloric_flux_emitter" | "theurgy:logistics_mercury_flux_connector" | "theurgy:mercury_flux_emitter" | "theurgy:sulfuric_flux_emitter" => Some(1_000),
        _ => None,
    };
    serde_json::json!({
        "pos": pos,
        "block": block,
        "properties": properties,
        "nbt": nbt,
        "items": items,
        "data": data,
        "capabilities": {
            "mercury_flux_capacity": mercury_flux_capacity,
            "insert_targets": crate::game::theurgy_logistics::probe_insert_targets(game, pos),
            "nexus_partners": crate::game::theurgy_logistics::probe_nexus_partners(game, pos),
        },
    })
}

pub fn register(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::op("theurgy_probe <pos:block_pos>", 4));
    commands.push(NativeCommand::op("theurgy_state <pos:block_pos> <key:word> <value:word>", 4));
    commands.push(NativeCommand::op("theurgy_worldgen <pos:block_pos>", 4));
    commands.push(NativeCommand::op("theurgy_interact <pos:block_pos> <face:word>", 4));
    commands.push(NativeCommand::op("theurgy_proxy_read <pos:block_pos>", 4));
    commands.push(NativeCommand::op("theurgy_proxy_drain <pos:block_pos> <amount:integer>", 4));
}

pub fn perform(
    game: &mut crate::game::Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let Some(arguments) = command.strip_prefix("theurgy_probe ") else {
        if let Some(arguments) = command.strip_prefix("theurgy_interact ") {
            *ran = true;
            let parts: Vec<_> = arguments.split_whitespace().collect();
            let coordinates: Option<Vec<i32>> = parts.iter().take(3).map(|part| part.parse()).collect::<Result<_, _>>().ok();
            let Some(coordinates) = coordinates.filter(|values| values.len() == 3 && parts.len() == 4) else {
                game.native_feedback(Text::literal("{\"error\":\"usage: theurgy_interact <x y z> <face>\"}"));
                return;
            };
            let face = match parts[3] {
                "down" => [0, -1, 0], "up" => [0, 1, 0], "north" => [0, 0, -1],
                "south" => [0, 0, 1], "west" => [-1, 0, 0], "east" => [1, 0, 0],
                _ => {
                    game.native_feedback(Text::literal("{\"error\":\"invalid face\"}"));
                    return;
                }
            };
            let pos = [coordinates[0], coordinates[1], coordinates[2]];
            let held = game.native_carried_stack(game.selected as i32).cloned();
            let mut canceled = false;
            if let Some(stack) = held.as_ref() {
                crate::game::theurgy_items::right_click_block(game, stack, pos, face, &mut canceled);
            }
            if !canceled {
                crate::game::theurgy_interactions::right_click(game, held.as_ref(), pos, face, &mut canceled);
            }
            let state = probe_state(game, pos);
            game.native_feedback(Text::literal(serde_json::json!({"ok": true, "canceled": canceled, "state": state}).to_string()));
            return;
        }
        if let Some(arguments) = command.strip_prefix("theurgy_state ") {
            *ran = true;
            let parts: Vec<_> = arguments.split_whitespace().collect();
            if parts.len() != 5 {
                game.native_feedback(Text::literal("{\"error\":\"usage: theurgy_state <x y z> <property> <value>\"}"));
                return;
            }
            let (Ok(x), Ok(y), Ok(z)) = (parts[0].parse(), parts[1].parse(), parts[2].parse()) else {
                game.native_feedback(Text::literal("{\"error\":\"invalid position\"}"));
                return;
            };
            let changed = game.native_set_block_property_with([x, y, z], parts[3], parts[4], 3);
            game.native_feedback(Text::literal(serde_json::json!({"changed": changed}).to_string()));
            return;
        }
        if let Some(arguments) = command.strip_prefix("theurgy_worldgen ") {
            *ran = true;
            let coordinates: Option<Vec<i32>> = arguments.split_whitespace().map(str::parse).collect::<Result<_, _>>().ok();
            let Some(coordinates) = coordinates.filter(|values| values.len() == 3) else {
                game.native_feedback(Text::literal("{\"error\":\"usage: theurgy_worldgen <x y z>\"}"));
                return;
            };
            let chunk = [coordinates[0].div_euclid(16), coordinates[2].div_euclid(16)];
            let (written, stats, origins) = crate::game::theurgy_worldgen::chunk_load(game, "minecraft:overworld", chunk, true);
            let sample = game.native_block([coordinates[0], coordinates[1], coordinates[2]]);
            let motion = game.res.items().block_in_tag("minecraft:blocks_motion_in_heightmap", &sample);
            let stone = game.res.items().block_in_tag("minecraft:stone_ore_replaceables", &sample);
            let deepslate = game.res.items().block_in_tag("minecraft:deepslate_ore_replaceables", &sample);
            let floor = crate::game::theurgy_worldgen::ocean_floor_heights(game, chunk);
            let max_floor = floor.iter().copied().max().unwrap_or(-64);
            game.native_feedback(Text::literal(serde_json::json!({"ok":true,"written":written,"stats":stats,"origins":origins,"sample":sample,"motion":motion,"stone":stone,"deepslate":deepslate,"max_floor":max_floor,"dimension":game.dim.id()}).to_string()));
            return;
        }
        if let Some(arguments) = command.strip_prefix("theurgy_proxy_read ") {
            *ran = true;
            let coordinates: Option<Vec<i32>> = arguments.split_whitespace().map(str::parse).collect::<Result<_, _>>().ok();
            let Some(coordinates) = coordinates.filter(|values| values.len() == 3) else {
                game.native_feedback(Text::literal("{\"error\":\"usage: theurgy_proxy_read <x y z>\"}"));
                return;
            };
            let result = crate::game::theurgy_logistics::proxy_read_fluid(game, [coordinates[0], coordinates[1], coordinates[2]]);
            game.native_feedback(Text::literal(serde_json::json!({"fluid": result} ).to_string()));
            return;
        }
        if let Some(arguments) = command.strip_prefix("theurgy_proxy_drain ") {
            *ran = true;
            let coordinates: Option<Vec<i32>> = arguments.split_whitespace().map(str::parse).collect::<Result<_, _>>().ok();
            let Some(coordinates) = coordinates.filter(|values| values.len() == 4) else {
                game.native_feedback(Text::literal("{\"error\":\"usage: theurgy_proxy_drain <x y z> <amount>\"}"));
                return;
            };
            let drained = crate::game::theurgy_logistics::proxy_drain_fluid(
                game, [coordinates[0], coordinates[1], coordinates[2]], coordinates[3]);
            game.native_feedback(Text::literal(serde_json::json!({"drained": drained} ).to_string()));
            return;
        }
        return;
    };
    *ran = true;
    let coordinates: Option<Vec<i32>> = arguments.split_whitespace().map(str::parse).collect::<Result<_, _>>().ok();
    let Some(coordinates) = coordinates.filter(|values| values.len() == 3) else {
        game.native_feedback(Text::literal("{\"error\":\"usage: theurgy_probe <x y z>\"}"));
        return;
    };
    let pos = [coordinates[0], coordinates[1], coordinates[2]];
    let reply = probe_state(game, pos);
    game.native_feedback(Text::literal(reply.to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_world::nbt::JavaString;

    #[test]
    fn nbt_projection_preserves_numbers_lists_and_compounds() {
        let mut compound = horizon_world::nbt::Compound::new();
        compound.insert(JavaString::from("progress"), Tag::Short(17));
        compound.insert(JavaString::from("Items"), Tag::List(vec![Tag::String(JavaString::from("minecraft:apple"))]));
        let value = tag_json(&Tag::Compound(compound));
        assert_eq!(value["progress"], 17);
        assert_eq!(value["Items"][0], "minecraft:apple");
    }
}
