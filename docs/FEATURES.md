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

Horizon pin (`config.json` `horizonSource`): `c9d474dd9116e84c243f8992d75c11314df56f06`.
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

Known-state facts at the time of writing (documented in `docs/VALIDATION.md`):
`native/theurgy/{fluids,flux,heat,incubation,logistics,machines,reformation,render}.rs` are
2–5-line placeholders; `native/theurgy/registry.rs` is a static block-string table without
BE-level semantics; `native/theurgy/engine` only contains `crafting`, `recipes`, `machines`
with a Calcination-like tick. Therefore every behaviour row below is `UNPORTED` unless
explicitly noted in the row's `notes`, and no row is `VERIFIED`.
EV=NOT EXECUTED means no Rust proof and no Java oracle run exists for the row.

## 1. Registries and identity surface

Rows are `id | behaviour | status | java (path:lines#member) | rust | evidence`.
Java paths are relative to `upstream/theurgy/src/main/java/com/klikli_dev/theurgy/`.

### 1.1 Blocks and block entities

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| REG-01 | **32** `BLOCKS.registerBlock(...)` entries (verified count): 28 content blocks + `sal_ammoniac` fluid block + `sal_ammoniac_ore` + `deepslate_sal_ammoniac_ore`; `SAL_AMMONIAC_FLUID` is a `LiquidBlock` (`BlockRegistry.java:255-256`) | UNPORTED | `registry/BlockRegistry.java:255-280` + 30 further `registerBlock` lines | `native/theurgy/registry.rs` (id strings only) | EV=NOT EXECUTED |
| REG-02 | **22** `BLOCKS.register("id", ...)` block-entity types in `BlockEntityRegistry` (verified count). Only 22 of the 32 blocks have a BE: the 4 double-high machines, 3 incubator vessels, tank/accumulator, nexus, catalyst, capacitor, caloric emitter, 3 reformation pedestals, sulfuric + mercury emitters, 2 vats, and 6 logistics nodes (item inserter/extractor, fluid inserter/extractor, capability proxy, mercury flux connector). `logistics_connector_node`, `logistics_capability_probe`, `sal_ammoniac`, ores and fluid have **no** BE | UNPORTED | `registry/BlockEntityRegistry.java:22-151` | `native/theurgy/registry.rs::is_be` | EV=NOT EXECUTED |
| REG-03 | double-high machines register BE only on `DOUBLE_BLOCK_HALF=LOWER`: calcination_oven, liquefaction_cauldron, distiller, incubator. **Correction:** `BlockEntityRegistry` only lists the block in `Set.of(...)`; the LOWER half check is in each block's `newBlockEntity`/`getStateForPlacement`, not in the registry (do not cite `BlockEntityRegistry.java` for this row; locate it per block class before porting) | UNPORTED | per-block `*Block.java` `newBlockEntity`; `content/apparatus/DirectionalBlockShape.java` | `native/theurgy/registry.rs::create` (no half check) | EV=NOT EXECUTED |
| REG-04 | item forms (`BlockItem` subclasses: `CalcinationOvenBlockItem`, `DistillerBlockItem`, `SalAmmoniacTankBlockItem`, ...) carry BE data components | UNPORTED | `content/apparatus/*/[A-Za-z]*BlockItem.java` | `native/theurgy/items.rs` (placeholder) | EV=NOT EXECUTED |
| REG-05 | `sal_ammoniac` (source + flowing, `BaseFlowingFluid.Source/Flowing`), bucket, `FluidType` (`SolventFluidType`) | UNPORTED | `registry/FluidRegistry.java:16-25`, `registry/FluidTypeRegistry.java`, `content/fluid/SolventFluidType.java` | `native/theurgy/fluids.rs` | EV=NOT EXECUTED |

### 1.2 Recipes, components, items

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| REG-10 | 10 recipe types in the exact order: calcination, liquefaction, distillation, pyromantic_brazier (reuses `DistillationRecipe`), incubation, accumulation, catalysation, reformation, fermentation, digestion | UNPORTED | `registry/RecipeTypeRegistry.java:16-31` | `native/theurgy/engine/src/recipes.rs` (generic) | EV=NOT EXECUTED |
| REG-11 | 10 recipe serializers + display registry + recipe-result registry | UNPORTED | `registry/RecipeSerializerRegistry.java`, `registry/RecipeDisplayRegistry.java`, `registry/RecipeResultRegistry.java` | - | EV=NOT EXECUTED |
| REG-12 | **26** `registerComponentType(...)` (verified count): `mercury_flux_storage`, `mercury_flux_to_convert`, `current_mercury_flux_per_tick`, 8 `divination_*`, `wire_end_point`, `mercurial_wand_item_mode`, `source_item/tag`, `mercury_catalyst_inventory`, 5 `filter_*`, `selected_frequency`, `logistics_nexus_id` | UNPORTED | `registry/DataComponentRegistry.java:36-194` | `native/theurgy/registry.rs::component_types` (string list) | EV=NOT EXECUTED |
| REG-13 | items: 8 icon items, 9 `DivinationRodItem`, `MercurialWandItem`, `WireItem(32)`, 2 filters, `sal_ammoniac_bucket`, mercury shard/crystal, purified gold, fermentation starter, crystallized water/lava, `TheHermeticaItem` | UNPORTED | `registry/ItemRegistry.java:39-252` | `native/theurgy/items.rs` | EV=NOT EXECUTED |
| REG-14 | derivatives: sulfur types `mobs/animals/logs/crops/other_minerals/gems/metals` + tiers; niters; salts `strata/mineral/plant/creature`; tier/type on item | UNPORTED | `registry/SulfurRegistry.java:29-120`, `registry/NiterRegistry.java:23-60`, `registry/SaltRegistry.java:23-55`, `content/item/derivative/AlchemicalDerivativeTier.java`, `content/item/sulfur/AlchemicalSulfurType.java` | `native/theurgy/items.rs` | EV=NOT EXECUTED |
| REG-15 | creative tabs, sounds, particles (`colored_bubble`), shaders, key mappings | UNPORTED | `registry/{CreativeModeTabRegistry,SoundRegistry,ParticleRegistry,ShaderRegistry,KeyMappingsRegistry}.java` | `native/theurgy/render.rs` | EV=NOT EXECUTED |
| REG-16 | tags: `c:ores`, `c:*_ores_in_ground`, `c:gems/ingots/raw_materials/foods`, `minecraft:mineable/*`, divination allowed/disallowed tags | UNPORTED | `registry/{BlockTagRegistry,ItemTagRegistry,FluidTagRegistry}.java`, `datagen/tag/*` | `tools/build-content.mjs` | EV=NOT EXECUTED |
| REG-17 | capabilities: heat provider/receiver, mercury flux handler, energy, heat-receiver lookup at `pos` | UNPORTED | `registry/CapabilityRegistry.java`, `content/capability/*.java` | `native/theurgy/host.rs` | EV=NOT EXECUTED |

