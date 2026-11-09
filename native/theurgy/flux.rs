// SPDX-License-Identifier: MIT
// Mercury flux conversion and distribution.
//
// Ports:
//  - content/capability/SimpleMercuryFluxHandler.java
//  - content/apparatus/mercurycatalyst/MercuryCatalystBlockEntity.java (+ crafting behaviour)
//  - content/apparatus/mercurycapacitor/MercuryCapacitorBlockEntity.java
//  - content/apparatus/mercuryfluxemitter/MercuryFluxEmitterBlockEntity.java
//  - content/apparatus/caloricfluxemitter/CaloricFluxEmitterBlockEntity.java
//
// The engine's existing `Crafting` state (engine/src/crafting.rs) only models
// progress-based machines; the catalyst is flux-budget based, so its own pure
// conversion state lives here. Item extraction, recipes and networking are host
// work and are kept behind explicit, checked helpers.

use std::sync::Arc;

use horizon_client::containers::ModBlockEntity;
use horizon_client::items::ItemStack;
use horizon_world::nbt::{Compound, JavaString, Tag};

use super::theurgy_host;

// ---------------------------------------------------------------------------
// Direction helpers (mirroring net.minecraft.core.Direction)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Direction {
    pub const ALL: [Direction; 6] = [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ];

    pub const UP: Direction = Direction::Up;
    pub const DOWN: Direction = Direction::Down;
    pub const NORTH: Direction = Direction::North;
    pub const SOUTH: Direction = Direction::South;
    pub const WEST: Direction = Direction::West;
    pub const EAST: Direction = Direction::East;

    pub fn name(self) -> &'static str {
        match self {
            Direction::Down => "DOWN",
            Direction::Up => "UP",
            Direction::North => "NORTH",
            Direction::South => "SOUTH",
            Direction::West => "WEST",
            Direction::East => "EAST",
        }
    }

    pub fn opposite(self) -> Direction {
        match self {
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
            Direction::East => Direction::West,
        }
    }

    pub fn offset(self, pos: [i32; 3]) -> [i32; 3] {
        let [x, y, z] = pos;
        match self {
            Direction::Down => [x, y - 1, z],
            Direction::Up => [x, y + 1, z],
            Direction::North => [x, y, z - 1],
            Direction::South => [x, y, z + 1],
            Direction::West => [x - 1, y, z],
            Direction::East => [x + 1, y, z],
        }
    }

    pub fn from_name(name: &str) -> Option<Direction> {
        Self::ALL.into_iter().find(|d| d.name() == name)
    }

    pub fn from_index(index: usize) -> Option<Direction> {
        Self::ALL.get(index).copied()
    }
}

// ---------------------------------------------------------------------------
// SideMode (net.minecraft SideModeSetter.SideMode)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideMode {
    None,
    Input,
    Output,
    Both,
}

impl SideMode {
    pub fn ordinal(self) -> i32 {
        match self {
            SideMode::None => 0,
            SideMode::Input => 1,
            SideMode::Output => 2,
            SideMode::Both => 3,
        }
    }

    pub fn from_ordinal(value: i32) -> SideMode {
        match value.clamp(0, 3) {
            0 => SideMode::None,
            1 => SideMode::Input,
            2 => SideMode::Output,
            _ => SideMode::Both,
        }
    }

    pub fn cycle(self) -> SideMode {
        match self {
            SideMode::None => SideMode::Input,
            SideMode::Input => SideMode::Output,
            SideMode::Output => SideMode::Both,
            SideMode::Both => SideMode::None,
        }
    }

    pub fn can_insert(self) -> bool {
        matches!(self, SideMode::Input | SideMode::Both)
    }

    pub fn can_extract(self) -> bool {
        matches!(self, SideMode::Output | SideMode::Both)
    }
}

// ---------------------------------------------------------------------------
// SimpleMercuryFluxHandler: the pure energy buffer.
// ---------------------------------------------------------------------------

/// Total result of one flux operation, used to mirror the Java
/// `Transaction` + `SnapshotJournal` commit semantics in pure code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FluxTransfer {
    pub inserted: i32,
    pub extracted: i32,
}

/// Port of `SimpleMercuryFluxHandler`. `energy`/`capacity`/`max_insert`/
/// `max_extract` mirror the Java fields. All operations are pure; the caller
/// decides when to fire change notifications.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimpleMercuryFluxHandler {
    pub energy: i32,
    pub capacity: i32,
    pub max_insert: i32,
    pub max_extract: i32,
}

impl SimpleMercuryFluxHandler {
    pub fn new(capacity: i32) -> Self {
        Self::with_transfer(capacity, capacity, capacity)
    }

    pub fn with_transfer(capacity: i32, max_insert: i32, max_extract: i32) -> Self {
        Self::with_energy(capacity, max_insert, max_extract, 0)
    }

