// SPDX-License-Identifier: MIT
use crate::game::theurgy_host::{bool, int, set_int};
use horizon_client::containers::ModBlockEntity;

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
///
/// Java's `Logistics` singleton is a global registry keyed by `GlobalPos`;
/// here the state is persisted in each block entity's NBT under
/// `LogisticsState`, and the network is resolved per-tick by scanning the
/// loaded block entities that share a nexus id. This keeps state close to the
/// block entity that owns it (matching what `saveAdditional` writes) while
/// preserving the observable connector behaviour.
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
    LogisticsState {
        enabled,
        direction_override,
        extraction_amount: int(nbt, "extractionAmount", 0),
        flux_buffer: int(nbt, "fluxBuffer", 0),
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
    set_int(nbt, "fluxBuffer", state.flux_buffer);
}

// ---------------------------------------------------------------------------
// Network assembly (pure)
// ---------------------------------------------------------------------------

/// Collects the leaf nodes reachable from a nexus given the positions of
/// connector block entities that are adjacent to the nexus and registered on
/// it. Kept as a small pure function so the network composition can be tested
/// without a world.
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

/// The leaf nodes an extractor may deliver to: insert-mode nodes of the same
/// capability kind, excluding the extractor itself.
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

/// Java's `(slowTickRandomOffset + gameTime) % period == 0` gating.
pub fn on_slow_tick(offset: i32, game_time: i64, period: i32) -> bool {
    if period <= 0 {
        return false;
    }
    let shifted = (game_time.rem_euclid(i64::from(period))) as i32;
    (offset + shifted).rem_euclid(period) == 0
}

/// Java's stable per-position slow-tick offset. Java uses `Math.random()` in
/// the constructor (unseeded), but a deterministic offset from position keeps
/// save/reloads stable and satisfies "one extraction per second per connector".
pub fn slow_tick_offset(pos: Pos, period: i32) -> i32 {
    if period <= 0 {
        return 0;
    }
    let h = (pos[0] as i64).wrapping_mul(73_856_093)
        ^ (pos[1] as i64).wrapping_mul(19_349_663)
        ^ (pos[2] as i64).wrapping_mul(83_492_791);
    (h.rem_euclid(i64::from(period))) as i32
}

/// Splits `total` evenly over `sink_count` sinks, giving the remainder to the
/// first sinks (Java's `total / N` + `total % N` distribution).
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

/// How much of the buffer may be pushed this transfer tick across all sinks.
pub fn flux_pushes_for(buffer: i32, sink_count: usize) -> i32 {
    if buffer <= 0 || sink_count == 0 {
        return 0;
    }
    let budget = FLUX_TRANSFER_RATE.saturating_mul(sink_count as i32);
    buffer.min(budget)
}

/// The direction a connector targets: override else the block's FACING.
pub fn connector_direction(override_dir: Option<u8>, facing: u8) -> u8 {
    override_dir.unwrap_or(facing)
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
    // The nexus stores its own uuid; connectors reference the nexus by uuid
    // under `nexusId` (written by the connect tool / capability proxy).
    java_nexus_id(&entity.nbt)
}

fn set_nexus_id(nbt: &mut horizon_world::nbt::Compound, id: &str) {
    use horizon_world::nbt::{JavaString, Tag};
    nbt.insert(
        JavaString::from("nexusId"),
        Tag::String(JavaString::from(id)),
    );
}

/// A connector's block identity (`block:id`) decoded from `entity.id`.
fn block_kind(block: &str) -> Option<(LeafNodeMode, CapabilityKind)> {
    match block {
        "theurgy:logistics_item_inserter" => Some((LeafNodeMode::Insert, CapabilityKind::Item)),
        "theurgy:logistics_item_extractor" => Some((LeafNodeMode::Extract, CapabilityKind::Item)),
        "theurgy:logistics_fluid_inserter" => Some((LeafNodeMode::Insert, CapabilityKind::Fluid)),
        "theurgy:logistics_fluid_extractor" => Some((LeafNodeMode::Extract, CapabilityKind::Fluid)),
        "theurgy:logistics_mercury_flux_connector" | "theurgy:logistics_capability_proxy" => {
            Some((LeafNodeMode::Insert, CapabilityKind::MercuryFlux))
        }
        _ => None,
    }
}

/// Maps a `facing` block-state direction name to the `Direction` id used by
/// the NBT `directionOverride` field (0 = down … 5 = east).
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

