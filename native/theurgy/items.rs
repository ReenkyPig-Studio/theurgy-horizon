// SPDX-License-Identifier: MIT
// Item behaviour that belongs to the block/item hooks. The carried content
// already supplies names, models, stack sizes, durability and tool rules; this
// module adds the two code behaviours that data cannot express: the upper half
// of the double-high apparatus, and the sal ammoniac bucket's fluid identity.
use horizon_client::items::ItemStack;
use horizon_world::chunk::BlockState;
use horizon_world::nbt::{Compound, JavaString, Tag};

/// Blocks whose Java class is a double-high apparatus; the lower half holds the
/// block entity, the upper half is only the model/collision.
const DOUBLE_HIGH: [&str; 4] = [
    "theurgy:calcination_oven",
    "theurgy:liquefaction_cauldron",
    "theurgy:distiller",
    "theurgy:incubator",
];

const WIRE_ITEM: &str = "theurgy:copper_wire";
const WIRE_END_POINT: &str = "theurgy:wire_end_point";
const WIRE_MAX_RANGE: i64 = 32;

const DIVINATION_LINKED_BLOCK: &str = "theurgy:divination_linked_block";
const DIVINATION_LINKED_TAG: &str = "theurgy:divination_linked_tag";
const DIVINATION_POS: &str = "theurgy:divination_pos";
const DIVINATION_DISTANCE: &str = "theurgy:divination_distance";
const DIVINATION_RANGE: &str = "theurgy:divination_settings_range";
const DIVINATION_MAX_RANGE: i32 = 96;
const DIVINATION_TIER: &str = "theurgy:divination_settings_tier";
const DIVINATION_NOT_FOUND: f32 = 7.0;

const WIRE_ENDPOINTS: [&str; 8] = [
    "theurgy:logistics_connector_node",
    "theurgy:logistics_item_inserter",
    "theurgy:logistics_item_extractor",
    "theurgy:logistics_fluid_inserter",
    "theurgy:logistics_fluid_extractor",
    "theurgy:logistics_capability_probe",
    "theurgy:logistics_capability_proxy",
    "theurgy:logistics_mercury_flux_connector",
];

/// Blocks that the Java classes break as a pair when either half is removed.
pub fn is_double_high(block: &str) -> bool {
    DOUBLE_HIGH.contains(&block)
}

/// The upper-half descriptor for `lower`, if `block` is double high.
fn upper_state(game: &crate::game::Game<'_>, state: horizon_world::registry::BlockStateId) -> Option<horizon_world::registry::BlockStateId> {
    let registry = &game.pack.assets.registry;
    let mut descriptor: BlockState = registry.state(state)?.clone();
    if !is_double_high(&descriptor.name) {
        return None;
    }
    descriptor.properties.insert("half".to_owned(), "upper".to_owned());
    registry.resolve(&descriptor).ok()
}

/// `DistillerBlock.setPlacedBy` and its siblings: place the upper half at
/// `pos.above()` with the same block's `half=upper` state.
pub fn placed(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
) {
    crate::game::theurgy_selection::placed(game, stack, pos);
    if !is_double_high(&stack.item) {
        return;
    }
    let Some(upper) = upper_state(game, state) else { return; };
    let dimension = game.dim.id();
    game.native_set_block(dimension, [pos[0], pos[1] + 1, pos[2]], upper, 3);
}

/// Read a registered native component from `stack` as a string.
fn component_string(
    game: &crate::game::Game<'_>,
    stack: &ItemStack,
    id: &str,
) -> Option<String> {
    match game.native_component(stack, id).ok().flatten()? {
        Tag::String(value) => Some(value.to_string_lossy()),
        _ => None,
    }
}

/// Read a registered native component as a boolean (Java persists booleans as
/// bytes/ints through NBT).
fn component_bool(game: &crate::game::Game<'_>, stack: &ItemStack, id: &str) -> bool {
    match game.native_component(stack, id).ok().flatten() {
        Some(Tag::Byte(value)) => *value != 0,
        Some(Tag::Int(value)) => *value != 0,
        Some(Tag::Short(value)) => *value != 0,
        _ => false,
    }
}

