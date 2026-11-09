// SPDX-License-Identifier: MIT
// Item behaviour that belongs to the block/item hooks. The carried content
// already supplies names, models, stack sizes, durability and tool rules; this
// module adds the two code behaviours that data cannot express: the upper half
// of the double-high apparatus, and the sal ammoniac bucket's fluid identity.
use horizon_client::items::ItemStack;
use horizon_world::chunk::BlockState;
use horizon_world::nbt::{Compound, JavaString, Tag};
use horizon_world::level::LevelAccess;

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
const DIVINATION_DURATION: &str = "theurgy:divination_settings_duration";
const DIVINATION_NOT_FOUND: f32 = 7.0;
const DIVINATION_SEARCHING: f32 = 8.0;

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

fn facing_name(direction: horizon_core::geometry::Direction) -> &'static str {
    match direction {
        horizon_core::geometry::Direction::Down => "down",
        horizon_core::geometry::Direction::Up => "up",
        horizon_core::geometry::Direction::North => "north",
        horizon_core::geometry::Direction::South => "south",
        horizon_core::geometry::Direction::West => "west",
        horizon_core::geometry::Direction::East => "east",
    }
}

fn horizontal_yaw(yaw: f32) -> &'static str {
    match (yaw.div_euclid(90.0).round() as i32).rem_euclid(4) {
        0 => "south",
        1 => "west",
        2 => "north",
        _ => "east",
    }
}

/// Native `getStateForPlacement` for facing, waterlogging and pipe/nexus
/// connection states. This happens before the block entity is created.
pub fn state_for_placement(
    level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId,
    context: &horizon_world::level::PlacementContext,
    fluid: horizon_world::fluid::FluidState,
    answer: &mut Option<Option<horizon_world::registry::BlockStateId>>,
) {
    let Some(mut descriptor) = level.registry.state(state).cloned() else {
        return;
    };
    let block = descriptor.name.as_str();
    if !block.starts_with("theurgy:") {
        return;
    }
    let connector = matches!(
        block,
        "theurgy:logistics_item_inserter"
            | "theurgy:logistics_item_extractor"
            | "theurgy:logistics_fluid_inserter"
            | "theurgy:logistics_fluid_extractor"
            | "theurgy:logistics_mercury_flux_connector"
            | "theurgy:logistics_capability_proxy"
            | "theurgy:caloric_flux_emitter"
            | "theurgy:mercury_flux_emitter"
            | "theurgy:sulfuric_flux_emitter"
    );
    if connector {
        let requested = descriptor.properties.get("facing").cloned();
        let mut facing = context.face;
        let neighbor = level.block_state(context.hit_pos);
        if level.registry.state(neighbor).is_some_and(|other| {
            other.name == block
                && other.properties.get("facing").is_some_and(|value| value == facing_name(facing))
        }) {
            facing = facing.opposite();
        }
        // An explicitly requested non-default state (commands/fixtures) must
        // not be replaced by the click's default north face. Ordinary item
        // placement still uses the clicked face, as Java getStateForPlacement.
        let facing = requested.filter(|value| value != "north")
            .unwrap_or_else(|| facing_name(facing).into());
        descriptor.properties.insert("facing".into(), facing);
    } else if matches!(block, "theurgy:fermentation_vat" | "theurgy:digestion_vat") {
        let facing = descriptor.properties.get("facing").cloned()
            .filter(|value| value != "north")
            .unwrap_or_else(|| horizontal_yaw(context.yaw).into());
        descriptor.properties.insert("facing".into(), facing);
        let open = descriptor.properties.get("open").cloned()
            .filter(|value| value == "false")
            .unwrap_or_else(|| "true".into());
        descriptor.properties.insert("open".into(), open);
    }
    if descriptor.properties.contains_key("waterlogged") {
        descriptor.properties.insert(
            "waterlogged".into(),
            (fluid.kind() == horizon_world::fluid::FluidKind::Water).to_string(),
        );
    }
    if block == "theurgy:logistics_nexus" {
        for direction in horizon_core::geometry::Direction::ALL {
            let at = horizon_core::position::BlockPos::new(
                context.pos.x + direction.normal()[0],
                context.pos.y + direction.normal()[1],
                context.pos.z + direction.normal()[2],
            );
            let neighbor = level.block_state(at);
            let present = level.registry.state(neighbor).is_some_and(|other| {
                other.name != "minecraft:air" && other.name != "minecraft:cave_air" && other.name != "minecraft:void_air"
            });
            descriptor.properties.insert(facing_name(direction).into(), present.to_string());
        }
    }
    if let Ok(resolved) = level.registry.resolve(&descriptor) {
        *answer = Some(Some(resolved));
    }
}

