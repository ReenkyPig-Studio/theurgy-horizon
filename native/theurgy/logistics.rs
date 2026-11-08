// SPDX-License-Identifier: MIT
use crate::game::theurgy_host::{bool, int, set_int};
use horizon_client::containers::{BlockEntity, ModBlockEntity};
use horizon_client::items::ItemStack;
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, JavaString, Tag};
use std::collections::BTreeSet;
use std::sync::Mutex;

static LAST_FILTER_ACTION: Mutex<Option<(u128, [i32; 3], i64)>> = Mutex::new(None);
static LAST_WIRE_ACTION: Mutex<Option<(u128, [i32; 3], i64)>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// Pure logic: logistics network model
// ---------------------------------------------------------------------------

/// The mode of a leaf node in the logistics network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeafNodeMode {
    Insert,
    Extract,
}

/// The kind of resource a connector transfers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityKind {
    Item,
    Fluid,
    MercuryFlux,
}

impl CapabilityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CapabilityKind::Item => "item",
            CapabilityKind::Fluid => "fluid",
            CapabilityKind::MercuryFlux => "mercury_flux",
        }
    }
}

/// A 3D position (dimension-agnostic; Nexus pairing uses the nexus UUID).
pub type Pos = [i32; 3];

/// A single leaf node registered on a network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeafNode {
    pub pos: Pos,
    pub mode: LeafNodeMode,
    pub kind: CapabilityKind,
}

/// The in-memory record of a block entity's logistics connector/nexus.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogisticsState {
    /// Whether this connector is enabled. Disabled == absent from network.
    pub enabled: bool,
    /// Optional direction override, or None to use the block's FACING.
    pub direction_override: Option<u8>,
    /// Item/fluid extractor: amount per extraction tick.
    pub extraction_amount: i32,
    /// Mercury-flux connector buffer contents.
    pub flux_buffer: i32,
}

impl LogisticsState {
    pub const DEFAULT_ENABLED: bool = true;

    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            direction_override: None,
            extraction_amount: 0,
            flux_buffer: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// NBT persistence
// ---------------------------------------------------------------------------

pub fn read_state(nbt: &horizon_world::nbt::Compound) -> LogisticsState {
    let enabled = nbt
        .get(&horizon_world::nbt::JavaString::from("enabled"))
        .map(|_| bool(nbt, "enabled"))
        .unwrap_or(LogisticsState::DEFAULT_ENABLED);
    let direction_override =
        match nbt.get(&horizon_world::nbt::JavaString::from("directionOverride")) {
            Some(horizon_world::nbt::Tag::Int(v)) if (0..6).contains(v) => Some(*v as u8),
            Some(horizon_world::nbt::Tag::Byte(v)) if (0..6).contains(v) => Some(*v as u8),
            Some(horizon_world::nbt::Tag::Short(v)) if (0..6).contains(v) => Some(*v as u8),
            _ => None,
        };
    let flux_buffer = match nbt.get(&horizon_world::nbt::JavaString::from("buffer")) {
        Some(horizon_world::nbt::Tag::Compound(child)) => int(child, "energy", 0),
        _ => match nbt.get(&horizon_world::nbt::JavaString::from("mercuryFluxHandler")) {
            Some(horizon_world::nbt::Tag::Compound(child)) => int(child, "energy", 0),
            _ => int(nbt, "fluxBuffer", 0),
        },
    };
    LogisticsState {
        enabled,
        direction_override,
        extraction_amount: int(nbt, "extractionAmount", 0),
        flux_buffer,
    }
}

pub fn write_state(nbt: &mut horizon_world::nbt::Compound, state: &LogisticsState) {
    set_int(nbt, "enabled", i32::from(state.enabled));
    match state.direction_override {
        Some(v) => set_int(nbt, "directionOverride", i32::from(v)),
        None => {
            nbt.remove(&horizon_world::nbt::JavaString::from("directionOverride"));
        }
    }
    set_int(nbt, "extractionAmount", state.extraction_amount);
    let mut buffer = Compound::new();
    set_int(&mut buffer, "energy", state.flux_buffer.clamp(0, FLUX_BUFFER_CAPACITY));
    nbt.insert(JavaString::from("buffer"), Tag::Compound(buffer.clone()));
    nbt.insert(JavaString::from("mercuryFluxHandler"), Tag::Compound(buffer));
    nbt.remove(&JavaString::from("fluxBuffer"));
}

// ---------------------------------------------------------------------------
// Network assembly (pure)
// ---------------------------------------------------------------------------

pub fn leaf_nodes_of(
    nexus_id: &str,
    candidates: &[(Pos, &str, LeafNodeMode, CapabilityKind)],
) -> Vec<LeafNode> {
    candidates
        .iter()
        .filter(|(_, candidate_nexus, _, _)| *candidate_nexus == nexus_id)
        .map(|(pos, _, mode, kind)| LeafNode {
            pos: *pos,
            mode: *mode,
            kind: *kind,
        })
        .collect()
}

pub fn insert_targets<'a>(
    nodes: &'a [LeafNode],
    extractor: Pos,
    kind: CapabilityKind,
) -> Vec<&'a LeafNode> {
    nodes
        .iter()
        .filter(|n| n.kind == kind && n.mode == LeafNodeMode::Insert && n.pos != extractor)
        .collect()
}

// ---------------------------------------------------------------------------
// Timing / transfer helpers (pure)
// ---------------------------------------------------------------------------

pub const ITEM_EXTRACTION_EVERY_N_TICKS: i32 = 20;
pub const FLUID_EXTRACTION_EVERY_N_TICKS: i32 = 20;
pub const FLUX_TRANSFER_EVERY_N_TICKS: i32 = 20;
pub const MAX_ITEM_EXTRACTION_AMOUNT: i32 = 64;
pub const MAX_FLUID_EXTRACTION_AMOUNT: i32 = 1000 * 5;
pub const FLUX_BUFFER_CAPACITY: i32 = 1000;
pub const FLUX_DEFAULT_TRANSFER_RATE: i32 = 100;
pub const FLUX_TRANSFER_RATE: i32 = FLUX_DEFAULT_TRANSFER_RATE;

pub fn on_slow_tick(offset: i32, game_time: i64, period: i32) -> bool {
    if period <= 0 {
        return false;
    }
    let shifted = (game_time.rem_euclid(i64::from(period))) as i32;
    (offset + shifted).rem_euclid(period) == 0
}

pub fn slow_tick_offset(pos: Pos, period: i32) -> i32 {
    if period <= 0 {
        return 0;
    }
    let h = (pos[0] as i64).wrapping_mul(73_856_093)
        ^ (pos[1] as i64).wrapping_mul(19_349_663)
        ^ (pos[2] as i64).wrapping_mul(83_492_791);
    (h.rem_euclid(i64::from(period))) as i32
}

pub fn distribute_evenly(total: i32, sink_count: usize) -> Vec<i32> {
    if sink_count == 0 || total <= 0 {
        return Vec::new();
    }
    let n = sink_count as i32;
    let per = total / n;
    let rem = total % n;
    (0..sink_count)
        .map(|i| per + i32::from((i as i32) < rem))
        .collect()
}

pub fn flux_pushes_for(buffer: i32, sink_count: usize) -> i32 {
    if buffer <= 0 || sink_count == 0 {
        return 0;
    }
    let budget = FLUX_TRANSFER_RATE.saturating_mul(sink_count as i32);
    buffer.min(budget)
}

