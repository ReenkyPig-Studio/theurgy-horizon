// SPDX-License-Identifier: MIT
// Bounded performance / allocation measurements for the pure engine.
//
// Java sources this guards (read-only reference, release/v26.3-1.120.1,
// revision 33b959c9ed0da1de2aff649e9728c1a03470b1bd):
//
// * content/behaviour/crafting/CraftingBehaviour.java
//     tickServer(canProcess, hasInput):
//       if (hasInput) {
//         recipe = getRecipe()            // LOOKUP #1, every tick
//         couldCraftLastTick = canCraft(recipe)   // no lookup: holder reused
//         if (canProcess && couldCraftLastTick) {
//           tryStartProcessing()          // sends a block update only if progress==0
//           progress++
//           tryFinishProcessing(recipe)   // only when progress >= totalTime
//         } else stopProcessing()         // sends a block update only if state changed
//       } else stopProcessing()
//     tryFinishProcessing: when progress >= totalTime:
//         progress = 0; totalTime = getTotalTime();  // LOOKUP #2, only on craft ticks
//         craft(pRecipe); sendBlockUpdated();
//     onInputItemChanged(old,new): totalTime = getTotalTime(); progress = 0;
//         // LOOKUP, but driven by the storage behaviour on slot content-type
//         // change (DistillationStorageBehaviour.InputInventory
//         // .onContentTypeChanged), NOT by tickServer.
//     canCraft: assemble() then a simulated insert into the output storage inside
//         a rolled-back Transaction — pure, owns no lasting state.
//     stopProcessing / tryStartProcessing / tryFinishProcessing only call
//     sendBlockUpdated() on a real state change, never unconditionally.
//
// * content/apparatus/distiller/DistillationCachedCheck.java
//     getRecipeFor(input, level) -> internal.getRecipeFor(container, serverLevel)
//     on the server, i.e. a full recipe.matches() walk (ingredients AND counts)
//     with a `lastRecipe` fast path; the raw getRecipeFor(stack, level) overload
//     scans all recipes of the type with ingredient().test(stack).
//
// * content/recipe/{Distillation,Calcination,Liquefaction,Fermentation,Digestion}Recipe.java
//     matches() = SizedIngredient / SizedFluidIngredient tests (count is part of
//     the match for sized ingredients and for sized *fluid* ingredients).
//
// Therefore, for a machine that is fed continuously and never changes its input
// content type, the exact lookup identity per measurement is
//     lookups == ticks + crafts + input_changes
// and NOT 2 * ticks. Each lookup proposes every row of that recipe type
// (the `lastRecipe` fast path is a host cache, not modelled here, so the
// measurement is the worst case a cold or churning cache would see).
//
// This module deliberately contains no timing-based assertions. A wall-clock
// number is not a portable fact, so instead every check below is a deterministic
// measurement of *work* (recipe rows proposed to the host matcher, matcher
// closures executed, block updates, commits, host plan allocations, growth of a
// host-owned Vec) or of an operation-count identity that must hold for any
// correct implementation. Timing is only measured and reported by
// `measure_*` / `report()` and by tools/bench-port.mjs; it is never asserted.
//
// Nothing here allocates on behalf of the engine itself: the counters live in
// caller-provided `PerfCounters` and `tick` takes caller-owned buffers.

use crate::crafting::Crafting;
use crate::machines::{CraftPlan, ProcessResult, layout, tick_machine};
use crate::recipes::{Item, Recipe, Tank, find_recipe};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

/// Deterministic work accounting shared by every measurement below.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PerfCounters {
    /// Recipe rows scanned anywhere (find_recipe candidates + host matcher rows).
    /// One `find_recipe` over a table of `n` rows accounts for `n` scans.
    pub recipe_scans: u64,
    /// `matches` closures invoked (host tag/component resolution work).
    pub matcher_calls: u64,
    /// `plan_for` calls (host plan construction, may allocate).
    pub plan_calls: u64,
    /// Block updates requested, i.e. CraftingBehaviour.sendBlockUpdated().
    pub block_updates: u64,
    /// `state.total_time != time` resets (onInputItemChanged).
    pub state_resets: u64,
    /// Atomic commit attempts (CraftingBehaviour.craft).
    pub commits: u64,
    /// CraftingBehaviour.getRecipe() calls (tickServer LOOKUP #1).
    pub lookups: u64,
    /// CraftingBehaviour.getTotalTime() calls (tryFinishProcessing LOOKUP #2
    /// and onInputItemChanged). Always one extra table walk each.
    pub total_time_lookups: u64,
}

impl PerfCounters {
    /// Total work units; useful to compare two implementations on identical input.
    pub fn work(&self) -> u64 {
        self.recipe_scans + self.matcher_calls + self.plan_calls
    }
    /// Every recipe lookup performed, of either kind.
    pub fn all_lookups(&self) -> u64 {
        self.lookups + self.total_time_lookups
    }
}

/// Output buffer for one tick. Allocated by the caller so a measurement never
/// adds engine-side allocations to the number it is reporting.
pub struct TickOutcome {
    pub result: Option<ProcessResult>,
    pub committed: bool,
    pub last_item_copy_len: usize,
}

pub struct TickHarness<'a> {
    pub kind: &'a str,
    pub state: Crafting,
    pub items: Vec<Option<Item>>,
    pub tanks: Vec<Tank>,
    pub can_process: bool,
    pub counters: PerfCounters,
}

impl<'a> TickHarness<'a> {
    /// Single input slot + single output slot, the distiller/calcination shape.
    pub fn new(kind: &'a str, input: Option<Item>, output: Option<Item>, tank: Option<Tank>) -> Self {
        let inputs = layout(kind).map(|l| l.inputs.len()).unwrap_or(1);
        let mut items = vec![None; inputs + 1];
        if inputs > 0 {
            items[0] = input;
        }
        items[inputs] = output;
        Self {
            kind,
            state: Crafting::default(),
            items,
            tanks: tank.into_iter().collect(),
            can_process: true,
            counters: PerfCounters::default(),
        }
    }

