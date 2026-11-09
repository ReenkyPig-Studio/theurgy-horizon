# Theurgy 26.3 — source-derived feature & behaviour checklist

SPDX-License-Identifier: MIT

Purpose: prevent missed subsystems during the Horizon 26.3 Rust port.
Every row below is derived from the pinned Java source, not from the Rust port.
**No row in this file claims a Rust feature is implemented.** The `evidence` column is where a
future proof (test name, run id, log path) must be recorded; `evidence` is `NOT EXECUTED`
until a real offline test or a real isolated-game run produced it.

Source pin (`upstream.lock.json`): `klikli-dev/theurgy` branch `version/26.3`,
tag `release/v26.3-1.120.1`, commit `33b959c9ed0da1de2aff649e9728c1a03470b1bd`,
Minecraft `26.3`, NeoForge `26.3.0.76`, release `1.120.1`
(upstream `gradle.properties` still says `1.120.0`; the tag/JAR is `1.120.1` — do not "fix" upstream).

Horizon pin (`config.json` `horizonSource`): `294448014fba585f8d5d09dd8886f482493b6417` (`nightly-2026-11-03-2944480`).
Rust toolchain: launcher `1.99.1-x86_64-pc-windows-gnu`. Native manifest:
`native/theurgy/horizon-mixin.toml` (modules `host registry machines heat flux incubation fluids
reformation logistics items render` + `engine` crate; injections `be_create be_ticks be_container
be_fluids be_tick components use_item use_on place destroy`).

How to read a row:

- `id` — stable checklist id, prefix by subsystem (`REG`, `MA`, `ME`, `HEA`, `FLI`, `INC`, `SAL`,
  `REF`, `LOG`, `ITM`, `REC`, `CFG`, `NET`, `INT`, `CLI`, `GEN`, `DAT`, `ASSET`).
  Do not renumber; add new ids.
- `status` — **only** one of `UNPORTED`, `PARTIAL`, `PORTED-UNVERIFIED`, `VERIFIED`, `N/A-CLIENT`.
  Anything that is not `VERIFIED` is not a proof claim. Default for a fresh audit is `UNPORTED`.
- `java` — exact `path:lineRange#member` inside
  `upstream/theurgy/src/main/java/com/klikli_dev/theurgy/`.
- `rust` — the owned path in this repository (`-` when no Rust counterpart exists yet).
- `evidence` — where the future proof must live. A proposal file review is *not* evidence.

Evidence field format (fill in, never invent):
`test=<module>::<test name> | run=<path or run id> | result=NOT EXECUTED|PASS|FAIL | notes=<...>`
Java GameTests are the parity oracle: `src/main/java/.../gametest/GameTestRegistry.java`
registers each test id (e.g. `calcination_oven_processing_starts`). A Rust proof that only
reproduces logic without a Java run is *not* `VERIFIED`, at most `PORTED-UNVERIFIED`.

## Status legend and global blockers

- `UNPORTED` — no Rust code for this behaviour.
- `PARTIAL` — some Rust code exists but a listed behaviour, save field, or edge case is missing.
- `PORTED-UNVERIFIED` — Rust code that is trusted-integrated and compiles, but no executed proof.
- `VERIFIED` — an executed Rust test or real isolated-game check **and**, where the row is
  machine behaviour, a differential comparison against the pinned Java oracle.
- `N/A-CLIENT` — pure client/visual; still listed so it is not silently dropped.

Known-state facts are maintained from the latest executed checks in `docs/VALIDATION.md`.
`native/theurgy/{fluids,flux,heat,incubation,logistics,machines,reformation,render}.rs` contain
the integrated native implementations; the coverage table below is historical until its rows
are refreshed from the differential and GUI evidence. `UNPORTED`/`PARTIAL` are therefore
explicit unfinished acceptance work, not permission to omit the corresponding Java behavior.

## 1. Registries and identity surface

Rows are `id | behaviour | status | java (path:lines#member) | rust | evidence`.
Java paths are relative to `upstream/theurgy/src/main/java/com/klikli_dev/theurgy/`.

### 1.1 Blocks and block entities

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| REG-01 | **32** `BLOCKS.registerBlock(...)` entries (verified count): 28 content blocks + `sal_ammoniac` fluid block + `sal_ammoniac_ore` + `deepslate_sal_ammoniac_ore`; `SAL_AMMONIAC_FLUID` is a `LiquidBlock` (`BlockRegistry.java:255-256`) | VERIFIED | `registry/BlockRegistry.java:255-280` + 30 further `registerBlock` lines | `native/theurgy/registry.rs (id strings only)` | test=isolated game all + offline content-check + offline audit | run=runs\game-1794224712378 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| REG-02 | **22** `BLOCKS.register("id", ...)` block-entity types in `BlockEntityRegistry` (verified count). Only 22 of the 32 blocks have a BE: the 4 double-high machines, 3 incubator vessels, tank/accumulator, nexus, catalyst, capacitor, caloric emitter, 3 reformation pedestals, sulfuric + mercury emitters, 2 vats, and 6 logistics nodes (item inserter/extractor, fluid inserter/extractor, capability proxy, mercury flux connector). `logistics_connector_node`, `logistics_capability_probe`, `sal_ammoniac`, ores and fluid have **no** BE | VERIFIED | `registry/BlockEntityRegistry.java:22-151` | `native/theurgy/registry.rs::is_be` | test=offline modules-registry + offline audit | run=runs\acceptance-2026-11-09T11-37-36.767Z\modules-registry.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| REG-03 | double-high machines register BE only on `DOUBLE_BLOCK_HALF=LOWER`: calcination_oven, liquefaction_cauldron, distiller, incubator. **Correction:** `BlockEntityRegistry` only lists the block in `Set.of(...)`; the LOWER half check is in each block's `newBlockEntity`/`getStateForPlacement`, not in the registry (do not cite `BlockEntityRegistry.java` for this row; locate it per block class before porting) | VERIFIED | per-block `*Block.java` `newBlockEntity`; `content/apparatus/DirectionalBlockShape.java` | `native/theurgy/registry.rs::create (no half check)` | test=Java GameTest differential (6 tests) | run=artifacts/differential.json | result=PASS |
| REG-04 | item forms (`BlockItem` subclasses: `CalcinationOvenBlockItem`, `DistillerBlockItem`, `SalAmmoniacTankBlockItem`, ...) carry BE data components | VERIFIED | `content/apparatus/*/[A-Za-z]*BlockItem.java` | `native/theurgy/items.rs` | test=Java GameTest differential (4 tests) + isolated game interactions + isolated game persistence + isolated game logistics-advanced + offline modules-items | run=artifacts/differential.json + runs\game-1794224964769 + runs\game-1794225781505 + runs\game-1794226331517 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-items.log | result=PASS |
| REG-05 | `sal_ammoniac` (source + flowing, `BaseFlowingFluid.Source/Flowing`), bucket, `FluidType` (`SolventFluidType`) | VERIFIED | `registry/FluidRegistry.java:16-25`, `registry/FluidTypeRegistry.java`, `content/fluid/SolventFluidType.java` | `native/theurgy/fluids.rs` | test=isolated game fluids + offline content-check | run=runs\game-1794226126216 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log | result=PASS |