pub fn connector_direction(override_dir: Option<u8>, facing: u8) -> u8 {
    override_dir.unwrap_or(facing)
}

// FilterBehaviour.writeNetwork uses ItemStack.OPTIONAL_CODEC (the saved
// ItemStackTemplate shape), so preserve id/count/components, not just an id.
pub fn stack_tag(stack: &ItemStack) -> Tag {
    let mut record = Compound::new();
    record.insert(JavaString::from("id"), Tag::String(JavaString::from(stack.item.as_ref())));
    if stack.count != 1 { record.insert(JavaString::from("count"), Tag::Int(stack.count as i32)); }
    let Some(data) = stack.data() else { return Tag::Compound(record); };
    let mut components = Compound::new();
    if stack.damage != 0 {
        components.insert(JavaString::from("minecraft:damage"), Tag::Int(stack.damage as i32));
    }
    for key in ["minecraft:custom_name", "minecraft:enchantments", "minecraft:custom_data",
        "theurgy:filter_items", "theurgy:filter_is_deny_list",
        "theurgy:filter_respects_data_components", "theurgy:filter_mode",
        "theurgy:filter_attributes", "codedefinedgui:list_filter_contents",
        "codedefinedgui:list_filter_config", "codedefinedgui:attribute_filter_reference",
        "codedefinedgui:attribute_filter_config"] {
        if let Some(tag) = data.other(key) { components.insert(JavaString::from(key), tag.clone()); }
    }
    if !components.is_empty() { record.insert(JavaString::from("components"), Tag::Compound(components)); }
    Tag::Compound(record)
}
pub fn template_stack(tag: &Tag) -> Option<ItemStack> {
    let Tag::Compound(record) = tag else { return None; };
    let Some(Tag::String(id)) = record.get(&JavaString::from("id")) else { return None; };
    let id = id.to_string_lossy();
    if id.is_empty() { return None; }
    let count = match record.get(&JavaString::from("count")) { Some(Tag::Int(v)) => *v, _ => 1 };
    if count <= 0 { return None; }
    let mut stack = ItemStack::new(&id, 1);
    let Some(Tag::Compound(components)) = record.get(&JavaString::from("components")) else { return Some(stack); };
    for (key, value) in components {
        let _ = gameless_set_component(&mut stack, &key.to_string_lossy(), value.clone());
    }
    Some(stack)
}
fn saved_filter(nbt: &Compound) -> Option<ItemStack> {
    let stack = template_stack(nbt.get(&JavaString::from("filter"))?)?;
    matches!(stack.item.as_ref(), "theurgy:list_filter" | "theurgy:attribute_filter").then_some(stack)
}
// The saved template's components are already valid NBT. Construct only a
// private clone here; the normal native_component path supplies defaults and
// validates registered ids when the player subsequently uses the filter.
fn gameless_set_component(stack: &mut ItemStack, id: &str, value: Tag) -> Result<(), ()> {
    stack.data_mut().set_other(id, Some(value));
    stack.tidy();
    Ok(())
}
fn filter_block(block: &str) -> bool {
    matches!(block, "theurgy:logistics_item_inserter" | "theurgy:logistics_item_extractor"
        | "theurgy:logistics_fluid_inserter" | "theurgy:logistics_fluid_extractor")
}
fn sync_filter_state(game: &mut crate::game::Game<'_>, pos: Pos, present: bool) {
    let state = game.world.block(pos[0], pos[1], pos[2]);
    let Some(mut descriptor) = game.pack.assets.registry.state(state).cloned() else { return; };
    descriptor.properties.insert("has_filter".to_owned(), present.to_string());
    if let Ok(next) = game.pack.assets.registry.resolve(&descriptor) {
        game.native_set_block(game.dim.id(), pos, next, 2);
    }
}
/// The main-hand half of FilterBehaviour.useItemOn, before a barrel menu opens.
pub fn filter_use_on(game: &mut crate::game::Game<'_>, pos: Pos) -> bool {
    if !filter_block(game.native_block(pos).as_str()) { return false; }
    let action = (game.native_acting_id(), pos, game.world.game_time);
    if let Ok(mut last) = LAST_FILTER_ACTION.lock() {
        if last.is_some_and(|(player, position, tick)| player == action.0 && position == action.1 && action.2 - tick <= 1) {
            return true;
        }
        *last = Some(action);
    }
    let slot = game.selected as i32;
    let held = game.native_carried_stack(slot).cloned()
        .filter(|stack| stack.count > 0 && stack.item.as_ref() != "minecraft:air");
    let existing = game.native_with_mod_block_entity(pos, |entity| saved_filter(&entity.nbt)).flatten();
    if let Some(stack) = existing {
        let player = game.native_acting_id();
        if !wire_sneaking(game) || held.is_some() { return false; }
        let player = game.native_acting_id();
        // Atomic give-back: only clear and consume the block state if the whole
        // preserved stack fits. No item duplication or destruction on full bag.
        let returned = game.native_give_stack(player, stack.clone());
        if returned.is_some() {
            // A local single-player inventory may not be wrapped by the LAN
            // helper. Use the same empty-slot transaction instead of losing
            // the filter; never clear the block if the slot is occupied.
            let slot = game.selected as i32;
            if game.inventory.get(slot as usize).and_then(Option::as_ref).is_some_and(|stack| stack.count > 0) {
                return true;
            }
            if let Some(current) = game.inventory.get_mut(slot as usize) {
                *current = Some(stack.clone());
            } else {
                return true;
            }
        }
        game.native_with_mod_block_entity(pos, |entity| { entity.nbt.remove(&JavaString::from("filter")); });
        sync_filter_state(game, pos, false);
        return true;
    }
    let Some(stack) = held else { return false; };
    if !matches!(stack.item.as_ref(), "theurgy:list_filter" | "theurgy:attribute_filter") { return false; }
    let inserted = game.native_with_mod_block_entity(pos, |entity| {
        entity.nbt.insert(JavaString::from("filter"), stack_tag(&stack));
    }).is_some();
    if inserted {
        if let Some(current) = game.native_carried_stack(slot) { current.count = current.count.saturating_sub(1); }
        sync_filter_state(game, pos, true);
    }
    inserted
}

fn mod_tank_capacity(id: &str) -> Option<i32> {
    match id {
        "theurgy:sal_ammoniac_tank" | "theurgy:liquefaction_cauldron" => Some(2_000),
        "theurgy:fermentation_vat" | "theurgy:digestion_vat" | "theurgy:sal_ammoniac_accumulator" => Some(10_000),
        _ => None,
    }
}

/// Fill a real mod tank without Horizon's water/lava-only helper restriction.
/// The complete accepted amount is committed; no source is drained before it.
fn fill_mod_tank(game: &mut crate::game::Game<'_>, pos: Pos, fluid: &str, amount: i32) -> i32 {
    if amount <= 0 {
        return 0;
    }
    game.native_with_mod_block_entity(pos, |entity| {
        let Some(capacity) = mod_tank_capacity(&entity.id) else { return 0; };
        let mut tank = crate::game::theurgy_host::tank(entity, 0, capacity);
        if tank.fluid.as_deref().is_some_and(|old| old != fluid) {
            return 0;
        }
        let accepted = amount.min((capacity - tank.amount).max(0));
        if accepted <= 0 {
            return 0;
        }
        tank.fluid = Some(fluid.to_owned());
        tank.amount += accepted;
        crate::game::theurgy_host::save_tanks(entity, &[tank]);
        accepted
    }).unwrap_or(0)
}