/// Read a registered native component as an integer.
fn component_i32(game: &crate::game::Game<'_>, stack: &ItemStack, id: &str) -> Option<i32> {
    match game.native_component(stack, id).ok().flatten()? {
        Tag::Int(value) => Some(*value),
        Tag::Short(value) => Some(*value as i32),
        Tag::Byte(value) => Some(*value as i32),
        _ => None,
    }
}

/// Write a component on the acting player's held stack. The hook's `stack` is
/// immutable, so the authoritative carried stack is updated by value.
fn set_held_component(game: &mut crate::game::Game<'_>, id: &str, value: Tag) {
    let slot = game.selected as i32;
    let Some(mut held) = game.native_carried_stack(slot).cloned() else {
        return;
    };
    if game.native_set_component(&mut held, id, value).is_ok()
        && let Some(current) = game.native_carried_stack(slot)
    {
        *current = held;
    }
}

/// Java `BlockPos.asLong`: x in the high 26 bits, z in the middle 26 bits,
/// y in the low 12 bits.
fn block_pos_long(pos: [i32; 3]) -> i64 {
    (((pos[0] as i64) & 0x3ff_ffff) << 38)
        | (((pos[2] as i64) & 0x3ff_ffff) << 12)
        | ((pos[1] as i64) & 0xfff)
}

/// Inverse of the Minecraft `BlockPos.asLong` packing above.
fn block_pos_from_long(value: i64) -> [i32; 3] {
    let x = (value >> 38) as i32;
    let y = ((value << 52) >> 52) as i32;
    let z = ((value << 26) >> 38) as i32;
    [x, y, z]
}

fn manhattan(a: [i32; 3], b: [i32; 3]) -> i64 {
    (a[0] as i64 - b[0] as i64).abs()
        + (a[1] as i64 - b[1] as i64).abs()
        + (a[2] as i64 - b[2] as i64).abs()
}

fn is_wire_endpoint(block: &str) -> bool {
    WIRE_ENDPOINTS.contains(&block)
}

fn is_sneaking(game: &crate::game::Game<'_>) -> bool {
    game.has_shift_down() || game.keys.contains(&winit::keyboard::KeyCode::ShiftLeft)
}

fn is_air(block: &str) -> bool {
    matches!(block, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
}

fn distance_bucket(distance: f32) -> f32 {
    if distance < 6.0 {
        0.0
    } else if distance < 15.0 {
        1.0
    } else if distance < 25.0 {
        2.0
    } else if distance < 35.0 {
        3.0
    } else if distance < 45.0 {
        4.0
    } else if distance < 65.0 {
        5.0
    } else {
        6.0
    }
}

/// `ToolMaterial.incorrectBlocksForDrops()` for every tier used by the rods.
/// The check is separate from the allowed/disallowed Theurgy tags in Java.
fn incorrect_blocks_tag(tier: &str) -> &'static str {
    match tier {
        "wood" | "wooden" => "minecraft:incorrect_for_wooden_tool",
        "gold" => "minecraft:incorrect_for_gold_tool",
        "iron" => "minecraft:incorrect_for_iron_tool",
        "diamond" => "minecraft:incorrect_for_diamond_tool",
        "netherite" => "minecraft:incorrect_for_netherite_tool",
        _ => "minecraft:incorrect_for_stone_tool",
    }
}

/// `DivinationRodItem.getOreTagFromBlockId`: try the two-word ore name before
/// the one-word name. Thus `sal_ammoniac_ore` resolves to `c:ores/sal_ammoniac`,
/// never to the partial `c:ores/ammoniac` tag.
fn ore_tag_candidates(block: &str) -> Vec<String> {
    let path = block.split_once(':').map_or(block, |(_, path)| path);
    let Some(stem) = path.strip_suffix("_ore") else {
        return vec![format!("c:ores/{path}")];
    };
    let words: Vec<&str> = stem.split('_').collect();
    let mut candidates = Vec::with_capacity(2);
    if words.len() >= 2 {
        candidates.push(format!(
            "c:ores/{}_{}",
            words[words.len() - 2],
            words[words.len() - 1]
        ));
    }
    if let Some(word) = words.last() {
        candidates.push(format!("c:ores/{word}"));
    }
    candidates
}

fn deepslate_variant(block: &str) -> Option<String> {
    let (namespace, path) = block.split_once(':').unwrap_or(("minecraft", block));
    (path.contains("_ore") && !path.contains("deepslate_"))
        .then(|| format!("{namespace}:deepslate_{path}"))
}