    pub fn with_energy(capacity: i32, max_insert: i32, max_extract: i32, energy: i32) -> Self {
        Self {
            energy: energy.max(0),
            capacity: capacity.max(0),
            max_insert: max_insert.max(0),
            max_extract: max_extract.max(0),
        }
    }

    pub fn amount(&self) -> i32 {
        self.energy
    }

    /// `SimpleMercuryFluxHandler.set`: clamps the amount to capacity.
    pub fn set(&mut self, amount: i32) {
        self.energy = amount.clamp(0, self.capacity.max(0));
    }

    /// `insert`: `min(capacity - energy, min(amount, max_insert))`.
    pub fn insert(&mut self, amount: i32) -> i32 {
        let amount = amount.max(0);
        let space = (self.capacity - self.energy).max(0);
        let inserted = space.min(amount.min(self.max_insert));
        if inserted > 0 {
            self.energy += inserted;
        }
        inserted
    }

    /// `extract`: `min(energy, min(amount, max_extract))`.
    pub fn extract(&mut self, amount: i32) -> i32 {
        let amount = amount.max(0);
        let extracted = self.energy.max(0).min(amount.min(self.max_extract));
        if extracted > 0 {
            self.energy -= extracted;
        }
        extracted
    }

    pub fn is_full(&self) -> bool {
        self.energy >= self.capacity
    }
}

// ---------------------------------------------------------------------------
// Catalyst conversion state (MercuryCatalystCraftingBehaviour)
// ---------------------------------------------------------------------------

/// Pure counterpart of the catalyst's three persisted fields plus progress.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CatalystConversion {
    pub mercury_flux_to_convert: i32,
    pub total_mercury_flux_to_convert: i32,
    pub current_mercury_flux_per_tick: i32,
}

/// One outcome of a server tick on the catalyst.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CatalystTick {
    pub flux_produced: i32,
    pub recipe_started: bool,
    pub block_updates: u8,
}

impl CatalystConversion {
    pub fn new() -> Self {
        Self::default()
    }

    /// `progressPercent`.
    pub fn progress_percent(&self) -> i32 {
        if self.total_mercury_flux_to_convert <= 0 {
            return 0;
        }
        let processed = self.total_mercury_flux_to_convert - self.mercury_flux_to_convert;
        ((processed as i64 * 100) / self.total_mercury_flux_to_convert as i64).clamp(0, 100) as i32
    }

    /// `canCraft`: any room in the buffer. `buffer_space` is
    /// `handler.getCapacityAsInt() - handler.getAmountAsInt()`.
    pub fn can_craft(buffer_space: i32, has_recipe: bool) -> bool {
        has_recipe && buffer_space > 0
    }

    /// `craft`: consumes input (host work) and installs the recipe's budget.
    pub fn begin(&mut self, total_flux: i32, flux_per_tick: i32) {
        let total = total_flux.max(0);
        self.mercury_flux_to_convert = total;
        self.total_mercury_flux_to_convert = total;
        self.current_mercury_flux_per_tick = flux_per_tick.max(0);
    }

    /// `tickServer` for the flux-conversion branch: push into the buffer.
    /// Returns how much flux was actually converted.
    pub fn convert_tick(&mut self, buffer: &mut SimpleMercuryFluxHandler) -> i32 {
        if self.mercury_flux_to_convert <= 0 {
            return 0;
        }
        if buffer.is_full() {
            return 0;
        }
        let wanted = self
            .mercury_flux_to_convert
            .min(self.current_mercury_flux_per_tick.max(0));
        if wanted <= 0 {
            return 0;
        }
        let before = buffer.energy;
        let _ = buffer.insert(wanted);
        let added = buffer.energy - before;
        if added > 0 {
            self.mercury_flux_to_convert = (self.mercury_flux_to_convert - added).max(0);
        }
        added
    }

    pub fn is_processing(&self) -> bool {
        self.mercury_flux_to_convert > 0
    }
}

// ---------------------------------------------------------------------------
// Distribution (MercuryCatalyst / MercuryCapacitor pushMercuryFlux)
// ---------------------------------------------------------------------------

pub const PUSH_TICK_INTERVAL: i32 = 20;
pub const PUSH_RATE_PER_SIDE_PER_TICK: i32 = 2;

/// Splits `total` evenly across `targets` targets, giving one extra to the
/// first `remainder` targets. Pure and deterministic.
pub fn split_evenly(total: i32, targets: usize) -> Vec<i32> {
    let mut shares = vec![0; targets];
    if targets == 0 {
        return shares;
    }
    let total = total.max(0);
    let base = total / targets as i32;
    let remainder = (total % targets as i32) as usize;
    for (i, share) in shares.iter_mut().enumerate() {
        *share = base + i32::from(i < remainder);
    }
    shares
}