fn drain_mod_tank(game: &mut crate::game::Game<'_>, pos: Pos, fluid: &str, amount: i32) -> i32 {
    if amount <= 0 {
        return 0;
    }
    game.native_with_mod_block_entity(pos, |entity| {
        let Some(capacity) = mod_tank_capacity(&entity.id) else { return 0; };
        let mut tank = crate::game::theurgy_host::tank(entity, 0, capacity);
        if tank.fluid.as_deref() != Some(fluid) {
            return 0;
        }
        let drained = amount.min(tank.amount).max(0);
        if drained <= 0 {
            return 0;
        }
        tank.amount -= drained;
        if tank.amount == 0 {
            tank.fluid = None;
        }
        crate::game::theurgy_host::save_tanks(entity, &[tank]);
        drained
    }).unwrap_or(0)
}

fn insert_energy(entity: &mut ModBlockEntity, amount: i32) -> i32 {
    let mut state = read_state(&entity.nbt);
    let accepted = amount.max(0).min((FLUX_BUFFER_CAPACITY - state.flux_buffer).max(0));
    if accepted <= 0 {
        return 0;
    }
    state.flux_buffer += accepted;
    write_state(&mut entity.nbt, &state);
    accepted
}

/// Capability entry used by catalyst/capacitor push logic for a connector.
pub fn receive_mercury_flux(entity: &mut ModBlockEntity, amount: i32) -> i32 {
    if !read_state(&entity.nbt).enabled {
        return 0;
    }
    insert_energy(entity, amount)
}

fn insert_flux_target(entity: &mut ModBlockEntity, amount: i32, face: u8) -> i32 {
    if amount <= 0 || entity.id == "theurgy:mercury_catalyst" {
        return 0;
    }
    if entity.id == "theurgy:logistics_mercury_flux_connector" {
        return receive_mercury_flux(entity, amount);
    }
    let capacity = match entity.id.as_str() {
        "theurgy:mercury_capacitor" => 500_000,
        "theurgy:mercury_flux_emitter" | "theurgy:caloric_flux_emitter" | "theurgy:sulfuric_flux_emitter" => 1_000,
        _ => return 0,
    };
    if entity.id == "theurgy:mercury_capacitor" {
        if !read_state(&entity.nbt).enabled {
            return 0;
        }
        let direction = match face {
            0 => "DOWN", 1 => "UP", 2 => "NORTH", 3 => "SOUTH", 4 => "WEST", _ => "EAST",
        };
        let default_mode = if matches!(face, 0 | 1) { 2 } else { 1 };
        let mode = match entity.nbt.get(&JavaString::from("sideModes")) {
            Some(Tag::Compound(modes)) => int(modes, direction, default_mode),
            _ => default_mode,
        };
        if mode != 1 && mode != 3 {
            return 0;
        }
    }
    let mut energy = match entity.nbt.get(&JavaString::from("mercuryFluxHandler")) {
        Some(Tag::Compound(child)) => int(child, "energy", 0).clamp(0, capacity),
        _ => 0,
    };
    let accepted = amount.max(0).min((capacity - energy).max(0));
    if accepted <= 0 {
        return 0;
    }
    energy += accepted;
    let mut child = Compound::new();
    set_int(&mut child, "energy", energy);
    entity.nbt.insert(JavaString::from("mercuryFluxHandler"), Tag::Compound(child));
    accepted
}

// ---------------------------------------------------------------------------
// Block entity tick
// ---------------------------------------------------------------------------

fn is_connector(block: &str) -> bool {
    matches!(
        block,
        "theurgy:logistics_item_inserter"
            | "theurgy:logistics_item_extractor"
            | "theurgy:logistics_fluid_inserter"
            | "theurgy:logistics_fluid_extractor"
            | "theurgy:logistics_mercury_flux_connector"
            | "theurgy:logistics_capability_proxy"
    )
}

fn is_nexus(block: &str) -> bool {
    block == "theurgy:logistics_nexus"
}

fn java_nexus_id(nbt: &horizon_world::nbt::Compound) -> Option<String> {
    use horizon_world::nbt::Tag;
    match nbt.get(&horizon_world::nbt::JavaString::from("nexusId")) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    }
}

fn nexus_id_for(entity: &ModBlockEntity) -> Option<String> {
    java_nexus_id(&entity.nbt)
}

fn set_nexus_id(nbt: &mut horizon_world::nbt::Compound, id: &str) {
    use horizon_world::nbt::{JavaString, Tag};
    nbt.insert(
        JavaString::from("nexusId"),
        Tag::String(JavaString::from(id)),
    );
}

fn block_kind(block: &str) -> Option<(LeafNodeMode, CapabilityKind)> {
    match block {
        "theurgy:logistics_item_inserter" => Some((LeafNodeMode::Insert, CapabilityKind::Item)),
        "theurgy:logistics_item_extractor" => Some((LeafNodeMode::Extract, CapabilityKind::Item)),
        "theurgy:logistics_fluid_inserter" => Some((LeafNodeMode::Insert, CapabilityKind::Fluid)),
        "theurgy:logistics_fluid_extractor" => Some((LeafNodeMode::Extract, CapabilityKind::Fluid)),
        "theurgy:logistics_mercury_flux_connector" => {
            Some((LeafNodeMode::Insert, CapabilityKind::MercuryFlux))
        }
        // A capability proxy is not itself an insert target: it forwards the
        // capabilities of connected probes. Its tick is handled separately.
        "theurgy:logistics_capability_proxy" => Some((LeafNodeMode::Insert, CapabilityKind::Item)),
        _ => None,
    }
}

pub fn facing_name_to_dir(value: &str) -> Option<u8> {
    match value {
        "down" => Some(0),
        "up" => Some(1),
        "north" => Some(2),
        "south" => Some(3),
        "west" => Some(4),
        "east" => Some(5),
        _ => None,
    }
}

fn facing_of(game: &crate::game::Game<'_>, pos: Pos) -> u8 {
    let state = game.world.block(pos[0], pos[1], pos[2]);
    game.native_block_state_property(state, "facing")
        .and_then(|value| facing_name_to_dir(&value))
        .unwrap_or(0)
}

/// Direction id -> offset vector (0 = down … 5 = east).
fn dir_offset(dir: u8) -> Pos {
    match dir {
        0 => [0, -1, 0],
        1 => [0, 1, 0],
        2 => [0, 0, -1],
        3 => [0, 0, 1],
        4 => [-1, 0, 0],
        5 => [1, 0, 0],
        _ => [0, -1, 0],
    }
}

fn opposite_dir(dir: u8) -> u8 {
    match dir {
        0 => 1,
        1 => 0,
        2 => 3,
        3 => 2,
        4 => 5,
        _ => 4,
    }
}

fn attached(pos: Pos, facing: u8) -> Pos {
    add(pos, dir_offset(opposite_dir(facing)))
}

fn direction_from_index(value: u8) -> Direction {
    match value {
        0 => Direction::Down,
        1 => Direction::Up,
        2 => Direction::North,
        3 => Direction::South,
        4 => Direction::West,
        _ => Direction::East,
    }
}