## 2. Common crafting contract (shared by 5 item machines + catalyst + accumulator)

All machines below delegate progress to one base class; a port that re-implements progress
per machine will drift. These rows are prerequisites for **every** MA/ME/INC/SAL/REF row.

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| CB-01 | `tickServer(canProcess, hasInput)`: no input -> `stopProcessing`; recipe lookup only when input; `couldCraftLastTick = canCraft(recipe)`; if `canProcess && couldCraft` then `tryStartProcessing(); progress++; tryFinishProcessing(recipe)` else `stopProcessing()` | PARTIAL | `content/behaviour/crafting/CraftingBehaviour.java:117-137` | `native/theurgy/engine/src/crafting.rs::Crafting::tick` | EV=NOT EXECUTED (Rust unit tests exist for progress/stop; no Java oracle) |
| CB-02 | `tryFinishProcessing` fires at `progress >= totalTime`, resets `progress=0`, `totalTime=getTotalTime()`, then `craft`, then `sendBlockUpdated` — no `setChanged()` there | PARTIAL | `CraftingBehaviour.java:191-200` | `engine/src/crafting.rs` | EV=NOT EXECUTED |
| CB-03 | `tryStartProcessing` sets `isProcessing=true` and sends block update only when `progress == 0` | PARTIAL | `CraftingBehaviour.java:202-210` | `engine/src/crafting.rs` | EV=NOT EXECUTED |
| CB-04 | `stopProcessing` only mutates/notifies if `progress != 0 \|\| isProcessing`; resets `progress` but **keeps** `couldCraftLastTick` when already idle (Java quirk — keep it) | PARTIAL | `CraftingBehaviour.java:212-223` | `engine/src/crafting.rs` (unit test `idle_stop_does_not_clear_could_craft_java_quirk`) | EV=NOT EXECUTED |
| CB-05 | `canCraft`: `assemble(input)` empty -> false; else dry-run insert into output and require *whole* stack fits | PARTIAL | `CraftingBehaviour.java:225-238` | `engine/src/machines.rs::can_fit` / `tick_machine` | EV=NOT EXECUTED |
| CB-06 | `craft`: insert assembled into output first; if any remainder -> false; then extract exactly `getIngredientCount(recipe)` from slot 0; commit only if both succeed (atomic, no dupe/void) | PARTIAL | `CraftingBehaviour.java:240-256` | `engine/src/machines.rs::tick_machine` (claimed atomic) | EV=NOT EXECUTED |
| CB-07 | `getTotalTime` = recipe time or `getDefaultCraftingTime()` when no level/recipe | PARTIAL | `CraftingBehaviour.java:258-267` | `engine/src/crafting.rs` | EV=NOT EXECUTED |
| CB-08 | `progressPercent = clamp(progress*100/totalTime, 0, 100)`, 0 when `totalTime <= 0` (no div-by-zero) | PARTIAL | `CraftingBehaviour.java:64-70` | `engine/src/crafting.rs::progress_percent` | EV=NOT EXECUTED |
| CB-09 | `onInputItemChanged` sets `totalTime=getTotalTime()`, `progress=0` | UNPORTED | `CraftingBehaviour.java:139-142` | `engine/src/crafting.rs::input_changed` | EV=NOT EXECUTED |
| CB-10 | `alreadyHasInput` early-out in `canProcess(ItemStack)` via `ItemStack.isSameItemSameComponents` | UNPORTED | `CraftingBehaviour.java:148-189` | - | EV=NOT EXECUTED |
| CB-11 | save/load: `progress`, `totalTime`, `isProcessing`, `couldCraftLastTick` persist across save/reload | PARTIAL | `CraftingBehaviour.java:80-100` | `native/theurgy/host.rs::{crafting,save_crafting}` | EV=NOT EXECUTED |
| CB-12 | network sync writes only `isProcessing` | UNPORTED | `CraftingBehaviour.java:72-78` | - | EV=NOT EXECUTED |
| CB-13 | `LevelAwareCachedCheck`/`LevelAwareRecipeCheck` caching semantics (cache invalidation on input change) | UNPORTED | `content/behaviour/crafting/LevelAwareCachedCheck.java`, `LevelAwareRecipeCheck.java` | - | EV=NOT EXECUTED |
| CB-14 | `SettableItemStorage` / `MonitoredItemStackHandler` / `DynamicOneOutputSlotItemHandlerBehaviour` output-slot sizing | UNPORTED | `content/storage/SettableItemStorage.java`, `content/storage/MonitoredItemStackHandler.java`, `content/behaviour/itemhandler/*.java` | `native/theurgy/registry.rs::container` (static slot counts) | EV=NOT EXECUTED |

## 3. Spagyric machines (heat consumers)

Heat source check is shared: `HeatConsumerBehaviour.isHeated()` samples every
`CHECK_HEAT_TICK_INTERVAL = 20` game ticks, prefers a `HeatProvider` **below**, falls back to a
`HeatReceiver` capability (`getIsHotUntil() > gameTime`), and flips `LIT` only on change.

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| HEA-01 | `HeatConsumerBehaviour.isHeated` 20-tick cache, provider-below priority, receiver fallback, `LIT` update + `setChanged` on transition only | UNPORTED | `content/behaviour/heat/HeatConsumerBehaviour.java:16-57` | `native/theurgy/heat.rs` (placeholder) | EV=NOT EXECUTED |
| HEA-02 | `DefaultHeatReceiver`: `isHotUntil` serialized, `readyToReceive`, `setHotUntil` | UNPORTED | `content/capability/DefaultHeatReceiver.java:15-52` | - | EV=NOT EXECUTED |
| HEA-03 | `CraftingHeatReceiver.readyToReceive` (only accepts heat while not already processing) | UNPORTED | `content/capability/CraftingHeatReceiver.java:12-25` | - | EV=NOT EXECUTED |
| HEA-04 | Pyromantic brazier: 1-slot inventory accepting only `COOKING_FUEL` with `burnTime > 0` (26.3 item component, resolved via empty loot context) | UNPORTED | `content/apparatus/pyromanticbrazier/PyromanticBrazierBlockEntity.java:96-240` | `native/theurgy/heat.rs` | EV=NOT EXECUTED |
| HEA-05 | Brazier tick: decrement `remainingLitTime` when lit; relight from fuel; consume exactly 1 fuel incl. crafting remainder; sync `LIT` blockstate; `setChanged` when transitioned | UNPORTED | `PyromanticBrazierBlockEntity.java:126-175` | `native/theurgy/heat.rs` | EV=NOT EXECUTED |
| HEA-06 | Brazier `remainingLitTime` is saved as `short` (`getShortOr`), inventory synced to clients | UNPORTED | `PyromanticBrazierBlockEntity.java:177-191` | - | EV=NOT EXECUTED |
| HEA-07 | Caloric flux emitter: `CAPACITY=1000`, `FLUX_PER_HEAT=100`, `HEAT_TARGET_FOR_TICKS=20*6`, `TICK_INTERVAL=20`; skips when `ENABLED=false`, no points, target invalid, receiving block not `readyToReceive`, or target still hot past next tick; extracts exactly 100 flux; sets `hotUntil = time + 120` | UNPORTED | `content/apparatus/caloricfluxemitter/CaloricFluxEmitterBlockEntity.java:32-91` | `native/theurgy/flux.rs` | EV=NOT EXECUTED (java id `caloric_flux_emitter_redstone`, `..._energy`) |