/// `totalToPush` for a push cycle: `min(stored, rate * interval * targets)`.
pub fn total_to_push(stored: i32, target_count: usize) -> i32 {
    stored
        .max(0)
        .min(PUSH_RATE_PER_SIDE_PER_TICK * PUSH_TICK_INTERVAL * target_count.max(1) as i32)
}

// ---------------------------------------------------------------------------
// Constants mirrored from the Java block entities
// ---------------------------------------------------------------------------

pub const CATALYST_CAPACITY: i32 = 50000;
pub const CATALYST_UPDATE_THRESHOLD: i32 = 100;

pub const CAPACITOR_CAPACITY: i32 = 1000 * 50 * 10;
pub const CAPACITOR_UPDATE_THRESHOLD: i32 = 1000;

pub const MERCURY_EMITTER_CAPACITY: i32 = 1000;
pub const MERCURY_EMITTER_FLUX_PER_TRANSFER: i32 = 10;
pub const MERCURY_EMITTER_TICK_INTERVAL: i32 = 20;

pub const CALORIC_CAPACITY: i32 = 1000;
pub const CALORIC_FLUX_PER_HEAT: i32 = 100;
pub const CALORIC_HEAT_TARGET_FOR_TICKS: i32 = 20 * 6;
pub const CALORIC_TICK_INTERVAL: i32 = 20;

// ---------------------------------------------------------------------------
// Capacitor side configuration persisted as a compound
// ---------------------------------------------------------------------------

pub fn default_capacitor_side_modes() -> [(Direction, SideMode); 6] {
    [
        (Direction::Up, SideMode::Output),
        (Direction::Down, SideMode::Output),
        (Direction::North, SideMode::Input),
        (Direction::South, SideMode::Input),
        (Direction::East, SideMode::Input),
        (Direction::West, SideMode::Input),
    ]
}

// ---------------------------------------------------------------------------
// Host persistence helpers
// ---------------------------------------------------------------------------

fn compound_child<'a>(nbt: &'a Compound, key: &str) -> Option<&'a Compound> {
    match nbt.get(&JavaString::from(key)) {
        Some(Tag::Compound(c)) => Some(c),
        _ => None,
    }
}

fn set_child_compound(nbt: &mut Compound, key: &str, child: Compound) {
    nbt.insert(JavaString::from(key), Tag::Compound(child));
}

pub fn tank_energy(nbt: &Compound, key: &str) -> i32 {
    compound_child(nbt, key)
        .map(|c| theurgy_host::int(c, "energy", 0))
        .unwrap_or(0)
}

fn store_energy_child(nbt: &mut Compound, key: &str, energy: i32) {
    let mut child = Compound::new();
    theurgy_host::set_int(&mut child, "energy", energy);
    set_child_compound(nbt, key, child);
}

fn store_energy_child_or_discard(nbt: &mut Compound, key: &str, energy: i32) {
    // SimpleMercuryFluxHandler.serialize always writes energy, including zero.
    store_energy_child(nbt, key, energy.max(0));
}

fn stack_present(stack: &ItemStack) -> bool {
    stack.count > 0
}

/// Java's `enabled` block property is true unless explicitly 0.
fn enabled(nbt: &Compound) -> bool {
    nbt.get(&JavaString::from("enabled")).is_none() || theurgy_host::bool(nbt, "enabled")
}

// ---------------------------------------------------------------------------
// Recipe JSON helpers (real distribution: read from recipe.json)
// ---------------------------------------------------------------------------

fn json_int(value: &serde_json::Value, key: &str) -> Option<i32> {
    value.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
}

/// Reads the flux budget of a catalysation recipe from its JSON.
fn recipe_flux_budget(json: &serde_json::Value) -> Option<(i32, i32)> {
    let total = json_int(json, "totalMercuryFlux").or_else(|| json_int(json, "mercuryFlux"))?;
    let per_tick = json_int(json, "mercuryFluxPerTick").unwrap_or(total);
    Some((total, per_tick))
}