fn add(a: Pos, b: Pos) -> Pos {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

// ---------------------------------------------------------------------------
// Copper wires (Java WireItem/Wires/Wire). Horizon has no SavedData hook, so
// each oriented Java wire record is persisted at its `to` block entity. The
// complete graph is reconstructed from the live endpoints and the ticking
// entity's own NBT (which Horizon temporarily removes from the world map).
// ---------------------------------------------------------------------------

const WIRE_ITEM: &str = "theurgy:copper_wire";
const WIRE_END_POINT: &str = "theurgy:wire_end_point";
const WIRE_CONNECTIONS: &str = "wireConnections";
const WIRE_MAX_RANGE: i64 = 32;
const WIRE_ENDPOINTS: [&str; 8] = [
    "theurgy:logistics_connector_node", "theurgy:logistics_item_inserter",
    "theurgy:logistics_item_extractor", "theurgy:logistics_fluid_inserter",
    "theurgy:logistics_fluid_extractor", "theurgy:logistics_capability_probe",
    "theurgy:logistics_capability_proxy", "theurgy:logistics_mercury_flux_connector",
];

fn wire_pos_long(pos: Pos) -> i64 {
    (((pos[0] as i64) & 0x3ff_ffff) << 38)
        | (((pos[2] as i64) & 0x3ff_ffff) << 12)
        | ((pos[1] as i64) & 0xfff)
}
fn wire_pos_from_long(value: i64) -> Pos {
    [((value >> 38) as i32), ((value << 52) >> 52) as i32, ((value << 26) >> 38) as i32]
}
fn wire_manhattan(a: Pos, b: Pos) -> i64 {
    (a[0] as i64 - b[0] as i64).abs()
        + (a[1] as i64 - b[1] as i64).abs()
        + (a[2] as i64 - b[2] as i64).abs()
}
fn wire_endpoint_block(block: &str) -> bool { WIRE_ENDPOINTS.contains(&block) }
fn wire_sneaking(game: &crate::game::Game<'_>) -> bool {
    game.has_shift_down() || game.keys.contains(&winit::keyboard::KeyCode::ShiftLeft)
}
fn load_wire_endpoint(game: &crate::game::Game<'_>, stack: &ItemStack) -> Option<(Pos, String)> {
    let Tag::Compound(entries) = game.native_component(stack, WIRE_END_POINT).ok().flatten()? else { return None; };
    let pos = match entries.get(&JavaString::from("pos")) { Some(Tag::Long(v)) => wire_pos_from_long(*v), _ => return None };
    let level = match entries.get(&JavaString::from("level")) { Some(Tag::String(v)) => v.to_string_lossy(), _ => return None };
    Some((pos, level))
}
fn change_wire_component(game: &mut crate::game::Game<'_>, value: Option<Tag>) {
    let slot = game.selected as i32;
    let Some(mut held) = game.native_carried_stack(slot).cloned() else { return; };
    let changed = match value {
        Some(tag) => game.native_set_component(&mut held, WIRE_END_POINT, tag).is_ok(),
        None => { let _ = game.native_remove_component(&mut held, WIRE_END_POINT); true }
    };
    if changed && let Some(current) = game.native_carried_stack(slot) { *current = held; }
}
fn store_wire_endpoint(game: &mut crate::game::Game<'_>, pos: Pos) {
    let mut point = Compound::new();
    point.insert(JavaString::from("pos"), Tag::Long(wire_pos_long(pos)));
    point.insert(JavaString::from("level"), Tag::String(JavaString::from(game.dim.id())));
    change_wire_component(game, Some(Tag::Compound(point)));
}
fn next_wire_count(count: u32, delta: i32, creative: bool) -> u32 {
    if creative { return count; }
    if delta < 0 { count.saturating_sub(1) } else { count.saturating_add(1).min(64) }
}
fn change_wire_count(game: &mut crate::game::Game<'_>, delta: i32) {
    let creative = game.native_acting_player().is_some_and(|player| player.game_mode == 1);
    let slot = game.selected as i32;
    let Some(mut held) = game.native_carried_stack(slot).cloned() else { return; };
    if held.item.as_ref() == WIRE_ITEM {
        held.count = next_wire_count(held.count, delta, creative);
        if let Some(current) = game.native_carried_stack(slot) { *current = held; }
    }
}
fn wire_records(nbt: &Compound) -> Vec<(Pos, Pos)> {
    let Some(Tag::List(list)) = nbt.get(&JavaString::from(WIRE_CONNECTIONS)) else { return Vec::new(); };
    list.iter().take(256).filter_map(|tag| {
        let Tag::Compound(record) = tag else { return None; };
        let from = match record.get(&JavaString::from("from")) { Some(Tag::Long(v)) => wire_pos_from_long(*v), _ => return None };
        let to = match record.get(&JavaString::from("to")) { Some(Tag::Long(v)) => wire_pos_from_long(*v), _ => return None };
        Some((from, to))
    }).collect()
}
fn same_wire(edge: (Pos, Pos), a: Pos, b: Pos) -> bool {
    (edge.0 == a && edge.1 == b) || (edge.0 == b && edge.1 == a)
}
fn find_wire(game: &crate::game::Game<'_>, a: Pos, b: Pos) -> Option<Pos> {
    for (&(x, y, z), block_entity) in game.world.block_entities.iter() {
        let BlockEntity::Mod(entity) = block_entity else { continue; };
        if wire_records(&entity.nbt).into_iter().any(|edge| same_wire(edge, a, b)) { return Some([x, y, z]); }
    }
    None
}
fn add_wire(game: &mut crate::game::Game<'_>, owner: Pos, from: Pos, to: Pos) -> bool {
    game.native_with_mod_block_entity(owner, |entity| {
        let mut records = match entity.nbt.get(&JavaString::from(WIRE_CONNECTIONS)) {
            Some(Tag::List(list)) => list.clone(), _ => Vec::new(),
        };
        let mut record = Compound::new();
        record.insert(JavaString::from("from"), Tag::Long(wire_pos_long(from)));
        record.insert(JavaString::from("to"), Tag::Long(wire_pos_long(to)));
        records.push(Tag::Compound(record));
        entity.nbt.insert(JavaString::from(WIRE_CONNECTIONS), Tag::List(records));
    }).is_some()
}
fn remove_wire(game: &mut crate::game::Game<'_>, owner: Pos, a: Pos, b: Pos) -> bool {
    game.native_with_mod_block_entity(owner, |entity| {
        let key = JavaString::from(WIRE_CONNECTIONS);
        let mut records = match entity.nbt.get(&key) { Some(Tag::List(list)) => list.clone(), _ => Vec::new() };
        records.retain(|tag| {
            let Tag::Compound(record) = tag else { return true; };
            let from = match record.get(&JavaString::from("from")) { Some(Tag::Long(v)) => wire_pos_from_long(*v), _ => return true };
            let to = match record.get(&JavaString::from("to")) { Some(Tag::Long(v)) => wire_pos_from_long(*v), _ => return true };
            !same_wire((from, to), a, b)
        });
        if records.is_empty() { entity.nbt.remove(&key); } else { entity.nbt.insert(key, Tag::List(records)); }
    }).is_some()
}

fn repeated_wire_action(last: Option<(u128, Pos, i64)>, action: (u128, Pos, i64)) -> bool {
    last.is_some_and(|(player, pos, tick)| player == action.0 && pos == action.1
        && matches!(action.2.checked_sub(tick), Some(0..=4)))
}

/// WireItem.useOn: select, connect, or disconnect an existing unordered edge.
pub fn wire_use_on(game: &mut crate::game::Game<'_>, stack: &ItemStack, pos: Pos, claimed: &mut bool) -> bool {
    *claimed = true;
    let action = (game.native_acting_id(), pos, game.world.game_time);
    if let Ok(mut last) = LAST_WIRE_ACTION.lock() {
        if repeated_wire_action(*last, action) { return true; }
        *last = Some(action);
    }
    if !wire_endpoint_block(game.native_block(pos).as_str()) {
        if wire_sneaking(game) { change_wire_component(game, None); }
        return true;
    }
    if wire_sneaking(game) { change_wire_component(game, None); return true; }
    let Some((stored, level)) = load_wire_endpoint(game, stack) else {
        store_wire_endpoint(game, pos);
        return true;
    };
    if level != game.dim.id() || stored == pos || wire_manhattan(stored, pos) > WIRE_MAX_RANGE { return true; }
    if let Some(owner) = find_wire(game, stored, pos) {
        if remove_wire(game, owner, stored, pos) {
            change_wire_component(game, None);
            change_wire_count(game, 1);
        }
    } else if add_wire(game, pos, stored, pos) {
        change_wire_component(game, None);
        change_wire_count(game, -1);
    }
    true
}

fn reachable_nodes(start: Pos, edges: &[(Pos, Pos)]) -> Vec<Pos> {
    let mut reached = BTreeSet::from([start]);
    // Every relaxation adds at least one node; the explicit bound also makes
    // malformed cyclic NBT harmless.
    for _ in 0..edges.len() {
        let before = reached.len();
        for (a, b) in edges {
            if reached.contains(a) { reached.insert(*b); }
            if reached.contains(b) { reached.insert(*a); }
        }
        if reached.len() == before { break; }
    }
    reached.into_iter().collect()
}
fn wire_network(game: &crate::game::Game<'_>, start: Pos, own_nbt: &Compound) -> Vec<Pos> {
    let mut edges = wire_records(own_nbt);
    for (&(x, y, z), block_entity) in game.world.block_entities.iter() {
        let BlockEntity::Mod(entity) = block_entity else { continue; };
        edges.extend(wire_records(&entity.nbt));
    }
    edges.retain(|(a, b)| wire_endpoint_block(game.native_block(*a).as_str()) && wire_endpoint_block(game.native_block(*b).as_str()));
    reachable_nodes(start, &edges)
}

pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let id = entity.id.clone();
    if is_nexus(&id) {
        sync_nexus_connections(game, pos);
        tick_nexus(pos, entity);
    } else if is_connector(&id) {
        tick_connector(game, pos, entity, &id);
    }
}