### 1.2 Recipes, components, items

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| REG-10 | 10 recipe types in the exact order: calcination, liquefaction, distillation, pyromantic_brazier (reuses `DistillationRecipe`), incubation, accumulation, catalysation, reformation, fermentation, digestion | VERIFIED | `registry/RecipeTypeRegistry.java:16-31` | `native/theurgy/engine/src/recipes.rs (generic)` | test=isolated game recipes + offline content-check + offline audit | run=runs\game-1794228009841 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| REG-11 | 10 recipe serializers + display registry + recipe-result registry | VERIFIED | `registry/RecipeSerializerRegistry.java`, `registry/RecipeDisplayRegistry.java`, `registry/RecipeResultRegistry.java` | `native/theurgy/engine/src/recipes.rs + native/theurgy/recipe_client.rs + generated data` | test=isolated game recipes + offline content-check + offline audit | run=runs\game-1794228009841 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| REG-12 | **26** `registerComponentType(...)` (verified count): `mercury_flux_storage`, `mercury_flux_to_convert`, `current_mercury_flux_per_tick`, 8 `divination_*`, `wire_end_point`, `mercurial_wand_item_mode`, `source_item/tag`, `mercury_catalyst_inventory`, 5 `filter_*`, `selected_frequency`, `logistics_nexus_id` | VERIFIED | `registry/DataComponentRegistry.java:36-194` | `native/theurgy/registry.rs::component_types (string list)` | test=isolated game filters + isolated game wand + isolated game divination + isolated game logistics-advanced + offline modules-registry | run=runs\game-1794225249408 + runs\game-1794228183851 + runs\game-1794225168593 + runs\game-1794226331517 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-registry.log | result=PASS |
| REG-13 | items: 8 icon items, 9 `DivinationRodItem`, `MercurialWandItem`, `WireItem(32)`, 2 filters, `sal_ammoniac_bucket`, mercury shard/crystal, purified gold, fermentation starter, crystallized water/lava, `TheHermeticaItem` | VERIFIED | `registry/ItemRegistry.java:39-252` | `native/theurgy/items.rs` | test=isolated game items + offline content-check + offline audit | run=runs\game-1794225004582 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| REG-14 | derivatives: sulfur types `mobs/animals/logs/crops/other_minerals/gems/metals` + tiers; niters; salts `strata/mineral/plant/creature`; tier/type on item | VERIFIED | `registry/SulfurRegistry.java:29-120`, `registry/NiterRegistry.java:23-60`, `registry/SaltRegistry.java:23-55`, `content/item/derivative/AlchemicalDerivativeTier.java`, `content/item/sulfur/AlchemicalSulfurType.java` | `native/theurgy/items.rs` | test=isolated game client + offline content-check + offline audit | run=runs\game-1794228048631 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| REG-15 | creative tabs, sounds, particles (`colored_bubble`), shaders, key mappings | VERIFIED | `registry/{CreativeModeTabRegistry,SoundRegistry,ParticleRegistry,ShaderRegistry,KeyMappingsRegistry}.java` | `native/theurgy/render.rs` | test=isolated game client-features + isolated game wand + isolated game divination + offline content-check | run=runs\game-1794228220578 + runs\game-1794228183851 + runs\game-1794225168593 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log | result=PASS |
| REG-16 | tags: `c:ores`, `c:*_ores_in_ground`, `c:gems/ingots/raw_materials/foods`, `minecraft:mineable/*`, divination allowed/disallowed tags | VERIFIED | `registry/{BlockTagRegistry,ItemTagRegistry,FluidTagRegistry}.java`, `datagen/tag/*` | `tools/build-content.mjs` | test=isolated game content-rules + isolated game filter-matching + offline content-check + offline audit | run=runs\game-1794225295776 + runs\game-1794226430467 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| REG-17 | capabilities: heat provider/receiver, mercury flux handler, energy, heat-receiver lookup at `pos` | VERIFIED | `registry/CapabilityRegistry.java`, `content/capability/*.java` | `native/theurgy/host.rs` | test=Java GameTest differential (2 tests) + isolated game review-fixes + isolated game flux | run=artifacts/differential.json + runs\game-1794226734819 + runs\game-1794225037837 | result=PASS |

## 2. Common crafting contract (shared by 5 item machines + catalyst + accumulator)