### 3.1 Calcination oven

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| MA-01 | `CalcinationRecipe` codec `{ingredient: SizedIngredient, result: ItemStackTemplate, time?: int = 100}`; `matches` only slot 0 | UNPORTED | `content/recipe/CalcinationRecipe.java:28-60` | `engine/src/recipes.rs` (generic) | EV=NOT EXECUTED |
| MA-02 | `CalcinationCraftingBehaviour`: ingredient count = `ingredient.count()`, time = recipe time, default `DEFAULT_TIME` | UNPORTED | `content/apparatus/calcinationoven/CalcinationCraftingBehaviour.java:20-52` | `native/theurgy/machines.rs` (placeholder) | EV=NOT EXECUTED |
| MA-03 | `isIngredient` consults cache with `level`; `CalcinationCachedCheck` recipe-type filter | UNPORTED | `CalcinationCraftingBehaviour.java:28-31`, `CalcinationCachedCheck.java` | - | EV=NOT EXECUTED |
| MA-04 | Double-high block: placement creates upper half, BE only lower, breaking either destroys the other | UNPORTED | `CalcinationOvenBlock.java`, `CalcinationOvenBlockItem.java` | `native/theurgy/machines.rs`, `hooks/place.rs`, `hooks/destroy.rs` | EV=NOT EXECUTED (java ids `calcination_oven_placement`, `..._be_only_lower`, `..._break_lower_destroys_upper`, `..._break_upper_destroys_lower`) |
| MA-05 | 2 slots (0 input, 1 output) with side-face restrictions | UNPORTED | `CalcinationStorageBehaviour.java`, `CalcinationOvenBlockEntity.java` | `registry.rs::container` | EV=NOT EXECUTED (java ids `calcination_oven_insert_input`, `..._extract_output`) |
| MA-06 | Processing lifecycle incl. `LIT` during processing, input consumed/output produced, blocked output no consume, stop on heat loss, stop on input loss | UNPORTED | `CalcinationOvenBlockEntity.java` (tickServer), `CraftingBehaviour.java:117-256` | `native/theurgy/machines.rs` | EV=NOT EXECUTED (java ids `calcination_oven_processing_starts`, `..._lit_during_processing`, `..._input_consumed_output_produced`, `..._blocked_output_does_not_consume_input`, `..._stops_without_heat`, `..._stops_without_input`) |
| MA-07 | drops inventory contents when broken | UNPORTED | `CalcinationOvenBlockEntity.java` (`preRemoveSideEffects`) | `hooks/destroy.rs` | EV=NOT EXECUTED (java id `calcination_oven_drops_items_when_broken`) |
| MA-08 | GeckoLib animation controller + item/BE renderers | N/A-CLIENT | `CalcinationAnimationBehaviour.java`, `render/CalcinationOvenRenderer.java`, `render/CalcinationOvenItemRenderer.java` | `native/theurgy/render.rs` (placeholder) | EV=NOT EXECUTED |

### 3.2 Distiller

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| MA-20 | `DistillationRecipe` `{ingredient: SizedIngredient, result, time?=100}`; also reused by recipe type `pyromantic_brazier` | UNPORTED | `content/recipe/DistillationRecipe.java:30-90`, `registry/RecipeTypeRegistry.java:21` | `native/theurgy/machines.rs` | EV=NOT EXECUTED |
| MA-21 | `DistillationCraftingBehaviour` count/time/default + `DistillationStorageBehaviour` (2 slots) | UNPORTED | `content/apparatus/distiller/DistillationCraftingBehaviour.java`, `DistillationStorageBehaviour.java` | `native/theurgy/machines.rs` | EV=NOT EXECUTED |
| MA-22 | double-high placement/removal, 2 slots, lifecycle, drops | UNPORTED | `DistillerBlock.java`, `DistillerBlockItem.java`, `DistillerBlockEntity.java` | `native/theurgy/machines.rs` | EV=NOT EXECUTED (java ids `distiller_placement`, `distiller_be_only_lower`, `distiller_break_lower_destroys_upper`, `distiller_break_upper_destroys_lower`, `distiller_insert_input`, `distiller_extract_output`, `distiller_processing_starts`, `distiller_lit_during_processing`, `distiller_input_consumed_output_produced`, `distiller_stops_without_heat`, `distiller_drops_items_when_broken`) |
| MA-23 | GeckoLib animation + BE/item renderers | N/A-CLIENT | `DistillerAnimationBehaviour.java`, `render/*.java` | `native/theurgy/render.rs` | EV=NOT EXECUTED |

### 3.3 Liquefaction cauldron

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| MA-30 | `LiquefactionRecipe` `{ingredient, solvent: SizedFluidIngredient, result, time?=100}`; `matches` = item ingredient **and** tank fluid matches sized solvent | UNPORTED | `content/recipe/LiquefactionRecipe.java:36-160` | `native/theurgy/machines.rs` | EV=NOT EXECUTED |
| MA-31 | craft consumes ingredient count + solvent amount; if tank lacks solvent the craft fails | UNPORTED | `content/apparatus/liquefactioncauldron/LiquefactionCraftingBehaviour.java:20-60` | `native/theurgy/machines.rs` | EV=NOT EXECUTED |
| MA-32 | one fluid tank + 2 slots, sided fluid access, `LiquefactionStorageBehaviour` | UNPORTED | `LiquefactionStorageBehaviour.java`, `LiquefactionCauldronBlockEntity.java` | `registry.rs::{container,fluids}` (10000 mB static) | EV=NOT EXECUTED (java ids `lc_insert_solvent_fluid`, `lc_extract_solvent_fluid`, `lc_processing_requires_solvent`) |
| MA-33 | double-high placement/removal, lifecycle, drops | UNPORTED | `LiquefactionCauldronBlock.java`, `LiquefactionCauldronBlockEntity.java` | `native/theurgy/machines.rs` | EV=NOT EXECUTED (java ids `lc_placement_creates_two_block_structure`, `lc_block_entity_only_on_lower_half`, `lc_breaking_lower_destroys_upper`, `lc_breaking_upper_destroys_lower`, `lc_input_consumed_output_produced`, `lc_processing_stops_when_heat_removed`, `lc_processing_requires_input`, `lc_drops_items_when_broken`, `lc_lit_state_true_during_processing`, `lc_insert_input_item`, `lc_extract_output_item`) |
| MA-34 | cauldron renderer with fluid surface | N/A-CLIENT | `render/LiquefactionCauldronRenderer.java` | `native/theurgy/render.rs` | EV=NOT EXECUTED |