fn sync_nexus_connections(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    for (property, offset) in [
        ("up", [0, 1, 0]), ("down", [0, -1, 0]), ("north", [0, 0, -1]),
        ("south", [0, 0, 1]), ("east", [1, 0, 0]), ("west", [-1, 0, 0]),
    ] {
        let neighbor = [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]];
        let block = game.native_block(neighbor);
        let connected = !block.is_empty() && block != "minecraft:air" && block != "minecraft:cave_air" && block != "minecraft:void_air";
        game.native_set_block_property_with(pos, property, if connected { "true" } else { "false" }, 3);
    }
}

fn tick_nexus(pos: [i32; 3], entity: &mut ModBlockEntity) {
    if nexus_id_for(entity).is_none() {
        let id = fresh_nexus_id(pos, entity);
        set_nexus_id(&mut entity.nbt, &id);
    }
}

fn fresh_nexus_id(pos: Pos, entity: &ModBlockEntity) -> String {
    let mut hasher = 0u64;
    for b in entity.id.bytes() {
        hasher = hasher.wrapping_mul(1_000_003).wrapping_add(u64::from(b));
    }
    hasher ^= (pos[0] as i64 as u64).rotate_left(11);
    hasher ^= (pos[1] as i64 as u64).rotate_left(27);
    hasher ^= (pos[2] as i64 as u64).rotate_left(43);
    format!("{hasher:016x}")
}

/// Scans loaded block entities for connectors sharing `nexus_id`.
fn collect_network(
    game: &crate::game::Game<'_>,
    nexus_id: Option<&str>,
    wired: &[Pos],
    frequency: i32,
    self_pos: Pos,
) -> Vec<(Pos, LeafNodeMode, CapabilityKind, u8, Option<ItemStack>)> {
    let mut out = Vec::new();
    for (&(x, y, z), block_entity) in game.world.block_entities.iter() {
        let p = [x, y, z];
        let BlockEntity::Mod(ent) = block_entity else {
            continue;
        };
        if p == self_pos || ent.id == "theurgy:logistics_capability_proxy" {
            continue;
        }
        let Some((mode, kind)) = block_kind(&ent.id) else {
            continue;
        };
        let connected = wired.contains(&p)
            || (nexus_id.is_some() && java_nexus_id(&ent.nbt).as_deref() == nexus_id.as_deref());
        if !connected || int(&ent.nbt, "frequency", 0) != frequency { continue; }
        let state = read_state(&ent.nbt);
        if !state.enabled {
            continue;
        }
        let facing = facing_of(game, p);
        let dir = connector_direction(state.direction_override, facing);
        out.push((p, mode, kind, dir, saved_filter(&ent.nbt)));
    }
    out
}

/// Read-only network facts for the operator differential probe.
pub fn probe_insert_targets(
    game: &crate::game::Game<'_>,
    pos: Pos,
) -> Vec<Pos> {
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get(&(pos[0], pos[1], pos[2])) else {
        return Vec::new();
    };
    let Some((_, kind)) = block_kind(&entity.id) else { return Vec::new(); };
    let nexus_id = java_nexus_id(&entity.nbt);
    let wired = wire_network(game, pos, &entity.nbt);
    collect_network(game, nexus_id.as_deref(), &wired, int(&entity.nbt, "frequency", 0), pos)
        .into_iter()
        .filter(|(_, mode, candidate, _, _)| *mode == LeafNodeMode::Insert && *candidate == kind)
        .map(|(target, _, _, _, _)| target)
        .collect()
}

pub fn probe_nexus_partners(game: &crate::game::Game<'_>, pos: Pos) -> usize {
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get(&(pos[0], pos[1], pos[2])) else {
        return 0;
    };
    let Some(id) = java_nexus_id(&entity.nbt) else { return 0; };
    game.world.block_entities.iter().filter(|(other, block_entity)| {
        let other = [other.0, other.1, other.2];
        other != pos
            && matches!(block_entity, BlockEntity::Mod(candidate)
                if is_nexus(&candidate.id) && java_nexus_id(&candidate.nbt).as_deref() == Some(id.as_str()))
    }).count()
}

fn proxy_fluid_target(
    game: &mut crate::game::Game<'_>,
    pos: Pos,
) -> Option<(Pos, String, i32)> {
    let own_nbt = game.native_with_mod_block_entity(pos, |entity| entity.nbt.clone())?;
    let nexus_id = java_nexus_id(&own_nbt);
    let wired = wire_network(game, pos, &own_nbt);
    let mut probes = Vec::new();
    for (&(x, y, z), block_entity) in game.world.block_entities.iter() {
        let p = [x, y, z];
        let BlockEntity::Mod(entity) = block_entity else { continue; };
        if entity.id != "theurgy:logistics_capability_probe" { continue; }
        let connected = wired.contains(&p)
            || (nexus_id.is_some() && java_nexus_id(&entity.nbt).as_deref() == nexus_id.as_deref());
        if connected { probes.push(p); }
    }
    probes.sort_by_key(|p| wire_pos_long(*p));
    if probes.is_empty() { return None; }
    let index = (int(&own_nbt, "roundRobinIndex", 0) + 1).rem_euclid(probes.len() as i32) as usize;
    game.native_with_mod_block_entity(pos, |entity| {
        set_int(&mut entity.nbt, "roundRobinIndex", index as i32);
    });
    let probe = probes[index];
    let facing = facing_of(game, probe);
    let target = attached(probe, facing);
    game.native_fluid_tanks(game.dim.id(), target, direction_from_index(facing))
        .into_iter().flatten()
        .find(|tank| tank.amount > 0)
        .and_then(|tank| tank.fluid.map(|fluid| (target, fluid, tank.amount)))
}