    /// Closest host equivalent of `matches` for the row layouts in
    /// `crate::recipes::ingredient_parts`: the input stack id must equal the
    /// ingredient id, and the ingredient row must actually be an item row.
    pub fn id_matches(ingredient: &Value, item: &Item) -> bool {
        match ingredient {
            Value::String(id) => id == &item.id,
            Value::Object(obj) => match obj.get("ingredient") {
                Some(Value::String(id)) => id == &item.id,
                Some(Value::Object(inner)) => inner
                    .get("ingredient")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id == item.id),
                _ => false,
            },
            Value::Array(any_of) => any_of.iter().any(|v| v.as_str().is_some_and(|id| id == item.id)),
            _ => false,
        }
    }
    /// Mirrors CraftingBehaviour.canCraft: assemble() must produce a non-empty
    /// stack and a simulated insertion into the output slot must fully succeed.
    /// In Java that simulation runs inside a rolled-back Transaction and owns no
    /// lasting state, so a pure can_fit check is the faithful equivalent.
    pub fn can_craft(&self, recipe: &Recipe) -> bool {
        let Some(plan) = plan_for(self.kind, recipe) else { return false };
        match plan.output {
            Some((slot, result)) => crate::recipes::can_fit(self.items.get(slot).and_then(Option::as_ref), &result),
            None => false,
        }
    }

    /// CraftingBehaviour.getRecipe(): the one lookup tickServer performs up front.
    fn lookup<'r>(&mut self, recipes: &'r [Recipe<'r>], counters: &mut PerfCounters) -> Option<&'r Recipe<'r>> {
        let tank = layout(self.kind).and_then(|l| l.tank).and_then(|i| self.tanks.get(i));
        counters.lookups += 1;
        counters.recipe_scans += recipes.len() as u64;
        find_recipe(self.kind, recipes, &self.items, tank, |ingredient, item| {
            counters.matcher_calls += 1;
            TickHarness::id_matches(ingredient, item)
        })
    }

    /// CraftingBehaviour.onInputItemChanged, driven by the storage behaviour when
    /// the *content type* of the input slot changes. Performs the second lookup.
    pub fn input_item_changed(&mut self, recipes: &[Recipe], counters: &mut PerfCounters) {
        counters.total_time_lookups += 1;
        counters.recipe_scans += recipes.len() as u64;
        let tank = layout(self.kind).and_then(|l| l.tank).and_then(|i| self.tanks.get(i));
        let time = find_recipe(self.kind, recipes, &self.items, tank, |ingredient, item| {
            counters.matcher_calls += 1;
            TickHarness::id_matches(ingredient, item)
        })
        .map(|r| r.time.max(0))
        .unwrap_or(0);
        if self.state.total_time != time {
            self.state.input_changed(time);
            counters.state_resets += 1;
        }
    }

    /// One faithful CraftingBehaviour.tickServer pass with deterministic work
    /// accounting. `out` and `buffer` are caller-owned so a measurement adds no
    /// engine-side allocation; a host keeps one `PlanBuffer` for all machines.
    pub fn tick(&mut self, recipes: &[Recipe], buffer: &mut PlanBuffer, out: &mut TickOutcome) {
        let mut counters = self.counters;
        let kind = self.kind;
        if self.items.first().and_then(Option::as_ref).is_none() {
            // hasInput == false: Java does not even look the recipe up.
            self.state.stop();
            self.counters = counters;
            out.result = None;
            out.committed = false;
            out.last_item_copy_len = self.items.len();
            return;
        }
        // LOOKUP #1: getRecipe(), once per ticking machine per tick.
        let selected = self.lookup(recipes, &mut counters);
        // Java re-reads totalTime through getTotalTime() and hands the result to
        // tryFinishProcessing. machines::tick_machine models that by resetting via
        // input_changed(time) only after find_recipe succeeds; mirror it exactly so
        // the two paths cannot drift.
        let expected_time = selected.map(|r| r.time.max(0));
        if let Some(time) = expected_time {
            if self.state.total_time != time {
                counters.total_time_lookups += 1;
                counters.recipe_scans += recipes.len() as u64;
                self.state.input_changed(time);
                counters.state_resets += 1;
            }
        }
        // canCraft(recipe) reuses that holder; it is a pure simulated insert.
        let can_craft = selected.is_some_and(|r| self.can_craft(r));
        let before = self.state.clone();
        let items_before = self.items.clone();
        let tanks_before = self.tanks.clone();
        // The state machine itself is Crafting::tick, the exact same function
        // machines::tick_machine delegates to, so the two paths cannot diverge.
        let effects = self.state.tick(self.can_process, true, can_craft, expected_time.unwrap_or(0));
        counters.block_updates += u64::from(effects.block_updates);
        let mut crafted_now = false;
        if effects.craft_attempt {
            // tryFinishProcessing LOOKUP #2: totalTime = getTotalTime() on the
            // finishing tick, which is a fresh table walk like getRecipe().
            counters.total_time_lookups += 1;
            counters.recipe_scans += recipes.len() as u64;
            // craft(pRecipe): one host plan build, then the atomic commit.
            if let Some(recipe) = selected {
                if let Some(plan) = plan_for(kind, recipe) {
                    counters.plan_calls += 1;
                    buffer.write(&plan.inputs);
                }
            }
            counters.commits += 1;
            crafted_now = commit_selected(self, selected);
        }
        debug_assert!(
            !crafted_now || can_craft,
            "craft committed without a successful canCraft simulation"
        );
        // Guard, debug builds only: the production tick_machine must reach the
        // same outcome as this faithful inlined model. It is compiled out in
        // release so the reported allocation/timing numbers are not polluted by
        // the probe's own clones.
        if cfg!(debug_assertions) {
            // The probe must start from the *pre-tick* snapshot: it replays the
            // tick independently, so feeding it the already-committed state would
            // count this tick's craft twice.
            let mut probe_state = before.clone();
            let mut probe_items = items_before.clone();
            let mut probe_tanks = tanks_before.clone();
            let probe = tick_machine(
                kind,
                &mut probe_state,
                &mut probe_items,
                &mut probe_tanks,
                recipes,
                self.can_process,
                TickHarness::id_matches,
                // CraftingBehaviour.canCraft assembles *and* simulates the output
                // insertion; returning no plan is how tick_machine sees that
                // failure at the plan stage.
                |recipe| {
                    let plan = plan_for(kind, recipe)?;
                    match &plan.output {
                        Some((slot, result))
                            if !crate::recipes::can_fit(self.items.get(*slot).and_then(Option::as_ref), result) => None,
                        _ => Some(plan),
                    }
                },
            );
            let probe_crafted = probe.as_ref().is_some_and(|r| r.crafted);
            debug_assert_eq!(
                (probe_items, probe_tanks, probe_state, probe_crafted),
                (self.items.clone(), self.tanks.clone(), self.state.clone(), crafted_now),
                "inlined tickServer model diverged from machines::tick_machine"
            );
        }
        self.counters = counters;
        out.result = Some(ProcessResult {
            selected_recipe: selected.map(|r| r.id.to_string()),
            crafted: crafted_now,
            state_changed: before != self.state || effects.block_updates != 0,
        });
        out.committed = crafted_now;
        out.last_item_copy_len = self.items.len();
    }
}