### 3.4 Fermentation vat and digestion vat

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| MA-40 | `FermentationRecipe` `{fluid: SizedFluidIngredient, ingredients: Ingredient[], result, time?=200}`; `matches` requires fluid match, **exactly** N non-empty items and a `RecipeMatcher.findMatches` bijection | UNPORTED | `content/recipe/FermentationRecipe.java:32-160` | `native/theurgy/machines.rs` | EV=NOT EXECUTED |
| MA-41 | `DigestionRecipe`, same shape with `DEFAULT_TIME = 200` | UNPORTED | `content/recipe/DigestionRecipe.java:38-160` | `native/theurgy/machines.rs` | EV=NOT EXECUTED |
| MA-42 | vat storage 4 slots (3 in / 1 out) + tank; `LID` open/close; `hasOutput`; `VatAnalogSignalOutputBehaviour`; `VatRedstoneChangeOpenCloseLidBehaviour.neighborChanged`; `VatRedstoneAutoCloseBehaviour` (`randomSlowTickOffset = Math.random()*20` at construct — deterministic seed needed for parity); `VatRedstoneHasOutputBehaviour` | UNPORTED | `content/apparatus/{fermentationvat,digestionvat}/*StorageBehaviour.java`, `content/behaviour/redstone/Vat*.java` | `registry.rs::container` | EV=NOT EXECUTED (java ids `fermentation_vat_lid_closes`, `fermentation_vat_has_output_true`, `fermentation_vat_has_output_false`, `digestion_vat_lid_closes`) |
| MA-43 | player interaction on vats (`GenericVatInteractionBehaviour.useItemOn`, closed/open messages) | UNPORTED | `content/behaviour/interaction/GenericVatInteractionBehaviour.java:21-80`, `{Digestion,Fermentation}VatInteractionBehaviour.java` | `hooks/use_on.rs` | EV=NOT EXECUTED |
| MA-44 | insert/insert fluid/extract output/blocked-output/drops/facing for both vats | UNPORTED | `content/apparatus/{fermentationvat,digestionvat}/*.java` | `native/theurgy/machines.rs` | EV=NOT EXECUTED (java ids `fermentation_vat_insert_input`, `fermentation_vat_insert_fluid`, `fermentation_vat_extract_output`, `fermentation_vat_blocked_output_does_not_consume_inputs`, `fermentation_vat_drops_items`, `fermentation_vat_facing`, `digestion_vat_insert_input`, `digestion_vat_insert_fluid`, `digestion_vat_extract_output`, `digestion_vat_blocked_output_does_not_consume_inputs`, `digestion_vat_drops_items`, `digestion_vat_facing`, `digestion_vat_placement`) |

## 4. Sal ammoniac chain (accumulator + tank)

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| SAL-01 | `AccumulationRecipe` `{evaporant?: SizedFluidIngredient, solute?: Ingredient, result: FluidStackTemplate, time?=100}`; `matches` is deliberately asymmetric: a container that *has* a solute must not match a solute-free recipe | UNPORTED | `content/recipe/AccumulationRecipe.java:38-150` | `native/theurgy/engine/src/recipes.rs` | EV=NOT EXECUTED |
| SAL-02 | accumulator `canCraft` requires the **whole** result volume to fit the output tank (dry-run fill) | UNPORTED | `content/apparatus/salammoniacaccumulator/SalAmmoniacAccumulatorCraftingBehaviour.java:70-100` | - | EV=NOT EXECUTED |
| SAL-03 | accumulator `craft` fills output first, consumes solute only when `hasSolute()`, drains evaporant only when `hasEvaporant()` | UNPORTED | `SalAmmoniacAccumulatorCraftingBehaviour.java:102-128` | - | EV=NOT EXECUTED |
| SAL-04 | accumulator input slot + water tank + output tank capacities and sided access | UNPORTED | `SalAmmoniacAccumulatorBlockEntity.java`, `SalAmmoniacAccumulatorCachedCheck.java` | `registry.rs::{container,fluids}` | EV=NOT EXECUTED (java ids `sal_ammoniac_accumulator_placement`, `sal_ammoniac_accumulator_insert_item`, `sal_ammoniac_accumulator_insert_water`) |
| SAL-05 | sal ammoniac tank: single fluid tank (sal_ammoniac only), insert/extract, fluid persists across save/load | UNPORTED | `content/apparatus/salammoniactank/SalAmmoniacTankBlockEntity.java` | `native/theurgy/fluids.rs` | EV=NOT EXECUTED (java ids `sal_ammoniac_tank_placement`, `sal_ammoniac_tank_insert_fluid`, `sal_ammoniac_tank_extract_fluid`, `sal_ammoniac_tank_persistence`) |
| SAL-06 | accumulator/tank renderers (fluid + GeckoLib models) | N/A-CLIENT | `content/apparatus/salammoniac{accumulator,tank}/render/*.java` | `native/theurgy/render.rs` | EV=NOT EXECUTED |

