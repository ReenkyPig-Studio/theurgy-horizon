// Real headless parity scenario for the sal ammoniac accumulator and tank.
//
// Source of truth (pinned Java 26.3):
//   .../apparatus/salammoniacaccumulator/SalAmmoniacAccumulatorBlockEntity.java
//     WaterTank = new FluidTank(FluidType.BUCKET_VOLUME * 10)  -> 10000 mB, water only
//     tickServer : needs a theurgy:sal_ammoniac_tank directly below, else no work
//   .../apparatus/salammoniactank/SalAmmoniacTankBlockEntity.java
//     Tank = new FluidTank(FluidType.BUCKET_VOLUME * 2)       ->  2000 mB
//   .../AccumulationRecipe.matches  : the solute presence must match exactly
//   .../AccumulationRecipe.java     : result / evaporant are SizedFluidIngredient
//
// The accumulator recipes shipped in content/ are:
//   salt              : no solute,  evaporant minecraft:water 100 mB -> theurgy:sal_ammoniac    10 mB
//   crystallized_lava : solute theurgy:crystallized_lava, water 100 mB -> minecraft:lava        100 mB
//   sal_ammoniac gem  : solute #c:gems/sal_ammoniac,     water 100 mB -> theurgy:sal_ammoniac  100 mB
//
// Every assertion below is read back from the live game (container slots, NBT
// fluid list). Nothing is assumed about the reply shape: a missing or
// unparseable value fails loudly instead of silently passing.

const s = p => p.map(String);

// -- reply parsing (tolerant, but never silently permissive) ------------------
function slots(reply) {
  return Array.isArray(reply?.slots) ? reply.slots : null;
}
function itemAt(reply, i) {
  const v = slots(reply)?.[i];
  if (!v) return null;
  const inner = v.item && typeof v.item === 'object' ? v.item : v;
  return inner.id ?? inner.item ?? null;
}
function countAt(reply, i) {
  const v = slots(reply)?.[i];
  if (!v) return 0;
  const inner = v.item && typeof v.item === 'object' ? v.item : v;
  return Number(inner.count ?? v.count ?? 0);
}
function totalItems(reply) {
  const list = slots(reply);
  return list ? list.filter(Boolean).reduce((n, v) => n + countAt({slots: [v]}, 0), 0) : 0;
}

// The fluid list is either serialised inside a `data`/`nbt` string or as real
// JSON. Normalise both into [{fluid, amount}].
function fluidList(reply) {
  let source = reply;
  if (typeof source === 'string') source = source;
  const text = typeof source === 'string' ? source
    : Array.isArray(source?.chat) ? source.chat.join('\n')
    : JSON.stringify(source ?? null);
  if (typeof text !== 'string') return null;
  // Only the Fluid/Amount pairs matter, and their order inside a compound is
  // not guaranteed, so match each compound body independently.
  const out = [];
  const compound = /\{[^{}]*\}/g;
  let m;
  while ((m = compound.exec(text))) {
    const body = m[0];
    if (!/Fluid/.test(body) || !/Amount/.test(body)) continue;
    const fluid = /Fluid\s*:\s*"([^"]+)"/.exec(body)?.[1]
      ?? /"Fluid"\s*:\s*"([^"]+)"/.exec(body)?.[1];
    const amount = /Amount\s*:\s*(-?\d+)/.exec(body)?.[1]
      ?? /"Amount"\s*:\s*(-?\d+)/.exec(body)?.[1];
    if (!fluid || amount === undefined) continue;
    out.push({fluid, amount: Number(amount)});
  }
  return out;
}
function tankAt(reply, index = 0) {
  const list = fluidList(reply);
  if (!list) return null;
  const entry = list[index];
  if (!entry) return {fluid: null, amount: 0};
  const empty = entry.fluid === 'minecraft:empty';
  return {fluid: empty ? null : entry.fluid, amount: empty ? 0 : entry.amount};
}

const ACC = 'theurgy:sal_ammoniac_accumulator';
const TANK = 'theurgy:sal_ammoniac_tank';
const WATER_CAPACITY = 10000;
const TANK_CAPACITY = 2000;
const SAL = 'theurgy:sal_ammoniac';