fn uuid_from_random(game: &mut crate::game::Game<'_>) -> (String, Tag) {
    use horizon_core::random::RandomSource;
    let mut most = game.world.random.next_long() as u64;
    let mut least = game.world.random.next_long() as u64;
    most = (most & 0xffff_ffff_ffff_0fff) | 0x0000_0000_0000_4000;
    least = (least & 0x3fff_ffff_ffff_ffff) | 0x8000_0000_0000_0000;
    let text = format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (most >> 32) as u32,
        ((most >> 16) & 0xffff) as u16,
        (most & 0xffff) as u16,
        (least >> 48) as u16,
        least & 0xffff_ffff_ffff
    );
    let values = vec![
        (most >> 32) as i32,
        most as i32,
        (least >> 32) as i32,
        least as i32,
    ];
    (text, Tag::IntArray(values))
}

fn uuid_text(game: &crate::game::Game<'_>, stack: &ItemStack) -> Option<String> {
    match game.native_component(stack, "theurgy:logistics_nexus_id").ok().flatten()? {
        Tag::String(value) => Some(value.to_string_lossy()),
        Tag::IntArray(values) if values.len() == 4 => {
            let most = ((values[0] as u32 as u64) << 32) | values[1] as u32 as u64;
            let least = ((values[2] as u32 as u64) << 32) | values[3] as u32 as u64;
            Some(format!(
                "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                (most >> 32) as u32,
                ((most >> 16) & 0xffff) as u16,
                (most & 0xffff) as u16,
                (least >> 48) as u16,
                least & 0xffff_ffff_ffff
            ))
        }
        _ => None,
    }
}

fn apply_block_item_components(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
) {
    let flux = component_i32(game, stack, "theurgy:mercury_flux_storage");
    let to_convert = component_i32(game, stack, "theurgy:mercury_flux_to_convert");
    let per_tick = component_i32(game, stack, "theurgy:current_mercury_flux_per_tick");
    let nexus = (stack.item.as_ref() == "theurgy:logistics_nexus")
        .then(|| uuid_text(game, stack))
        .flatten();
    let catalyst = (stack.item.as_ref() == "theurgy:mercury_catalyst")
        .then(|| game.native_component(stack, "theurgy:mercury_catalyst_inventory").ok().flatten().cloned())
        .flatten();
    game.native_with_mod_block_entity(pos, |entity| {
        if let Some(amount) = flux {
            let mut handler = Compound::new();
            handler.insert(JavaString::from("energy"), Tag::Int(amount.max(0)));
            entity.nbt.insert(JavaString::from("mercuryFluxHandler"), Tag::Compound(handler));
        }
        if let Some(amount) = to_convert {
            entity.nbt.insert(JavaString::from("mercuryFluxToConvert"), Tag::Int(amount));
        }
        if let Some(amount) = per_tick {
            entity.nbt.insert(JavaString::from("currentMercuryFluxPerTick"), Tag::Int(amount));
        }
        if let Some(id) = nexus {
            entity.nbt.insert(JavaString::from("nexusId"), Tag::String(JavaString::from(id.as_str())));
        }
        if let Some(contents) = catalyst {
            let records: Vec<Tag> = match contents {
                Tag::List(list) => list,
                Tag::Compound(map) => match map.get(&JavaString::from("items")).or_else(|| map.get(&JavaString::from("slots"))) {
                    Some(Tag::List(list)) => list.clone(),
                    _ => Vec::new(),
                },
                _ => Vec::new(),
            };
            if let Some(record) = records.first() {
                let template = match record {
                    Tag::Compound(map) => map.get(&JavaString::from("item")).unwrap_or(record),
                    other => other,
                };
                if let Some(item) = crate::game::theurgy_logistics::template_stack(template) {
                    if entity.items.is_empty() {
                        entity.items.resize(1, None);
                    }
                    entity.items[0] = Some(item);
                }
            }
        }
    });
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
    apply_block_item_components(game, stack, pos);
    if !is_double_high(&stack.item) {
        return;
    }
    let Some(upper) = upper_state(game, state) else { return; };
    let dimension = game.dim.id();
    game.native_set_block(dimension, [pos[0], pos[1] + 1, pos[2]], upper, 3);
}

/// `LogisticsNexusBlockItem.inventoryTick`: every stack receives one stable
/// random UUID before it can be paired, without changing an existing id.
pub fn inventory_tick(game: &mut crate::game::Game<'_>, slot: i32) {
    let Some(stack) = game.native_carried_stack(slot).cloned() else {
        return;
    };
    if stack.item.as_ref() != "theurgy:logistics_nexus_id" && stack.item.as_ref() != "theurgy:logistics_nexus" {
        return;
    }
    if uuid_text(game, &stack).is_some() {
        return;
    }
    let (_, tag) = uuid_from_random(game);
    let Some(mut stack) = game.native_carried_stack(slot).cloned() else {
        return;
    };
    if game.native_set_component(&mut stack, "theurgy:logistics_nexus_id", tag).is_ok()
        && let Some(current) = game.native_carried_stack(slot)
    {
        *current = stack;
    }
}