/// CraftingBehaviour.craft(): extract getIngredientCount() inputs and insert the
/// assembled result, atomically. Returns false (nothing changes) on any failure.
fn commit_selected(harness: &mut TickHarness, recipe: Option<&Recipe>) -> bool {
    let Some(recipe) = recipe else { return false };
    let Some(plan) = plan_for(harness.kind, recipe) else { return false };
    crate::machines::commit_plan(&mut harness.items, &mut harness.tanks, &plan)
}

/// Caller-owned scratch space for the host's plan inputs (CraftingBehaviour.craft
/// extracts from the input storage; the host builds that list first to keep the
/// extraction atomic). Its capacity is the allocation high-water mark to measure.
#[derive(Clone, Debug, Default)]
pub struct PlanBuffer {
    pub entries: Vec<(usize, u32)>,
}

impl PlanBuffer {
    pub fn with_capacity(n: usize) -> Self {
        Self { entries: Vec::with_capacity(n) }
    }
    pub fn reserve_to(&mut self, n: usize) {
        if self.entries.capacity() < n {
            self.entries.reserve(n - self.entries.len());
        }
    }
    pub fn write(&mut self, inputs: &[(usize, u32)]) {
        self.entries.clear();
        self.entries.extend_from_slice(inputs);
    }
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The `result` row of every Theurgy machine recipe is an ItemStackTemplate:
/// a bare item id or {"id": "...", "count": n}. A row the host cannot assemble
/// yields no plan, which is exactly CraftingBehaviour.canCraft's `isEmpty()` case.
pub fn plan_for(kind: &str, recipe: &Recipe) -> Option<CraftPlan> {
    let layout = layout(kind)?;
    let value = recipe.json.get("result")?;
    let (id, count) = match value {
        Value::String(id) => (id.as_str(), 1u32),
        Value::Object(o) => (o.get("id")?.as_str()?, o.get("count").and_then(Value::as_u64).unwrap_or(1) as u32),
        _ => return None,
    };
    if count == 0 || id.is_empty() {
        return None;
    }
    // CraftingBehaviour.craft extracts getIngredientCount() from the *single*
    // input resource; fermentation/digestion consume one of every ingredient.
    let inputs = crate::recipes::ingredient_parts(kind, recipe.json)
        .iter()
        .enumerate()
        .filter_map(|(i, (_, count))| {
            let slot = *layout.inputs.get(i)?;
            Some((slot, *count))
        })
        .collect();
    let fluid = crate::recipes::fluid_part(kind, recipe.json)
        .and_then(|(_, amount)| layout.tank.map(|slot| (slot, amount)));
    Some(CraftPlan {
        inputs,
        output: Some((layout.output, Item { id: id.into(), count, max_stack: 64 })),
        fluid,
    })
}

/// Deterministic, reproducible so a fixed seed gives the same JSON every run.
#[derive(Clone, Copy, Debug)]
pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u32() % n }
    }
}

/// Fixed workload description. Everything is deterministic; nothing is sampled.
#[derive(Clone, Debug)]
pub enum Fixture {
    /// Distiller-style single sized ingredient + single output.
    Distiller,
    /// Liquefaction: unsized ingredient + sized fluid solvent.
    Liquefaction,
    /// Fermentation/digestion: three ingredients + fluid.
    Vat,
    /// Distiller table of `n` rows where the last one matches, to expose the
    /// linear scan cost of a cache miss without lastRecipe.
    LinearScan(u32),
}