## 5. Mercury flux and energy

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| FLU-01 | `MercuryCatalyst`: capacity 50 000 (`1000*50`); `CatalysationRecipe` `{ingredient, totalMercuryFlux, mercuryFluxPerTick}`; `assemble` is empty (no output item) | UNPORTED | `content/apparatus/mercurycatalyst/MercuryCatalystBlockEntity.java`, `content/recipe/CatalysationRecipe.java:33-140` | `native/theurgy/flux.rs` | EV=NOT EXECUTED (java ids `mercury_catalyst_placement`, `mercury_catalyst_shard_generates_flux`, `mercury_catalyst_shard_is_consumed`, `mercury_catalyst_disabled_still_generates_flux`) |
| FLU-02 | Catalyst tick override: spend leftover `mercuryFluxToConvert` at `currentMercuryFluxPerTick` via `addInternalFlux`, else re-consume an input item; `canCraft` only requires `amount < capacity`; progress = processed/total | UNPORTED | `content/apparatus/mercurycatalyst/MercuryCatalystCraftingBehaviour.java:120-196` | `native/theurgy/flux.rs` | EV=NOT EXECUTED |
| FLU-03 | Catalyst saves `mercuryFluxToConvert`, `totalMercuryFluxToConvert`, `currentMercuryFluxPerTick`; applies `MERCURY_CATALYST_INVENTORY` component on place (rejected placeable must not duplicate) | UNPORTED | `MercuryCatalystCraftingBehaviour.java:60-115` | - | EV=NOT EXECUTED (java id `mercury_catalyst_rejected_placeable_dont_duplicate`) |
| FLU-04 | `MercuryCapacitor`: `CAPACITY = 1000*50*10 = 500 000`, `PUSH_TICK_INTERVAL = 20`, `PUSH_RATE_PER_SIDE_PER_TICK = 2`; defaults UP/DOWN=OUTPUT, horizontals=INPUT, unset=NONE; `cycleSideMode` NONE->INPUT->OUTPUT->BOTH->NONE | UNPORTED | `content/apparatus/mercurycapacitor/MercuryCapacitorBlockEntity.java:44-140`, `SideMode.java` | `native/theurgy/flux.rs` | EV=NOT EXECUTED (java ids `mercury_capacitor_placement`, `mercury_capacitor_starts_empty`, `mercury_capacitor_has_correct_capacity`, `mercury_capacitor_side_modes`) |
| FLU-05 | Capacitor push: only on `gameTime % 20 == 0`, only when `ENABLED`; shuffled directions; `totalToPush = min(amount, 2*20*targets)`, integer split with remainder to the first targets; insert-then-extract inside one transaction | UNPORTED | `MercuryCapacitorBlockEntity.java:174-241` | `native/theurgy/flux.rs` | EV=NOT EXECUTED (java ids `mercury_capacitor_receives_flux`, `mercury_capacitor_disabled_does_not_receive_flux`, `mercury_capacitor_no_ping_pong`) |
| FLU-06 | Side-aware wrappers (`SideAwareMercuryFluxHandler`) preventing INPUT-only sides from being extracted | UNPORTED | `MercuryCapacitorBlockEntity.java:75-100, 260-330` | - | EV=NOT EXECUTED |
| FLU-07 | Capacitor persistence + implicit `MERCURY_FLUX_STORAGE` component round-trip | UNPORTED | `MercuryCapacitorBlockEntity.java:140-172, 260-330` | - | EV=NOT EXECUTED |
| FLU-08 | `MercuryFluxEmitter`: `CAPACITY=1000`, `FLUX_PER_TRANSFER=10`, `TICK_INTERVAL=20`; selected target points; links target, transfers flux | UNPORTED | `content/apparatus/mercuryfluxemitter/MercuryFluxEmitterBlockEntity.java:33-170`, `MercuryFluxEmitterMercuryFluxBehaviour.java`, `MercuryFluxEmitterSelectionBehaviour.java` | `native/theurgy/flux.rs` | EV=NOT EXECUTED (java ids `mercury_flux_emitter_placement`, `mercury_flux_emitter_energy`, `mercury_flux_emitter_linking`, `mercury_flux_emitter_flux_transfer`) |
| FLU-09 | `LogisticsMercuryFluxConnector` buffer/forwarding to one or multiple capacitors | UNPORTED | `content/apparatus/logisticsmercuryfluxconnector/*.java` | `native/theurgy/logistics.rs` | EV=NOT EXECUTED (java ids `mercury_flux_connector_placement`, `_buffer_empty`, `_exposes_capability`, `_forwards_flux`, `_forwards_to_multiple`) |
| FLU-10 | Energy storages (`SimpleTheurgyEnergyHandler`) on emitters; `SimpleMercuryFluxHandler` serialization of amount | UNPORTED | `content/capability/SimpleMercuryFluxHandler.java`, `SimpleTheurgyEnergyHandler.java` | - | EV=NOT EXECUTED |
| FLU-11 | network-only visuals for flux/caloric/sulfuric (`MessageShow*`), selection packet round-trips | N/A-CLIENT | `network/messages/MessageShow{Caloric,Mercury,Sulfuric}Flux*.java`, `Message*Selection.java` | `native/theurgy/render.rs` | EV=NOT EXECUTED |

## 6. Incubator and vessels

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| INC-01 | `IncubationRecipe` `{mercury: Ingredient, salt: Ingredient, sulfur: Ingredient, result: RecipeResult, time?=100}`; `matches` = all three vessel slot-0 stacks test their ingredient | UNPORTED | `content/recipe/IncubationRecipe.java:36-170`, `content/recipe/result/{RecipeResult,ItemRecipeResult,TagRecipeResult}.java` | `native/theurgy/incubation.rs` (placeholder) | EV=NOT EXECUTED |
| INC-02 | `IncubatorRecipeInput` over three vessel inventories; `getTotalTime` falls back to default when input is `null` | UNPORTED | `content/recipe/input/IncubatorRecipeInput.java`, `content/apparatus/incubator/IncubatorCraftingBehaviour.java:78-82` | - | EV=NOT EXECUTED |
| INC-03 | `craft` is a 4-way atomic transaction: full output insert, **then** 1 mercury, 1 salt, 1 sulfur extraction; any shortfall rolls all back | UNPORTED | `IncubatorCraftingBehaviour.java:42-64` | `native/theurgy/incubation.rs` | EV=NOT EXECUTED (java ids `incubator_inputs_consumed_output_produced`, `incubator_blocked_output_does_not_consume_vessel_inputs`) |
| INC-04 | multiblock discovery: scan all 4 horizontal neighbours for `IncubatorMercury/Salt/SulfurVesselBlockEntity`; valid only when all three present; assemble/disassemble swap the recipe input; vessel change calls `onInputItemChanged(EMPTY, EMPTY)` | UNPORTED | `content/apparatus/incubator/IncubatorBlockEntity.java:195-240` | `native/theurgy/incubation.rs` | EV=NOT EXECUTED (java ids `incubator_valid_multiblock`, `incubator_mercury_vessel_connection`, `incubator_salt_vessel_connection`, `incubator_sulfur_vessel_connection`, `incubator_removing_vessel_connection`, `incubator_connection_states_default_false`) |
| INC-05 | `setRemoved` detaches vessels (no stale back-references after break) | UNPORTED | `IncubatorBlockEntity.java:236-250` | `hooks/destroy.rs` | EV=NOT EXECUTED (java ids `incubator_break_lower_destroys_upper`, `incubator_break_upper_destroys_lower`) |
| INC-06 | vessel block item carries vessel inventory component and re-attaches on place | UNPORTED | `Incubator{Mercury,Salt,Sulfur}VesselBlockItem.java`, `Incubator*VesselBlockEntity.java` | `native/theurgy/items.rs` | EV=NOT EXECUTED (java ids `incubator_insert_mercury`, `incubator_insert_salt`, `incubator_insert_sulfur`, `incubator_extract_items`, `incubator_extract_output`, `incubator_processing_starts`, `incubator_stops_without_heat`, `incubator_placement`) |
| INC-07 | three vessel GeckoLib renderers + incubator render | N/A-CLIENT | `content/apparatus/incubator/render/*.java` | `native/theurgy/render.rs` | EV=NOT EXECUTED |