fn component_string_with_defaults(
    game: &crate::game::Game<'_>,
    stack: &ItemStack,
    id: &str,
) -> Option<String> {
    component_string(game, stack, id).or_else(|| {
        let info = game.res.items().get(&stack.item)?;
        match info.custom_components.get(id) {
            Some(Tag::String(value)) => Some(value.to_string_lossy()),
            _ => None,
        }
    })
}

/// Read a registered native component from `stack` as a string.
pub(crate) fn component_string(
    game: &crate::game::Game<'_>,
    stack: &ItemStack,
    id: &str,
) -> Option<String> {
    if let Some(Tag::String(value)) = stack.data().and_then(|data| data.other(id)) {
        return Some(value.to_string_lossy());
    }
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
pub(crate) fn component_i32(game: &crate::game::Game<'_>, stack: &ItemStack, id: &str) -> Option<i32> {
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

fn acting_position(game: &crate::game::Game<'_>) -> [f64; 3] {
    game.native_acting_player()
        .map(|player| player.position)
        .or_else(|| game.player.as_ref().map(|p| [p.position.x, p.position.y, p.position.z]))
        .unwrap_or([0.0; 3])
}

fn play_tuning_fork(game: &mut crate::game::Game<'_>) {
    let at = acting_position(game);
    game.with_level(|_, level, _| {
        level.native_play_sound("theurgy:tuning_fork", at, 7, 1.0, 1.0)
    });
}

fn overlay(game: &mut crate::game::Game<'_>, key: &str, args: &[horizon_client::text::Text]) {
    let player = game.native_acting_id();
    game.native_action_bar(player, horizon_client::text::Text::translate(key, args));
}

fn held_distance(game: &crate::game::Game<'_>, stack: &ItemStack) -> Option<[i32; 3]> {
    match game.native_component(stack, DIVINATION_POS).ok().flatten()? {
        Tag::IntArray(values) if values.len() == 3 => Some([values[0], values[1], values[2]]),
        Tag::List(values) if values.len() == 3 => {
            let mut pos = [0; 3];
            for (index, value) in values.iter().enumerate() {
                let Tag::Int(value) = value else { return None; };
                pos[index] = *value;
            }
            Some(pos)
        }
        _ => None,
    }
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
    let linked = translated_source_block(game, &block).unwrap_or(block);
    set_held_component(
        game,
        DIVINATION_LINKED_BLOCK,
        Tag::String(JavaString::from(linked.as_str())),
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
pub(crate) fn wand_mode(game: &crate::game::Game<'_>, stack: &ItemStack) -> i32 {
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
fn change_wand_mode(game: &mut crate::game::Game<'_>, direction: i32) -> bool {
    let slot = game.selected as i32;
    let Some(held) = game.native_carried_stack(slot).cloned() else { return false; };
    if held.item.as_ref() != MERCURIAL_WAND { return false; }
    let current = wand_mode(game, &held);
    let next = (current + direction).rem_euclid(5);
    set_held_component(game, MERCURIAL_WAND_MODE,
        Tag::String(JavaString::from(WAND_MODE_NAMES[next as usize])));
    let frequency = horizon_client::text::Text::literal(
        component_i32(game, &held, SELECTED_FREQUENCY).unwrap_or(0).to_string());
    match next {
        0 => overlay(game, "item.mode.theurgy.mercurial_wand.rotate_selected_direction", &[]),
        1 => overlay(game, "item.mode.theurgy.mercurial_wand.switch_logistics_enabled", &[]),
        2 => overlay(game, "item.mode.theurgy.mercurial_wand.select_frequency", &[frequency.clone()]),
        3 => overlay(game, "item.mode.theurgy.mercurial_wand.set_selected_frequency", &[frequency]),
        4 => overlay(game, "item.mode.theurgy.mercurial_wand.cycle_capacitor_side", &[]),
        _ => {}
    }
    true
}

/// KeyMappingsRegistry.CHANGE_ITEM_MODE sends MessageSetMode(1), the same
/// server-side transition as one positive mode-scroll step without sneaking.
pub fn key_change_mode(game: &mut crate::game::Game<'_>) -> bool {
    change_wand_mode(game, 1)
}

pub fn mouse_scroll(game: &mut crate::game::Game<'_>, _scroll: [f64; 2], accumulated: [i32; 2], canceled: &mut bool) {
    if *canceled || !is_sneaking(game) || accumulated[1] == 0 { return; }
    let direction = if accumulated[1] > 0 { 1 } else { -1 };
    if change_wand_mode(game, direction) {
        *canceled = true;
    }
}

fn bucket_fluid(item: &str) -> Option<&'static str> {
    match item {
        "minecraft:water_bucket" => Some("minecraft:water"),
        "minecraft:lava_bucket" => Some("minecraft:lava"),
        "theurgy:sal_ammoniac_bucket" => Some("theurgy:sal_ammoniac"),
        _ => None,
    }
}

fn fluid_bucket(fluid: &str) -> Option<&'static str> {
    match fluid {
        "minecraft:water" => Some("minecraft:water_bucket"),
        "minecraft:lava" => Some("minecraft:lava_bucket"),
        "theurgy:sal_ammoniac" => Some("theurgy:sal_ammoniac_bucket"),
        _ => None,
    }
}

fn mod_tank(block: &str) -> Option<i32> {
    match block {
        "theurgy:sal_ammoniac_tank" => Some(2_000),
        "theurgy:liquefaction_cauldron" => Some(2_000),
        "theurgy:fermentation_vat" | "theurgy:digestion_vat" | "theurgy:sal_ammoniac_accumulator" => Some(10_000),
        _ => None,
    }
}

fn tank_accepts(block: &str, fluid: &str) -> bool {
    match block {
        "theurgy:liquefaction_cauldron" | "theurgy:sal_ammoniac_accumulator" => {
            fluid == if block == "theurgy:liquefaction_cauldron" { "theurgy:sal_ammoniac" } else { "minecraft:water" }
        }
        "theurgy:fermentation_vat" | "theurgy:digestion_vat" => {
            matches!(fluid, "minecraft:water" | "theurgy:sal_ammoniac")
        }
        "theurgy:sal_ammoniac_tank" => true,
        _ => false,
    }
}

fn replace_held(game: &mut crate::game::Game<'_>, item: &str) {
    if game.native_acting_player().is_some_and(|player| player.game_mode == 1) {
        return;
    }
    if let Some(held) = game.native_carried_stack(game.selected as i32) {
        *held = ItemStack::new(item, 1);
    }
}

/// Bucket and empty-hand interactions of `OneTankFluidHandlerBehaviour`, plus
/// the custom fluid identity of the sal ammoniac bucket in the world.
fn fluid_use_on(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    face: [i32; 3],
) -> bool {
    let block = game.native_block(pos);
    if let Some(capacity) = mod_tank(&block) {
        let open = game.native_block_property(pos, "open").as_deref() != Some("false");
        let shift = is_sneaking(game);
        if stack.item.as_ref() == "minecraft:air" && shift {
            if open {
                game.native_with_mod_block_entity(pos, |entity| {
                    let empty = theurgy_engine::recipes::Tank { fluid: None, amount: 0, capacity };
                    crate::game::theurgy_host::save_tanks(entity, &[empty]);
                });
                game.native_block_entity_sync(pos);
            }
            return true;
        }
        if !open {
            return false;
        }
        if let Some(fluid) = bucket_fluid(&stack.item) {
            if !tank_accepts(&block, fluid) {
                return false;
            }
            let accepted = game.native_with_mod_block_entity(pos, |entity| {
                let mut tank = crate::game::theurgy_host::tank(entity, 0, capacity);
                if tank.fluid.as_deref().is_some_and(|old| old != fluid) {
                    return false;
                }
                let accepted = 1000.min(capacity - tank.amount);
                if accepted <= 0 {
                    return false;
                }
                tank.fluid = Some(fluid.to_owned());
                tank.amount += accepted;
                crate::game::theurgy_host::save_tanks(entity, &[tank]);
                true
            }).unwrap_or(false);
            if accepted {
                replace_held(game, "minecraft:bucket");
                game.native_block_entity_sync(pos);
            }
            return accepted;
        }
        if stack.item.as_ref() == "minecraft:bucket" {
            let drained = game.native_with_mod_block_entity(pos, |entity| {
                let mut tank = crate::game::theurgy_host::tank(entity, 0, capacity);
                let Some(fluid) = tank.fluid.clone().filter(|_| tank.amount >= 1000) else {
                    return None;
                };
                tank.amount -= 1000;
                if tank.amount == 0 {
                    tank.fluid = None;
                }
                crate::game::theurgy_host::save_tanks(entity, &[tank]);
                fluid_bucket(&fluid)
            }).flatten();
            if let Some(bucket) = drained {
                replace_held(game, bucket);
                game.native_block_entity_sync(pos);
                return true;
            }
        }
        return false;
    }

    // Vanilla BucketItem knows only vanilla fluids. Handle the mod bucket's
    // world pickup/placement explicitly and leave vanilla buckets untouched.
    if stack.item.as_ref() == "theurgy:sal_ammoniac_bucket" {
        let clicked_state = game.world.block(pos[0], pos[1], pos[2]);
        let clicked_replaceable = game.with_level(|b, _, _| b.is_replaceable(clicked_state));
        let target = if clicked_replaceable {
            pos
        } else {
            [pos[0] + face[0], pos[1] + face[1], pos[2] + face[2]]
        };
        let target_state = game.world.block(target[0], target[1], target[2]);
        if !game.with_level(|b, _, _| b.is_replaceable(target_state)) {
            return false;
        }
        let Some(fluid) = game.pack.assets.registry.default_state("theurgy:sal_ammoniac") else {
            return false;
        };
        if game.native_set_block(game.dim.id(), target, fluid, 3) {
            replace_held(game, "minecraft:bucket");
            return true;
        }
    } else if stack.item.as_ref() == "minecraft:bucket"
        && block == "theurgy:sal_ammoniac"
        && game.native_block_property(pos, "level").as_deref().unwrap_or("0") == "0"
    {
        let Some(air) = game.pack.assets.registry.default_state("minecraft:air") else {
            return false;
        };
        if game.native_set_block(game.dim.id(), pos, air, 3) {
            replace_held(game, "theurgy:sal_ammoniac_bucket");
            return true;
        }
    }
    false
}

pub fn right_click_block(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    _face: [i32; 3],
    canceled: &mut bool,
) {
    if *canceled { return; }
    if fluid_use_on(game, stack, pos, _face) {
        *canceled = true;
        return;
    }
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

fn translated_source_block(game: &crate::game::Game<'_>, source: &str) -> Option<String> {
    if let Some(mapped) = crate::game::theurgy_config::source_override(source) {
        return Some(mapped.to_owned());
    }
    if game.pack.assets.registry.default_state(source).is_some() {
        return Some(source.to_owned());
    }
    let (namespace, path) = source.split_once(':')?;
    let mut translated = path.to_owned();
    for (needle, replacement) in [("raw_", ""), ("_nugget", ""), ("_ingot", ""), ("_dust", ""), ("_block", "")] {
        if translated.contains(needle) {
            translated = translated.replace(needle, replacement);
            break;
        }
    }
    if translated.contains("deepslate_") {
        translated = translated.replace("deepslate_", "");
    } else if translated.contains("_ore") {
        translated = translated.replace("_ore", "");
    }
    translated.push_str("_ore");
    let candidate = format!("{namespace}:{translated}");
    game.pack.assets.registry.default_state(&candidate).map(|_| candidate)
}

fn translated_source_tag(game: &crate::game::Game<'_>, source: &str) -> Option<String> {
    if let Some(mapped) = crate::game::theurgy_config::source_override(source) {
        return Some(mapped.to_owned());
    }
    let exists = |tag: &str| game.res.items().block_tags.get(tag).is_some_and(|blocks| !blocks.is_empty());
    if exists(source) {
        return Some(source.to_owned());
    }
    if source == "minecraft:coals" {
        return Some("c:ores/coal".to_owned());
    }
    let (namespace, path) = source.split_once(':')?;
    let mut translated = path.to_owned();
    for prefix in ["ingots/", "nuggets/", "raw_materials/", "dusts/", "storage_blocks/", "gems/"] {
        if translated.contains(prefix) {
            translated = translated.replace(prefix, "ores/");
            break;
        }
    }
    if translated.ends_with("/netherite") {
        translated = translated.replace("/netherite", "/netherite_scrap");
    }
    let candidate = format!("{namespace}:{translated}");
    exists(&candidate).then_some(candidate)
}

/// DivinationRodRecipe.assemble: copy the sulfur's source into the rod's
/// linked block/tag components on the authoritative crafting result.
pub fn prepare_item_craft(
    game: &mut crate::game::Game<'_>,
    result: &mut Option<ItemStack>,
) {
    let Some((recipe, _)) = game.craft_recipe() else { return; };
    if !recipe.contains("sulfur_attuned_divination_rod") {
        return;
    }
    let Some(result) = result else { return; };
    if component_string(game, result, DIVINATION_LINKED_BLOCK).is_some()
        || component_string(game, result, DIVINATION_LINKED_TAG).is_some()
    {
        return;
    }
    for stack in game.craft_grid.iter().flatten() {
        if let Some(source) = component_string_with_defaults(game, stack, "theurgy:source_item") {
            if let Some(block) = translated_source_block(game, &source) {
                result.data_mut().set_other(DIVINATION_LINKED_BLOCK, Some(Tag::String(JavaString::from(block.as_str()))));
                break;
            }
        }
        if let Some(source) = component_string_with_defaults(game, stack, "theurgy:source_tag") {
            if let Some(tag) = translated_source_tag(game, &source) {
                result.data_mut().set_other(DIVINATION_LINKED_TAG, Some(Tag::String(JavaString::from(tag.as_str()))));
                break;
            }
        }
    }
}

/// Start the Java timed use. The actual scan and durability cost happen only
/// when the full duration completes; an early release restores the old result.
pub fn use_item(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    claimed: &mut bool,
) {
    if *claimed { return; }
    if stack.item.as_ref() == "theurgy:the_hermetica" {
        // TheHermeticaItem.use -> Modonomicon's client book. Headless and
        // dedicated-server calls simply claim the use without a fake window.
        let _ = crate::game::theurgy_client::open_book(game);
        *claimed = true;
        return;
    }
    if stack.item.as_ref() == MERCURIAL_WAND && wand_mode(game, stack) == 2 {
        let frequency = component_i32(game, stack, SELECTED_FREQUENCY).unwrap_or(0) + 1;
        set_held_component(game, SELECTED_FREQUENCY, Tag::Int(frequency));
        *claimed = true;
        return;
    }
    if stack.item.as_ref() == WIRE_ITEM {
        if is_sneaking(game) {
            clear_wire_endpoint(game);
            *claimed = true;
        }
        return;
    }
    if is_sneaking(game) || !stack.item.contains("divination_rod") {
        return;
    }
    let linked = component_string(game, stack, DIVINATION_LINKED_BLOCK).is_some()
        || component_string(game, stack, DIVINATION_LINKED_TAG).is_some();
    if !linked {
        *claimed = true;
        overlay(game, "message.theurgy.divination_rod.no_link", &[]);
        return;
    }
    // Leave the click unclaimed so Horizon's `start_mod_use` starts the timed
    // native use. Claiming here would skip `native_living_entity_use_item_finish`.
    set_held_component(game, DIVINATION_DISTANCE, Tag::Float(DIVINATION_SEARCHING.to_bits()));
    play_tuning_fork(game);
}

/// Server half of `DivinationRodItem.finishUsingItem`: scan, apply cooldown
/// and durability, and break a rod which reached its maximum on the prior use.
fn scan_rod(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
) {
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

    let origin = acting_position(game);
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

/// `DivinationRodItem.getUseDuration`.
pub fn use_duration(
    game: &crate::game::Game<'_>,
    stack: &ItemStack,
    duration: &mut i32,
) {
    if stack.item.contains("divination_rod") {
        *duration = component_i32(game, stack, DIVINATION_DURATION).unwrap_or(40).max(0);
    }
}

/// `DivinationRodItem.releaseUsing`: no scan and no durability cost. Preserve
/// a previous result, or restore NOT_FOUND when there was none.
pub fn release_using(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    _entity: Option<i32>,
    _time_left: i32,
    used: &mut bool,
) {
    if !stack.item.contains("divination_rod") {
        return;
    }
    let distance = held_distance(game, stack).map_or(DIVINATION_NOT_FOUND, |pos| {
        let player = acting_position(game);
        let dx = pos[0] as f64 + 0.5 - player[0];
        let dz = pos[2] as f64 + 0.5 - player[2];
        distance_bucket((dx * dx + dz * dz).sqrt() as f32)
    });
    set_held_component(game, DIVINATION_DISTANCE, Tag::Float(distance.to_bits()));
    *used = false;
}

/// Completion hook. `result` is the actual stack returned by Horizon, so this
/// works for either hand even though the older scan helper writes main-hand
/// components for compatibility with the existing use hooks.
pub fn finish_using(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    _entity: Option<i32>,
    hand: horizon_client::first_person::Hand,
    _duration: i32,
    result: &mut Option<ItemStack>,
) {
    if !stack.item.contains("divination_rod") {
        return;
    }
    let mut result_stack = stack.clone();
    let max_damage = component_i32(game, stack, "theurgy:divination_settings_max_damage")
        .unwrap_or(1)
        .max(1) as u32;
    if result_stack.damage >= max_damage {
        result_stack.count = result_stack.count.saturating_sub(1);
        result_stack.damage = 0;
        *result = (result_stack.count > 0).then_some(result_stack);
        return;
    }

    scan_rod(game, stack);
    if hand == horizon_client::first_person::Hand::Main {
        result_stack = game
            .native_carried_stack(game.selected as i32)
            .cloned()
            .unwrap_or(result_stack);
    } else {
        // Off-hand scans use the same world result. Copy the two result
        // components directly rather than accidentally editing the main hand.
        if let Some(main) = game.native_carried_stack(game.selected as i32).cloned() {
            for id in [DIVINATION_POS, DIVINATION_DISTANCE] {
                if let Ok(Some(value)) = game.native_component(&main, id) {
                    let _ = game.native_set_component(&mut result_stack, id, value.clone());
                }
            }
        }
    }

    let duration = component_i32(game, stack, DIVINATION_DURATION).unwrap_or(40).max(0);
    game.add_cooldown(std::sync::Arc::from(stack.item.as_ref()), duration);
    let creative = game.native_acting_player().is_some_and(|player| player.game_mode == 1);
    if !creative {
        result_stack.damage = result_stack.damage.saturating_add(1);
    }
    *result = Some(result_stack);
}

/// Java uses the ordinary item pose, not bow/spyglass animations.
pub fn use_animation(
    stack: &ItemStack,
    animation: &mut horizon_world::consumable::UseAnimation,
) {
    if stack.item.contains("divination_rod") {
        *animation = horizon_world::consumable::UseAnimation::None;
    }
}

/// Client tooltip parity for linked rods and their last result.
pub fn tooltip(
    game: &crate::game::Game<'_>,
    stack: &ItemStack,
    lines: &mut Vec<(horizon_client::text::Text, u32)>,
) {
    if let Some(id) = uuid_text(game, stack) {
        lines.push((horizon_client::text::Text::literal(format!("Nexus ID: {id}")), u32::MAX));
    }
    if let Some(stored) = component_i32(game, stack, "theurgy:mercury_flux_storage") {
        let capacity = match stack.item.as_ref() {
            "theurgy:mercury_catalyst" => 50_000,
            "theurgy:mercury_capacitor" => 500_000,
            _ => 1_000,
        };
        lines.push((horizon_client::text::Text::literal(format!("Mercury Flux: {stored} / {capacity}")), u32::MAX));
    }
    let sulfur = stack.item.starts_with("theurgy:alchemical_sulfur_");
    let niter = stack.item.starts_with("theurgy:alchemical_niter_");
    let salt = stack.item.starts_with("theurgy:alchemical_salt_");
    if sulfur || niter || salt {
        // AlchemicalDerivativeItem.getTooltipData and AlchemicalSaltItem:
        // source name, [tier], then sulfur/niter type. The pinned lang has a
        // `.source` entry for every generated derivative, including tag-based
        // and direct-item sources.
        let source_key = format!("item.{}.source", stack.item.replace(':', "."));
        lines.push((horizon_client::text::Text::translate(&source_key, &[]), 0xFF55_FF55));
        let tier = if niter {
            ["abundant", "common", "commond", "rare", "precious"].into_iter()
                .find(|tier| stack.item.ends_with(&format!("_{tier}")))
                .map(|tier| if tier == "commond" { "common" } else { tier })
        } else if sulfur {
            ["abundant", "common", "rare", "precious"].into_iter().find(|tier| {
                game.res.items().item_in_tag(&format!("theurgy:alchemical_sulfurs/{tier}"), &stack.item)
            })
        } else {
            None
        };
        if let Some(tier) = tier {
            lines.push((horizon_client::text::Text::translate(&format!("theurgytheurgy.alchemical_tier.{tier}"), &[]), 0xFFAA_AA00));
        }
        if niter {
            lines.push((horizon_client::text::Text::translate("item.theurgy.alchemical_derivative.type.niter", &[]), 0xFF55_5555));
        } else if sulfur {
            let kind = ["animals", "crops", "earthen_matters", "gems", "logs", "metals", "mobs", "other_minerals"]
                .into_iter().find(|kind| game.res.items().item_in_tag(&format!("theurgy:alchemical_sulfurs/{kind}"), &stack.item))
                .unwrap_or("misc");
            lines.push((horizon_client::text::Text::translate(&format!("theurgytheurgy.sulfur_type.{kind}"), &[]), 0xFF55_5555));
        }
    }
    if !stack.item.contains("divination_rod") {
        return;
    }
    let linked = component_string(game, stack, DIVINATION_LINKED_BLOCK)
        .or_else(|| component_string(game, stack, DIVINATION_LINKED_TAG));
    let Some(linked) = linked else {
        lines.push((horizon_client::text::Text::translate("tooltip.theurgy..divination_rod.no_link", &[]), u32::MAX));
        return;
    };
    let target = if let Some((namespace, path)) = linked.strip_prefix('#').unwrap_or(&linked).split_once(':') {
        if linked.starts_with('#') {
            horizon_client::text::Text::literal(linked.clone())
        } else {
            horizon_client::text::Text::translate(&format!("block.{namespace}.{path}"), &[])
        }
    } else {
        horizon_client::text::Text::literal(linked.clone())
    };
    lines.push((horizon_client::text::Text::translate("tooltip.theurgy..divination_rod.linked_to", &[target.clone()]), u32::MAX));
    if let Some(pos) = held_distance(game, stack) {
        let coordinates = horizon_client::text::Text::literal(format!("{}, {}, {}", pos[0], pos[1], pos[2]));
        lines.push((horizon_client::text::Text::translate("tooltip.theurgy..divination_rod.last_result", &[target, coordinates]), u32::MAX));
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
        // `right_click_block` owns the complete WireItem.useOn transaction.
        // Running it again here would store a fresh endpoint after the early
        // hook had just connected/disconnected and cleared the old one.
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

/// Java `canSurvive` of directional attachments. Logistics connectors/node/
/// probe/proxy (`LogisticsItemConnectorBlock.canSurvive` and siblings) need
/// any non-air block behind them; the three flux emitters
/// (`CaloricFluxEmitterBlock.canSurvive`) need a sturdy face.
/// `facing` is the block's FACING, `support_air`/`support_sturdy` describe the
/// block at `pos.relative(facing.opposite())`.
fn attachment_survives(block: &str, support_air: bool, support_sturdy: bool) -> Option<bool> {
    if WIRE_ENDPOINTS.contains(&block) {
        Some(!support_air)
    } else if matches!(block, "theurgy:caloric_flux_emitter" | "theurgy:mercury_flux_emitter" | "theurgy:sulfuric_flux_emitter") {
        Some(support_sturdy)
    } else {
        None
    }
}

fn is_air_name(name: &str) -> bool {
    matches!(name, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
}

/// `updateShape` (connectors) / `neighborChanged`+`destroyBlock` (emitters):
/// when the support behind the block changes and no longer holds it, the
/// block becomes air and drops, as Java.
pub fn update_shape(
    level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId,
    direction: horizon_core::geometry::Direction,
    neighbour: horizon_world::registry::BlockStateId,
    new: &mut Option<horizon_world::registry::BlockStateId>,
) {
    let Some(descriptor) = level.registry.state(state) else { return; };
    if descriptor.properties.get("facing").map(String::as_str) != Some(facing_name(direction.opposite())) {
        return;
    }
    let support_air = level.registry.state(neighbour).is_none_or(|other| is_air_name(&other.name));
    let support_sturdy = level.behaviours.is_face_sturdy(neighbour, direction.opposite());
    if attachment_survives(&descriptor.name, support_air, support_sturdy) == Some(false) {
        *new = level.registry.default_state("minecraft:air");
    }
}

/// `canSurvive` as asked by `BlockItem.canPlace`: refuse a spot without support.
pub fn can_survive(
    level: &crate::game::GameLevel<'_>,
    pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId,
    survives: &mut Option<bool>,
) {
    let Some(descriptor) = level.registry.state(state) else { return; };
    let Some(facing) = descriptor.properties.get("facing") else { return; };
    let Some(direction) = [
        horizon_core::geometry::Direction::Down,
        horizon_core::geometry::Direction::Up,
        horizon_core::geometry::Direction::North,
        horizon_core::geometry::Direction::South,
        horizon_core::geometry::Direction::West,
        horizon_core::geometry::Direction::East,
    ].into_iter().find(|d| facing_name(*d) == facing) else { return; };
    let [dx, dy, dz] = direction.opposite().normal();
    let support = level.block_state(pos.offset(dx, dy, dz));
    let support_air = level.registry.state(support).is_none_or(|other| is_air_name(&other.name));
    let support_sturdy = level.behaviours.is_face_sturdy(support, direction);
    if let Some(answer) = attachment_survives(&descriptor.name, support_air, support_sturdy) {
        *survives = Some(answer);
    }
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
    fn attachments_follow_java_can_survive() {
        // Connectors: any non-air support, even a non-sturdy one.
        assert_eq!(attachment_survives("theurgy:logistics_fluid_inserter", true, false), Some(false));
        assert_eq!(attachment_survives("theurgy:logistics_fluid_inserter", false, false), Some(true));
        assert_eq!(attachment_survives("theurgy:logistics_mercury_flux_connector", true, true), Some(false));
        // Emitters: sturdy face only.
        assert_eq!(attachment_survives("theurgy:caloric_flux_emitter", false, false), Some(false));
        assert_eq!(attachment_survives("theurgy:sulfuric_flux_emitter", false, true), Some(true));
        assert_eq!(attachment_survives("theurgy:calcination_oven", true, false), None);
        assert!(is_air_name("minecraft:cave_air") && !is_air_name("minecraft:stone"));
    }

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