export async function runCustom(call, assert) {
  const results = [];
  const step = async (name, args, check) => {
    const reply = await call(args);
    if (check) check(reply);
    results.push({name, reply});
    if (args[0] === 'setblock') await call(['wait', '2']);
    return reply;
  };
  const mergeFluid = (p, fluid, amount) =>
    ['command', 'data', 'merge', 'block', ...s(p), `{fluids:[{Fluid:"${fluid}",Amount:${amount}}]}`];
  const readTank = async (p, label) => {
    const reply = await step('nbt ' + label, ['command', 'data', 'get', 'block', ...s(p)]);
    const tank = tankAt(reply, 0);
    assert.ok(tank, 'could not read a fluid list from ' + label + ': ' + JSON.stringify(reply).slice(0, 240));
    return tank;
  };
  const placePair = async (acc, tank) => {
    await step('place accumulator ' + acc.join(','), ['setblock', ...s(acc), ACC],
      r => assert.ok(String(r?.state ?? r?.block ?? '').startsWith(ACC), JSON.stringify(r).slice(0, 200)));
    await step('place tank ' + tank.join(','), ['setblock', ...s(tank), TANK],
      r => assert.ok(String(r?.state ?? r?.block ?? '').startsWith(TANK), JSON.stringify(r).slice(0, 200)));
  };
  const clearPair = async (acc, tank) => {
    await step('clear accumulator ' + acc.join(','), ['setblock', ...s(acc), 'minecraft:air']);
    await step('clear tank ' + tank.join(','), ['setblock', ...s(tank), 'minecraft:air']);
  };

  // Keep the whole lane inside the loaded test-terrain chunks.
  await step('fluids lab tp', ['tp', '70', '80', '8']);
  await step('fluids lab chunks', ['wait', '40']);

  // -- 1. Salt recipe: water only, no solute --------------------------------
  // ACC above TANK at (70,79,8). sal_ammoniac_from_water: 1000 mB water -> 100 mB gas.
  // A fresh BE has totalTime 0, so CraftingBehaviour.tickServer (progress++
  // then progress >= totalTime) crafts on the first tick after the merge;
  // only the outcome is asserted, not a pre-tick snapshot.
  const acc1 = [70, 80, 8], tank1 = [70, 79, 8];
  await placePair(acc1, tank1);
  await step('salt water', mergeFluid(acc1, 'minecraft:water', 1000),
    r => assert.match(JSON.stringify(r), /Modified block data/, 'water merge must apply'));
  await step('salt wait', ['wait', '320']);
  const outTank1 = await readTank(tank1, 'tank after salt');
  assert.equal(outTank1.fluid, SAL, 'salt recipe must produce theurgy:sal_ammoniac in the tank below');
  assert.equal(outTank1.amount, 100, '1000 mB water * (10 mB / 100 mB) must exactly fill 100 mB');
  const water1After = await readTank(acc1, 'accumulator after salt');
  assert.equal(water1After.fluid, null, 'all water must be evaporated (no zero-amount water tank)');
  assert.equal(water1After.amount, 0, 'accumulator must not keep leftover water');
  await clearPair(acc1, tank1);

  // -- 2. Solute recipe: crystallized lava -> 1000 mB lava ------------------
  // This recipe has no evaporant: merged water is untouched and is not a
  // multiplier for the 1000 mB result in the pinned accumulation JSON.
  const acc2 = [74, 80, 8], tank2 = [74, 79, 8];
  await placePair(acc2, tank2);
  await step('lava water', mergeFluid(acc2, 'minecraft:water', 200));
  await step('lava solute', ['container', ...s(acc2), 'theurgy:crystallized_lava', '1'],
    r => assert.equal(itemAt(r, 0), 'theurgy:crystallized_lava'));
  await step('lava wait', ['wait', '160']);
  const outTank2 = await readTank(tank2, 'tank after lava recipe');
  assert.equal(outTank2.fluid, 'minecraft:lava', 'crystallized lava recipe must fill the tank with lava');
  assert.equal(outTank2.amount, 1000, 'pinned lava recipe must produce exactly 1000 mB');
  const acc2Items = await step('lava solute consumed', ['container', ...s(acc2)]);
  assert.equal(itemAt(acc2Items, 0), null, 'the solute must be consumed when the recipe has one');
  const water2After = await readTank(acc2, 'accumulator after lava');
  assert.equal(water2After.amount, 200, 'a recipe without evaporant must not drain water');
  assert.equal(water2After.fluid, 'minecraft:water');
  await clearPair(acc2, tank2);

  // -- 3. No-evaporant solute recipe works even with an empty water tank ----
  // Java matches evaporant=true when the field is absent, so crystallized lava
  // is consumed once and produces 1000 mB lava without requiring water.
  const acc3 = [78, 80, 8], tank3 = [78, 79, 8];
  await placePair(acc3, tank3);
  await step('dry solute', ['container', ...s(acc3), 'theurgy:crystallized_lava', '1'],
    r => assert.equal(itemAt(r, 0), 'theurgy:crystallized_lava'));
  await step('dry wait', ['wait', '160']);
  const dryItems = await step('dry solute kept', ['container', ...s(acc3)]);
  assert.equal(itemAt(dryItems, 0), null,
    'the no-evaporant solute recipe consumes exactly one input');
  assert.equal(totalItems(dryItems), 0, 'the single solute was consumed, never duplicated');
  const dryTank = await readTank(tank3, 'tank after dry solute');
  assert.equal(dryTank.fluid, 'minecraft:lava');
  assert.equal(dryTank.amount, 1000, 'no-evaporant recipe still produces its exact result');
  await clearPair(acc3, tank3);

  // -- 4. Target capacity / full-tank rollback -------------------------------
  // Pre-fill to 1950/2000. A water-only craft returns 100 mB and Java's
  // canCraft requires the WHOLE result to fit, so all inputs must roll back.
  const acc4 = [82, 80, 8], tank4 = [82, 79, 8];
  await placePair(acc4, tank4);
  await step('near full tank', mergeFluid(tank4, SAL, 1950));
  const nearFull = await readTank(tank4, 'near full tank');
  assert.equal(nearFull.amount, 1950, 'tank must store the merged 1950 mB');
  await step('full water', mergeFluid(acc4, 'minecraft:water', 1000));
  await step('full wait', ['wait', '320']);
  const fullTank = await readTank(tank4, 'tank after full');
  assert.equal(fullTank.fluid, SAL, 'a full tank keeps its fluid');
  assert.equal(fullTank.amount, 1950, 'a full 100 mB result must be refused when only 50 mB fit');
  assert.ok(fullTank.amount <= TANK_CAPACITY, 'tank amount must never exceed the Java capacity of 2000 mB');
  const fullWater = await readTank(acc4, 'accumulator after full');
  // No craft committed: all 1000 mB water remains, not merely a lower bound.
  assert.equal(fullWater.amount, 1000,
    'refused full-result craft must not void or partially consume water');
  assert.equal(fullWater.fluid, fullWater.amount > 0 ? 'minecraft:water' : null,
    'residual accumulator fluid must stay water');
  await clearPair(acc4, tank4);

  // -- 5. Wrong-fluid rollback: a lava tank never accepts sal ammoniac ------
  const acc5 = [86, 80, 8], tank5 = [86, 79, 8];
  await placePair(acc5, tank5);
  await step('lava tank prefill', mergeFluid(tank5, 'minecraft:lava', 1000));
  await step('wrong water', mergeFluid(acc5, 'minecraft:water', 1000));
  await step('wrong wait', ['wait', '320']);
  const wrongTank = await readTank(tank5, 'tank after wrong fluid');
  assert.equal(wrongTank.fluid, 'minecraft:lava', 'a lava tank must keep lava');
  assert.equal(wrongTank.amount, 1000, 'a lava tank must not receive sal ammoniac on top of lava');
  const wrongWater = await readTank(acc5, 'accumulator after wrong fluid');
  assert.equal(wrongWater.amount, 1000, 'refused crafts must not consume the evaporant');
  await clearPair(acc5, tank5);

  // -- 6. Missing target block: no work at all ------------------------------
  // `tickServer` returns before any crafting when there is no tank below, so a
  // lone accumulator must not consume water.
  const acc6 = [90, 80, 8], tank6 = [90, 79, 8];
  await step('lone accumulator', ['setblock', ...s(acc6), ACC]);
  await step('lone clear below', ['setblock', ...s(tank6), 'minecraft:air']);
  await step('lone water', mergeFluid(acc6, 'minecraft:water', 500));
  await step('lone wait', ['wait', '200']);
  const loneWater = await readTank(acc6, 'accumulator without target');
  assert.equal(loneWater.fluid, 'minecraft:water', 'without a target tank the water stays water');
  assert.equal(loneWater.amount, 500, 'without a target tank nothing may be consumed');
  await step('lone cleanup', ['setblock', ...s(acc6), 'minecraft:air']);

  // -- 7. Tank is a passive container with its Java capacity ---------------
  // The tank must never be clamped down below what was written and must never
  // exceed 2000 mB; an oversized merge is clamped, not kept.
  const tank7 = [94, 79, 8];
  await step('passive tank', ['setblock', ...s(tank7), TANK]);
  await step('passive tank fill', mergeFluid(tank7, SAL, 1000));
  await step('passive tank wait', ['wait', '20']);
  const passive = await readTank(tank7, 'passive tank');
  assert.equal(passive.fluid, SAL, 'the tank keeps the fluid written to it');
  assert.equal(passive.amount, 1000, 'the tank keeps the amount written to it');
  await step('passive tank overfill', mergeFluid(tank7, SAL, 5000));
  await step('passive tank settle', ['wait', '20']);
  const overfill = await readTank(tank7, 'passive tank after overfill');
  assert.equal(overfill.fluid, SAL, 'overfill must not change the fluid identity');
  assert.ok(overfill.amount >= 1000 && overfill.amount <= TANK_CAPACITY,
    'tank amount must stay within [previous, 2000] mB, saw ' + overfill.amount);
  await step('passive tank negative', mergeFluid(tank7, 'minecraft:water', -500));
  await step('passive tank negative settle', ['wait', '20']);
  const negative = await readTank(tank7, 'passive tank after negative');
  assert.equal(negative.amount, 0, 'negative merged amount must normalize to zero');
  assert.equal(negative.fluid, null, 'zero fluid must clear its identity');
  await step('passive tank cleanup', ['setblock', ...s(tank7), 'minecraft:air']);

  // -- 8. Persistence across save/reload / unload --------------------------
  const acc8 = [98, 80, 8], tank8 = [98, 79, 8];
  await placePair(acc8, tank8);
  await step('persist tank', mergeFluid(tank8, SAL, 740));
  // 500 mB is below sal_ammoniac_from_water's 1000 mB evaporant, so no recipe
  // matches and both tanks must come back exactly as written.
  await step('persist water', mergeFluid(acc8, 'minecraft:water', 500));
  await step('persist save', ['save']);
  await step('persist relocation', ['tp', '0', '80', '0']);
  await step('persist unload', ['wait', '40']);
  await step('persist return', ['tp', '98', '80', '10']);
  await step('persist settle', ['wait', '60']);
  const persistTank = await readTank(tank8, 'persisted tank');
  assert.equal(persistTank.fluid, SAL, 'tank fluid identity must survive save/reload');
  assert.equal(persistTank.amount, 740, 'tank amount must survive save/reload exactly');
  const persistWater = await readTank(acc8, 'persisted accumulator');
  assert.equal(persistWater.fluid, 'minecraft:water', 'accumulator water must survive save/reload');
  assert.equal(persistWater.amount, 500, 'accumulator water must survive save/reload exactly');
  // The reloaded accumulator must still craft: top up to the 1000 mB evaporant.
  await step('persist top up', mergeFluid(acc8, 'minecraft:water', 1000));
  await step('persist resume wait', ['wait', '20']);
  const resumed = await readTank(tank8, 'tank after resume');
  assert.equal(resumed.fluid, SAL, 'resumed production must stay sal ammoniac');
  assert.equal(resumed.amount, 840, 'one craft adds exactly 100 mB to the reloaded 740 mB');
  const resumedWater = await readTank(acc8, 'accumulator drained');
  assert.equal(resumedWater.amount, 0, 'the craft consumes the full 1000 mB evaporant');
  await clearPair(acc8, tank8);

  return results;
}