/// `WireEndPoint.load(ItemStack)`: the component is a compound with a packed
/// `pos` long and a `level` dimension id string.
fn load_wire_endpoint(
    game: &crate::game::Game<'_>,
    stack: &ItemStack,
) -> Option<([i32; 3], String)> {
    let tag = game.native_component(stack, WIRE_END_POINT).ok().flatten()?;
    let Tag::Compound(entries) = tag else {
        return None;
    };
    let mut pos = None;
    let mut level = None;
    for (key, value) in entries {
        if key.to_string_lossy() == "pos" {
            if let Tag::Long(value) = value {
                pos = Some(block_pos_from_long(*value));
            }
        } else if key.to_string_lossy() == "level" {
            if let Tag::String(value) = value {
                level = Some(value.to_string_lossy());
            }
        }
    }
    Some((pos?, level?))
}

fn store_wire_endpoint(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    let level = game.dim.id().to_string();
    let mut entries = Compound::new();
    entries.insert(JavaString::from("pos"), Tag::Long(block_pos_long(pos)));
    entries.insert(
        JavaString::from("level"),
        Tag::String(JavaString::from(level.as_str())),
    );

    let slot = game.selected as i32;
    let Some(mut held) = game.native_carried_stack(slot).cloned() else {
        return;
    };
    if game
        .native_set_component(&mut held, WIRE_END_POINT, Tag::Compound(entries))
        .is_ok()
        && let Some(current) = game.native_carried_stack(slot)
    {
        *current = held;
    }
}

fn clear_wire_endpoint(game: &mut crate::game::Game<'_>) {
    let slot = game.selected as i32;
    let Some(mut held) = game.native_carried_stack(slot).cloned() else {
        return;
    };
    let _ = game.native_remove_component(&mut held, WIRE_END_POINT);
    if let Some(current) = game.native_carried_stack(slot) {
        *current = held;
    }
}

fn consume_one_wire(game: &mut crate::game::Game<'_>) {
    let slot = game.selected as i32;
    let Some(mut held) = game.native_carried_stack(slot).cloned() else {
        return;
    };
    if held.item.as_ref() == WIRE_ITEM && held.count > 0 {
        held.count = held.count.saturating_sub(1);
    }
    if let Some(current) = game.native_carried_stack(slot) {
        *current = held;
    }
}

fn wire_use_on(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    claimed: &mut bool,
) -> bool {
    crate::game::theurgy_logistics::wire_use_on(game, stack, pos, claimed)
}

/// `DivinationRodItem.useOn`: sneak-use links the rod to an allowed block. The
/// attached block is stored in the rod's `divination_linked_block` component.
fn attune_rod(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    claimed: &mut bool,
) -> bool {
    let sneaking = game.has_shift_down() || game.keys.contains(&winit::keyboard::KeyCode::ShiftLeft);
    if !sneaking {
        return false;
    }
    *claimed = true;
    if !component_bool(game, stack, "theurgy:divination_settings_allow_attuning") {
        return true;
    }
    let block = game.native_block(pos);
    if block == "minecraft:air" {
        return true;
    }
    let tier = component_string(game, stack, DIVINATION_TIER).unwrap_or_else(|| "stone".into());
    if game
        .res
        .items()
        .block_in_tag(incorrect_blocks_tag(&tier), &block)
    {
        return true;
    }
    let allowed = component_string(game, stack, "theurgy:divination_settings_allowed_blocks_tag");
    let disallowed =
        component_string(game, stack, "theurgy:divination_settings_disallowed_blocks_tag");
    if let Some(tag) = allowed.as_deref()
        && !tag.is_empty()
        && !game.res.items().block_in_tag(tag, &block)
    {
        return true;
    }
    if let Some(tag) = disallowed.as_deref()
        && !tag.is_empty()
        && game.res.items().block_in_tag(tag, &block)
    {
        return true;
    }
    set_held_component(
        game,
        DIVINATION_LINKED_BLOCK,
        Tag::String(JavaString::from(block.as_str())),
    );
    true
}