## 7. Reformation array

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| REF-01 | `ReformationRecipe` `{sources: SizedIngredient[], target: Ingredient, result: ItemStackTemplate, mercuryFlux: int, time?=100}`; field order in codec is load-bearing | UNPORTED | `content/recipe/ReformationRecipe.java:36-160` | `native/theurgy/reformation.rs` (placeholder) | EV=NOT EXECUTED |
| REF-02 | `matches`: fail early if flux `< mercuryFlux`; target must match; sources solved as a bipartite assignment where a count>1 SizedIngredient may be split across pedestals and **no pedestal is double-counted** (each matched stack is zeroed) | UNPORTED | `ReformationRecipe.java:92-124` | `native/theurgy/reformation.rs` | EV=NOT EXECUTED |
| REF-03 | `assemble` copies **all target item components** onto the result (upstream TODO: should be a recipe flag — port as is, mark divergence) | UNPORTED | `ReformationRecipe.java:126-138` | - | EV=NOT EXECUTED |
| REF-04 | `SulfuricFluxEmitter`: `CAPACITY = 1000`; `validateMultiblock` requires target pedestal BE, result pedestal BE and >=1 source pedestal BE, else invalid; recompute is edge-triggered by `checkValidMultiblockOnNextQuery`; assemble/disassemble wire pedestals to the emitter | UNPORTED | `content/apparatus/reformationarray/SulfuricFluxEmitterBlockEntity.java:44-200` | `native/theurgy/reformation.rs` | EV=NOT EXECUTED (java ids `sulfuric_flux_emitter_placement`, `sulfuric_flux_emitter_linking`, `sulfuric_flux_emitter_processing`) |
| REF-05 | emitter tick passes `canProcess = true` and `hasInput = valid && hasSourceItems && hasTargetItem`; flux is *not* recharged by the emitter | UNPORTED | `SulfuricFluxEmitterBlockEntity.java:197-201` | - | EV=NOT EXECUTED |
| REF-06 | insufficient mercury flux must consume nothing and output nothing | UNPORTED | `ReformationArrayCraftingBehaviour.java` | `native/theurgy/reformation.rs` | EV=NOT EXECUTED (java id `sulfuric_flux_emitter_insufficient_mercury_flux`) |
| REF-07 | pedestal selection: `SulfuricFluxEmitterSelectionBehaviour` block range, `canCreate` (pedestal type), `getOutputInventory` from the result pedestal, selection save/load of 3 point lists | UNPORTED | `SulfuricFluxEmitterSelectionBehaviour.java:22-120`, `content/behaviour/selection/{SelectionBehaviour,SelectedPoint}.java` | `native/theurgy/reformation.rs`, `hook*use_item.rs` | EV=NOT EXECUTED |
| REF-08 | source/target/result pedestal slots + `setSulfuricFluxEmitter` back-references + content-change callbacks into the emitter | UNPORTED | `content/apparatus/reformationarray/Reformation*Pedestal*.java` | - | EV=NOT EXECUTED (java ids `reformation_source_placement`, `reformation_source_insert`, `reformation_source_rejects_invalid_held_item`, `reformation_source_extract`, `reformation_target_placement`, `reformation_target_insert`, `reformation_result_placement`, `reformation_result_extract`) |
| REF-09 | `DistHelper` projectile + orbital visuals | N/A-CLIENT | `SulfuricFluxEmitterBlockEntity.java:352-406` | `native/theurgy/render.rs` | EV=NOT EXECUTED |

## 8. Logistics

This is the largest subsystem; each row is a separate escape surface for dupes and item/fuid desync.

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| LOG-01 | `LogisticsNetwork`: per-network node set + leaf-node index by capability+frequency; add/remove/track; network identity by pair id | UNPORTED | `logistics/LogisticsNetwork.java:28-90`, `logistics/Logistics.java` | `native/theurgy/logistics.rs` (placeholder) | EV=NOT EXECUTED |
| LOG-02 | `LeafNodeBehaviour`: `frequency`, `enabled`, `filter`, `targets`, global-pos key, save/load, network handling on load/unload/destroy | UNPORTED | `content/behaviour/logistics/LeafNodeBehaviour.java:25-160`, `LeafNodeMode.java` | `native/theurgy/logistics.rs` | EV=NOT EXECUTED |
| LOG-03 | extractor: discovers inserter targets of the same network/frequency, rebuilds capability caches, extracts with filter, distributes via `Distributor` | UNPORTED | `content/behaviour/logistics/ExtractorNodeBehaviour.java:26-200`, `content/behaviour/logistics/distribution/{Distributor,RoundRobinDistributor,DistributionMode}.java` | `native/theurgy/logistics.rs` | EV=NOT EXECUTED (java ids `logistics_item_extractor_targets_in_any_registration_order`, `..._reversed`) |
| LOG-04 | inserter: resolves target capabilities, inserts per filter; connector restores attached target after chunk load | UNPORTED | `content/behaviour/logistics/InserterNodeBehaviour.java:21-90` | `native/theurgy/logistics.rs` | EV=NOT EXECUTED (java ids `logistics_item_connector_restores_attached_target_on_load`, `logistics_fluid_connector_restores_attached_target_on_load`) |
| LOG-05 | item inserter/extractor BEs, slot-per-face wrappers, whitelist of touched faces | UNPORTED | `content/apparatus/logisticsitemconnector/**` | `native/theurgy/logistics.rs` | EV=NOT EXECUTED (java ids `logistics_inserter_placement`, `logistics_extractor_placement`) |
| LOG-06 | fluid inserter/extractor: same rules over fluid capabilities; world-source extraction | UNPORTED | `content/apparatus/logisticsfluidconnector/**` | `native/theurgy/logistics.rs` | EV=NOT EXECUTED (java id `logistics_fluid_extracts_world_source`) |
| LOG-07 | connection node: only relays when it is part of a nexus-linked network | UNPORTED | `content/apparatus/logisticsconnectionnode/LogisticsConnectionNodeBlock.java` | `native/theurgy/logistics.rs` | EV=NOT EXECUTED (java ids `logistics_node_placement`, `logistics_node_only_connects_when_attached_to_nexus`) |
| LOG-08 | nexus: pairing id (`LOGISTICS_NEXUS_ID` UUID), pairs across placement, BE accepts item uuid, portal rendering | UNPORTED | `content/apparatus/logisticsnexus/{LogisticsNexusBlock,LogisticsNexusBlockEntity,LogisticsNexusPairing}.java`, `render/LogisticsNexusRenderer.java` | `native/theurgy/logistics.rs`, `native/theurgy/render.rs` | EV=NOT EXECUTED (java ids `logistics_nexus_pairs_across_placement`, `logistics_nexus_be_accepts_item_uuid`) |
| LOG-09 | capability proxy: forwards item/fluid capabilities, round-robin across multiple sides | UNPORTED | `content/apparatus/logisticscapabilityproxy/*.java`, `content/storage/{CombinedItemStorage,SelectItemHandlerWrapper,SelectFluidHandlerWrapper}.java` | `native/theurgy/logistics.rs` | EV=NOT EXECUTED (java ids `logistics_capability_proxy_placement`, `logistics_capability_proxy_fluid_forwarding`, `logistics_capability_proxy_fluid_round_robin`) |
| LOG-10 | capability probe (debug block) | UNPORTED | `content/apparatus/logisticscapabilityprobe/LogisticsCapabilityProbeBlock.java` | - | EV=NOT EXECUTED |
| LOG-11 | mercury flux connector (see FLU-09) | UNPORTED | `content/apparatus/logisticsmercuryfluxconnector/**` | `native/theurgy/logistics.rs` | EV=NOT EXECUTED |
| LOG-12 | wire graph: `Wires` SavedData (`theurgy.wires`), `Wire` codec, add/remove per pos/chunk, client sync, slack helper | UNPORTED | `logistics/{Wires,Wire,WireEndPoint,WireSlackHelper,WireSync}.java`, `content/item/wire/WireItem.java` | `native/theurgy/logistics.rs` | EV=NOT EXECUTED |
| LOG-13 | `WIRE_END_POINT` component lifecycle (save/load/remove on item) | UNPORTED | `logistics/WireEndPoint.java:21-60` | `native/theurgy/items.rs` | EV=NOT EXECUTED |
| LOG-14 | filters: `ListFilter` (items + fluids, deny-list, respect data components), `AttributeFilter` (+6 attributes), `EmptyFilter`, mode (`FilterMode`), filter item UI, `FilterBehaviour.useItemOn` drop/filter apply | PARTIAL | `content/behaviour/filter/*.java`, `content/behaviour/filter/attribute/*.java` | `native/theurgy/{filter.rs,filter_client.rs,filtered_transfer.rs}` | EV=`runs/game-1794065368573`: real GUI list/attribute edits, ghost add/remove, authoritative NBT; advanced attribute browser remains |
| LOG-15 | wire renderer (`WireRenderer`, `useSimpleWireRenderer`/`wireLineWidth` config) | N/A-CLIENT | `logistics/WireRenderer.java` | `native/theurgy/render.rs` | EV=NOT EXECUTED |
| LOG-16 | node status HUD packets (`MessageShowLogisticsNodeStatus`) and wand-driven reconfiguration | N/A-CLIENT | `network/messages/MessageShowLogisticsNodeStatus.java`, `content/item/mercurialwand/mode/SwitchLogisticsEnabledMode.java`, `SetSelectedFrequencyMode.java`, `SelectFrequencyMode.java` | `native/theurgy/render.rs` | EV=NOT EXECUTED |