/// Finds the first catalysation recipe whose ingredient matches `stack`.
fn find_catalysation_recipe(
    game: &crate::game::Game<'_>,
    stack: &ItemStack,
) -> Option<(Arc<str>, i32, i32)> {
    let recipes = game.native_recipes_of_type("theurgy:catalysation");
    for recipe in recipes {
        let Some(ingredient) = recipe.json.get("ingredient").and_then(|value| game.native_recipe_ingredient(value)) else {
            continue;
        };
        if ingredient.test(stack) {
            let (total, per_tick) = recipe_flux_budget(&recipe.json)?;
            return Some((recipe.id.clone().into(), total, per_tick));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Catalyst
// ---------------------------------------------------------------------------

fn load_catalyst_conversion(nbt: &Compound) -> CatalystConversion {
    CatalystConversion {
        mercury_flux_to_convert: theurgy_host::int(nbt, "mercuryFluxToConvert", 0),
        total_mercury_flux_to_convert: theurgy_host::int(nbt, "totalMercuryFluxToConvert", 0),
        current_mercury_flux_per_tick: theurgy_host::int(nbt, "currentMercuryFluxPerTick", 0),
    }
}

fn save_catalyst_conversion(nbt: &mut Compound, state: &CatalystConversion) {
    theurgy_host::set_int(nbt, "mercuryFluxToConvert", state.mercury_flux_to_convert);
    theurgy_host::set_int(
        nbt,
        "totalMercuryFluxToConvert",
        state.total_mercury_flux_to_convert,
    );
    theurgy_host::set_int(
        nbt,
        "currentMercuryFluxPerTick",
        state.current_mercury_flux_per_tick,
    );
}

fn sync_enabled_from_redstone(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    let enabled = game.native_neighbor_signal(game.dim.id(), pos) == 0;
    if game.native_block_property(pos, "enabled").as_deref() != Some(if enabled { "true" } else { "false" }) {
        game.native_set_block_property_with(pos, "enabled", if enabled { "true" } else { "false" }, 3);
    }
}

fn tick_catalyst(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    sync_enabled_from_redstone(game, pos);
    let mut conversion = load_catalyst_conversion(&entity.nbt);
    let stored_energy = tank_energy(&entity.nbt, "mercuryFluxHandler");
    // Non-receiving: only internal generation can fill this buffer.
    let mut buffer = SimpleMercuryFluxHandler::with_energy(
        CATALYST_CAPACITY,
        CATALYST_CAPACITY,
        CATALYST_CAPACITY,
        stored_energy.clamp(0, CATALYST_CAPACITY),
    );

    let has_input = entity
        .items
        .first()
        .and_then(Option::as_ref)
        .is_some_and(stack_present);

    if conversion.mercury_flux_to_convert > 0 {
        let _ = conversion.convert_tick(&mut buffer);
    } else if has_input {
        let input = entity
            .items
            .first()
            .and_then(Option::as_ref)
            .cloned();
        if let Some(stack) = input {
            if let Some((_recipe_id, total, per_tick)) = find_catalysation_recipe(game, &stack) {
                let space = (CATALYST_CAPACITY - buffer.energy).max(0);
                if CatalystConversion::can_craft(space, true) {
                    conversion.begin(total, per_tick);
                    // Consume exactly one input item.
                    if let Some(slot) = entity.items.first_mut() {
                        if let Some(item) = slot.as_mut() {
                            item.count -= 1;
                            if item.count <= 0 {
                                *slot = None;
                            }
                        }
                    }
                }
            }
        }
    }

    save_catalyst_conversion(&mut entity.nbt, &conversion);
    store_energy_child_or_discard(&mut entity.nbt, "mercuryFluxHandler", buffer.energy);

    // Push flux out every PUSH_TICK_INTERVAL ticks when enabled.
    let game_time = game.world.game_time;
    if game_time % i64::from(PUSH_TICK_INTERVAL) == 0 {
        let enabled = enabled(&entity.nbt)
            && game.native_block_property(pos, "enabled").as_deref() != Some("false");
        if enabled {
            push_flux(game, pos, entity, &mut buffer, &Direction::ALL, None);
            store_energy_child_or_discard(&mut entity.nbt, "mercuryFluxHandler", buffer.energy);
        }
    }
}

fn receiver_limits(id: &str) -> Option<(i32, i32)> {
    // (capacity, maxInsert). Catalyst explicitly rejects external flux.
    Some(match id {
        "theurgy:mercury_catalyst" => (CATALYST_CAPACITY, 0),
        "theurgy:mercury_capacitor" => (CAPACITOR_CAPACITY, CAPACITOR_CAPACITY),
        "theurgy:mercury_flux_emitter"
        | "theurgy:caloric_flux_emitter"
        | "theurgy:sulfuric_flux_emitter"
        | "theurgy:logistics_mercury_flux_connector" => (1_000, 1_000),
        _ => return None,
    })
}

/// Pushes flux from `buffer` into adjacent flux handlers, conserving energy.
fn push_flux(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
    buffer: &mut SimpleMercuryFluxHandler,
    dirs: &[Direction],
    modes: Option<&[(Direction, SideMode); 6]>,
) {
    let mut directions: Vec<Direction> = Vec::new();
    for dir in dirs.iter().copied() {
        if let Some(modes) = modes {
            let mode = modes.iter().find(|(candidate, _)| *candidate == dir).map(|(_, mode)| *mode).unwrap_or(SideMode::None);
            if !mode.can_extract() {
                continue;
            }
        }
        let neighbor = dir.offset(pos);
        let neighbor_enabled = game.native_block_property(neighbor, "enabled").as_deref() != Some("false");
        let allowed = game.native_with_mod_block_entity(neighbor, |neighbor_entity| {
            if receiver_limits(&neighbor_entity.id).is_none() {
                return false;
            }
            if neighbor_entity.id == "theurgy:mercury_capacitor" {
                if !neighbor_enabled || !enabled(&neighbor_entity.nbt) {
                    return false;
                }
                let neighbor_modes = load_side_modes(&neighbor_entity.nbt);
                let side_mode = neighbor_modes.iter().find(|(candidate, _)| *candidate == dir.opposite()).map(|(_, mode)| *mode).unwrap_or(SideMode::None);
                if !side_mode.can_insert() {
                    return false;
                }
            }
            true
        }).unwrap_or(false);
        if allowed {
            directions.push(dir);
        }
    }
    if directions.is_empty() {
        return;
    }
    // Direction.allShuffled(level.random) for both catalyst and capacitor.
    use horizon_core::random::RandomSource;
    for index in (1..directions.len()).rev() {
        let other = game.world.random.next_int_bound((index + 1) as i32).unwrap_or(0) as usize;
        directions.swap(index, other);
    }
    let total = total_to_push(buffer.energy, directions.len());
    if total <= 0 {
        return;
    }
    let shares = split_evenly(total, directions.len());
    for (dir, share) in directions.iter().zip(shares.iter()) {
        let neighbor = dir.offset(pos);
        if *share <= 0 {
            continue;
        }
        let accepted = game.native_with_mod_block_entity(neighbor, |neighbor_entity| {
            let Some((capacity, max_insert)) = receiver_limits(&neighbor_entity.id) else {
                return 0;
            };
            if neighbor_entity.id == "theurgy:logistics_mercury_flux_connector" {
                return crate::game::theurgy_logistics::receive_mercury_flux(neighbor_entity, *share);
            }
            let mut neighbor_buffer = SimpleMercuryFluxHandler::with_energy(
                capacity,
                max_insert,
                capacity,
                tank_energy(&neighbor_entity.nbt, "mercuryFluxHandler")
                    .clamp(0, capacity),
            );
            let accepted = neighbor_buffer.insert(*share);
            if accepted > 0 {
                store_energy_child_or_discard(
                    &mut neighbor_entity.nbt,
                    "mercuryFluxHandler",
                    neighbor_buffer.energy,
                );
            }
            accepted
        }).unwrap_or(0);
        if accepted > 0 {
            let extracted = buffer.extract(accepted);
            debug_assert_eq!(extracted, accepted);
        }
    }
    let _ = entity;
}

// ---------------------------------------------------------------------------
// Capacitor
// ---------------------------------------------------------------------------

fn load_side_modes(nbt: &Compound) -> [(Direction, SideMode); 6] {
    let mut out = default_capacitor_side_modes();
    if let Some(child) = compound_child(nbt, "sideModes") {
        for (dir, mode) in out.iter_mut() {
            let fallback = mode.ordinal();
            let ordinal = theurgy_host::int(child, dir.name(), fallback);
            *mode = SideMode::from_ordinal(ordinal);
        }
    }
    out
}

fn save_side_modes(nbt: &mut Compound, modes: &[(Direction, SideMode); 6]) {
    let mut child = Compound::new();
    for (dir, mode) in modes {
        theurgy_host::set_int(&mut child, dir.name(), mode.ordinal());
    }
    set_child_compound(nbt, "sideModes", child);
}

fn tick_capacitor(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    sync_enabled_from_redstone(game, pos);
    let energy = tank_energy(&entity.nbt, "mercuryFluxHandler").clamp(0, CAPACITOR_CAPACITY);
    store_energy_child_or_discard(&mut entity.nbt, "mercuryFluxHandler", energy);
    let modes = load_side_modes(&entity.nbt);
    save_side_modes(&mut entity.nbt, &modes);

    let game_time = game.world.game_time;
    if game_time % i64::from(PUSH_TICK_INTERVAL) == 0 && game.native_block_property(pos, "enabled").as_deref() != Some("false") {
        let mut buffer = SimpleMercuryFluxHandler::with_energy(
            CAPACITOR_CAPACITY,
            CAPACITOR_CAPACITY,
            CAPACITOR_CAPACITY,
            energy,
        );
        push_flux(game, pos, entity, &mut buffer, &Direction::ALL, Some(&modes));
        store_energy_child_or_discard(&mut entity.nbt, "mercuryFluxHandler", buffer.energy);
    }
}

// ---------------------------------------------------------------------------
// Mercury flux emitter
// ---------------------------------------------------------------------------

fn tick_mercury_emitter(game: &mut crate::game::Game<'_>, _pos: [i32; 3], entity: &mut ModBlockEntity) {
    let energy = tank_energy(&entity.nbt, "mercuryFluxHandler").clamp(0, MERCURY_EMITTER_CAPACITY);

    let game_time = game.world.game_time;
    if game_time % i64::from(MERCURY_EMITTER_TICK_INTERVAL) != 0
        || !enabled(&entity.nbt)
        || game.native_block_property(_pos, "enabled").as_deref() == Some("false") {
        store_energy_child_or_discard(&mut entity.nbt, "mercuryFluxHandler", energy);
        return;
    }
    let Some(target) = crate::game::theurgy_selection::read_points(&entity.nbt, "selectedPoints")
        .first()
        .copied()
    else {
        store_energy_child_or_discard(&mut entity.nbt, "mercuryFluxHandler", energy);
        return;
    };

    // MercuryFluxEmitterMercuryFluxBehaviour.tickServer: exactly one selected
    // target and exactly ten units per slow tick, never all six neighbours.
    if energy >= MERCURY_EMITTER_FLUX_PER_TRANSFER {
        let accepted = game
            .native_with_mod_block_entity(target, |target_entity| {
                let Some((capacity, max_insert)) = receiver_limits(&target_entity.id) else {
                    return 0;
                };
                if target_entity.id == "theurgy:logistics_mercury_flux_connector" {
                    return crate::game::theurgy_logistics::receive_mercury_flux(target_entity, MERCURY_EMITTER_FLUX_PER_TRANSFER);
                }
                let stored = tank_energy(&target_entity.nbt, "mercuryFluxHandler")
                    .clamp(0, capacity);
                let mut receiver = SimpleMercuryFluxHandler::with_energy(
                    capacity,
                    max_insert,
                    capacity,
                    stored,
                );
                let accepted = receiver.insert(MERCURY_EMITTER_FLUX_PER_TRANSFER);
                if accepted > 0 {
                    store_energy_child_or_discard(
                        &mut target_entity.nbt,
                        "mercuryFluxHandler",
                        receiver.energy,
                    );
                }
                accepted
            })
            .unwrap_or(0);
        let remaining = energy - accepted;
        store_energy_child_or_discard(&mut entity.nbt, "mercuryFluxHandler", remaining);
        if accepted > 0 {
            game.native_block_entity_sync(target);
        }
    } else {
        store_energy_child_or_discard(&mut entity.nbt, "mercuryFluxHandler", energy);
    }

    // MercuryFluxEmitterEnergyBehaviour uses the same selected point. FE and
    // mercury flux are separate buffers; never convert one into the other.
    let energy_stored = compound_child(&entity.nbt, "energyStorage")
        .map(|child| theurgy_host::int(child, "energy", 0).clamp(0, MERCURY_EMITTER_CAPACITY))
        .unwrap_or(0);
    if energy_stored >= MERCURY_EMITTER_FLUX_PER_TRANSFER {
        let accepted = game
            .native_with_mod_block_entity(target, |target_entity| {
                let key = if compound_child(&target_entity.nbt, "energyStorage").is_some() {
                    "energyStorage"
                } else if compound_child(&target_entity.nbt, "energyBuffer").is_some() {
                    "energyBuffer"
                } else {
                    return 0;
                };
                let stored = compound_child(&target_entity.nbt, key)
                    .map(|child| theurgy_host::int(child, "energy", 0))
                    .unwrap_or(0)
                    .clamp(0, 1_000);
                let accepted = MERCURY_EMITTER_FLUX_PER_TRANSFER.min(1_000 - stored);
                if accepted > 0 {
                    store_energy_child(&mut target_entity.nbt, key, stored + accepted);
                }
                accepted
            })
            .unwrap_or(0);
        store_energy_child_or_discard(
            &mut entity.nbt,
            "energyStorage",
            energy_stored - accepted,
        );
    } else {
        store_energy_child_or_discard(&mut entity.nbt, "energyStorage", energy_stored);
    }
}

// ---------------------------------------------------------------------------
// Caloric flux emitter
// ---------------------------------------------------------------------------

fn selected_target(nbt: &Compound) -> Option<[i32; 3]> {
    crate::game::theurgy_selection::read_points(nbt, "selectedPoints")
        .first()
        .copied()
}

fn tick_caloric_emitter(game: &mut crate::game::Game<'_>, _pos: [i32; 3], entity: &mut ModBlockEntity) {
    sync_enabled_from_redstone(game, _pos);
    let energy = tank_energy(&entity.nbt, "mercuryFluxHandler").clamp(0, CALORIC_CAPACITY);
    store_energy_child_or_discard(&mut entity.nbt, "mercuryFluxHandler", energy);

    let game_time = game.world.game_time;
    if game_time % i64::from(CALORIC_TICK_INTERVAL) != 0 {
        return;
    }
    let enabled = enabled(&entity.nbt)
        && game.native_block_property(_pos, "enabled").as_deref() != Some("false");
    if !enabled {
        return;
    }
    let Some(target) = selected_target(&entity.nbt) else {
        return;
    };
    if energy < CALORIC_FLUX_PER_HEAT {
        return;
    }

    let valid_target = game
        .native_with_mod_block_entity(target, |target_entity| {
            crate::game::theurgy_selection::heat_receiver(&target_entity.id)
                && target_entity.data.get(4).copied().unwrap_or(0) == 0
        })
        .unwrap_or(false);
    if !valid_target {
        return;
    }
    let hot_until = game.native_with_mod_block_entity(target, |target_entity| {
        theurgy_host::int(&target_entity.nbt, "hotUntil", 0)
    });
    if hot_until.unwrap_or(0) > game_time as i32 + CALORIC_TICK_INTERVAL {
        return;
    }

    let new_hot_until = game_time as i32 + CALORIC_HEAT_TARGET_FOR_TICKS;
    let applied = game.native_with_mod_block_entity(target, |target_entity| {
        theurgy_host::set_int(&mut target_entity.nbt, "hotUntil", new_hot_until);
        true
    });
    if applied.unwrap_or(false) {
        let remaining = (energy - CALORIC_FLUX_PER_HEAT).max(0);
        store_energy_child_or_discard(&mut entity.nbt, "mercuryFluxHandler", remaining);
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
) {
    match entity.id.as_str() {
        "theurgy:mercury_catalyst" => tick_catalyst(game, pos, entity),
        "theurgy:mercury_capacitor" => tick_capacitor(game, pos, entity),
        "theurgy:mercury_flux_emitter" => tick_mercury_emitter(game, pos, entity),
        "theurgy:caloric_flux_emitter" => tick_caloric_emitter(game, pos, entity),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_sums_and_shares_remainder() {
        assert_eq!(split_evenly(10, 3), vec![4, 3, 3]);
        assert_eq!(split_evenly(0, 4), vec![0, 0, 0, 0]);
        assert_eq!(split_evenly(-5, 2), vec![0, 0]);
        assert!(split_evenly(7, 0).is_empty());
        let total: i32 = split_evenly(17, 5).iter().sum();
        assert_eq!(total, 17);
    }

    #[test]
    fn total_to_push_scales_with_targets() {
        assert_eq!(total_to_push(0, 3), 0);
        assert_eq!(
            total_to_push(1_000_000, 1),
            PUSH_RATE_PER_SIDE_PER_TICK * PUSH_TICK_INTERVAL
        );
        let many = total_to_push(1_000_000, 5);
        assert_eq!(many, PUSH_RATE_PER_SIDE_PER_TICK * PUSH_TICK_INTERVAL * 5);
    }

    #[test]
    fn simple_handler_respects_insert_and_extract_limits() {
        let mut h = SimpleMercuryFluxHandler::with_transfer(100, 20, 7);
        assert_eq!(h.insert(100), 20);
        assert_eq!(h.amount(), 20);
        assert_eq!(h.extract(100), 7);
        assert_eq!(h.amount(), 13);
        h.set(500);
        assert_eq!(h.amount(), 100);
        assert_eq!(h.insert(5), 0);
    }

    #[test]
    fn catalyst_progress_percent_tracks_converted_flux() {
        let mut c = CatalystConversion::new();
        c.begin(400, 10);
        assert_eq!(c.mercury_flux_to_convert, 400);
        assert_eq!(c.total_mercury_flux_to_convert, 400);
        assert_eq!(c.progress_percent(), 0);
        c.mercury_flux_to_convert = 300;
        assert_eq!(c.progress_percent(), 25);
        c.mercury_flux_to_convert = 0;
        assert_eq!(c.progress_percent(), 100);
        assert!(!c.is_processing());
    }

    #[test]
    fn catalyst_progress_percent_safe_on_zero_total() {
        let mut c = CatalystConversion::new();
        c.total_mercury_flux_to_convert = 0;
        c.mercury_flux_to_convert = -50;
        assert_eq!(c.progress_percent(), 0);
    }

    #[test]
    fn catalyst_can_craft_needs_recipe_and_space() {
        assert!(!CatalystConversion::can_craft(0, true));
        assert!(!CatalystConversion::can_craft(10, false));
        assert!(CatalystConversion::can_craft(10, true));
    }

    #[test]
    fn catalyst_convert_tick_caps_by_per_tick_rate_and_space() {
        let mut buffer = SimpleMercuryFluxHandler::new(100);
        let mut conv = CatalystConversion::new();
        conv.begin(1_000, 10);
        assert_eq!(conv.convert_tick(&mut buffer), 10);
        assert_eq!(buffer.energy, 10);
        assert_eq!(conv.mercury_flux_to_convert, 990);
        buffer.set(95);
        assert_eq!(conv.convert_tick(&mut buffer), 5);
        assert_eq!(conv.mercury_flux_to_convert, 985);
        buffer.set(100);
        assert_eq!(conv.convert_tick(&mut buffer), 0);
        assert_eq!(conv.mercury_flux_to_convert, 985);
    }

    #[test]
    fn catalyst_convert_tick_noop_when_drained_or_zero_rate() {
        let mut buffer = SimpleMercuryFluxHandler::new(100);
        let mut conv = CatalystConversion::new();
        conv.begin(50, 0);
        assert_eq!(conv.convert_tick(&mut buffer), 0);
        assert_eq!(buffer.energy, 0);
    }

    #[test]
    fn side_mode_can_insert_and_extract() {
        assert!(SideMode::Both.can_insert());
        assert!(SideMode::Both.can_extract());
        assert!(SideMode::Input.can_insert());
        assert!(!SideMode::Input.can_extract());
        assert!(SideMode::Output.can_extract());
        assert!(!SideMode::Output.can_insert());
        assert!(!SideMode::None.can_insert());
        assert!(!SideMode::None.can_extract());
    }

    #[test]
    fn side_mode_cycle_visits_every_variant() {
        let mut m = SideMode::None;
        for want in [
            SideMode::Input,
            SideMode::Output,
            SideMode::Both,
            SideMode::None,
        ] {
            m = m.cycle();
            assert_eq!(m, want);
        }
    }

    #[test]
    fn side_mode_ordinal_round_trip() {
        for m in [
            SideMode::None,
            SideMode::Input,
            SideMode::Output,
            SideMode::Both,
        ] {
            assert_eq!(SideMode::from_ordinal(m.ordinal()), m);
        }
        assert_eq!(SideMode::from_ordinal(-3), SideMode::None);
        assert_eq!(SideMode::from_ordinal(9), SideMode::Both);
    }

    #[test]
    fn direction_opposite_and_offset_are_involutions() {
        for d in Direction::ALL {
            assert_eq!(d.opposite().opposite(), d);
            assert_eq!(d.offset(d.opposite().offset([0, 0, 0])), [0, 0, 0]);
        }
        assert_eq!(Direction::Up.offset([1, 2, 3]), [1, 3, 3]);
        assert_eq!(Direction::North.offset([0, 0, 0]), [0, 0, -1]);
    }

    #[test]
    fn direction_name_round_trip() {
        for d in Direction::ALL {
            assert_eq!(Direction::from_name(d.name()), Some(d));
        }
        assert_eq!(Direction::from_name("NOPE"), None);
        assert_eq!(Direction::from_index(0), Some(Direction::Down));
        assert_eq!(Direction::from_index(6), None);
    }

    #[test]
    fn default_capacitor_modes_are_top_bottom_out_rest_in() {
        let modes = default_capacitor_side_modes();
        assert_eq!(
            modes.iter().find(|(d, _)| *d == Direction::Up).unwrap().1,
            SideMode::Output
        );
        assert_eq!(
            modes.iter().find(|(d, _)| *d == Direction::Down).unwrap().1,
            SideMode::Output
        );
        assert_eq!(
            modes.iter().find(|(d, _)| *d == Direction::North).unwrap().1,
            SideMode::Input
        );
    }

    #[test]
    fn side_modes_round_trip_through_nbt() {
        let mut modes = default_capacitor_side_modes();
        modes[0].1 = SideMode::Both;
        modes[3].1 = SideMode::None;
        let mut nbt = Compound::new();
        save_side_modes(&mut nbt, &modes);
        let loaded = load_side_modes(&nbt);
        assert_eq!(loaded, modes);

        // Missing prefix quietly uses defaults.
        let loaded_default = load_side_modes(&Compound::new());
        assert_eq!(loaded_default, default_capacitor_side_modes());
    }

    #[test]
    fn stack_present_only_when_count_positive() {
        let empty = ItemStack::new("minecraft:air", 0);
        assert!(!stack_present(&empty));
        let one = ItemStack::new("minecraft:stone", 1);
        assert!(stack_present(&one));
    }

    #[test]
    fn recipe_flux_budget_reads_json_fields() {
        let json = serde_json::json!({
            "mercuryFlux": 800,
            "mercuryFluxPerTick": 20
        });
        assert_eq!(recipe_flux_budget(&json), Some((800, 20)));

        let crystal = serde_json::json!({
            "mercuryFlux": 3200,
            "mercuryFluxPerTick": 20
        });
        assert_eq!(recipe_flux_budget(&crystal), Some((3200, 20)));

        let missing = serde_json::json!({});
        assert_eq!(recipe_flux_budget(&missing), None);
    }
}