/// Reads the block's current `facing` direction at `pos` via the host API.
/// Falls back to `0` (down) when the block has no such property.
fn facing_of(game: &crate::game::Game<'_>, pos: Pos) -> u8 {
    let state = game.world.block(pos[0], pos[1], pos[2]);
    game.native_block_state_property(state, "facing")
        .and_then(|value| facing_name_to_dir(&value))
        .unwrap_or(0)
}

pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let id = entity.id.clone();
    if is_nexus(&id) {
        tick_nexus(pos, entity);
    } else if is_connector(&id) {
        tick_connector(game, pos, entity, &id);
    }
}

fn tick_nexus(pos: [i32; 3], entity: &mut ModBlockEntity) {
    // Ensure a nexus id exists on the server; new connectors pair by uuid.
    if nexus_id_for(entity).is_none() {
        let id = fresh_nexus_id(pos, entity);
        set_nexus_id(&mut entity.nbt, &id);
    }
}

/// Generates a deterministic uuid for a newly placed nexus. Java's
/// `UUID.randomUUID()` is fine but we avoid non-determinism here: a uuid is
/// derived from the block entity's position (persisted on first tick).
fn fresh_nexus_id(pos: Pos, entity: &ModBlockEntity) -> String {
    let mut hasher = 0u64;
    for b in entity.id.bytes() {
        hasher = hasher.wrapping_mul(1_000_003).wrapping_add(u64::from(b));
    }
    // Fold in the position so two newly placed nexuses differ.
    hasher ^= (pos[0] as i64 as u64).rotate_left(11);
    hasher ^= (pos[1] as i64 as u64).rotate_left(27);
    hasher ^= (pos[2] as i64 as u64).rotate_left(43);
    format!("{hasher:016x}")
}

fn tick_connector(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
    block: &str,
) {
    let Some((_mode, kind)) = block_kind(block) else {
        return;
    };
    let mut state = read_state(&entity.nbt);
    let mut state_changed = false;
    if state.extraction_amount == 0 {
        state.extraction_amount = match kind {
            CapabilityKind::Item => MAX_ITEM_EXTRACTION_AMOUNT,
            CapabilityKind::Fluid => MAX_FLUID_EXTRACTION_AMOUNT,
            CapabilityKind::MercuryFlux => FLUX_DEFAULT_TRANSFER_RATE,
        };
        state_changed = true;
    }
    if state.flux_buffer < 0 {
        state.flux_buffer = 0;
        state_changed = true;
    }
    if state.flux_buffer > FLUX_BUFFER_CAPACITY {
        state.flux_buffer = FLUX_BUFFER_CAPACITY;
        state_changed = true;
    }
    if state.extraction_amount <= 0 {
        state.extraction_amount = 1;
        state_changed = true;
    }

    // Resolve the connector's target direction from block state.
    let facing = facing_of(game, pos);
    let _target_dir = connector_direction(state.direction_override, facing);

    if !state.enabled {
        if state_changed {
            write_state(&mut entity.nbt, &state);
        }
        return;
    }

    let now = game.world.game_time as i64;
    let period = match kind {
        CapabilityKind::Item => ITEM_EXTRACTION_EVERY_N_TICKS,
        CapabilityKind::Fluid => FLUID_EXTRACTION_EVERY_N_TICKS,
        CapabilityKind::MercuryFlux => FLUX_TRANSFER_EVERY_N_TICKS,
    };
    let offset = slow_tick_offset(pos, period);
    if on_slow_tick(offset, now, period) {
        match kind {
            CapabilityKind::MercuryFlux => {
                // Pulse the buffer by draining it; the actual push is
                // performed by the flux host helper on the network tick.
                if state.flux_buffer > 0 {
                    let drain = state.flux_buffer.min(FLUX_TRANSFER_RATE);
                    state.flux_buffer = state.flux_buffer.saturating_sub(drain);
                    state_changed = true;
                }
            }
            CapabilityKind::Item | CapabilityKind::Fluid => {
                // Extraction is handled by the host's network/push scan.
            }
        }
    }

    if state_changed {
        write_state(&mut entity.nbt, &state);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
        // Vanilla game times are non-negative, but the helper must be total.
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
    fn fresh_nexus_id_is_stable_and_position_sensitive() {
        let nbt = horizon_world::nbt::Compound::new();
        let entity = ModBlockEntity {
            id: "theurgy:logistics_nexus".to_owned(),
            nbt,
            items: Vec::new(),
        };
        let a = fresh_nexus_id([0, 0, 0], &entity);
        let b = fresh_nexus_id([0, 0, 0], &entity);
        let c = fresh_nexus_id([1, 0, 0], &entity);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