## 9. Items and tools

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| ITM-01 | `DivinationRodItem`: defaults `NOT_FOUND=7.0f`, `SEARCHING=8.0f` as item-property floats; per-rod tier/allowed-tag/disallowed-tag/range/duration/durability/allowAttuning | UNPORTED | `content/item/divinationrod/DivinationRodItem.java:53-80`, `content/item/renderer/DivinationDistanceProperty.java` | `native/theurgy/items.rs` (placeholder) | EV=NOT EXECUTED |
| ITM-02 | divination scan: `useOn` linking a block/tag (with size limits), `use` starting a scan, `getOreTagFromBlockId`/`getOreTagFromOreName` via `([a-z]+)_ore` and `([a-z]+_[a-z]+)_ore`, `ServerConfig` `sulfurSourceToBlockMapping` | UNPORTED | `DivinationRodItem.java:88-166, 195-345, 435-460` | `native/theurgy/items.rs`, `config` port | EV=NOT EXECUTED |
| ITM-03 | rod durability (`getMaxDamage`, `onUseTick` damage), `releaseUsing` early cancel, `finishUsingItem` result, `DivinationRodRecipe` attunement | UNPORTED | `DivinationRodItem.java:183-372`, `content/recipe/DivinationRodRecipe.java` | `native/theurgy/items.rs` | EV=NOT EXECUTED (java ids `divination_rod_craft_t1`, `divination_rod_craft_abundant_dirt_sulfur`, `divination_rod_craft_abundant_copper_sulfur`) |
| ITM-04 | `MercurialWandItem`: cycled modes (capacitor side, selected direction, logistics enabled, select/set frequency), scroll/left-click handling, mode component `MERCURIAL_WAND_ITEM_MODE`, per-mode render handlers | UNPORTED | `content/item/mercurialwand/MercurialWandItem.java:27-100`, `mode/*.java`, `content/item/mode/*.java` | `native/theurgy/items.rs` | EV=NOT EXECUTED |
| ITM-05 | sal ammoniac bucket (fluid transport, empty/fill, dispensers) | UNPORTED | `registry/ItemRegistry.java:154-159`, `registry/FluidRegistry.java` | `native/theurgy/items.rs`, `hooks/use_on.rs` | EV=NOT EXECUTED |
| ITM-06 | `WireItem` (max linkage length 32) + `WireEndPoint` component + add/remove wire packets | UNPORTED | `content/item/wire/WireItem.java`, `network/messages/Message{Add,Remove}Wires.java` | `native/theurgy/items.rs` | EV=NOT EXECUTED |
| ITM-07 | filters as items (list/attribute) carrying `FILTER_*` components | UNPORTED | `registry/ItemRegistry.java:150-151`, `content/item/filter/*.java` | `native/theurgy/items.rs` | EV=NOT EXECUTED |
| ITM-08 | `TheHermetica` book item (Modonomicon integration, book data) | UNPORTED | `content/item/book/TheHermeticaItem.java`, `integration/modonomicon/**` | - | EV=NOT EXECUTED |
| ITM-09 | derivative items: sulfur/niter/salt render + `getSourceItem` display | UNPORTED | `content/item/{salt,sulfur,niter}/**`, `content/item/derivative/**` | `native/theurgy/items.rs` | EV=NOT EXECUTED |
| ITM-10 | held-stack-fit provider (`HeldStackFitProvider`, `MessageRequest/SyncHeldStackFit`, outline renderer) | N/A-CLIENT | `content/render/{HeldStackFitProvider,HeldStackFitStatus,HeldStackFitOutline}.java` | `native/theurgy/render.rs` | EV=NOT EXECUTED |