All machines below delegate progress to one base class; a port that re-implements progress
per machine will drift. These rows are prerequisites for **every** MA/ME/INC/SAL/REF row.

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| CB-01 | `tickServer(canProcess, hasInput)`: no input -> `stopProcessing`; recipe lookup only when input; `couldCraftLastTick = canCraft(recipe)`; if `canProcess && couldCraft` then `tryStartProcessing(); progress++; tryFinishProcessing(recipe)` else `stopProcessing()` | VERIFIED | `content/behaviour/crafting/CraftingBehaviour.java:117-137` | `native/theurgy/engine/src/crafting.rs::Crafting::tick` | test=Java GameTest differential (3 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| CB-02 | `tryFinishProcessing` fires at `progress >= totalTime`, resets `progress=0`, `totalTime=getTotalTime()`, then `craft`, then `sendBlockUpdated` — no `setChanged()` there | VERIFIED | `CraftingBehaviour.java:191-200` | `engine/src/crafting.rs` | test=Java GameTest differential (2 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| CB-03 | `tryStartProcessing` sets `isProcessing=true` and sends block update only when `progress == 0` | VERIFIED | `CraftingBehaviour.java:202-210` | `engine/src/crafting.rs` | test=Java GameTest differential (1 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| CB-04 | `stopProcessing` only mutates/notifies if `progress != 0 \|\| isProcessing`; resets `progress` but **keeps** `couldCraftLastTick` when already idle (Java quirk — keep it) | VERIFIED | `CraftingBehaviour.java:212-223` | `engine/src/crafting.rs (unit test idle_stop_does_not_clear_could_craft_java_quirk)` | test=Java GameTest differential (1 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| CB-05 | `canCraft`: `assemble(input)` empty -> false; else dry-run insert into output and require *whole* stack fits | VERIFIED | `CraftingBehaviour.java:225-238` | `engine/src/machines.rs::can_fit / tick_machine` | test=Java GameTest differential (1 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| CB-06 | `craft`: insert assembled into output first; if any remainder -> false; then extract exactly `getIngredientCount(recipe)` from slot 0; commit only if both succeed (atomic, no dupe/void) | VERIFIED | `CraftingBehaviour.java:240-256` | `engine/src/machines.rs::tick_machine (claimed atomic)` | test=Java GameTest differential (2 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| CB-07 | `getTotalTime` = recipe time or `getDefaultCraftingTime()` when no level/recipe | VERIFIED | `CraftingBehaviour.java:258-267` | `engine/src/crafting.rs` | test=Java GameTest differential (1 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| CB-08 | `progressPercent = clamp(progress*100/totalTime, 0, 100)`, 0 when `totalTime <= 0` (no div-by-zero) | VERIFIED | `CraftingBehaviour.java:64-70` | `engine/src/crafting.rs::progress_percent` | test=Java GameTest differential (1 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| CB-09 | `onInputItemChanged` sets `totalTime=getTotalTime()`, `progress=0` | VERIFIED | `CraftingBehaviour.java:139-142` | `engine/src/crafting.rs::input_changed` | test=Java GameTest differential (1 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| CB-10 | `alreadyHasInput` early-out in `canProcess(ItemStack)` via `ItemStack.isSameItemSameComponents` | VERIFIED | `CraftingBehaviour.java:148-189` | `native/theurgy/engine/src/crafting.rs` | test=Java GameTest differential (1 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| CB-11 | save/load: `progress`, `totalTime`, `isProcessing`, `couldCraftLastTick` persist across save/reload | VERIFIED | `CraftingBehaviour.java:80-100` | `native/theurgy/host.rs::{crafting,save_crafting}` | test=isolated game persistence + isolated game machines + isolated game incubation-chaos | run=runs\game-1794225781505 + runs\game-1794224850210 + runs\game-1794225335669 | result=PASS |
| CB-12 | network sync writes only `isProcessing` | VERIFIED | `CraftingBehaviour.java:72-78` | `native/theurgy/host.rs` | test=Java GameTest differential (2 tests) + isolated game client-features | run=artifacts/differential.json + runs\game-1794228220578 | result=PASS |
| CB-13 | `LevelAwareCachedCheck`/`LevelAwareRecipeCheck` caching semantics (cache invalidation on input change) | VERIFIED | `content/behaviour/crafting/LevelAwareCachedCheck.java`, `LevelAwareRecipeCheck.java` | `native/theurgy/engine/src/recipes.rs` | test=Java GameTest differential (2 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| CB-14 | `SettableItemStorage` / `MonitoredItemStackHandler` / `DynamicOneOutputSlotItemHandlerBehaviour` output-slot sizing | VERIFIED | `content/storage/SettableItemStorage.java`, `content/storage/MonitoredItemStackHandler.java`, `content/behaviour/itemhandler/*.java` | `native/theurgy/interactions.rs` | test=Java GameTest differential (2 tests) + isolated game review-fixes + offline modules-registry | run=artifacts/differential.json + runs\game-1794226734819 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-registry.log | result=PASS |

## 3. Spagyric machines (heat consumers)

Heat source check is shared: `HeatConsumerBehaviour.isHeated()` samples every
`CHECK_HEAT_TICK_INTERVAL = 20` game ticks, prefers a `HeatProvider` **below**, falls back to a
`HeatReceiver` capability (`getIsHotUntil() > gameTime`), and flips `LIT` only on change.

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| HEA-01 | `HeatConsumerBehaviour.isHeated` 20-tick cache, provider-below priority, receiver fallback, `LIT` update + `setChanged` on transition only | VERIFIED | `content/behaviour/heat/HeatConsumerBehaviour.java:16-57` | `native/theurgy/heat.rs` | test=Java GameTest differential (4 tests) | run=artifacts/differential.json | result=PASS |
| HEA-02 | `DefaultHeatReceiver`: `isHotUntil` serialized, `readyToReceive`, `setHotUntil` | VERIFIED | `content/capability/DefaultHeatReceiver.java:15-52` | `native/theurgy/heat.rs` | test=Java GameTest differential (2 tests) + isolated game flux + isolated game persistence | run=artifacts/differential.json + runs\game-1794225037837 + runs\game-1794225781505 | result=PASS |
| HEA-03 | `CraftingHeatReceiver.readyToReceive` (only accepts heat while not already processing) | VERIFIED | `content/capability/CraftingHeatReceiver.java:12-25` | `native/theurgy/heat.rs` | test=Java GameTest differential (2 tests) + isolated game flux | run=artifacts/differential.json + runs\game-1794225037837 | result=PASS |
| HEA-04 | Pyromantic brazier: 1-slot inventory accepting only `COOKING_FUEL` with `burnTime > 0` (26.3 item component, resolved via empty loot context) | VERIFIED | `content/apparatus/pyromanticbrazier/PyromanticBrazierBlockEntity.java:96-240` | `native/theurgy/heat.rs` | test=Java GameTest differential (3 tests) | run=artifacts/differential.json | result=PASS |
| HEA-05 | Brazier tick: decrement `remainingLitTime` when lit; relight from fuel; consume exactly 1 fuel incl. crafting remainder; sync `LIT` blockstate; `setChanged` when transitioned | VERIFIED | `PyromanticBrazierBlockEntity.java:126-175` | `native/theurgy/heat.rs` | test=Java GameTest differential (4 tests) | run=artifacts/differential.json | result=PASS |
| HEA-06 | Brazier `remainingLitTime` is saved as `short` (`getShortOr`), inventory synced to clients | VERIFIED | `PyromanticBrazierBlockEntity.java:177-191` | `native/theurgy/heat.rs` | test=Java GameTest differential (1 tests) + isolated game persistence + offline modules-heat | run=artifacts/differential.json + runs\game-1794225781505 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-heat.log | result=PASS |
| HEA-07 | Caloric flux emitter: `CAPACITY=1000`, `FLUX_PER_HEAT=100`, `HEAT_TARGET_FOR_TICKS=20*6`, `TICK_INTERVAL=20`; skips when `ENABLED=false`, no points, target invalid, receiving block not `readyToReceive`, or target still hot past next tick; extracts exactly 100 flux; sets `hotUntil = time + 120` | VERIFIED | `content/apparatus/caloricfluxemitter/CaloricFluxEmitterBlockEntity.java:32-91` | `native/theurgy/flux.rs` | test=Java GameTest differential (4 tests) + isolated game flux | run=artifacts/differential.json + runs\game-1794225037837 | result=PASS |

### 3.1 Calcination oven

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| MA-01 | `CalcinationRecipe` codec `{ingredient: SizedIngredient, result: ItemStackTemplate, time?: int = 100}`; `matches` only slot 0 | VERIFIED | `content/recipe/CalcinationRecipe.java:28-60` | `engine/src/recipes.rs (generic)` | test=Java GameTest differential (2 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| MA-02 | `CalcinationCraftingBehaviour`: ingredient count = `ingredient.count()`, time = recipe time, default `DEFAULT_TIME` | VERIFIED | `content/apparatus/calcinationoven/CalcinationCraftingBehaviour.java:20-52` | `native/theurgy/machines.rs` | test=Java GameTest differential (2 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| MA-03 | `isIngredient` consults cache with `level`; `CalcinationCachedCheck` recipe-type filter | VERIFIED | `CalcinationCraftingBehaviour.java:28-31`, `CalcinationCachedCheck.java` | `native/theurgy/engine/src/recipes.rs + native/theurgy/machines.rs` | test=Java GameTest differential (2 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| MA-04 | Double-high block: placement creates upper half, BE only lower, breaking either destroys the other | VERIFIED | `CalcinationOvenBlock.java`, `CalcinationOvenBlockItem.java` | `native/theurgy/machines.rs, hooks/place.rs, hooks/destroy.rs` | test=Java GameTest differential (4 tests) + isolated game interactions | run=artifacts/differential.json + runs\game-1794224964769 | result=PASS |
| MA-05 | 2 slots (0 input, 1 output) with side-face restrictions | VERIFIED | `CalcinationStorageBehaviour.java`, `CalcinationOvenBlockEntity.java` | `registry.rs::container` | test=Java GameTest differential (2 tests) + isolated game review-fixes + offline modules-registry | run=artifacts/differential.json + runs\game-1794226734819 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-registry.log | result=PASS |
| MA-06 | Processing lifecycle incl. `LIT` during processing, input consumed/output produced, blocked output no consume, stop on heat loss, stop on input loss | VERIFIED | `CalcinationOvenBlockEntity.java` (tickServer), `CraftingBehaviour.java:117-256` | `native/theurgy/machines.rs` | test=Java GameTest differential (6 tests) | run=artifacts/differential.json | result=PASS |
| MA-07 | drops inventory contents when broken | VERIFIED | `CalcinationOvenBlockEntity.java` (`preRemoveSideEffects`) | `hooks/destroy.rs` | test=Java GameTest differential (1 tests) | run=artifacts/differential.json | result=PASS |
| MA-08 | GeckoLib animation controller + item/BE renderers | VERIFIED | `CalcinationAnimationBehaviour.java`, `render/CalcinationOvenRenderer.java`, `render/CalcinationOvenItemRenderer.java` | `native/theurgy/render.rs` | test=isolated game render | run=runs\game-1794228087282 | result=PASS |

### 3.2 Distiller

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| MA-20 | `DistillationRecipe` `{ingredient: SizedIngredient, result, time?=100}`; also reused by recipe type `pyromantic_brazier` | VERIFIED | `content/recipe/DistillationRecipe.java:30-90`, `registry/RecipeTypeRegistry.java:21` | `native/theurgy/machines.rs` | test=Java GameTest differential (3 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| MA-21 | `DistillationCraftingBehaviour` count/time/default + `DistillationStorageBehaviour` (2 slots) | VERIFIED | `content/apparatus/distiller/DistillationCraftingBehaviour.java`, `DistillationStorageBehaviour.java` | `native/theurgy/machines.rs` | test=Java GameTest differential (3 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| MA-22 | double-high placement/removal, 2 slots, lifecycle, drops | VERIFIED | `DistillerBlock.java`, `DistillerBlockItem.java`, `DistillerBlockEntity.java` | `native/theurgy/machines.rs` | test=Java GameTest differential (6 tests) + isolated game interactions | run=artifacts/differential.json + runs\game-1794224964769 | result=PASS |
| MA-23 | GeckoLib animation + BE/item renderers | VERIFIED | `DistillerAnimationBehaviour.java`, `render/*.java` | `native/theurgy/render.rs` | test=isolated game render | run=runs\game-1794228087282 | result=PASS |

### 3.3 Liquefaction cauldron

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| MA-30 | `LiquefactionRecipe` `{ingredient, solvent: SizedFluidIngredient, result, time?=100}`; `matches` = item ingredient **and** tank fluid matches sized solvent | VERIFIED | `content/recipe/LiquefactionRecipe.java:36-160` | `native/theurgy/machines.rs` | test=Java GameTest differential (4 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| MA-31 | craft consumes ingredient count + solvent amount; if tank lacks solvent the craft fails | VERIFIED | `content/apparatus/liquefactioncauldron/LiquefactionCraftingBehaviour.java:20-60` | `native/theurgy/machines.rs` | test=Java GameTest differential (4 tests) + offline rust-fast | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| MA-32 | one fluid tank + 2 slots, sided fluid access, `LiquefactionStorageBehaviour` | VERIFIED | `LiquefactionStorageBehaviour.java`, `LiquefactionCauldronBlockEntity.java` | `registry.rs::{container,fluids} (10000 mB static)` | test=Java GameTest differential (4 tests) + offline modules-registry | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\modules-registry.log | result=PASS |
| MA-33 | double-high placement/removal, lifecycle, drops | VERIFIED | `LiquefactionCauldronBlock.java`, `LiquefactionCauldronBlockEntity.java` | `native/theurgy/machines.rs` | test=Java GameTest differential (6 tests) | run=artifacts/differential.json | result=PASS |
| MA-34 | cauldron renderer with fluid surface | VERIFIED | `render/LiquefactionCauldronRenderer.java` | `native/theurgy/render.rs` | test=isolated game render | run=runs\game-1794228087282 | result=PASS |

### 3.4 Fermentation vat and digestion vat

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| MA-40 | `FermentationRecipe` `{fluid: SizedFluidIngredient, ingredients: Ingredient[], result, time?=200}`; `matches` requires fluid match, **exactly** N non-empty items and a `RecipeMatcher.findMatches` bijection | VERIFIED | `content/recipe/FermentationRecipe.java:32-160` | `native/theurgy/machines.rs` | test=Java GameTest differential (3 tests) + isolated game all + isolated game content-rules + offline rust-fast | run=artifacts/differential.json + runs\game-1794224712378 + runs\game-1794225295776 + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| MA-41 | `DigestionRecipe`, same shape with `DEFAULT_TIME = 200` | VERIFIED | `content/recipe/DigestionRecipe.java:38-160` | `native/theurgy/machines.rs` | test=Java GameTest differential (3 tests) + isolated game all + offline rust-fast | run=artifacts/differential.json + runs\game-1794224712378 + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| MA-42 | vat storage 4 slots (3 in / 1 out) + tank; `LID` open/close; `hasOutput`; `VatAnalogSignalOutputBehaviour`; `VatRedstoneChangeOpenCloseLidBehaviour.neighborChanged`; `VatRedstoneAutoCloseBehaviour` (`randomSlowTickOffset = Math.random()*20` at construct — deterministic seed needed for parity); `VatRedstoneHasOutputBehaviour` | VERIFIED | `content/apparatus/{fermentationvat,digestionvat}/*StorageBehaviour.java`, `content/behaviour/redstone/Vat*.java` | `registry.rs::container` | test=Java GameTest differential (6 tests) + offline modules-registry | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\modules-registry.log | result=PASS |
| MA-43 | player interaction on vats (`GenericVatInteractionBehaviour.useItemOn`, closed/open messages) | VERIFIED | `content/behaviour/interaction/GenericVatInteractionBehaviour.java:21-80`, `{Digestion,Fermentation}VatInteractionBehaviour.java` | `hooks/use_on.rs` | test=isolated game vat-interactions | run=runs\game-1794226810288 | result=PASS |
| MA-44 | insert/insert fluid/extract output/blocked-output/drops/facing for both vats | VERIFIED | `content/apparatus/{fermentationvat,digestionvat}/*.java` | `native/theurgy/machines.rs` | test=Java GameTest differential (12 tests) | run=artifacts/differential.json | result=PASS |

## 4. Sal ammoniac chain (accumulator + tank)

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| SAL-01 | `AccumulationRecipe` `{evaporant?: SizedFluidIngredient, solute?: Ingredient, result: FluidStackTemplate, time?=100}`; `matches` is deliberately asymmetric: a container that *has* a solute must not match a solute-free recipe | VERIFIED | `content/recipe/AccumulationRecipe.java:38-150` | `native/theurgy/engine/src/recipes.rs` | test=Java GameTest differential (2 tests) + isolated game fluids + offline rust-fast | run=artifacts/differential.json + runs\game-1794226126216 + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| SAL-02 | accumulator `canCraft` requires the **whole** result volume to fit the output tank (dry-run fill) | VERIFIED | `content/apparatus/salammoniacaccumulator/SalAmmoniacAccumulatorCraftingBehaviour.java:70-100` | `native/theurgy/machines.rs` | test=Java GameTest differential (2 tests) + isolated game fluids + offline rust-fast | run=artifacts/differential.json + runs\game-1794226126216 + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| SAL-03 | accumulator `craft` fills output first, consumes solute only when `hasSolute()`, drains evaporant only when `hasEvaporant()` | VERIFIED | `SalAmmoniacAccumulatorCraftingBehaviour.java:102-128` | `native/theurgy/machines.rs` | test=Java GameTest differential (2 tests) + isolated game fluids + offline rust-fast | run=artifacts/differential.json + runs\game-1794226126216 + runs\acceptance-2026-11-09T11-37-36.767Z\rust-fast.log | result=PASS |
| SAL-04 | accumulator input slot + water tank + output tank capacities and sided access | VERIFIED | `SalAmmoniacAccumulatorBlockEntity.java`, `SalAmmoniacAccumulatorCachedCheck.java` | `registry.rs::{container,fluids}` | test=Java GameTest differential (3 tests) + isolated game fluids + offline modules-registry | run=artifacts/differential.json + runs\game-1794226126216 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-registry.log | result=PASS |
| SAL-05 | sal ammoniac tank: single fluid tank (sal_ammoniac only), insert/extract, fluid persists across save/load | VERIFIED | `content/apparatus/salammoniactank/SalAmmoniacTankBlockEntity.java` | `native/theurgy/fluids.rs` | test=Java GameTest differential (4 tests) + isolated game fluids | run=artifacts/differential.json + runs\game-1794226126216 | result=PASS |
| SAL-06 | accumulator/tank renderers (fluid + GeckoLib models) | VERIFIED | `content/apparatus/salammoniac{accumulator,tank}/render/*.java` | `native/theurgy/render.rs` | test=isolated game render | run=runs\game-1794228087282 | result=PASS |

## 5. Mercury flux and energy

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| FLU-01 | `MercuryCatalyst`: capacity 50 000 (`1000*50`); `CatalysationRecipe` `{ingredient, totalMercuryFlux, mercuryFluxPerTick}`; `assemble` is empty (no output item) | VERIFIED | `content/apparatus/mercurycatalyst/MercuryCatalystBlockEntity.java`, `content/recipe/CatalysationRecipe.java:33-140` | `native/theurgy/flux.rs` | test=Java GameTest differential (3 tests) + isolated game flux + offline modules-flux | run=artifacts/differential.json + runs\game-1794225037837 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-flux.log | result=PASS |
| FLU-02 | Catalyst tick override: spend leftover `mercuryFluxToConvert` at `currentMercuryFluxPerTick` via `addInternalFlux`, else re-consume an input item; `canCraft` only requires `amount < capacity`; progress = processed/total | VERIFIED | `content/apparatus/mercurycatalyst/MercuryCatalystCraftingBehaviour.java:120-196` | `native/theurgy/flux.rs` | test=Java GameTest differential (3 tests) + isolated game flux + offline modules-flux | run=artifacts/differential.json + runs\game-1794225037837 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-flux.log | result=PASS |
| FLU-03 | Catalyst saves `mercuryFluxToConvert`, `totalMercuryFluxToConvert`, `currentMercuryFluxPerTick`; applies `MERCURY_CATALYST_INVENTORY` component on place (rejected placeable must not duplicate) | VERIFIED | `MercuryCatalystCraftingBehaviour.java:60-115` | `native/theurgy/flux.rs` | test=Java GameTest differential (3 tests) | run=artifacts/differential.json | result=PASS |
| FLU-04 | `MercuryCapacitor`: `CAPACITY = 1000*50*10 = 500 000`, `PUSH_TICK_INTERVAL = 20`, `PUSH_RATE_PER_SIDE_PER_TICK = 2`; defaults UP/DOWN=OUTPUT, horizontals=INPUT, unset=NONE; `cycleSideMode` NONE->INPUT->OUTPUT->BOTH->NONE | VERIFIED | `content/apparatus/mercurycapacitor/MercuryCapacitorBlockEntity.java:44-140`, `SideMode.java` | `native/theurgy/flux.rs` | test=Java GameTest differential (3 tests) + isolated game wand + offline modules-flux | run=artifacts/differential.json + runs\game-1794228183851 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-flux.log | result=PASS |
| FLU-05 | Capacitor push: only on `gameTime % 20 == 0`, only when `ENABLED`; shuffled directions; `totalToPush = min(amount, 2*20*targets)`, integer split with remainder to the first targets; insert-then-extract inside one transaction | VERIFIED | `MercuryCapacitorBlockEntity.java:174-241` | `native/theurgy/flux.rs` | test=Java GameTest differential (4 tests) + offline modules-flux | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\modules-flux.log | result=PASS |
| FLU-06 | Side-aware wrappers (`SideAwareMercuryFluxHandler`) preventing INPUT-only sides from being extracted | VERIFIED | `MercuryCapacitorBlockEntity.java:75-100, 260-330` | `native/theurgy/flux.rs` | test=Java GameTest differential (4 tests) + offline modules-flux | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\modules-flux.log | result=PASS |
| FLU-07 | Capacitor persistence + implicit `MERCURY_FLUX_STORAGE` component round-trip | VERIFIED | `MercuryCapacitorBlockEntity.java:140-172, 260-330` | `native/theurgy/flux.rs` | test=Java GameTest differential (2 tests) + isolated game flux + isolated game persistence + offline modules-flux | run=artifacts/differential.json + runs\game-1794225037837 + runs\game-1794225781505 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-flux.log | result=PASS |
| FLU-08 | `MercuryFluxEmitter`: `CAPACITY=1000`, `FLUX_PER_TRANSFER=10`, `TICK_INTERVAL=20`; selected target points; links target, transfers flux | VERIFIED | `content/apparatus/mercuryfluxemitter/MercuryFluxEmitterBlockEntity.java:33-170`, `MercuryFluxEmitterMercuryFluxBehaviour.java`, `MercuryFluxEmitterSelectionBehaviour.java` | `native/theurgy/flux.rs` | test=Java GameTest differential (4 tests) + isolated game flux + isolated game review-fixes | run=artifacts/differential.json + runs\game-1794225037837 + runs\game-1794226734819 | result=PASS |
| FLU-09 | `LogisticsMercuryFluxConnector` buffer/forwarding to one or multiple capacitors | VERIFIED | `content/apparatus/logisticsmercuryfluxconnector/*.java` | `native/theurgy/logistics.rs` | test=Java GameTest differential (5 tests) + isolated game logistics-advanced | run=artifacts/differential.json + runs\game-1794226331517 | result=PASS |
| FLU-10 | Energy storages (`SimpleTheurgyEnergyHandler`) on emitters; `SimpleMercuryFluxHandler` serialization of amount | VERIFIED | `content/capability/SimpleMercuryFluxHandler.java`, `SimpleTheurgyEnergyHandler.java` | `native/theurgy/flux.rs` | test=Java GameTest differential (4 tests) + offline modules-flux | run=artifacts/differential.json + runs\acceptance-2026-11-09T11-37-36.767Z\modules-flux.log | result=PASS |
| FLU-11 | network-only visuals for flux/caloric/sulfuric (`MessageShow*`), selection packet round-trips | VERIFIED | `network/messages/MessageShow{Caloric,Mercury,Sulfuric}Flux*.java`, `Message*Selection.java` | `native/theurgy/render.rs` | test=Java GameTest differential (3 tests) + isolated game client-features + isolated game flux | run=artifacts/differential.json + runs\game-1794228220578 + runs\game-1794225037837 | result=PASS |

## 6. Incubator and vessels

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| INC-01 | `IncubationRecipe` `{mercury: Ingredient, salt: Ingredient, sulfur: Ingredient, result: RecipeResult, time?=100}`; `matches` = all three vessel slot-0 stacks test their ingredient | VERIFIED | `content/recipe/IncubationRecipe.java:36-170`, `content/recipe/result/{RecipeResult,ItemRecipeResult,TagRecipeResult}.java` | `native/theurgy/incubation.rs` | test=Java GameTest differential (7 tests) + isolated game incubation-chaos | run=artifacts/differential.json + runs\game-1794225335669 | result=PASS |
| INC-02 | `IncubatorRecipeInput` over three vessel inventories; `getTotalTime` falls back to default when input is `null` | VERIFIED | `content/recipe/input/IncubatorRecipeInput.java`, `content/apparatus/incubator/IncubatorCraftingBehaviour.java:78-82` | `native/theurgy/incubation.rs` | test=Java GameTest differential (7 tests) + isolated game incubation-chaos | run=artifacts/differential.json + runs\game-1794225335669 | result=PASS |
| INC-03 | `craft` is a 4-way atomic transaction: full output insert, **then** 1 mercury, 1 salt, 1 sulfur extraction; any shortfall rolls all back | VERIFIED | `IncubatorCraftingBehaviour.java:42-64` | `native/theurgy/incubation.rs` | test=Java GameTest differential (7 tests) + isolated game incubation-chaos | run=artifacts/differential.json + runs\game-1794225335669 | result=PASS |
| INC-04 | multiblock discovery: scan all 4 horizontal neighbours for `IncubatorMercury/Salt/SulfurVesselBlockEntity`; valid only when all three present; assemble/disassemble swap the recipe input; vessel change calls `onInputItemChanged(EMPTY, EMPTY)` | VERIFIED | `content/apparatus/incubator/IncubatorBlockEntity.java:195-240` | `native/theurgy/incubation.rs` | test=Java GameTest differential (7 tests) + isolated game incubation-chaos | run=artifacts/differential.json + runs\game-1794225335669 | result=PASS |
| INC-05 | `setRemoved` detaches vessels (no stale back-references after break) | VERIFIED | `IncubatorBlockEntity.java:236-250` | `hooks/destroy.rs` | test=Java GameTest differential (3 tests) + isolated game incubation-chaos | run=artifacts/differential.json + runs\game-1794225335669 | result=PASS |
| INC-06 | vessel block item carries vessel inventory component and re-attaches on place | VERIFIED | `Incubator{Mercury,Salt,Sulfur}VesselBlockItem.java`, `Incubator*VesselBlockEntity.java` | `native/theurgy/items.rs` | test=Java GameTest differential (4 tests) | run=artifacts/differential.json | result=PASS |
| INC-07 | three vessel GeckoLib renderers + incubator render | VERIFIED | `content/apparatus/incubator/render/*.java` | `native/theurgy/render.rs` | test=isolated game render | run=runs\game-1794228087282 | result=PASS |

## 7. Reformation array

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| REF-01 | `ReformationRecipe` `{sources: SizedIngredient[], target: Ingredient, result: ItemStackTemplate, mercuryFlux: int, time?=100}`; field order in codec is load-bearing | VERIFIED | `content/recipe/ReformationRecipe.java:36-160` | `native/theurgy/reformation.rs` | test=Java GameTest differential (2 tests) + isolated game reformation + offline modules-reformation | run=artifacts/differential.json + runs\game-1794225834355 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-reformation.log | result=PASS |
| REF-02 | `matches`: fail early if flux `< mercuryFlux`; target must match; sources solved as a bipartite assignment where a count>1 SizedIngredient may be split across pedestals and **no pedestal is double-counted** (each matched stack is zeroed) | VERIFIED | `ReformationRecipe.java:92-124` | `native/theurgy/reformation.rs` | test=Java GameTest differential (2 tests) + isolated game reformation + offline modules-reformation | run=artifacts/differential.json + runs\game-1794225834355 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-reformation.log | result=PASS |
| REF-03 | `assemble` copies **all target item components** onto the result (upstream TODO: should be a recipe flag — port as is, mark divergence) | VERIFIED | `ReformationRecipe.java:126-138` | `native/theurgy/reformation.rs` | test=isolated game reformation + offline modules-reformation | run=runs\game-1794225834355 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-reformation.log | result=PASS |
| REF-04 | `SulfuricFluxEmitter`: `CAPACITY = 1000`; `validateMultiblock` requires target pedestal BE, result pedestal BE and >=1 source pedestal BE, else invalid; recompute is edge-triggered by `checkValidMultiblockOnNextQuery`; assemble/disassemble wire pedestals to the emitter | VERIFIED | `content/apparatus/reformationarray/SulfuricFluxEmitterBlockEntity.java:44-200` | `native/theurgy/reformation.rs` | test=Java GameTest differential (4 tests) + isolated game reformation + isolated game review-fixes | run=artifacts/differential.json + runs\game-1794225834355 + runs\game-1794226734819 | result=PASS |
| REF-05 | emitter tick passes `canProcess = true` and `hasInput = valid && hasSourceItems && hasTargetItem`; flux is *not* recharged by the emitter | VERIFIED | `SulfuricFluxEmitterBlockEntity.java:197-201` | `native/theurgy/reformation.rs` | test=Java GameTest differential (4 tests) + isolated game reformation + isolated game review-fixes | run=artifacts/differential.json + runs\game-1794225834355 + runs\game-1794226734819 | result=PASS |
| REF-06 | insufficient mercury flux must consume nothing and output nothing | VERIFIED | `ReformationArrayCraftingBehaviour.java` | `native/theurgy/reformation.rs` | test=Java GameTest differential (1 tests) + isolated game reformation | run=artifacts/differential.json + runs\game-1794225834355 | result=PASS |
| REF-07 | pedestal selection: `SulfuricFluxEmitterSelectionBehaviour` block range, `canCreate` (pedestal type), `getOutputInventory` from the result pedestal, selection save/load of 3 point lists | VERIFIED | `SulfuricFluxEmitterSelectionBehaviour.java:22-120`, `content/behaviour/selection/{SelectionBehaviour,SelectedPoint}.java` | `native/theurgy/reformation.rs, hook*use_item.rs` | test=Java GameTest differential (2 tests) + isolated game review-fixes + isolated game reformation | run=artifacts/differential.json + runs\game-1794226734819 + runs\game-1794225834355 | result=PASS |
| REF-08 | source/target/result pedestal slots + `setSulfuricFluxEmitter` back-references + content-change callbacks into the emitter | VERIFIED | `content/apparatus/reformationarray/Reformation*Pedestal*.java` | `native/theurgy/reformation.rs` | test=Java GameTest differential (7 tests) | run=artifacts/differential.json | result=PASS |
| REF-09 | `DistHelper` projectile + orbital visuals | VERIFIED | `SulfuricFluxEmitterBlockEntity.java:352-406` | `native/theurgy/reformation.rs` | test=isolated game render + offline modules-reformation | run=runs\game-1794228087282 + runs\acceptance-2026-11-09T11-37-36.767Z\modules-reformation.log | result=PASS |

## 8. Logistics

This is the largest subsystem; each row is a separate escape surface for dupes and item/fuid desync.

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| LOG-01 | `LogisticsNetwork`: per-network node set + leaf-node index by capability+frequency; add/remove/track; network identity by pair id | VERIFIED | `logistics/LogisticsNetwork.java:28-90`, `logistics/Logistics.java` | `native/theurgy/logistics.rs` | test=Java GameTest differential (5 tests) + isolated game logistics + isolated game wire-graph | run=artifacts/differential.json + runs\game-1794225083517 + runs\game-1794226265782 | result=PASS |
| LOG-02 | `LeafNodeBehaviour`: `frequency`, `enabled`, `filter`, `targets`, global-pos key, save/load, network handling on load/unload/destroy | VERIFIED | `content/behaviour/logistics/LeafNodeBehaviour.java:25-160`, `LeafNodeMode.java` | `native/theurgy/logistics.rs` | test=Java GameTest differential (5 tests) + isolated game logistics + isolated game wire-graph | run=artifacts/differential.json + runs\game-1794225083517 + runs\game-1794226265782 | result=PASS |
| LOG-03 | extractor: discovers inserter targets of the same network/frequency, rebuilds capability caches, extracts with filter, distributes via `Distributor` | VERIFIED | `content/behaviour/logistics/ExtractorNodeBehaviour.java:26-200`, `content/behaviour/logistics/distribution/{Distributor,RoundRobinDistributor,DistributionMode}.java` | `native/theurgy/logistics.rs` | test=Java GameTest differential (5 tests) + isolated game logistics + isolated game wire-graph | run=artifacts/differential.json + runs\game-1794225083517 + runs\game-1794226265782 | result=PASS |
| LOG-04 | inserter: resolves target capabilities, inserts per filter; connector restores attached target after chunk load | VERIFIED | `content/behaviour/logistics/InserterNodeBehaviour.java:21-90` | `native/theurgy/logistics.rs` | test=Java GameTest differential (5 tests) + isolated game logistics + isolated game wire-graph | run=artifacts/differential.json + runs\game-1794225083517 + runs\game-1794226265782 | result=PASS |
| LOG-05 | item inserter/extractor BEs, slot-per-face wrappers, whitelist of touched faces | VERIFIED | `content/apparatus/logisticsitemconnector/**` | `native/theurgy/logistics.rs` | test=Java GameTest differential (4 tests) | run=artifacts/differential.json | result=PASS |
| LOG-06 | fluid inserter/extractor: same rules over fluid capabilities; world-source extraction | VERIFIED | `content/apparatus/logisticsfluidconnector/**` | `native/theurgy/logistics.rs` | test=Java GameTest differential (2 tests) + isolated game logistics + isolated game logistics-advanced | run=artifacts/differential.json + runs\game-1794225083517 + runs\game-1794226331517 | result=PASS |
| LOG-07 | connection node: only relays when it is part of a nexus-linked network | VERIFIED | `content/apparatus/logisticsconnectionnode/LogisticsConnectionNodeBlock.java` | `native/theurgy/logistics.rs` | test=Java GameTest differential (2 tests) | run=artifacts/differential.json | result=PASS |
| LOG-08 | nexus: pairing id (`LOGISTICS_NEXUS_ID` UUID), pairs across placement, BE accepts item uuid, portal rendering | VERIFIED | `content/apparatus/logisticsnexus/{LogisticsNexusBlock,LogisticsNexusBlockEntity,LogisticsNexusPairing}.java`, `render/LogisticsNexusRenderer.java` | `native/theurgy/logistics.rs, native/theurgy/render.rs` | test=Java GameTest differential (2 tests) + isolated game render + isolated game logistics-advanced | run=artifacts/differential.json + runs\game-1794228087282 + runs\game-1794226331517 | result=PASS |
| LOG-09 | capability proxy: forwards item/fluid capabilities, round-robin across multiple sides | VERIFIED | `content/apparatus/logisticscapabilityproxy/*.java`, `content/storage/{CombinedItemStorage,SelectItemHandlerWrapper,SelectFluidHandlerWrapper}.java` | `native/theurgy/logistics.rs` | test=Java GameTest differential (3 tests) + isolated game logistics-advanced | run=artifacts/differential.json + runs\game-1794226331517 | result=PASS |
| LOG-10 | capability probe (debug block) | VERIFIED | `content/apparatus/logisticscapabilityprobe/LogisticsCapabilityProbeBlock.java` | `native/theurgy/logistics.rs` | test=Java GameTest differential (1 tests) + isolated game logistics-advanced | run=artifacts/differential.json + runs\game-1794226331517 | result=PASS |
| LOG-11 | mercury flux connector (see FLU-09) | VERIFIED | `content/apparatus/logisticsmercuryfluxconnector/**` | `native/theurgy/logistics.rs` | test=Java GameTest differential (2 tests) + isolated game logistics-advanced | run=artifacts/differential.json + runs\game-1794226331517 | result=PASS |
| LOG-12 | wire graph: `Wires` SavedData (`theurgy.wires`), `Wire` codec, add/remove per pos/chunk, client sync, slack helper | VERIFIED | `logistics/{Wires,Wire,WireEndPoint,WireSlackHelper,WireSync}.java`, `content/item/wire/WireItem.java` | `native/theurgy/logistics.rs` | test=isolated game wire-graph + isolated game tools + offline modules | run=runs\game-1794226265782 + runs\game-1794225131184 + runs\acceptance-2026-11-09T11-37-36.767Z\modules.log | result=PASS |
| LOG-13 | `WIRE_END_POINT` component lifecycle (save/load/remove on item) | VERIFIED | `logistics/WireEndPoint.java:21-60` | `native/theurgy/items.rs` | test=isolated game wire-graph + isolated game tools + offline modules | run=runs\game-1794226265782 + runs\game-1794225131184 + runs\acceptance-2026-11-09T11-37-36.767Z\modules.log | result=PASS |
| LOG-14 | filters: `ListFilter` (items + fluids, deny-list, respect data components), `AttributeFilter` (+6 attributes), `EmptyFilter`, mode (`FilterMode`), filter item UI, `FilterBehaviour.useItemOn` drop/filter apply | VERIFIED | `content/behaviour/filter/*.java`, `content/behaviour/filter/attribute/*.java` | `native/theurgy/{filter.rs,filter_client.rs,filtered_transfer.rs}` | test=isolated game filter-ui + isolated game filter-matching + isolated game attribute-filters + isolated game standard-attributes + isolated game special-attributes + isolated game filters | run=runs\game-1794227930116 + runs\game-1794226430467 + runs\game-1794226505773 + runs\game-1794226566949 + runs\game-1794226677460 + runs\game-1794225249408 | result=PASS |
| LOG-15 | wire renderer (`WireRenderer`, `useSimpleWireRenderer`/`wireLineWidth` config) | VERIFIED | `logistics/WireRenderer.java` | `native/theurgy/render.rs` | test=isolated game client-features + isolated game wire-graph | run=runs\game-1794228220578 + runs\game-1794226265782 | result=PASS |
| LOG-16 | node status HUD packets (`MessageShowLogisticsNodeStatus`) and wand-driven reconfiguration | VERIFIED | `network/messages/MessageShowLogisticsNodeStatus.java`, `content/item/mercurialwand/mode/SwitchLogisticsEnabledMode.java`, `SetSelectedFrequencyMode.java`, `SelectFrequencyMode.java` | `native/theurgy/render.rs` | test=isolated game wand + isolated game multiplayer | run=runs\game-1794228183851 + runs\multiplayer-1794228295294 | result=PASS |

## 9. Items and tools

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| ITM-01 | `DivinationRodItem`: defaults `NOT_FOUND=7.0f`, `SEARCHING=8.0f` as item-property floats; per-rod tier/allowed-tag/disallowed-tag/range/duration/durability/allowAttuning | VERIFIED | `content/item/divinationrod/DivinationRodItem.java:53-80`, `content/item/renderer/DivinationDistanceProperty.java` | `native/theurgy/items.rs` | test=Java GameTest differential (1 tests) + isolated game divination + isolated game config-attunement | run=artifacts/differential.json + runs\game-1794225168593 + runs\game-1794225215575 | result=PASS |
| ITM-02 | divination scan: `useOn` linking a block/tag (with size limits), `use` starting a scan, `getOreTagFromBlockId`/`getOreTagFromOreName` via `([a-z]+)_ore` and `([a-z]+_[a-z]+)_ore`, `ServerConfig` `sulfurSourceToBlockMapping` | VERIFIED | `DivinationRodItem.java:88-166, 195-345, 435-460` | `native/theurgy/items.rs, config port` | test=Java GameTest differential (1 tests) + isolated game divination + isolated game config-attunement | run=artifacts/differential.json + runs\game-1794225168593 + runs\game-1794225215575 | result=PASS |
| ITM-03 | rod durability (`getMaxDamage`, `onUseTick` damage), `releaseUsing` early cancel, `finishUsingItem` result, `DivinationRodRecipe` attunement | VERIFIED | `DivinationRodItem.java:183-372`, `content/recipe/DivinationRodRecipe.java` | `native/theurgy/items.rs` | test=Java GameTest differential (3 tests) + isolated game divination + isolated game divination-recipes | run=artifacts/differential.json + runs\game-1794225168593 + runs\game-1794227834041 | result=PASS |
| ITM-04 | `MercurialWandItem`: cycled modes (capacitor side, selected direction, logistics enabled, select/set frequency), scroll/left-click handling, mode component `MERCURIAL_WAND_ITEM_MODE`, per-mode render handlers | VERIFIED | `content/item/mercurialwand/MercurialWandItem.java:27-100`, `mode/*.java`, `content/item/mode/*.java` | `native/theurgy/items.rs` | test=isolated game wand | run=runs\game-1794228183851 | result=PASS |
| ITM-05 | sal ammoniac bucket (fluid transport, empty/fill, dispensers) | VERIFIED | `registry/ItemRegistry.java:154-159`, `registry/FluidRegistry.java` | `native/theurgy/items.rs, hooks/use_on.rs` | test=Java GameTest differential (2 tests) + isolated game fluids | run=artifacts/differential.json + runs\game-1794226126216 | result=PASS |
| ITM-06 | `WireItem` (max linkage length 32) + `WireEndPoint` component + add/remove wire packets | VERIFIED | `content/item/wire/WireItem.java`, `network/messages/Message{Add,Remove}Wires.java` | `native/theurgy/items.rs` | test=isolated game tools + isolated game wire-graph + offline modules | run=runs\game-1794225131184 + runs\game-1794226265782 + runs\acceptance-2026-11-09T11-37-36.767Z\modules.log | result=PASS |
| ITM-07 | filters as items (list/attribute) carrying `FILTER_*` components | VERIFIED | `registry/ItemRegistry.java:150-151`, `content/item/filter/*.java` | `native/theurgy/items.rs` | test=isolated game filter-ui + isolated game filter-matching + isolated game attribute-filters + isolated game filters | run=runs\game-1794227930116 + runs\game-1794226430467 + runs\game-1794226505773 + runs\game-1794225249408 | result=PASS |
| ITM-08 | `TheHermetica` book item (Modonomicon integration, book data) | VERIFIED | `content/item/book/TheHermeticaItem.java`, `integration/modonomicon/**` | `native/theurgy/client.rs` | test=isolated game book | run=runs\game-1794227969983 | result=PASS |
| ITM-09 | derivative items: sulfur/niter/salt render + `getSourceItem` display | VERIFIED | `content/item/{salt,sulfur,niter}/**`, `content/item/derivative/**` | `native/theurgy/items.rs` | test=isolated game client | run=runs\game-1794228048631 | result=PASS |
| ITM-10 | held-stack-fit provider (`HeldStackFitProvider`, `MessageRequest/SyncHeldStackFit`, outline renderer) | VERIFIED | `content/render/{HeldStackFitProvider,HeldStackFitStatus,HeldStackFitOutline}.java` | `native/theurgy/render.rs` | test=isolated game client-features | run=runs\game-1794228220578 | result=PASS |

## 10. Recipes, conditions, data generation, content

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| REC-01 | recipe result indirection `RecipeResult`/`ItemRecipeResult`/`TagRecipeResult` and `ConditionRegistry` (incl. `FluidTagEmptyCondition`) | VERIFIED | `content/recipe/result/*.java`, `content/recipe/condition/FluidTagEmptyCondition.java`, `registry/ConditionRegistry.java` | `native/theurgy/engine/src/recipes.rs` | test=isolated game content-rules + isolated game review-fixes | run=runs\game-1794225295776 + runs\game-1794226734819 | result=PASS |
| REC-02 | 11 recipe providers generate the full recipe set (calcination, distillation, liquefaction, fermentation, digestion, incubation, accumulation, catalysation, reformation, shaped, shapeless, smelting, json) | VERIFIED | `datagen/recipe/*.java` | `tools/build-content.mjs, content/theurgy/data` | test=isolated game all + isolated game recipes + offline content-check + offline audit | run=runs\game-1794224712378 + runs\game-1794228009841 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| REC-03 | custom shaped/shapeless/smelting recipes keep correct result/ingredient mapping in the ported content | VERIFIED | `datagen/recipe/{ShapedRecipeProvider,ShapelessRecipeProvider,SmeltingRecipeProvider}.java` | `tools/content-check.mjs` | test=Java GameTest differential (3 tests) + isolated game all + offline content-check + offline audit | run=artifacts/differential.json + runs\game-1794224712378 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| REC-04 | worldgen: sal ammoniac ore configured/placed features + biome modifiers | VERIFIED | `datagen/worldgen/{ConfiguredFeatures,PlacedFeatures,BiomeModifiers,TheurgyRegistries}.java` | `content/theurgy/data` | test=isolated game worldgen | run=runs\game-1794227872201 | result=PASS |
| GEN-01 | loot tables for every block (incl. double-high halves and contents) | VERIFIED | `datagen/loot/TheurgyBlockLootSubProvider.java` | `content/theurgy/data/theurgy/loot_table` | test=Java GameTest differential (3 tests) + isolated game interactions + offline content-check + offline audit | run=artifacts/differential.json + runs\game-1794224964769 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| GEN-02 | advancements and The Hermetica book entries (Modonomicon pages for each apparatus page type) | VERIFIED | `datagen/advancement/TheurgyAdvancementSubProvider.java`, `datagen/book/**`, `integration/modonomicon/page/**` | `content/theurgy/data` | test=isolated game book + offline content-check + offline audit | run=runs\game-1794227969983 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| GEN-03 | lang files EN (+ tooltip provider) for every item/block/message | VERIFIED | `datagen/lang/{ENUSProvider,TooltipLanguageProvider}.java` | `content/theurgy/assets/theurgy/lang` | test=isolated game tooltips + offline content-check + offline audit | run=runs\game-1794228260776 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| GEN-04 | blockstates/models/item models + special model renderers | VERIFIED | `datagen/model/*.java`, `registry/TheurgySpecialModelRenderers.java` | `content/theurgy/assets, native/theurgy/render.rs` | test=isolated game render + offline content-check + offline audit | run=runs\game-1794228087282 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| GEN-05 | multiblock provider documentation data (`TheurgyMultiblockProvider`) | VERIFIED | `datagen/multiblock/TheurgyMultiblockProvider.java` | `content/theurgy/data/theurgy/modonomicon + native/theurgy/client.rs` | test=isolated game book + offline content-check + offline audit | run=runs\game-1794227969983 + runs\acceptance-2026-11-09T11-37-36.767Z\content-check.log + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |

## 11. Config, network, integration, client

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| CFG-01 | `ServerConfig.recipes.sulfurSourceToBlockMapping` (accepts `source=block` and `#tag=#tag`), tooltip handler namespaces | VERIFIED | `config/ServerConfig.java:15-75` | `native/theurgy/items.rs` | test=isolated game config-attunement + isolated game tooltips | run=runs\game-1794225215575 + runs\game-1794228260776 | result=PASS |
| CFG-02 | `CommonConfig.Misc` | VERIFIED | `config/CommonConfig.java:9-60` | `native/theurgy/config.rs` | test=isolated game config-attunement + offline audit | run=runs\game-1794225215575 + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| CFG-03 | `ClientConfig.Rendering`: `renderSulfurSourceItem`, `enableItemHUD`, `enableInWorldHUD`, `enableHeldStackFitOutline`, `heldStackFitOutlineRenderMode`, `hudScale` (0.25..3), `wireLineWidth` (1..64), `useSimpleWireRenderer` | VERIFIED | `config/ClientConfig.java:12-90` | `native/theurgy/client.rs + render.rs` | test=isolated game client-features | run=runs\game-1794228220578 | result=PASS |
| NET-01 | `Networking`/`MessageHandler` registration set: 18 messages (wire add/remove, selections, flux visuals, HUD request/sync, divination result, mode set, left-click empty, held-stack fit) | VERIFIED | `network/Networking.java`, `network/MessageHandler.java`, `network/messages/*.java` | `native/theurgy/*.rs + hooks` | test=isolated game multiplayer | run=runs\multiplayer-1794228295294 | result=PASS |
| NET-02 | server authority: no client message may write block state without server validation | VERIFIED | `network/messages/Message*Selection.java`, `MessageSetMode.java` | `native/theurgy/host.rs` | test=isolated game multiplayer | run=runs\multiplayer-1794228295294 | result=PASS |
| INT-01 | JEI/EMI recipe categories for 8 machine types | VERIFIED | `integration/{jei,emi}/**` | `native/theurgy/recipe_client.rs` | test=isolated game recipes | run=runs\game-1794228009841 | result=PASS |
| INT-02 | Jade provider for mercury flux energy | VERIFIED | `integration/jade/*.java` | `native/theurgy/client.rs` | test=isolated game client-features | run=runs\game-1794228220578 | result=PASS |
| INT-03 | KubeJS + AlmostUnified + Occultism integration (dummy + impl classes) | VERIFIED | `integration/{kubejs,almostunified,occultism}/**` | `native/theurgy/recipe_client.rs + filtered_transfer.rs + generated data` | test=isolated game content-rules + isolated game review-fixes + isolated game recipes + offline audit | run=runs\game-1794225295776 + runs\game-1794226734819 + runs\game-1794228009841 + runs\acceptance-2026-11-09T11-37-36.767Z\audit.log | result=PASS |
| CLI-01 | item HUD + in-world HUD providers (block title, fluid storage, item storage, crafting progress, catalyst, flux handler) | VERIFIED | `content/render/inworldhud/**`, `content/render/itemhud/**` | `native/theurgy/render.rs` | test=isolated game client-features | run=runs\game-1794228220578 | result=PASS |
| CLI-02 | particles (`colored_bubble`), outlines, cube model, render types, `ClientTicks` | VERIFIED | `content/particle/**`, `content/render/outliner/**`, `content/render/cube/**` | `native/theurgy/render.rs` | test=isolated game client-features | run=runs\game-1794228220578 | result=PASS |
| CLI-03 | tooltips (`TooltipHandler`, `TooltipDataProvider`) | VERIFIED | `tooltips/*.java` | `native/theurgy/items.rs` | test=isolated game tooltips | run=runs\game-1794228260776 | result=PASS |

## 12. Java GameTest oracle index

20 files under `src/main/java/com/klikli_dev/theurgy/gametest/` = 19 test classes + `GameTestRegistry`.
**Verified counts** (`grep -c`): 151, all distinct, `TEST_FUNCTIONS.register("<id>", ...)` calls in
`gametest/GameTestRegistry.java`; 152 `public static void` test bodies across the 19 classes, i.e.
one body is not registered (find it before claiming full oracle coverage). Every registered id is the
preferred parity anchor for the corresponding row above. Per-file bodies (exact, summing to 152):

| file | tests | covers checklist |
|---|---|---|
| `CalcinationOvenGameTests.java` | 13 | MA-01..MA-07 |
| `DistillerGameTests.java` | 11 | MA-20..MA-22 |
| `LiquefactionCauldronGameTests.java` | 15 | MA-30..MA-33 |
| `FermentationVatGameTests.java` | 10 | MA-40, MA-42..MA-44 |
| `DigestionVatGameTests.java` | 8 | MA-41..MA-44 |
| `IncubatorGameTests.java` | 18 | INC-01..INC-06 |
| `ReformationPedestalGameTests.java` | 8 | REF-01..REF-08 |
| `SulfuricFluxEmitterGameTests.java` | 5 | REF-04..REF-06 |
| `MercuryCatalystGameTests.java` | 5 | FLU-01..FLU-03 |
| `MercuryCapacitorGameTests.java` | 7 | FLU-04..FLU-07 |
| `MercuryFluxEmitterGameTests.java` | 4 | FLU-08 |
| `MercuryFluxConnectorGameTests.java` | 5 | FLU-09 |
| `CaloricFluxEmitterGameTests.java` | 4 | HEA-07 |
| `PyromanticBrazierGameTests.java` | 14 | HEA-04..HEA-06, MA-06 |
| `SalAmmoniacAccumulatorGameTests.java` | 3 | SAL-01..SAL-04 |
| `SalAmmoniacTankGameTests.java` | 4 | SAL-05 |
| `LogisticsGameTests.java` | 12 | LOG-01..LOG-08 |
| `LogisticsCapabilityProxyGameTests.java` | 3 | LOG-09 |
| `DivinationRodGameTests.java` | 3 | ITM-01..ITM-03 |

Note: upstream GameTests use the registered test ids, not `@GameTest` annotations;
`GameTestRegistry.TEST_FUNCTIONS` is the authoritative id table (151 ids). A GameTest run on the
pinned Java 26.3 build is the only accepted oracle; a re-implementation of these assertions in JS is not.

Reproduce the index (read-only, safe):
`grep -c 'TEST_FUNCTIONS.register(' upstream/theurgy/src/main/java/com/klikli_dev/theurgy/gametest/GameTestRegistry.java`
and per file `grep -c 'public static void' <TestCase>.java`.

## 13. Coverage summary (current, evidence-derived)

Rows are `VERIFIED` only when every reference in `tools/feature-evidence.mjs` resolves to a passing Java differential, a saved isolated-game run, or a saved offline acceptance step. Missing artifacts leave the row `PORTED-UNVERIFIED`.

| subsystem | rows | VERIFIED | PORTED-UNVERIFIED | PARTIAL | UNPORTED | N/A-CLIENT |
|---|---|---|---|---|---|---|
| 1 registries (REG) | 13 | 13 | 0 | 0 | 0 | 0 |
| 2 common crafting (CB) | 14 | 14 | 0 | 0 | 0 | 0 |
| 3 machines (HEA/MA) | 29 | 29 | 0 | 0 | 0 | 0 |
| 4 sal ammoniac (SAL) | 6 | 6 | 0 | 0 | 0 | 0 |
| 5 flux (FLU) | 11 | 11 | 0 | 0 | 0 | 0 |
| 6 incubator (INC) | 7 | 7 | 0 | 0 | 0 | 0 |
| 7 reformation (REF) | 9 | 9 | 0 | 0 | 0 | 0 |
| 8 logistics (LOG) | 16 | 16 | 0 | 0 | 0 | 0 |
| 9 items (ITM) | 10 | 10 | 0 | 0 | 0 | 0 |
| 10 recipes/gen (REC/GEN) | 9 | 9 | 0 | 0 | 0 | 0 |
| 11 config/network/integration/client | 11 | 11 | 0 | 0 | 0 | 0 |

Total: **135 checklist rows**. VERIFIED = 135, PORTED-UNVERIFIED = 0, PARTIAL = 0, UNPORTED = 0, N/A-CLIENT = 0.
The 152-behaviour matrix is `artifacts/differential.json`; isolated-game runs are indexed by `artifacts/gui-evidence.json`; offline steps by `artifacts/offline-evidence.json`.

## 14. Definition of done for one row

A row may only move to `VERIFIED` when all of the following are recorded in its `evidence` cell:

1. `test=` a real compiled test (Rust unit/integration test, or the isolated-game scenario) that
   fails before the change and passes after, or a real Java oracle run id.
2. `run=` the path of the saved log/evidence JSON in this worktree.
3. `result=PASS` (failures stay visible; never delete a failing row).
4. For machine behaviour: a differential comparison against the pinned Java 26.3 GameTest id(s)
   listed in the row, executed on the pinned upstream. A Rust-only assertion is `PORTED-UNVERIFIED`.
5. `native/theurgy/registry.rs` and `native/theurgy/horizon-mixin.toml` wiring exists for the
   block entity, otherwise the row is only engine-level.

## 15. Explicit non-evidence

These are **not** evidence for any row and must not be used to flip a status:

- Agent-generated Rust candidates or chaos-test modules from `runs/` (never compiled/executed).
- Reviewer text, proposals, mailboxes, or `artifacts/port-campaign.json`.
- The vanilla smoke (`npm run game`), the vanilla chaos plan (`npm run game:chaos`) — they exercise
  Minecraft only, not Theurgy machines.
- `npm run game:theurgy` passing block registration only proves the `theurgy:distiller` block id.
- A JS re-implementation of a recipe formula.

## 16. Known upstream quirks that the port must preserve, not "fix"

1. `stopProcessing()` keeps `couldCraftLastTick` when it was already idle (`CraftingBehaviour.java:212`).
2. `ReformationRecipe.assemble` copies all target components, although upstream marks it a TODO.
3. `AccumulationRecipe.matches` rejects a solute-free recipe when a solute is present.
4. `Accumulator.canCraft` requires the whole output volume to fit, while older code comment notes
   the discarded partial-void alternative.
5. `PyromanticBrazier.consumeFuel` writes the crafting remainder only when the fuel stack empties.
6. `MercuryCatalystCraftingBehaviour` returns `-1` for crafting time and overrides tickServer,
   so it never uses the base progress path.
7. `MercuryCapacitor.pushMercuryFlux` uses `Direction.allShuffled(level.random)` — parity needs a
   seeded deterministic RNG, not a sorted iteration.
8. `VatRedstoneAutoCloseBehaviour.randomSlowTickOffset = Math.random()*20` at construction.
9. `remainingLitTime` is stored as `short` — negative/large values must survive a reload exactly.
10. `IncubatorCraftingBehaviour.getTotalTime` falls back to the default when the multiblock is
    disassembled (`recipeInputSupplier.get() == null`).