impl Fixture {
    pub fn kind(&self) -> &'static str {
        match self {
            Fixture::Distiller | Fixture::LinearScan(_) => "theurgy:distillation",
            Fixture::Liquefaction => "theurgy:liquefaction",
            Fixture::Vat => "theurgy:fermentation",
        }
    }
    /// Recipe rows are owned by the caller so `Recipe<'a>` can borrow them.
    pub fn rows(&self) -> Vec<Value> {
        match self {
            Fixture::Distiller => vec![
                json!({"ingredient": {"ingredient": "minecraft:beef", "count": 2}, "result": "theurgy:mercury", "time": 100}),
                json!({"ingredient": {"ingredient": "minecraft:coal", "count": 1}, "result": "theurgy:sulfur", "time": 100}),
            ],
            Fixture::Liquefaction => vec![
                json!({"ingredient": "minecraft:oak_log", "solvent": {"ingredient": "theurgy:sal_ammoniac", "amount": 10},
                       "result": "theurgy:mercury", "time": 100}),
                json!({"ingredient": "minecraft:birch_log", "solvent": {"ingredient": "theurgy:sal_ammoniac", "amount": 10},
                       "result": "theurgy:sulfur", "time": 100}),
            ],
            Fixture::Vat => vec![
                json!({"fluid": {"ingredient": "minecraft:water", "amount": 500},
                       "ingredients": ["minecraft:beef", {"ingredient": "minecraft:wheat", "count": 2}, "minecraft:sugar"],
                       "result": "theurgy:mercury", "time": 200}),
            ],
            Fixture::LinearScan(n) => (0..*n)
                .map(|i| {
                    if i + 1 == *n {
                        json!({"ingredient": {"ingredient": "minecraft:beef", "count": 2},
                               "result": "theurgy:mercury", "time": 100})
                    } else {
                        json!({"ingredient": {"ingredient": format!("theurgy:filler_{i}"), "count": 1},
                               "result": "theurgy:sulfur", "time": 100})
                    }
                })
                .collect(),
        }
    }
    /// The item a continuous feed puts back into slot 0 once it is consumed.
    pub fn feed(&self) -> Item {
        match self {
            Fixture::Distiller | Fixture::LinearScan(_) =>
                Item { id: "minecraft:beef".into(), count: 64, max_stack: 64 },
            Fixture::Liquefaction => Item { id: "minecraft:oak_log".into(), count: 16, max_stack: 64 },
            Fixture::Vat => Item { id: "minecraft:beef".into(), count: 8, max_stack: 64 },
        }
    }
    pub fn harness<'a>(&self, kind: &'a str) -> TickHarness<'a> {
        match self {
            Fixture::Distiller | Fixture::LinearScan(_) => TickHarness::new(kind, Some(self.feed()), None, None),
            Fixture::Liquefaction => TickHarness::new(
                kind,
                Some(self.feed()),
                None,
                Some(Tank { fluid: Some("theurgy:sal_ammoniac".into()), amount: 1000, capacity: 4000 }),
            ),
            Fixture::Vat => {
                let mut h = TickHarness::new(
                    kind,
                    Some(self.feed()),
                    None,
                    Some(Tank { fluid: Some("minecraft:water".into()), amount: 4000, capacity: 8000 }),
                );
                h.items[1] = Some(Item { id: "minecraft:wheat".into(), count: 8, max_stack: 64 });
                h.items[2] = Some(Item { id: "minecraft:sugar".into(), count: 8, max_stack: 64 });
                h
            }
        }
    }
}

/// Result of a bounded measurement. `elapsed` is reported, never asserted.
#[derive(Clone, Copy, Debug, Default)]
pub struct Measurement {
    pub ticks: u64,
    pub elapsed: Duration,
    pub counters: PerfCounters,
    pub crafts: u64,
    pub final_progress: i32,
    pub final_total_time: i32,
    pub item_checksum: u64,
}

impl Measurement {
    pub fn nanos_per_tick(&self) -> f64 {
        if self.ticks == 0 { return 0.0 }
        self.elapsed.as_nanos() as f64 / self.ticks as f64
    }
    pub fn work_per_tick(&self) -> f64 {
        if self.ticks == 0 { return 0.0 }
        self.counters.work() as f64 / self.ticks as f64
    }
    /// Deterministic fingerprint of inventory/fluid state, so two implementations
    /// can be compared without trusting a timer.
    pub fn checksum(&self) -> u64 {
        self.item_checksum ^ (self.crafts << 8) ^ self.counters.work()
    }
}