## 10. Recipes, conditions, data generation, content

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| REC-01 | recipe result indirection `RecipeResult`/`ItemRecipeResult`/`TagRecipeResult` and `ConditionRegistry` (incl. `FluidTagEmptyCondition`) | UNPORTED | `content/recipe/result/*.java`, `content/recipe/condition/FluidTagEmptyCondition.java`, `registry/ConditionRegistry.java` | - | EV=NOT EXECUTED |
| REC-02 | 11 recipe providers generate the full recipe set (calcination, distillation, liquefaction, fermentation, digestion, incubation, accumulation, catalysation, reformation, shaped, shapeless, smelting, json) | UNPORTED | `datagen/recipe/*.java` | `tools/build-content.mjs`, `content/theurgy/data` | EV=NOT EXECUTED (`npm run content`) |
| REC-03 | custom shaped/shapeless/smelting recipes keep correct result/ingredient mapping in the ported content | UNPORTED | `datagen/recipe/{ShapedRecipeProvider,ShapelessRecipeProvider,SmeltingRecipeProvider}.java` | `tools/content-check.mjs` | EV=NOT EXECUTED |
| REC-04 | worldgen: sal ammoniac ore configured/placed features + biome modifiers | UNPORTED | `datagen/worldgen/{ConfiguredFeatures,PlacedFeatures,BiomeModifiers,TheurgyRegistries}.java` | `content/theurgy/data` | EV=NOT EXECUTED |
| GEN-01 | loot tables for every block (incl. double-high halves and contents) | UNPORTED | `datagen/loot/TheurgyBlockLootSubProvider.java` | `content/theurgy/data/theurgy/loot_table` | EV=NOT EXECUTED |
| GEN-02 | advancements and The Hermetica book entries (Modonomicon pages for each apparatus page type) | UNPORTED | `datagen/advancement/TheurgyAdvancementSubProvider.java`, `datagen/book/**`, `integration/modonomicon/page/**` | `content/theurgy/data` | EV=NOT EXECUTED |
| GEN-03 | lang files EN (+ tooltip provider) for every item/block/message | UNPORTED | `datagen/lang/{ENUSProvider,TooltipLanguageProvider}.java` | `content/theurgy/assets/theurgy/lang` | EV=NOT EXECUTED |
| GEN-04 | blockstates/models/item models + special model renderers | UNPORTED | `datagen/model/*.java`, `registry/TheurgySpecialModelRenderers.java` | `content/theurgy/assets`, `native/theurgy/render.rs` | EV=NOT EXECUTED |
| GEN-05 | multiblock provider documentation data (`TheurgyMultiblockProvider`) | UNPORTED | `datagen/multiblock/TheurgyMultiblockProvider.java` | - | EV=NOT EXECUTED |

## 11. Config, network, integration, client

| id | behaviour | status | java | rust | evidence |
|---|---|---|---|---|---|
| CFG-01 | `ServerConfig.recipes.sulfurSourceToBlockMapping` (accepts `source=block` and `#tag=#tag`), tooltip handler namespaces | UNPORTED | `config/ServerConfig.java:15-75` | - | EV=NOT EXECUTED |
| CFG-02 | `CommonConfig.Misc` | UNPORTED | `config/CommonConfig.java:9-60` | - | EV=NOT EXECUTED |
| CFG-03 | `ClientConfig.Rendering`: `renderSulfurSourceItem`, `enableItemHUD`, `enableInWorldHUD`, `enableHeldStackFitOutline`, `heldStackFitOutlineRenderMode`, `hudScale` (0.25..3), `wireLineWidth` (1..64), `useSimpleWireRenderer` | N/A-CLIENT | `config/ClientConfig.java:12-90` | - | EV=NOT EXECUTED |
| NET-01 | `Networking`/`MessageHandler` registration set: 18 messages (wire add/remove, selections, flux visuals, HUD request/sync, divination result, mode set, left-click empty, held-stack fit) | UNPORTED | `network/Networking.java`, `network/MessageHandler.java`, `network/messages/*.java` | `native/theurgy/*.rs` + hooks | EV=NOT EXECUTED |
| NET-02 | server authority: no client message may write block state without server validation | UNPORTED | `network/messages/Message*Selection.java`, `MessageSetMode.java` | `native/theurgy/host.rs` | EV=NOT EXECUTED |
| INT-01 | JEI/EMI recipe categories for 8 machine types | N/A-CLIENT | `integration/{jei,emi}/**` | - | EV=NOT EXECUTED |
| INT-02 | Jade provider for mercury flux energy | N/A-CLIENT | `integration/jade/*.java` | - | EV=NOT EXECUTED |
| INT-03 | KubeJS + AlmostUnified + Occultism integration (dummy + impl classes) | UNPORTED | `integration/{kubejs,almostunified,occultism}/**` | - | EV=NOT EXECUTED |
| CLI-01 | item HUD + in-world HUD providers (block title, fluid storage, item storage, crafting progress, catalyst, flux handler) | N/A-CLIENT | `content/render/inworldhud/**`, `content/render/itemhud/**` | `native/theurgy/render.rs` | EV=NOT EXECUTED |
| CLI-02 | particles (`colored_bubble`), outlines, cube model, render types, `ClientTicks` | N/A-CLIENT | `content/particle/**`, `content/render/outliner/**`, `content/render/cube/**` | `native/theurgy/render.rs` | EV=NOT EXECUTED |
| CLI-03 | tooltips (`TooltipHandler`, `TooltipDataProvider`) | N/A-CLIENT | `tooltips/*.java` | - | EV=NOT EXECUTED |

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

## 13. Coverage summary (current, honest)

Counts by status as of this audit. **All numbers below were recomputed from this file** with a
pipe-escape-aware split (`re.split(r'(?<!\\)\|', line)`, 8 cells per row, 0 malformed rows),
not carried over from the previous draft. Re-run that check before publishing any progress figure.

| subsystem | rows | VERIFIED | PORTED-UNVERIFIED | PARTIAL | UNPORTED | N/A-CLIENT |
|---|---|---|---|---|---|---|
| 1 registries (REG) | 13 | 0 | 0 | 0 | 13 | 0 |
| 2 common crafting (CB) | 14 | 0 | 0 | 9 | 5 | 0 |
| 3 machines (HEA/MA) | 29 | 0 | 0 | 0 | 26 | 3 |
| 4 sal ammoniac (SAL) | 6 | 0 | 0 | 0 | 5 | 1 |
| 5 flux (FLU) | 11 | 0 | 0 | 0 | 10 | 1 |
| 6 incubator (INC) | 7 | 0 | 0 | 0 | 6 | 1 |
| 7 reformation (REF) | 9 | 0 | 0 | 0 | 8 | 1 |
| 8 logistics (LOG) | 16 | 0 | 0 | 0 | 14 | 2 |
| 9 items (ITM) | 10 | 0 | 0 | 0 | 9 | 1 |
| 10 recipes/gen (REC/GEN) | 9 | 0 | 0 | 0 | 9 | 0 |
| 11 config/network/integration/client | 11 | 0 | 0 | 0 | 5 | 6 |

Total: **135 checklist rows** (13+14+29+6+11+7+9+16+10+9+11 = 135).
Status totals: `VERIFIED = 0`, `PORTED-UNVERIFIED = 0`, `PARTIAL = 9` (CB-01..CB-08 and CB-11,
all in the pure engine's crafting/tick slice), `UNPORTED = 110`, `N/A-CLIENT = 16`
(9 + 110 + 16 = 135). These counts describe **the checklist's coverage target**, not the port.
The Rust-side column in the tables names the intended owner file; a non-placeholder file there does
not by itself make a row `PARTIAL`, and no `PARTIAL` row is evidence that the Java behaviour matches.

Row-count drift warning from this audit: an earlier draft of this file stated 28 blocks, 32 block
entities and 27 data components; the pinned source has **32 blocks, 22 block entities and 26 data
components**. Registry counts must be recomputed from the Java, never inherited from prose.

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