/// Successive capability reads advance the same round-robin distributor as
/// Java's LogisticsCapabilityProxyBlockEntity.
pub fn proxy_read_fluid(game: &mut crate::game::Game<'_>, pos: Pos) -> Option<(String, i32)> {
    proxy_fluid_target(game, pos).map(|(_, fluid, amount)| (fluid, amount))
}

pub fn proxy_drain_fluid(game: &mut crate::game::Game<'_>, pos: Pos, amount: i32) -> i32 {
    let Some((target, fluid, _)) = proxy_fluid_target(game, pos) else { return 0; };
    drain_mod_tank(game, target, &fluid, amount)
}

fn restore_attached_target_if_empty(game: &crate::game::Game<'_>, pos: Pos, entity: &mut ModBlockEntity) {
    if entity.id == "theurgy:logistics_capability_proxy" { return; }
    let phase = entity.data.get(8).copied().unwrap_or(0);
    let target = attached(pos, facing_of(game, pos));
    let populated = matches!(entity.nbt.get(&JavaString::from("targets")), Some(Tag::List(targets)) if !targets.is_empty());
    if populated {
        // A block-entity tick can precede Horizon's final facing-state write.
        // Only our automatically restored adjacent target may follow that
        // facing correction; explicitly selected remote targets are preserved.
        if phase >= 2
            && let Some(Tag::List(targets)) = entity.nbt.get_mut(&JavaString::from("targets"))
            && targets.len() == 1
            && matches!(targets.first(), Some(Tag::Long(value)) if wire_manhattan(wire_pos_from_long(*value), pos) == 1)
        {
            targets[0] = Tag::Long(wire_pos_long(target));
        }
        return;
    }
    // Defer onLoad-style restoration until the block state has settled. This
    // also preserves Java's empty tick-0 placement snapshot.
    if phase == 0 {
        if entity.data.len() <= 8 { entity.data.resize(9, 0); }
        entity.data[8] = 1;
        return;
    }
    entity.nbt.insert(JavaString::from("targets"), Tag::List(vec![Tag::Long(wire_pos_long(target))]));
    if entity.data.len() <= 8 { entity.data.resize(9, 0); }
    entity.data[8] = 2;
}