/// Early `right_click_block` hook: logistics blocks use a barrel template that
/// consumes the block interaction before `use_item_on` runs. Handle the wire
/// before that so a valid endpoint click is never lost.
const MERCURIAL_WAND: &str = "theurgy:mercurial_wand";
const MERCURIAL_WAND_MODE: &str = "theurgy:mercurial_wand_item_mode";
const SELECTED_FREQUENCY: &str = "theurgy:selected_frequency";
// Java Logistics{Item,Fluid}ConnectorBlockEntity and
// LogisticsMercuryFluxConnectorBlockEntity implement all three wand targets.
const WAND_CONNECTORS: [&str; 5] = [
    "theurgy:logistics_item_inserter", "theurgy:logistics_item_extractor",
    "theurgy:logistics_fluid_inserter", "theurgy:logistics_fluid_extractor",
    "theurgy:logistics_mercury_flux_connector",
];
const WAND_MODE_NAMES: [&str; 5] = [
    "cycle_direction", "switch_logistics_enabled", "select_frequency",
    "set_selected_frequency", "cycle_capacitor_side",
];
fn wand_mode(game: &crate::game::Game<'_>, stack: &ItemStack) -> i32 {
    // MercurialWandItemMode.Type.CODEC persists names, not ordinals. Accept
    // legacy integer stacks, but always write the Java string representation.
    if let Some(mode) = component_i32(game, stack, MERCURIAL_WAND_MODE) {
        return mode.clamp(0, 4);
    }
    component_string(game, stack, MERCURIAL_WAND_MODE)
        .and_then(|name| WAND_MODE_NAMES.iter().position(|candidate| *candidate == name.as_str()))
        .unwrap_or(0) as i32
}
// CycleSelectedDirectionMode.nextDirection; ordinals are Direction.get3DDataValue.
fn next_direction(ordinal: i32) -> i32 {
    match ordinal { 1 => 2, 2 => 4, 4 => 3, 3 => 5, 5 => 0, 0 => 1, _ => 2 }
}
fn direction_ordinal(name: &str) -> i32 {
    match name {
        "down" | "DOWN" => 0, "up" | "UP" => 1, "north" | "NORTH" => 2,
        "south" | "SOUTH" => 3, "west" | "WEST" => 4, "east" | "EAST" => 5,
        _ => 2,
    }
}
fn block_facing(game: &crate::game::Game<'_>, pos: [i32; 3]) -> i32 {
    let state = game.world.block(pos[0], pos[1], pos[2]);
    game.pack.assets.registry.state(state)
        .and_then(|descriptor| descriptor.properties.get("facing"))
        .map(|name| direction_ordinal(name)).unwrap_or(2)
}
fn face_direction(face: [i32; 3]) -> &'static str {
    match face { [0, -1, 0] => "DOWN", [0, 1, 0] => "UP", [0, 0, -1] => "NORTH",
        [0, 0, 1] => "SOUTH", [-1, 0, 0] => "WEST", [1, 0, 0] => "EAST", _ => "NORTH" }
}
fn wand_use_on(game: &mut crate::game::Game<'_>, stack: &ItemStack, pos: [i32; 3], face: [i32; 3]) -> bool {
    if stack.item.as_ref() != MERCURIAL_WAND { return false; }
    let block = game.native_block(pos);
    let connector = WAND_CONNECTORS.contains(&block.as_str());
    match wand_mode(game, stack) {
        0 if connector => {
            // With no override Java uses the block's FACING, not a hard-coded north.
            let facing = block_facing(game, pos);
            game.native_with_mod_block_entity(pos, |entity| {
                let current = match entity.nbt.get(&JavaString::from("directionOverride")) {
                    Some(Tag::Int(v)) => *v, Some(Tag::Byte(v)) => *v as i32, _ => facing };
                entity.nbt.insert(JavaString::from("directionOverride"), Tag::Int(next_direction(current)));
            });
            true
        }
        1 if connector => {
            game.native_with_mod_block_entity(pos, |entity| {
                let current = match entity.nbt.get(&JavaString::from("enabled")) {
                    Some(Tag::Byte(v)) => *v != 0, Some(Tag::Int(v)) => *v != 0, _ => true };
                entity.nbt.insert(JavaString::from("enabled"), Tag::Byte(if current { 0 } else { 1 }));
            });
            true
        }
        2 => {
            let frequency = component_i32(game, stack, SELECTED_FREQUENCY).unwrap_or(0) + 1;
            set_held_component(game, SELECTED_FREQUENCY, Tag::Int(frequency));
            true
        }
        3 if connector => {
            let frequency = component_i32(game, stack, SELECTED_FREQUENCY).unwrap_or(0);
            game.native_with_mod_block_entity(pos, |entity| {
                entity.nbt.insert(JavaString::from("frequency"), Tag::Int(frequency));
            });
            true
        }
        4 if block == "theurgy:mercury_capacitor" => {
            let face_key = JavaString::from(face_direction(face));
            game.native_with_mod_block_entity(pos, |entity| {
                let side_key = JavaString::from("sideModes");
                let mut modes = match entity.nbt.get(&side_key) { Some(Tag::Compound(map)) => map.clone(), _ => Compound::new() };
                // MercuryCapacitorBlockEntity constructor: UP/DOWN OUTPUT(2),
                // horizontal sides INPUT(1); an absent entry is not NONE here.
                let fallback = if matches!(face_direction(face), "UP" | "DOWN") { 2 } else { 1 };
                let current = match modes.get(&face_key) { Some(Tag::Int(v)) => *v, Some(Tag::Byte(v)) => *v as i32, _ => fallback };
                modes.insert(face_key, Tag::Int((current + 1).rem_euclid(4)));
                entity.nbt.insert(side_key, Tag::Compound(modes));
            });
            true
        }
        _ => false,
    }
}
pub fn mouse_scroll(game: &mut crate::game::Game<'_>, _scroll: [f64; 2], accumulated: [i32; 2], canceled: &mut bool) {
    if *canceled || !is_sneaking(game) || accumulated[1] == 0 { return; }
    let slot = game.selected as i32;
    let Some(held) = game.native_carried_stack(slot).cloned() else { return; };
    if held.item.as_ref() != MERCURIAL_WAND { return; }
    let current = wand_mode(game, &held);
    let next = if accumulated[1] > 0 { (current + 1).rem_euclid(5) } else { (current - 1).rem_euclid(5) };
    set_held_component(game, MERCURIAL_WAND_MODE,
        Tag::String(JavaString::from(WAND_MODE_NAMES[next as usize])));
    *canceled = true;
}