fn state_checksum(items: &[Option<Item>], tanks: &[Tank]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for item in items {
        let (id, count) = match item {
            Some(i) => (i.id.as_str(), i.count),
            None => ("", 0),
        };
        for b in id.as_bytes() {
            h = (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3);
        }
        h = (h ^ u64::from(count)).wrapping_mul(0x100_0000_01b3);
    }
    for tank in tanks {
        h = (h ^ tank.amount as u32 as u64).wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Runs exactly `ticks` ticks over `fixture` with `recipes` supplied by the caller
/// (must borrow `fixture.rows()`), returning measured work and elapsed time.
/// The input is topped back up whenever it runs out, which models a hopper-fed
/// machine; that refill is a host action, so it is *not* reported as an
/// onInputItemChanged lookup.
pub fn measure_tick(fixture: &Fixture, recipes: &[Recipe], ticks: u32) -> Measurement {
    let mut harness = fixture.harness(fixture.kind());
    let mut out = TickOutcome { result: None, committed: false, last_item_copy_len: 0 };
    let mut buffer = PlanBuffer::with_capacity(layout(fixture.kind()).map_or(0, |l| l.inputs.len()));
    let mut m = Measurement::default();
    let feed = fixture.feed();
    // Java sets totalTime via onInputItemChanged when the slot first receives an
    // item, so the machine starts with a known duration instead of 0.
    let mut counters = PerfCounters::default();
    harness.input_item_changed(recipes, &mut counters);
    harness.counters = counters;
    let start = Instant::now();
    for _ in 0..ticks {
        harness.tick(recipes, &mut buffer, &mut out);
        m.ticks += 1;
        if out.committed {
            m.crafts += 1;
            // Java has no auto-restock: once the input stack is exhausted the
            // machine stops. Replenish so the measurement covers crafting ticks.
            if harness.items[0].as_ref().is_none_or(|i| i.count < 2) {
                harness.items[0] = Some(Item { count: feed.count, ..feed.clone() });
            }
        }
    }
    m.elapsed = start.elapsed();
    m.counters = harness.counters;
    m.final_progress = harness.state.progress;
    m.final_total_time = harness.state.total_time;
    m.item_checksum = state_checksum(&harness.items, &harness.tanks);
    m
}

/// Idle ticks (no input): Java takes the `else` branch of tickServer, so it must
/// perform no recipe lookup, matcher, plan, commit or block-update work at all.
pub fn measure_idle(recipes: &[Recipe], ticks: u32) -> Measurement {
    let kind = "theurgy:distillation";
    let mut harness = TickHarness::new(kind, None, None, None);
    let mut out = TickOutcome { result: None, committed: false, last_item_copy_len: 0 };
    let mut buffer = PlanBuffer::default();
    let mut m = Measurement::default();
    let start = Instant::now();
    for _ in 0..ticks {
        harness.tick(recipes, &mut buffer, &mut out);
        m.ticks += 1;
    }
    m.elapsed = start.elapsed();
    m.counters = harness.counters;
    m.item_checksum = state_checksum(&harness.items, &harness.tanks);
    m
}

/// Host buffer reuse: a warm caller-owned plan buffer must never reallocate,
/// independent of how long the machine runs.
pub fn plan_buffer_capacity(fixture: &Fixture, recipes: &[Recipe], ticks: u32) -> (usize, usize) {
    let mut harness = fixture.harness(fixture.kind());
    let mut out = TickOutcome { result: None, committed: false, last_item_copy_len: 0 };
    // The host sets totalTime via onInputItemChanged when the slot first receives
    // its item, so the machine knows its duration before the first tick.
    let mut counters = PerfCounters::default();
    harness.input_item_changed(recipes, &mut counters);
    harness.counters = counters;
    // Start from an empty buffer, exactly as a host does on first tick.
    let mut buffer = PlanBuffer::default();
    // Run past one full recipe duration so at least one craft writes a plan.
    let span = ticks.max(harness.state.total_time.max(1) as u32 + ticks);
    let mut cold = 0usize;
    for _ in 0..span {
        harness.tick(recipes, &mut buffer, &mut out);
        // Capture the capacity the first time a plan is actually written; before
        // that the buffer is empty by definition and its capacity says nothing.
        if cold == 0 && !buffer.is_empty() {
            cold = buffer.capacity();
        }
    }
    (cold, buffer.capacity())
}

/// Builds the borrowable recipe table the engine functions consume. `time` is
/// what Recipe.time carries; other fields stay inside the serde_json::Value.
fn recipe_table<'a>(fixture: &Fixture, rows: &'a [Value]) -> Vec<Recipe<'a>> {
    let kind = fixture.kind();
    rows.iter()
        .map(|json| Recipe {
            id: kind,
            json,
            time: json.get("time").and_then(Value::as_i64).unwrap_or(0) as i32,
        })
        .collect()
}

/// Stable machine-readable names for the benchmark rows (tools/bench-port.mjs).
impl Fixture {
    pub fn label(&self) -> String {
        match self {
            Fixture::Distiller => "distiller".to_string(),
            Fixture::Liquefaction => "liquefaction".to_string(),
            Fixture::Vat => "vat".to_string(),
            Fixture::LinearScan(n) => format!("linear_scan_{n}"),
        }
    }
}

const REPORT_FIXTURES: &[(Fixture, u32)] = &[
    (Fixture::Distiller, 400),
    (Fixture::Liquefaction, 400),
    (Fixture::Vat, 400),
    (Fixture::LinearScan(64), 64),
];

fn report_line(label: &str, m: &Measurement) -> String {
    format!(
        "fixture={label} ticks={} scans={} matcher_calls={} plan_calls={} block_updates={} commits={} crafts={} lookups={} total_time_lookups={} work_per_tick={:.3} ns_per_tick={:.1} checksum={:#018x}",
        m.ticks, m.counters.recipe_scans, m.counters.matcher_calls, m.counters.plan_calls,
        m.counters.block_updates, m.counters.commits, m.crafts,
        m.counters.lookups, m.counters.total_time_lookups,
        m.work_per_tick(), m.nanos_per_tick(), m.checksum()
    )
}