fn tick_connector(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
    block: &str,
) {
    let is_proxy = block == "theurgy:logistics_capability_proxy";
    let Some((mode, kind)) = block_kind(block) else { return; };
    restore_attached_target_if_empty(game, pos, entity);
    let mut state = read_state(&entity.nbt);
    let filter = saved_filter(&entity.nbt);
    if state.extraction_amount == 0 {
        state.extraction_amount = match kind {
            CapabilityKind::Item => MAX_ITEM_EXTRACTION_AMOUNT,
            CapabilityKind::Fluid => MAX_FLUID_EXTRACTION_AMOUNT,
            CapabilityKind::MercuryFlux => FLUX_DEFAULT_TRANSFER_RATE,
        };
    }
    state.flux_buffer = state.flux_buffer.clamp(0, FLUX_BUFFER_CAPACITY);
    state.extraction_amount = state.extraction_amount.max(1);

    let facing = facing_of(game, pos);
    let target_dir = connector_direction(state.direction_override, facing);
    if !state.enabled {
        write_state(&mut entity.nbt, &state);
        return;
    }

    let period = if is_proxy { 20 } else { match kind {
        CapabilityKind::Item => ITEM_EXTRACTION_EVERY_N_TICKS,
        CapabilityKind::Fluid => FLUID_EXTRACTION_EVERY_N_TICKS,
        CapabilityKind::MercuryFlux => FLUX_TRANSFER_EVERY_N_TICKS,
    }};
    if !on_slow_tick(slow_tick_offset(pos, period), game.world.game_time, period) {
        write_state(&mut entity.nbt, &state);
        return;
    }

    let nexus_id = java_nexus_id(&entity.nbt);
    let wired = wire_network(game, pos, &entity.nbt);
    if nexus_id.is_none() && wired.len() <= 1 {
        write_state(&mut entity.nbt, &state);
        return;
    }
    let frequency = int(&entity.nbt, "frequency", 0);
    let network = collect_network(game, nexus_id.as_deref(), &wired, frequency, pos);

    if is_proxy {
        let mut probes = Vec::new();
        for (&(x, y, z), block_entity) in game.world.block_entities.iter() {
            let p = [x, y, z];
            let BlockEntity::Mod(ent) = block_entity else { continue; };
            if ent.id != "theurgy:logistics_capability_probe" { continue; }
            let connected = wired.contains(&p)
                || (nexus_id.is_some() && java_nexus_id(&ent.nbt).as_deref() == nexus_id.as_deref());
            if connected { probes.push(p); }
        }
        probes.sort_by_key(|p| wire_pos_long(*p));
        if probes.is_empty() {
            write_state(&mut entity.nbt, &state);
            return;
        }
        let index = (int(&entity.nbt, "roundRobinIndex", 0) + 1).rem_euclid(probes.len() as i32) as usize;
        let probe = probes[index];
        set_int(&mut entity.nbt, "roundRobinIndex", index as i32);
        let probe_facing = facing_of(game, probe);
        let source = attached(probe, probe_facing);
        let from_face = direction_from_index(probe_facing);
        let source_is_fluid = game.native_fluid_tanks(game.dim.id(), source, from_face)
            .is_some_and(|tanks| !tanks.is_empty());
        if source_is_fluid {
            let sinks: Vec<_> = network.iter().filter(|(_, m, k, _, _)| {
                *m == LeafNodeMode::Insert && *k == CapabilityKind::Fluid
            }).collect();
            if let Some((_, _, _, sink_dir, sink_filter)) = sinks.first() {
                let sink = attached(sinks[0].0, *sink_dir);
                let to_face = direction_from_index(*sink_dir);
                let source_fluid = game.native_fluid_tanks(game.dim.id(), source, from_face)
                    .into_iter().flatten().find(|tank| tank.amount > 0)
                    .and_then(|tank| tank.fluid.map(|fluid| (fluid, tank.amount)));
                if let Some((fluid, available)) = source_fluid {
                    let sink_rule = game.native_list_filter(sink_filter.as_ref());
                    if sink_rule.allows_fluid(game, &fluid) {
                        let accepted = fill_mod_tank(game, sink, &fluid, MAX_FLUID_EXTRACTION_AMOUNT.min(available));
                        if accepted > 0 {
                            let _ = drain_mod_tank(game, source, &fluid, accepted);
                        }
                    }
                }
                let _ = to_face;
            }
        } else if let Some((_, _, _, sink_dir, sink_filter)) = network.iter().find(|(_, m, k, _, _)| {
            *m == LeafNodeMode::Insert && *k == CapabilityKind::Item
        }) {
            let sink = attached(network.iter().find(|(_, m, k, _, _)| {
                *m == LeafNodeMode::Insert && *k == CapabilityKind::Item
            }).unwrap().0, *sink_dir);
            let sink_rule = game.native_list_filter(sink_filter.as_ref());
            let accept = |stack: &ItemStack| sink_rule.allows_item(stack);
            let _ = game.native_move_items_filtered(
                game.dim.id(), source, from_face, sink, direction_from_index(*sink_dir),
                &accept, state.extraction_amount as u32,
            );
        }
        write_state(&mut entity.nbt, &state);
        return;
    }

    match (mode, kind) {
        (LeafNodeMode::Extract, CapabilityKind::Item) => {
            let sinks: Vec<_> = network.iter().filter(|(_, m, k, _, _)| {
                *m == LeafNodeMode::Insert && *k == CapabilityKind::Item
            }).collect();
            if sinks.is_empty() {
                write_state(&mut entity.nbt, &state);
                return;
            }
            let index = (int(&entity.nbt, "distributor", 0) + 1).rem_euclid(sinks.len() as i32) as usize;
            set_int(&mut entity.nbt, "distributor", index as i32);
            let (_, _, _, sink_dir, sink_filter) = sinks[index];
            let source = attached(pos, target_dir);
            let sink = attached(sinks[index].0, *sink_dir);
            let source_rule = game.native_list_filter(filter.as_ref());
            let sink_rule = game.native_list_filter(sink_filter.as_ref());
            let source_attribute = game.native_attribute_filter(filter.as_ref());
            let sink_attribute = game.native_attribute_filter(sink_filter.as_ref());
            let accept = |stack: &ItemStack| source_rule.allows_item(stack)
                && sink_rule.allows_item(stack)
                && source_attribute.allows_item(stack)
                && sink_attribute.allows_item(stack);
            let moved = game.native_move_items_filtered(
                game.dim.id(), source, direction_from_index(target_dir), sink,
                direction_from_index(*sink_dir), &accept,
                state.extraction_amount.clamp(1, MAX_ITEM_EXTRACTION_AMOUNT) as u32,
            );
            if moved > 0 {
                set_int(&mut entity.nbt, "lastTransfer", moved as i32);
            }
        }
        (LeafNodeMode::Extract, CapabilityKind::Fluid) => {
            let sinks: Vec<_> = network.iter().filter(|(_, m, k, _, _)| {
                *m == LeafNodeMode::Insert && *k == CapabilityKind::Fluid
            }).collect();
            if sinks.is_empty() {
                write_state(&mut entity.nbt, &state);
                return;
            }
            let index = (int(&entity.nbt, "distributor", 0) + 1).rem_euclid(sinks.len() as i32) as usize;
            set_int(&mut entity.nbt, "distributor", index as i32);
            let (_, _, _, sink_dir, sink_filter) = sinks[index];
            let source = attached(pos, target_dir);
            let sink = attached(sinks[index].0, *sink_dir);
            let source_rule = game.native_list_filter(filter.as_ref());
            let sink_rule = game.native_list_filter(sink_filter.as_ref());
            let world_fluid = match game.native_block(source).as_str() {
                "minecraft:water" => Some("minecraft:water".to_owned()),
                "minecraft:lava" => Some("minecraft:lava".to_owned()),
                "theurgy:sal_ammoniac" => Some("theurgy:sal_ammoniac".to_owned()),
                _ => None,
            };
            let source_fluid = world_fluid.map(|fluid| (fluid, MAX_FLUID_EXTRACTION_AMOUNT)).or_else(|| {
                game.native_fluid_tanks(game.dim.id(), source, direction_from_index(target_dir))
                    .into_iter().flatten().find(|tank| tank.amount > 0)
                    .and_then(|tank| tank.fluid.map(|fluid| (fluid, tank.amount)))
            });
            if let Some((fluid, available)) = source_fluid {
                if source_rule.allows_fluid(game, &fluid) && sink_rule.allows_fluid(game, &fluid) {
                    let accepted = fill_mod_tank(game, sink, &fluid, state.extraction_amount.min(available));
                    if accepted > 0 {
                        // A world source block is infinite in Java and remains
                        // untouched. Only a real tank is drained, by accepted.
                        let _ = drain_mod_tank(game, source, &fluid, accepted);
                        set_int(&mut entity.nbt, "lastTransfer", accepted);
                    }
                }
            }
        }
        (LeafNodeMode::Insert, CapabilityKind::MercuryFlux) => {
            let sinks: Vec<_> = network.iter().filter(|(p, m, k, _, _)| {
                *p != pos && *m == LeafNodeMode::Insert && *k == CapabilityKind::MercuryFlux
            }).map(|(p, _, _, d, _)| (attached(*p, *d), *d)).collect();
            if !sinks.is_empty() && state.flux_buffer > 0 {
                let total = flux_pushes_for(state.flux_buffer, sinks.len());
                let shares = distribute_evenly(total, sinks.len());
                let mut moved_total = 0;
                for ((sink, face), amount) in sinks.iter().zip(shares) {
                    if game.native_block_property(*sink, "enabled").as_deref() == Some("false") {
                        continue;
                    }
                    let moved = game.native_with_mod_block_entity(*sink, |target| {
                        insert_flux_target(target, amount, *face)
                    }).unwrap_or(0);
                    if moved > 0 {
                        state.flux_buffer -= moved;
                        moved_total += moved;
                    }
                }
                if moved_total > 0 {
                    set_int(&mut entity.nbt, "lastTransfer", moved_total);
                }
            }
        }
        _ => {}
    }
    write_state(&mut entity.nbt, &state);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_java_pos_roundtrip_and_negative_coordinates() {
        for p in [[30,80,28],[-1,-100,-1],[-1000,320,2000],[-33554432,-2048,-33554432]] {
            assert_eq!(wire_pos_from_long(wire_pos_long(p)), p);
        }
        assert_eq!(wire_pos_long([30,80,28]), 8_246_337_323_088);
    }
    #[test]
    fn wire_graph_reaches_cycles_and_not_disconnected_components() {
        let edges=[([0,0,0],[1,0,0]),([2,0,0],[0,0,0]),([2,0,0],[1,0,0]),([9,0,0],[10,0,0])];
        assert_eq!(reachable_nodes([0,0,0],&edges),vec![[0,0,0],[1,0,0],[2,0,0]]);
        assert!(same_wire(edges[0],[1,0,0],[0,0,0]));
    }
    #[test]
    fn wire_records_reject_malformed_nbt() {
        let mut record=Compound::new();
        record.insert(JavaString::from("from"),Tag::String(JavaString::from("bad")));
        let mut nbt=Compound::new();
        nbt.insert(JavaString::from("wireConnections"),Tag::List(vec![Tag::Compound(record)]));
        assert!(wire_records(&nbt).is_empty());
    }
    #[test]
    fn wire_records_persist_java_longs() {
        let mut nbt=Compound::new();
        let mut wire=Compound::new();
        wire.insert(JavaString::from("from"),Tag::Long(wire_pos_long([40,80,40])));
        wire.insert(JavaString::from("to"),Tag::Long(wire_pos_long([47,80,40])));
        nbt.insert(JavaString::from("wireConnections"),Tag::List(vec![Tag::Compound(wire)]));
        assert_eq!(wire_records(&nbt),vec![([40,80,40],[47,80,40])]);
    }
    #[test]
    fn filter_template_preserves_registered_component() {
        let mut original=ItemStack::new("theurgy:list_filter",1);
        original.data_mut().set_other("theurgy:filter_mode",Some(Tag::String(JavaString::from("match_all_any"))));
        let saved=stack_tag(&original);
        let mut nbt=Compound::new();nbt.insert(JavaString::from("filter"),saved);
        assert_eq!(saved_filter(&nbt).as_ref(),Some(&original));
    }

    #[test]
    fn slow_tick_respects_offset() {
        assert!(on_slow_tick(0, 0, 20));
        assert!(on_slow_tick(0, 20, 20));
        assert!(!on_slow_tick(0, 1, 20));
        assert!(on_slow_tick(5, 15, 20));
        assert!(!on_slow_tick(5, 16, 20));
    }

    #[test]
    fn slow_tick_handles_negative_time() {
        assert!(on_slow_tick(0, -20, 20));
        assert!(!on_slow_tick(0, -19, 20));
    }

    #[test]
    fn slow_tick_zero_period_never_fires() {
        assert!(!on_slow_tick(0, 0, 0));
        assert!(!on_slow_tick(0, 0, -1));
    }

    #[test]
    fn distribute_evenly_splits_remainder() {
        assert_eq!(distribute_evenly(10, 3), vec![4, 3, 3]);
        assert_eq!(distribute_evenly(5, 5), vec![1, 1, 1, 1, 1]);
        assert!(distribute_evenly(0, 3).is_empty());
        assert!(distribute_evenly(10, 0).is_empty());
    }

    #[test]
    fn flux_pushes_bounded_by_rate_and_buffer() {
        assert_eq!(flux_pushes_for(1000, 3), 300);
        assert_eq!(flux_pushes_for(50, 3), 50);
        assert_eq!(flux_pushes_for(0, 3), 0);
        assert_eq!(flux_pushes_for(1000, 0), 0);
    }

    #[test]
    fn insert_targets_excludes_extractor_and_other_kinds() {
        let nodes = vec![
            LeafNode { pos: [0, 0, 0], mode: LeafNodeMode::Extract, kind: CapabilityKind::Item },
            LeafNode { pos: [1, 0, 0], mode: LeafNodeMode::Insert, kind: CapabilityKind::Item },
            LeafNode { pos: [2, 0, 0], mode: LeafNodeMode::Insert, kind: CapabilityKind::Fluid },
        ];
        let t = insert_targets(&nodes, [0, 0, 0], CapabilityKind::Item);
        assert_eq!(t, vec![&nodes[1]]);
    }

    #[test]
    fn leaf_nodes_filter_by_nexus() {
        let cands = [
            ([0, 0, 0], "a", LeafNodeMode::Insert, CapabilityKind::Item),
            ([1, 0, 0], "b", LeafNodeMode::Insert, CapabilityKind::Item),
            ([2, 0, 0], "a", LeafNodeMode::Extract, CapabilityKind::Fluid),
        ];
        let nodes = leaf_nodes_of("a", &cands);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[1].mode, LeafNodeMode::Extract);
    }

    #[test]
    fn state_roundtrips_through_nbt() {
        let mut nbt = horizon_world::nbt::Compound::new();
        let state = LogisticsState {
            enabled: false,
            direction_override: Some(3),
            extraction_amount: 42,
            flux_buffer: 7,
        };
        write_state(&mut nbt, &state);
        let read = read_state(&nbt);
        assert_eq!(read, state);
    }

    #[test]
    fn state_defaults_enabled_when_absent() {
        let nbt = horizon_world::nbt::Compound::new();
        let state = read_state(&nbt);
        assert!(state.enabled);
        assert_eq!(state.direction_override, None);
    }

    #[test]
    fn direction_override_out_of_range_ignored() {
        use horizon_world::nbt::{Compound, JavaString, Tag};
        let mut nbt = Compound::new();
        nbt.insert(JavaString::from("directionOverride"), Tag::Int(9));
        assert_eq!(read_state(&nbt).direction_override, None);
    }

    #[test]
    fn direction_override_none_removes_key() {
        let mut nbt = horizon_world::nbt::Compound::new();
        write_state(&mut nbt, &LogisticsState { direction_override: Some(2), ..Default::default() });
        assert!(nbt.contains_key(&horizon_world::nbt::JavaString::from("directionOverride")));
        write_state(&mut nbt, &LogisticsState { direction_override: None, ..Default::default() });
        assert!(!nbt.contains_key(&horizon_world::nbt::JavaString::from("directionOverride")));
    }

    #[test]
    fn connector_direction_prefers_override() {
        assert_eq!(connector_direction(Some(2), 4), 2);
        assert_eq!(connector_direction(None, 4), 4);
    }

    #[test]
    fn facing_names_map_to_direction_ids() {
        assert_eq!(facing_name_to_dir("down"), Some(0));
        assert_eq!(facing_name_to_dir("up"), Some(1));
        assert_eq!(facing_name_to_dir("north"), Some(2));
        assert_eq!(facing_name_to_dir("south"), Some(3));
        assert_eq!(facing_name_to_dir("west"), Some(4));
        assert_eq!(facing_name_to_dir("east"), Some(5));
        assert_eq!(facing_name_to_dir("nonsense"), None);
    }

    #[test]
    fn slow_tick_offset_is_stable() {
        assert_eq!(slow_tick_offset([1, 2, 3], 20), slow_tick_offset([1, 2, 3], 20));
        assert!(slow_tick_offset([0, 0, 0], 20) < 20);
    }

    #[test]
    fn block_kinds_match_registry() {
        assert_eq!(
            block_kind("theurgy:logistics_item_inserter"),
            Some((LeafNodeMode::Insert, CapabilityKind::Item))
        );
        assert_eq!(
            block_kind("theurgy:logistics_fluid_extractor"),
            Some((LeafNodeMode::Extract, CapabilityKind::Fluid))
        );
        assert_eq!(
            block_kind("theurgy:logistics_mercury_flux_connector"),
            Some((LeafNodeMode::Insert, CapabilityKind::MercuryFlux))
        );
        assert_eq!(block_kind("theurgy:logistics_nexus"), None);
    }

    #[test]
    fn wire_count_respects_java_creative_and_survival_rules() {
        assert_eq!(next_wire_count(2, -1, false), 1);
        assert_eq!(next_wire_count(1, 1, false), 2);
        assert_eq!(next_wire_count(2, -1, true), 2);
        assert_eq!(next_wire_count(2, 1, true), 2);
        assert_eq!(next_wire_count(0, -1, false), 0);
    }

    #[test]
    fn wire_callback_deduplication_is_scoped_to_one_action() {
        let action = (7, [1, 80, 2], 100);
        assert!(!repeated_wire_action(None, action));
        assert!(repeated_wire_action(Some(action), action));
        assert!(repeated_wire_action(Some(action), (7, [1, 80, 2], 104)));
        assert!(!repeated_wire_action(Some(action), (8, [1, 80, 2], 100)));
        assert!(!repeated_wire_action(Some(action), (7, [2, 80, 2], 100)));
        assert!(!repeated_wire_action(Some(action), (7, [1, 80, 2], 99)));
        assert!(!repeated_wire_action(Some(action), (7, [1, 80, 2], 105)));
    }

    #[test]
    fn fresh_nexus_id_is_stable_and_position_sensitive() {
        let nbt = horizon_world::nbt::Compound::new();
        let entity = ModBlockEntity::new(horizon_client::containers::ModBlockEntityFields {
            id: "theurgy:logistics_nexus".to_owned(), nbt, ..Default::default()
        });
        let a = fresh_nexus_id([0, 0, 0], &entity);
        let b = fresh_nexus_id([0, 0, 0], &entity);
        let c = fresh_nexus_id([1, 0, 0], &entity);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