pub fn right_click_block(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    _face: [i32; 3],
    canceled: &mut bool,
) {
    if *canceled { return; }
    if stack.item.as_ref() == MERCURIAL_WAND && wand_use_on(game, stack, pos, _face) {
        *canceled = true;
        return;
    }
    if stack.item.as_ref() != WIRE_ITEM {
        return;
    }
    let mut claimed = false;
    wire_use_on(game, stack, pos, &mut claimed);
    if claimed {
        *canceled = true;
    }
}

/// Right-click in air with a divination rod scans a bounded local cube for the
/// linked block or linked tag and writes the result into the rod's components.
pub fn use_item(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    claimed: &mut bool,
) {
    if *claimed { return; }
    if stack.item.as_ref() == MERCURIAL_WAND && wand_mode(game, stack) == 2 {
        let frequency = component_i32(game, stack, SELECTED_FREQUENCY).unwrap_or(0) + 1;
        set_held_component(game, SELECTED_FREQUENCY, Tag::Int(frequency));
        *claimed = true;
        return;
    }
    if is_sneaking(game) || !stack.item.contains("divination_rod") {
        return;
    }
    *claimed = true;

    let linked_block =
        component_string(game, stack, DIVINATION_LINKED_BLOCK).filter(|value| !value.is_empty());
    let linked_tag =
        component_string(game, stack, DIVINATION_LINKED_TAG).filter(|value| !value.is_empty());
    if linked_block.is_none() && linked_tag.is_none() {
        return;
    }

    let radius = component_i32(game, stack, DIVINATION_RANGE)
        .unwrap_or(DIVINATION_MAX_RANGE)
        .clamp(0, DIVINATION_MAX_RANGE);

    let origin = game
        .player
        .as_ref()
        .map(|p| [p.position.x, p.position.y, p.position.z])
        .unwrap_or([0.0, 0.0, 0.0]);
    let ox = origin[0] as f64;
    let oy = origin[1] as f64;
    let oz = origin[2] as f64;
    let origin_block = [
        origin[0].floor() as i32,
        origin[1].floor() as i32,
        origin[2].floor() as i32,
    ];

    // Java first expands a linked block through its non-empty ore tag. Only
    // when no such tag exists does it scan the block and its deepslate variant.
    let linked_ore_tag = linked_block.as_deref().and_then(|block| {
        ore_tag_candidates(block).into_iter().find(|tag| {
            game.res
                .items()
                .block_tags
                .get(tag)
                .is_some_and(|blocks| !blocks.is_empty())
        })
    });
    let linked_deepslate = linked_block.as_deref().and_then(|block| {
        deepslate_variant(block).filter(|candidate| {
            game.pack.assets.registry.default_state(candidate).is_some()
        })
    });

    let mut best: Option<([i32; 3], f32)> = None;
    let mut best_distance2 = f64::INFINITY;

    for dx in -radius..=radius {
        let x = origin_block[0] + dx;
        for dy in -radius..=radius {
            let y = origin_block[1] + dy;
            for dz in -radius..=radius {
                let z = origin_block[2] + dz;
                let pos = [x, y, z];
                let block = game.native_block(pos);
                if block.is_empty() || is_air(&block) {
                    continue;
                }

                let matches = if let Some(tag) = linked_tag.as_deref() {
                    game.res.items().block_in_tag(tag, block.as_str())
                } else if let Some(tag) = linked_ore_tag.as_deref() {
                    game.res.items().block_in_tag(tag, block.as_str())
                } else if let Some(id) = linked_block.as_deref() {
                    block.as_str() == id
                        || linked_deepslate.as_deref() == Some(block.as_str())
                } else {
                    false
                };
                if !matches {
                    continue;
                }

                let bx = x as f64 + 0.5;
                let by = y as f64 + 0.5;
                let bz = z as f64 + 0.5;
                let ddx = bx - ox;
                let ddy = by - oy;
                let ddz = bz - oz;
                let distance2 = ddx * ddx + ddy * ddy + ddz * ddz;
                // Scanner.scan rejects every centre outside the spherical
                // radius; a cube scan can otherwise select an out-of-range ore.
                if distance2 > f64::from(radius * radius) {
                    continue;
                }
                if distance2 < best_distance2 {
                    best_distance2 = distance2;
                    // DivinationRodItem.getDistance deliberately ignores Y.
                    // Keep nearest-result selection 3D, but bucket display 2D.
                    best = Some((pos, (ddx * ddx + ddz * ddz).sqrt() as f32));
                }
            }
        }
    }

    if let Some((pos, distance)) = best {
        set_held_component(
            game,
            DIVINATION_POS,
            Tag::IntArray(pos.to_vec()),
        );
        let bucket = distance_bucket(distance);
        set_held_component(
            game,
            DIVINATION_DISTANCE,
            Tag::Float(f32::to_bits(bucket)),
        );
    } else {
        set_held_component(
            game,
            DIVINATION_DISTANCE,
            Tag::Float(f32::to_bits(DIVINATION_NOT_FOUND)),
        );
    }
}