/// Machine-readable, deterministic report body. `ns_per_tick` is a measured
/// observation; consumers must not turn it into a pass/fail threshold.
pub fn report() -> String {
    let mut lines = vec!["# theurgy_engine bounded performance report".to_string()];
    for (fixture, ticks) in REPORT_FIXTURES {
        let rows = fixture.rows();
        let recipes = recipe_table(fixture, &rows);
        let m = measure_tick(fixture, &recipes, *ticks);
        let (cold, warm) = plan_buffer_capacity(fixture, &recipes, *ticks);
        // matcher work is per (recipe row, non-empty input slot), so report the
        // slot count too; the JS checker derives its bound from it instead of
        // guessing.
        let slots = fixture.harness(fixture.kind()).items.iter().flatten().count();
        // The host matcher closure runs once per (row, ingredient part, candidate
        // slot) visited by find_recipe before it hits or exhausts the slot list, so
        // the JS checker needs the largest per-row part count to bound it instead
        // of guessing from the recipe type.
        let parts = rows
            .iter()
            .map(|row| crate::recipes::ingredient_parts(fixture.kind(), row).len())
            .max()
            .unwrap_or(0);
        let mut line = report_line(&fixture.label(), &m);
        line.push_str(&format!(
            " item_slots={slots} parts={parts} rows={} plan_buf_capacity={cold}->{warm}",
            rows.len()
        ));
        lines.push(line);
    }
    let fixture = Fixture::Distiller;
    let rows = fixture.rows();
    let recipes = recipe_table(&fixture, &rows);
    // The idle row carries the same trailing fields so the JavaScript checker
    // can validate every line uniformly instead of special-casing once.
    let mut idle = report_line("idle", &measure_idle(&recipes, 64));
    idle.push_str(&format!(
        " item_slots=0 parts={} rows={} plan_buf_capacity=0->0",
        rows.iter()
            .map(|row| crate::recipes::ingredient_parts(fixture.kind(), row).len())
            .max()
            .unwrap_or(0),
        rows.len()
    ));
    lines.push(idle);
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------
    // Work-identity helpers. Every assertion below is a deterministic integer
    // relation taken from CraftingBehaviour.java, never a clock reading.
    // ---------------------------------------------------------------------

    /// Java: lookups == ticks + crafts + input_changes, and every lookup walks
    /// the whole table, so scans == lookups * rows.
    fn assert_lookup_identity(m: &Measurement, rows: usize, input_changes: u64) {
        let rows = rows as u64;
        assert_eq!(
            m.counters.all_lookups(),
            m.ticks + m.crafts + input_changes,
            "lookups {} must equal ticks + crafts + input_changes",
            m.counters.all_lookups()
        );
        assert_eq!(m.counters.recipe_scans, m.counters.all_lookups() * rows);
        assert_eq!(m.counters.lookups, m.ticks - 0, "getRecipe runs once per tick only");
        assert_eq!(m.counters.total_time_lookups, m.crafts + input_changes,
            "getTotalTime runs on craft ticks and on input content-type changes");
    }

    #[test]
    fn idle_machine_performs_no_work_at_all() {
        let fixture = Fixture::Distiller;
        let rows = fixture.rows();
        let recipes = recipe_table(&fixture, &rows);
        let m = measure_idle(&recipes, 32);
        assert_eq!(m.ticks, 32);
        // Java: `if (hasInput) {...} else { stopProcessing(); }` — an empty
        // machine never reaches getRecipe().
        assert_eq!(m.counters.lookups, 0, "no input must not look a recipe up");
        assert_eq!(m.counters.total_time_lookups, 0);
        assert_eq!(m.counters.recipe_scans, 0);
        assert_eq!(m.counters.matcher_calls, 0, "no input must not run matchers");
        assert_eq!(m.counters.plan_calls, 0);
        assert_eq!(m.counters.commits, 0);
        assert_eq!(m.counters.block_updates, 0, "stopped machine must not spam block updates");
        assert_eq!(m.crafts, 0);
        assert_eq!(m.counters.work(), 0);
    }

    #[test]
    fn ticking_machine_looks_up_exactly_once_per_tick_plus_crafts() {
        let fixture = Fixture::Distiller; // beef count=2 -> mercury, time 100
        let rows = fixture.rows();
        let recipes = recipe_table(&fixture, &rows);
        let m = measure_tick(&fixture, &recipes, 300);
        // 300 ticks / 100 = exactly 3 crafts, no off-by-one.
        assert_eq!(m.crafts, 3);
        assert_eq!(m.counters.commits, 3, "one commit per craft, no retries");
        assert_lookup_identity(&m, rows.len(), 1); // 1 = initial onInputItemChanged
        assert_eq!(m.counters.lookups, 300, "ticks 300 == lookups");
        assert_eq!(m.counters.total_time_lookups, 4, "3 craft ticks + 1 input change");
    }

    #[test]
    fn crafting_ticks_are_linear_in_ticks_not_quadratic() {
        let fixture = Fixture::Distiller;
        let rows = fixture.rows();
        let recipes = recipe_table(&fixture, &rows);
        let a = measure_tick(&fixture, &recipes, 40);
        let b = measure_tick(&fixture, &recipes, 160);
        assert_eq!(a.ticks, 40);
        assert_eq!(b.ticks, 160);
        // 4x the ticks may not cost more than 4x the work (plus one tick of slack).
        assert!(b.counters.work() <= a.counters.work() * 4 + 8,
            "work grows faster than ticks: {} vs {}", a.counters.work(), b.counters.work());
        // At most one matcher call per proposed row per lookup, and at most one
        // row-set proposal per lookup at minimum.
        assert!(b.counters.matcher_calls <= b.counters.recipe_scans,
            "matcher calls {} exceed scans {}", b.counters.matcher_calls, b.counters.recipe_scans);
        assert!(b.counters.lookups <= b.ticks, "never more than one getRecipe per tick");
    }

    #[test]
    fn failure_ticks_do_no_commit_or_plan_work() {
        let kind = "theurgy:distillation";
        // Output slot already holds a different item: canCraft is false every tick.
        let rows = Fixture::Distiller.rows();
        let recipes = recipe_table(&Fixture::Distiller, &rows);
        let mut harness = TickHarness::new(
            kind,
            Some(Item { id: "minecraft:beef".into(), count: 64, max_stack: 64 }),
            Some(Item { id: "theurgy:sulfur".into(), count: 64, max_stack: 64 }),
            None,
        );
        harness.state.total_time = 100;
        let mut out = TickOutcome { result: None, committed: false, last_item_copy_len: 0 };
        let mut buffer = PlanBuffer::default();
        for _ in 0..200 {
            harness.tick(&recipes, &mut buffer, &mut out);
            assert!(!out.committed);
        }
        assert_eq!(harness.counters.commits, 0);
        assert_eq!(harness.counters.plan_calls, 0, "a blocked output must not build a plan");
        assert_eq!(harness.counters.total_time_lookups, 0, "no craft tick, no getTotalTime");
        assert_eq!(harness.state.progress, 0, "blocked output must not accumulate progress");
        // getRecipe still ran once per tick: the lookup is the only per-tick cost.
        assert_eq!(harness.counters.lookups, 200);
        // Output slot untouched and input never consumed.
        assert_eq!(harness.items[1].as_ref().unwrap().count, 64);
        assert_eq!(harness.items[0].as_ref().unwrap().count, 64);
        // stopProcessing only sends a block update when the state really changed.
        assert_eq!(harness.counters.block_updates, 0,
            "a machine that never started must not send block updates");
    }

    #[test]
    fn loss_of_heat_costs_one_block_update_then_nothing() {
        let fixture = Fixture::Distiller;
        let rows = fixture.rows();
        let recipes = recipe_table(&fixture, &rows);
        let mut harness = fixture.harness(fixture.kind());
        harness.state.total_time = 100;
        let mut out = TickOutcome { result: None, committed: false, last_item_copy_len: 0 };
        let mut buffer = PlanBuffer::default();
        for _ in 0..10 {
            harness.tick(&recipes, &mut buffer, &mut out);
        }
        // Started processing: one update for tryStartProcessing, none else.
        assert_eq!(harness.counters.block_updates, 1, "start of processing sends one update");
        assert_eq!(harness.state.progress, 10);
        harness.can_process = false;
        let before = harness.counters.block_updates;
        for _ in 0..10 {
            harness.tick(&recipes, &mut buffer, &mut out);
        }
        assert_eq!(harness.counters.block_updates - before, 1,
            "cooling down sends exactly one update, not one per tick");
        assert_eq!(harness.state.progress, 0);
        // Java keeps looking the recipe up while input is present, even when cold.
        assert_eq!(harness.counters.lookups, 20);
    }

    #[test]
    fn linear_scan_cost_is_bounded_by_table_size() {
        for n in [1u32, 8, 64] {
            let fixture = Fixture::LinearScan(n);
            let rows = fixture.rows();
            let recipes = recipe_table(&fixture, &rows);
            let m = measure_tick(&fixture, &recipes, 20);
            // getRecipe() is the only lookup per tick; when the table matches it
            // returns early after (n-1) misses + 1 hit for the first-lookup case,
            // but find_recipe always walks the whole table for accounting, so the
            // scan count is exactly lookups * n.
            assert_eq!(m.counters.recipe_scans, m.counters.all_lookups() * n as u64,
                "n={n}: scan cost must be lookups*rows");
            assert!(m.counters.matcher_calls <= m.counters.recipe_scans,
                "n={n}: {} matcher calls exceed {} scans", m.counters.matcher_calls, m.counters.recipe_scans);
            assert!(m.counters.matcher_calls >= m.counters.plan_calls,
                "n={n}: a selected recipe implies at least one matcher call");
            // Filler rows are single-slot id misses, so each lookup runs exactly
            // one matcher closure per non-empty input slot per row walked.
            assert_eq!(m.counters.matcher_calls, m.counters.all_lookups() * n as u64,
                "n={n}: one matcher closure per row per lookup");
        }
    }

    #[test]
    fn warm_plan_buffer_does_not_grow() {
        for (fixture, ticks) in [(Fixture::Distiller, 50u32), (Fixture::Vat, 50)] {
            let rows = fixture.rows();
            let recipes = recipe_table(&fixture, &rows);
            let (cold, warm) = plan_buffer_capacity(&fixture, &recipes, ticks);
            assert!(cold > 0, "{fixture:?}: first tick must populate the plan buffer");
            assert_eq!(cold, warm, "{fixture:?}: buffer grew from {cold} to {warm} over {ticks} ticks");
        }
    }

    #[test]
    fn measurements_are_deterministic_for_a_fixed_seed() {
        // The seed pins the fixture generator for forward-compatible randomized
        // fixtures without making this test flaky.
        let mut rng = Lcg::new(0x1_2063_1201);
        let drift = rng.below(1_000_000);
        let _ = drift;
        let fixture = Fixture::Distiller;
        let rows = fixture.rows();
        let recipes = recipe_table(&fixture, &rows);
        let a = measure_tick(&fixture, &recipes, 120);
        let b = measure_tick(&fixture, &recipes, 120);
        assert_eq!(a.checksum(), b.checksum(), "same input must give the same work/state fingerprint");
        assert_eq!(a.counters, b.counters);
    }

    #[test]
    fn per_tick_work_identity_holds_for_running_machine() {
        // 10 < time(100), so the machine is mid-craft for the whole run.
        let fixture = Fixture::Distiller;
        let rows = fixture.rows();
        let recipes = recipe_table(&fixture, &rows);
        let ticks = 10u64;
        let m = measure_tick(&fixture, &recipes, ticks as u32);
        assert_lookup_identity(&m, rows.len(), 1);
        assert_eq!(m.counters.lookups, ticks);
        assert_eq!(m.counters.plan_calls, 0, "no plan is built before a craft tick");
        assert_eq!(m.counters.commits, 0, "10 ticks cannot finish a 100 tick recipe");
        // One update when processing starts, then none until the craft tick.
        assert_eq!(m.counters.block_updates, 1);
        assert_eq!(m.counters.state_resets, 1, "onInputItemChanged runs once, up front");
    }

    #[test]
    fn first_craft_tick_has_the_exact_expected_shape() {
        // time 3 => craft on the 3rd tick. Checks the craft-tick accounting:
        // getTotalTime (lookup), one plan build, one commit, two block updates.
        let mut rows = Fixture::Distiller.rows();
        rows[0]["time"] = json!(3);
        let recipes = recipe_table(&Fixture::Distiller, &rows);
        let mut harness = Fixture::Distiller.harness("theurgy:distillation");
        harness.state.total_time = 3;
        let mut out = TickOutcome { result: None, committed: false, last_item_copy_len: 0 };
        let mut buffer = PlanBuffer::default();
        for _ in 0..3 {
            harness.tick(&recipes, &mut buffer, &mut out);
        }
        assert!(out.committed, "3rd tick must craft");
        assert_eq!(harness.counters.lookups, 3, "one getRecipe per tick");
        assert_eq!(harness.counters.total_time_lookups, 1, "one getTotalTime on the craft tick");
        assert_eq!(harness.counters.plan_calls, 1);
        assert_eq!(harness.counters.commits, 1);
        // start(1) + craft-sendBlockUpdated(1)
        assert_eq!(harness.counters.block_updates, 2);
        // CraftingBehaviour.craft extracts getIngredientCount() == 2 beef.
        assert_eq!(harness.items[0].as_ref().unwrap().count, 62, "one recipe consumes exactly 2 beef");
        assert_eq!(harness.items[1].as_ref().unwrap().id, "theurgy:mercury");
        assert_eq!(harness.items[1].as_ref().unwrap().count, 1);
    }

    #[test]
    fn vat_fixture_looks_up_once_per_tick() {
        let fixture = Fixture::Vat;
        let rows = fixture.rows();
        let recipes = recipe_table(&fixture, &rows);
        let m = measure_tick(&fixture, &recipes, 400);
        // time 200 => 2 crafts in 400 ticks (but input is 8 beef and consumes 1
        // per craft, so no starvation within 400 ticks).
        assert_lookup_identity(&m, rows.len(), 1);
        assert_eq!(m.counters.lookups, 400);
        assert!(m.crafts >= 1, "vat must craft at least once in 400 ticks");
    }

    #[test]
    fn liquefaction_needs_solvent_and_matches_identity() {
        let fixture = Fixture::Liquefaction;
        let rows = fixture.rows();
        let recipes = recipe_table(&fixture, &rows);
        let m = measure_tick(&fixture, &recipes, 120);
        assert_lookup_identity(&m, rows.len(), 1);
        assert_eq!(m.counters.lookups, 120);
        assert!(m.crafts >= 1, "liquefaction must craft with enough solvent");
    }

    #[test]
    fn matcher_closure_count_is_bounded_by_rows_parts_and_slots() {
        // The host matcher closure runs once per (recipe row, ingredient part,
        // non-empty input slot candidate) visited by find_recipe before it either
        // hits or exhausts the slot list. That product is the hard ceiling.
        for (fixture, ticks) in [(Fixture::Distiller, 50u32), (Fixture::Vat, 50), (Fixture::Liquefaction, 50)] {
            let rows = fixture.rows();
            let recipes = recipe_table(&fixture, &rows);
            let m = measure_tick(&fixture, &recipes, ticks);
            let slots = fixture.harness(fixture.kind()).items.iter().flatten().count() as u64;
            let parts = rows
                .iter()
                .map(|row| crate::recipes::ingredient_parts(fixture.kind(), row).len() as u64)
                .max()
                .unwrap_or(0);
            let bound = m.counters.all_lookups() * rows.len() as u64 * parts.max(1) * slots;
            assert!(m.counters.matcher_calls <= bound,
                "{fixture:?}: {} matcher calls exceed the {bound} bound",
                m.counters.matcher_calls);
            // Every row must actually be proposed at least once per lookup, so the
            // closure can never be skipped entirely (that would hide a lookup).
            assert!(m.counters.matcher_calls >= m.counters.all_lookups(),
                "{fixture:?}: {} matcher calls for {} lookups",
                m.counters.matcher_calls, m.counters.all_lookups());
        }
    }

    #[test]
    fn linear_scan_has_no_craft_within_its_short_report_window() {
        // The report row for LinearScan(64) is only 64 ticks while the row's time
        // is 100, so it measures pure lookup cost with zero craft ticks. Pin that
        // instead of leaving crafts=0 implicit.
        let fixture = Fixture::LinearScan(64);
        let rows = fixture.rows();
        let recipes = recipe_table(&fixture, &rows);
        let m = measure_tick(&fixture, &recipes, 64);
        assert_eq!(m.crafts, 0);
        assert_eq!(m.counters.total_time_lookups, 1, "only the up-front input change");
        assert_eq!(m.counters.plan_calls, 0);
        // And it does craft once the window exceeds the recipe time.
        let m2 = measure_tick(&fixture, &recipes, 300);
        assert_eq!(m2.crafts, 3);
        assert_eq!(m2.counters.total_time_lookups, 4);
    }

    /// Emits the report for tools/bench-port.mjs. It only prints; the assertions
    /// that matter live in the work-identity tests above.
    #[test]
    fn prints_measurement_report() {
        for line in report().lines().filter(|l| l.starts_with("fixture=")) {
            println!("{line}");
        }
    }

    #[test]
    fn report_lines_carry_every_field_the_javascript_parser_needs() {
        let text = report();
        assert!(text.lines().any(|l| l.starts_with("fixture=distiller ")));
        assert!(text.lines().any(|l| l.starts_with("fixture=idle ")));
        for line in text.lines().filter(|l| l.starts_with("fixture=")) {
            for key in [
                "ticks=", "scans=", "matcher_calls=", "plan_calls=", "block_updates=",
                "commits=", "crafts=", "lookups=", "total_time_lookups=", "checksum=",
                "item_slots=", "parts=", "rows=",
            ] {
                assert!(line.contains(key), "report line misses {key}: {line}");
            }
        }
    }

    #[test]
    fn fixture_rows_have_expected_sizes() {
        assert_eq!(Fixture::Distiller.rows().len(), 2);
        assert_eq!(Fixture::Liquefaction.rows().len(), 2);
        assert_eq!(Fixture::Vat.rows().len(), 1);
        assert_eq!(Fixture::LinearScan(9).rows().len(), 9);
    }
}