/// `getStateForPlacement`: a double-high apparatus is only placeable when the
/// block above can be replaced. Claim a prohibited click before the lower half
/// and its item are committed.
pub fn use_on(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    face: [i32; 3],
    claimed: &mut bool,
) {
    if *claimed {
        return;
    }
    if stack.item.as_ref() == MERCURIAL_WAND && wand_use_on(game, stack, pos, face) {
        *claimed = true;
        return;
    }
    if stack.item.as_ref() == WIRE_ITEM {
        wire_use_on(game, stack, pos, claimed);
        return;
    }
    if stack.item.contains("divination_rod") && attune_rod(game, stack, pos, claimed) {
        return;
    }
    if !is_double_high(&stack.item) {
        return;
    }
    let clicked = game.world.block(pos[0], pos[1], pos[2]);
    let clicked_replaceable = game.with_level(|b, _level, _ticks| b.is_replaceable(clicked));
    let placement = if clicked_replaceable {
        pos
    } else {
        [pos[0] + face[0], pos[1] + face[1], pos[2] + face[2]]
    };
    let above = [placement[0], placement[1] + 1, placement[2]];
    let state = game.world.block(above[0], above[1], above[2]);
    let replaceable = game.with_level(|b, _level, _ticks| b.is_replaceable(state));
    if !replaceable {
        *claimed = true;
    }
}

/// The two halves are one apparatus. Horizon's vanilla template does not know
/// Theurgy's `updateShape` rule, so a player breaking either half removes the
/// matching other half through the normal level path (including BE spill).
pub fn destroyed(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
    canceled: &mut bool,
) {
    if *canceled {
        return;
    }
    let Some(descriptor) = game.pack.assets.registry.state(state).cloned() else {
        return;
    };
    if !is_double_high(&descriptor.name) {
        return;
    }
    let Some(half) = descriptor.properties.get("half") else {
        return;
    };
    let (other, expected) = match half.as_str() {
        "lower" => ([pos[0], pos[1] + 1, pos[2]], "upper"),
        "upper" => ([pos[0], pos[1] - 1, pos[2]], "lower"),
        _ => return,
    };
    if game.native_block(other) != descriptor.name
        || game.native_block_property(other, "half").as_deref() != Some(expected)
    {
        return;
    }
    let Some(air) = game.pack.assets.registry.default_state("minecraft:air") else {
        return;
    };
    let dimension = game.dim.id();
    game.native_set_block(dimension, other, air, 3);
}

/// `playerWillDestroy`: return true when this is the lower half of a double
/// block broken in creative, where Java suppresses the duplicate drop. The
/// caller decides how to suppress it; this function never cancels the break.
pub fn destroy_suppresses_drop(
    stack: &ItemStack,
    state: horizon_world::registry::BlockStateId,
    creative: bool,
) -> bool {
    let _ = (stack, state);
    creative
}

/// The sal ammoniac bucket leaves an empty bucket, as the Java item's
/// `craftRemainder(Items.BUCKET)` does. The content declaration already
/// carries `crafting_remainder`; this is a pure helper for hosts that model
/// use rather than crafting.
pub fn bucket_remainder(item: &str) -> Option<&'static str> {
    (item == "theurgy:sal_ammoniac_bucket").then_some("minecraft:bucket")
}

/// Whether an item is one of Theurgy's own apparatus block items.
pub fn is_theurgy_item(item: &str) -> bool {
    item.starts_with("theurgy:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_high_set_matches_the_apparatus() {
        for id in DOUBLE_HIGH {
            assert!(is_double_high(id), "{id}");
        }
        assert!(!is_double_high("theurgy:calcination_oven_top"));
        assert!(!is_double_high("minecraft:furnace"));
    }

    #[test]
    fn bucket_keeps_the_empty_bucket() {
        assert_eq!(bucket_remainder("theurgy:sal_ammoniac_bucket"), Some("minecraft:bucket"));
        assert_eq!(bucket_remainder("minecraft:water_bucket"), None);
    }

    #[test]
    fn divination_ore_tags_prefer_the_full_two_word_name() {
        assert_eq!(
            ore_tag_candidates("theurgy:sal_ammoniac_ore"),
            vec!["c:ores/sal_ammoniac", "c:ores/ammoniac"]
        );
        assert_eq!(
            ore_tag_candidates("minecraft:deepslate_iron_ore"),
            vec!["c:ores/deepslate_iron", "c:ores/iron"]
        );
        assert_eq!(ore_tag_candidates("minecraft:stone"), vec!["c:ores/stone"]);
        assert_eq!(
            deepslate_variant("minecraft:iron_ore").as_deref(),
            Some("minecraft:deepslate_iron_ore")
        );
        assert_eq!(deepslate_variant("minecraft:deepslate_iron_ore"), None);
    }

    #[test]
    fn divination_tiers_use_the_vanilla_incorrect_block_tags() {
        assert_eq!(incorrect_blocks_tag("stone"), "minecraft:incorrect_for_stone_tool");
        assert_eq!(incorrect_blocks_tag("iron"), "minecraft:incorrect_for_iron_tool");
        assert_eq!(incorrect_blocks_tag("diamond"), "minecraft:incorrect_for_diamond_tool");
        assert_eq!(incorrect_blocks_tag("netherite"), "minecraft:incorrect_for_netherite_tool");
    }

    #[test]
    fn creative_drop_suppression_is_explicit() {
        assert!(destroy_suppresses_drop(&ItemStack::new("theurgy:distiller", 1), horizon_world::registry::BlockStateId(0), true));
        assert!(!destroy_suppresses_drop(&ItemStack::new("theurgy:distiller", 1), horizon_world::registry::BlockStateId(0), false));
    }
}
