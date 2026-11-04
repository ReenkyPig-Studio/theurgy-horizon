// SPDX-License-Identifier: MIT
//
// Real headless integration scenario for the reformation array. Every probe
// runs against the actual native game through horizon-ctl; nothing is faked or
// pre-computed. World: `.local/agent-reformation` / TheurgyReformation.
//
// Recipe source of truth: the pinned data pack under
// `content/theurgy/data/theurgy/recipe/reformation/*.json` (627 recipes, all
// `{mercuryFlux, sources: SizedIngredient[], target, result, time}`). The plan
// pins one absolute fixture so every assertion is an exact number instead of a
// runtime "discovery" that can silently weaken itself:
//   flux 50, time 100, target `theurgy:alchemical_niter_animals_abundant`,
//   two *nested* source entries `{ingredient, count:1}` of
//   `theurgy:alchemical_niter_crops_abundant`, result count 1.
// The nested source entries are deliberate: `SizedIngredient` must be decoded
// from `{ingredient, count}` and one pedestal must never serve both entries.
//
// Contract probed (Java `ReformationRecipe.matches`,
// `ReformationArrayCraftingBehaviour.craft`, `CraftingBehaviour`, and
// `SulfuricFluxEmitterBlockEntity#saveAdditional`):
//   1. flux gate        - fewer units than `mercuryFlux` => nothing consumed
//   2. exact multiset   - every source entry needs its own pedestal holding its
//                         exact `count`; too few items => rollback, never a
//                         partially emptied pedestal
//   3. uniqueness       - one pedestal never satisfies two sources; the target
//                         and the result pedestal are never sources
//   4. missing target   - sources and flux untouched
//   5. missing result   - sources and flux untouched
//   6. output full      - flux and sources roll back, the output stays untouched
//   7. success          - exact flux cost, exact source consumption, target kept
//   8. progress reset   - a blocked craft never accumulates progress and the
//                         timer falls back to the recipe time / DEFAULT_TIME
//   9. save-safe NBT    - flux/progress are persisted in the Java ranges
const EMITTER = 'theurgy:sulfuric_flux_emitter';
const SOURCE = 'theurgy:reformation_source_pedestal';
const TARGET = 'theurgy:reformation_target_pedestal';
const RESULT = 'theurgy:reformation_result_pedestal';

export const RECIPE = Object.freeze({
  id: 'theurgy:alchemical_niter_animals_abundant_from_alchemical_niter_crops_abundant',
  flux: 50,
  time: 100,
  target: 'theurgy:alchemical_niter_animals_abundant',
  source: 'theurgy:alchemical_niter_crops_abundant',
  sources: [1, 1], // SizedIngredient counts, recipe order
  result: 'theurgy:alchemical_niter_animals_abundant',
  resultCount: 1,
});

export const LAYOUT = Object.freeze({
  emitter: [2, 80, 2],
  target: [4, 80, 2],
  result: [2, 80, 0],
  sourceA: [2, 80, 4],
  sourceB: [4, 80, 4],
  sourceC: [0, 80, 4],
});

const S = p => p.map(String);
const LAB = Object.freeze({x: [0, 8], y: [79, 85], z: [0, 8]});
const SETTLE = 40;
/// Any recipe finishes well inside this; the budget also covers a failed
/// attempt retrying from zero more than once (each attempt costs `time` ticks).
const CRAFT_WINDOW = 400;

function state(reply) {
  const v = reply?.state ?? reply?.block ?? '';
  return v && typeof v === 'object' ? String(v.id ?? v.name ?? '') : String(v);
}
function blockId(reply) {
  return state(reply).split('[')[0];
}
function slots(reply) {
  return Array.isArray(reply?.slots) ? reply.slots : null;
}
function inner(v) {
  if (!v) return null;
  return v.item && typeof v.item === 'object' ? v.item : v;
}
function itemAt(reply, i) {
  const x = inner(slots(reply)?.[i]);
  if (!x) return null;
  const id = x.id ?? x.item ?? null;
  return typeof id === 'string' ? id : null;
}
function countAt(reply, i) {
  const x = inner(slots(reply)?.[i]);
  return x ? Number(x.count ?? 0) : 0;
}
function nbt(reply) {
  const v = reply?.nbt ?? reply?.data ?? reply?.tag ?? null;
  if (v && typeof v === 'object') return v;
  const text = Array.isArray(reply?.chat) ? reply.chat.join('\n') : '';
  if (!text.includes('has the following block data:')) return null;
  const out = {};
  for (const key of ['progress', 'totalTime', 'mercuryFlux']) {
    const match = new RegExp('\\b' + key + '\\s*:\\s*(-?\\d+)').exec(text);
    if (match) out[key] = Number(match[1]);
  }
  const handler = /mercuryFluxHandler\s*:\s*\{([^{}]*)\}/.exec(text);
  if (handler) {
    const energy = /\benergy\s*:\s*(-?\d+)/.exec(handler[1]);
    if (energy) out.mercuryFluxHandler = {energy: Number(energy[1])};
  }
  return out;
}
/// Short/byte wrappers serialise as `{type:'short',value:n}`; plain ints as numbers.
function num(tag, key) {
  if (!tag || typeof tag !== 'object') return null;
  const raw = tag[key];
  if (typeof raw === 'number') return raw;
  if (raw && typeof raw === 'object') {
    if (typeof raw.value === 'number') return raw.value;
    if (typeof raw.Value === 'number') return raw.Value;
  }
  return null;
}
function handlerOf(reply) {
  const tag = nbt(reply);
  if (!tag) return null;
  const raw = tag.mercuryFluxHandler;
  const handler = raw && typeof raw === 'object' && typeof raw === 'object' && raw.value && typeof raw.value === 'object'
    ? raw.value
    : raw;
  return handler && typeof handler === 'object' ? handler : null;
}
function fluxOf(reply) {
  const tag = nbt(reply);
  if (!tag) return null;
  const handler = handlerOf(reply);
  if (handler) {
    const amount = num(handler, 'energy');
    if (amount !== null) return {amount, capacity: num(handler, 'capacity')};
  }
  const flat = num(tag, 'mercuryFlux');
  return flat === null ? {amount: 0, capacity: null} : {amount: flat, capacity: null};
}
function progressOf(reply) {
  const tag = nbt(reply);
  if (!tag) return null;
  const progress = num(tag, 'progress');
  const total = num(tag, 'totalTime');
  return progress === null && total === null ? null : {progress, total};
}
const full = id => (String(id).includes(':') ? String(id) : 'minecraft:' + id);
function detail(reply) {
  return JSON.stringify(reply ?? null).slice(0, 240);
}

// ---------------------------------------------------------------------------
// Offline self-test: deterministic, absolute, lab-contained. Runs without a
// game so plan drift is caught before launching the world.
// ---------------------------------------------------------------------------
export function selfTest(plan = reformationPlan()) {
  if (!plan || !Array.isArray(plan.steps)) throw new Error('reformation plan has no steps');
  if (plan.steps.length > 200) throw new Error('reformation plan is not bounded');
  const names = plan.steps.map(s => s.name);
  if (new Set(names).size !== names.length) throw new Error('reformation step names must be unique');
  for (const required of [
    'success crafts exactly once',
    'insufficient flux does not craft',
    'duplicate source pedestal does not craft',
    'nested SizedIngredient count is enforced',
    'missing target pedestal',
    'missing result pedestal',
    'output full rolls back',
    'a partial output stack is topped up',
    'flux survives a save',
    'progress resets after a blocked craft',
  ]) {
    if (!names.some(n => n === required)) throw new Error('reformation invariant missing: ' + required);
  }
  for (const step of plan.steps) {
    for (const arg of step.args || []) {
      if (typeof arg !== 'string') continue;
      if (arg.includes('..') || /[A-Za-z]:\\/.test(arg)) throw new Error('reformation step escapes the lab: ' + step.name);
    }
    if (!step.pos) continue;
    const [x, y, z] = step.pos;
    if (x < LAB.x[0] || x > LAB.x[1] || y < LAB.y[0] || y > LAB.y[1] || z < LAB.z[0] || z > LAB.z[1])
      throw new Error('reformation step outside the lab: ' + step.name);
    if (step.args[0] === 'setblock' || step.args[0] === 'container') {
      const [ax, ay, az] = step.args.slice(1, 4).map(Number);
      if (ax !== x || ay !== y || az !== z) throw new Error('reformation step pos/args disagree: ' + step.name);
    }
  }
  if (RECIPE.flux !== 50 || RECIPE.time !== 100 || RECIPE.sources.length !== 2 || RECIPE.resultCount !== 1)
    throw new Error('pinned reformation fixture drifted from the content pack');
  if (!plan.steps.some(s => s.args[0] === 'data' && String(s.args.at(-1)).includes('mercuryFluxHandler')))
    throw new Error('reformation plan never writes the flux handler NBT');
  return true;
}

// ---------------------------------------------------------------------------
// The plan: pure data. `args` are horizon-ctl arguments; `check(reply, assert)`
// is a pure predicate. The live runner in `runCustom` executes it in order.
// ---------------------------------------------------------------------------
export function reformationPlan() {
  const steps = [];
  const step = s => {
    steps.push(s);
    return s;
  };
  const cmd = (name, args, pos = null) => step({name, args, pos});
  const container = (name, pos, check = null) => step({name, args: ['container', ...S(pos)], pos, check});
  const block = (name, pos) => step({name, args: ['block', ...S(pos)], pos});
  const expectAir = (name, pos) => step({name, args: ['block', ...S(pos)], pos,
    check: (r, assert) => assert.equal(blockId(r), 'minecraft:air', name)});
  const data = (name, pos, check = null) => step({name, args: ['data', 'get', 'block', ...S(pos)], pos, check});
  const put = (name, pos, item, count) => step({name, args: ['container', ...S(pos), full(item), String(count)], pos});
  const setSlot = (name, pos, item, count) => cmd(name,
    ['data', 'merge', 'block', ...S(pos),
      `{Items:[{Slot:0b,id:"${full(item)}",count:${count}}]}`], pos);
  const settle = (name, ticks = SETTLE) => cmd(name, ['wait', String(ticks)]);
  const setFlux = (name, amount) =>
    cmd(name, ['data', 'merge', 'block', ...S(LAYOUT.emitter), `{mercuryFluxHandler:{energy:${amount}}}`],
      LAYOUT.emitter);
  const P = LAYOUT;

  // Assertions used across probes -----------------------------------------
  const expectFlux = (name, amount) =>
    data(name, P.emitter, (r, assert) => {
      const flux = fluxOf(r);
      assert.ok(flux !== null, name + ': emitter exposes no readable flux: ' + detail(r?.nbt ?? r));
      assert.equal(flux.amount, amount, name + ': flux mismatch');
    });
  const expectPedestal = (name, pos, item, count) =>
    container(name, pos, (r, assert) => {
      assert.ok(slots(r) !== null, name + ': pedestal has no readable inventory: ' + detail(r));
      assert.equal(itemAt(r, 0), item === null ? null : full(item), name + ': wrong item in slot 0');
      assert.equal(countAt(r, 0), item === null ? 0 : count, name + ': wrong count in slot 0');
    });
  const expectProgressIdle = name =>
    data(name, P.emitter, (r, assert) => {
      const p = progressOf(r);
      if (p?.progress != null) assert.equal(p.progress, 0, name + ': progress must be zero while idle');
      if (p?.total != null) assert.ok(p.total === RECIPE.time, name + ': total time must stay at the recipe time');
    });
  const clearResult = label => {
    cmd(label + ': clear result pedestal', ['setblock', ...S(P.result), 'minecraft:air'], P.result);
    cmd(label + ': replace result pedestal', ['setblock', ...S(P.result), RESULT], P.result);
  };

  // --- 0. placement -------------------------------------------------------
  cmd('teleport into the lab', ['tp', '3', '82', '3']);
  settle('load chunks');
  cmd('clear the lab', ['fill', '0', '79', '0', '8', '85', '8', 'minecraft:air']);
  cmd('place emitter', ['setblock', ...S(P.emitter), EMITTER], P.emitter);
  block('emitter is placed', P.emitter);
  cmd('place target pedestal', ['setblock', ...S(P.target), TARGET], P.target);
  block('target pedestal is placed', P.target);
  cmd('place result pedestal', ['setblock', ...S(P.result), RESULT], P.result);
  block('result pedestal is placed', P.result);
  cmd('place source pedestal A', ['setblock', ...S(P.sourceA), SOURCE], P.sourceA);
  block('source pedestal A is placed', P.sourceA);
  cmd('place source pedestal B', ['setblock', ...S(P.sourceB), SOURCE], P.sourceB);
  block('source pedestal B is placed', P.sourceB);
  // Every readback above is checked by `runCustom`, which knows the expected id.
  expectPedestal('fresh result pedestal is empty', P.result, null, 0);
  expectPedestal('fresh source pedestal is empty', P.sourceA, null, 0);
  expectFlux('fresh emitter holds no flux', 0);

  // --- 1. exact success ---------------------------------------------------
  setSlot('target plaza holds the target item', P.target, RECIPE.target, 1);
  setSlot('source A holds one source item', P.sourceA, RECIPE.source, RECIPE.sources[0]);
  setSlot('source B holds one source item', P.sourceB, RECIPE.source, RECIPE.sources[1]);
  setFlux('flux is set above the recipe cost', RECIPE.flux + 7);
  cmd('craft window', ['wait', String(CRAFT_WINDOW)]);
  expectPedestal('success crafts exactly once', P.result, RECIPE.result, RECIPE.resultCount);
  expectFlux('success consumed the exact flux', 7);
  expectPedestal('success consumed source A exactly', P.sourceA, null, 0);
  expectPedestal('success consumed source B exactly', P.sourceB, null, 0);
  expectPedestal('the target is never consumed', P.target, RECIPE.target, 1);
  expectProgressIdle('progress is zero after a craft');

  // --- 2. flux gate -------------------------------------------------------
  clearResult('flux gate');
  setSlot('flux gate: source A refilled', P.sourceA, RECIPE.source, RECIPE.sources[0]);
  setSlot('flux gate: source B refilled', P.sourceB, RECIPE.source, RECIPE.sources[1]);
  setFlux('flux is one unit short', RECIPE.flux - 1);
  cmd('flux gate window', ['wait', String(CRAFT_WINDOW)]);
  expectPedestal('insufficient flux does not craft', P.result, null, 0);
  expectFlux('insufficient flux is not consumed', RECIPE.flux - 1);
  expectPedestal('insufficient flux keeps source A', P.sourceA, RECIPE.source, RECIPE.sources[0]);
  expectPedestal('insufficient flux keeps source B', P.sourceB, RECIPE.source, RECIPE.sources[1]);
  expectProgressIdle('progress resets while the flux gate blocks');

  // --- 3. one pedestal cannot satisfy two source entries ------------------
  cmd('duplicate: remove source B', ['setblock', ...S(P.sourceB), 'minecraft:air'], P.sourceB);
  settle('duplicate: let the array rebuild');
  setSlot('duplicate: source A holds two items', P.sourceA, RECIPE.source, 2);
  setFlux('duplicate: plenty of flux', RECIPE.flux * 2);
  cmd('duplicate window', ['wait', String(CRAFT_WINDOW)]);
  expectPedestal('duplicate source pedestal does not craft', P.result, null, 0);
  expectPedestal('duplicate probe leaves source A intact', P.sourceA, RECIPE.source, 2);
  expectFlux('duplicate probe consumes no flux', RECIPE.flux * 2);

  // --- 4. exact multiset: nested SizedIngredient count is not ignored -----
  clearResult('exact');
  setSlot('exact: target plaza refilled', P.target, RECIPE.target, 1);
  setSlot('exact: source A holds exactly one item', P.sourceA, RECIPE.source, RECIPE.sources[0]);
  cmd('exact: restore source B', ['setblock', ...S(P.sourceB), SOURCE], P.sourceB);
  setSlot('exact: source B holds one item', P.sourceB, RECIPE.source, RECIPE.sources[1]);
  settle('exact: let the array rebuild');
  // Two entries, two pedestals, one item each => the recipe matches and must
  // consume exactly one item from each. (A single item in each pedestal is the
  // exact multiset; a matcher that only compares ids would also pass here, so
  // this probe is paired with the duplicate probe above and probe 5 below.)
  cmd('exact window', ['wait', String(CRAFT_WINDOW)]);
  expectPedestal('exact multiset crafts on both pedestals', P.result, RECIPE.result, RECIPE.resultCount);
  expectPedestal('exact multiset empties source A', P.sourceA, null, 0);
  expectPedestal('exact multiset empties source B', P.sourceB, null, 0);
  expectFlux('exact multiset spends one recipe worth of flux', RECIPE.flux * 2 - RECIPE.flux);

  // --- 5. a pedestal holding too few items rolls back ---------------------
  clearResult('short');
  setSlot('short: source A holds only one item', P.sourceA, RECIPE.source, 1);
  cmd('short: remove source B', ['setblock', ...S(P.sourceB), 'minecraft:air'], P.sourceB);
  settle('short: let the array rebuild');
  setFlux('short: known flux before refusal', RECIPE.flux * 2);
  // validation and leave source A untouched (Java never empties a pedestal it
  // could not fully extract from).
  cmd('short window', ['wait', String(CRAFT_WINDOW)]);
  expectPedestal('nested SizedIngredient count is enforced', P.result, null, 0);
  expectPedestal('a short source pedestal is not emptied', P.sourceA, RECIPE.source, 1);
  expectFlux('a short source pedestal spends no flux', RECIPE.flux * 2);

  // --- 6. missing target pedestal -----------------------------------------
  cmd('missing target: restore source B', ['setblock', ...S(P.sourceB), SOURCE], P.sourceB);
  cmd('missing target: break target pedestal', ['setblock', ...S(P.target), 'minecraft:air'], P.target);
  settle('missing target: let the array rebuild');
  setSlot('missing target: source A refilled', P.sourceA, RECIPE.source, RECIPE.sources[0]);
  setSlot('missing target: source B refilled', P.sourceB, RECIPE.source, RECIPE.sources[1]);
  cmd('missing target window', ['wait', String(CRAFT_WINDOW)]);
  expectPedestal('missing target pedestal', P.result, null, 0);
  expectPedestal('missing target keeps source A', P.sourceA, RECIPE.source, RECIPE.sources[0]);
  expectPedestal('missing target keeps source B', P.sourceB, RECIPE.source, RECIPE.sources[1]);
  expectFlux('missing target consumes no flux', RECIPE.flux * 2);

  // --- 7. empty target ----------------------------------------------------
  cmd('empty target: replace target pedestal', ['setblock', ...S(P.target), TARGET], P.target);
  settle('empty target: let the array rebuild');
  cmd('empty target window', ['wait', String(CRAFT_WINDOW)]);
  expectPedestal('empty target does not craft', P.result, null, 0);
  expectPedestal('empty target keeps source A', P.sourceA, RECIPE.source, RECIPE.sources[0]);
  expectFlux('empty target consumes no flux', RECIPE.flux * 2);

  // --- 8. missing result pedestal -----------------------------------------
  setSlot('missing result: target plaza refilled', P.target, RECIPE.target, 1);
  cmd('missing result: break result pedestal', ['setblock', ...S(P.result), 'minecraft:air'], P.result);
  settle('missing result: let the array rebuild');
  cmd('missing result window', ['wait', String(CRAFT_WINDOW)]);
  expectAir('missing result pedestal', P.result);
  expectPedestal('missing result keeps source A', P.sourceA, RECIPE.source, RECIPE.sources[0]);
  expectPedestal('missing result keeps source B', P.sourceB, RECIPE.source, RECIPE.sources[1]);
  expectFlux('missing result consumes no flux', RECIPE.flux * 2);

  // --- 9. output full rolls everything back -------------------------------
  cmd('output full: replace result pedestal', ['setblock', ...S(P.result), RESULT], P.result);
  settle('output full: let the array rebuild');
  setSlot('output full: result pedestal filled to the limit', P.result, 'minecraft:cobblestone', 64);
  cmd('output full window', ['wait', String(CRAFT_WINDOW)]);
  expectPedestal('output full rolls back', P.result, 'minecraft:cobblestone', 64);
  expectFlux('output full consumes no flux', RECIPE.flux * 2);
  expectPedestal('output full keeps source A', P.sourceA, RECIPE.source, RECIPE.sources[0]);
  expectPedestal('output full keeps source B', P.sourceB, RECIPE.source, RECIPE.sources[1]);
  data('progress resets after a blocked craft', P.emitter, (r, assert) => {
    const p = progressOf(r);
    if (p?.progress != null) assert.equal(p.progress, 0, 'a full output must never leave progress behind');
    if (p?.total != null) assert.ok(p.total === RECIPE.time, 'total time must stay at the recipe time');
  });
  expectProgressIdle('flux and progress stay idle after the rollback');

  // --- 10. an output that still fits is topped up -------------------------
  clearResult('fit');
  setSlot('fit: result pedestal has room for one more', P.result, RECIPE.result, 63);
  setFlux('fit: exactly enough flux', RECIPE.flux);
  cmd('fit window', ['wait', String(CRAFT_WINDOW)]);
  expectPedestal('a partial output stack is topped up', P.result, RECIPE.result, 64);
  expectFlux('topping up spends exactly one recipe worth of flux', 0);
  expectPedestal('fit: source A consumed', P.sourceA, null, 0);
  expectPedestal('fit: source B consumed', P.sourceB, null, 0);

  // --- 11. a foreign stack in the output never merges ---------------------
  clearResult('foreign');
  setSlot('foreign: source A refilled', P.sourceA, RECIPE.source, RECIPE.sources[0]);
  setSlot('foreign: source B refilled', P.sourceB, RECIPE.source, RECIPE.sources[1]);
  setSlot('foreign: result pedestal holds one cobblestone', P.result, 'minecraft:cobblestone', 1);
  setFlux('foreign: flux available', RECIPE.flux * 2);
  cmd('foreign window', ['wait', String(CRAFT_WINDOW)]);
  expectPedestal('a foreign output stack is not merged into', P.result, 'minecraft:cobblestone', 1);
  expectFlux('foreign output wastes no flux', RECIPE.flux * 2);
  expectPedestal('foreign output keeps source A', P.sourceA, RECIPE.source, RECIPE.sources[0]);

  // --- 12. save-safe NBT --------------------------------------------------
  cmd('save: clear the result pedestal', ['setblock', ...S(P.result), 'minecraft:air'], P.result);
  setFlux('save: park flux at 250', 250);
  cmd('save the world', ['save']);
  data('flux survives a save', P.emitter, (r, assert) => {
    const flux = fluxOf(r);
    assert.ok(flux !== null, 'the emitter exposes no readable flux after a save: ' + detail(r?.nbt ?? r));
    assert.equal(flux.amount, 250, 'flux must be persisted exactly');
    if (flux.capacity !== null) {
      assert.equal(flux.capacity, 1000, 'the emitter capacity must be persisted');
    }
    const p = progressOf(r);
    if (p?.progress != null) {
      assert.ok(Number.isInteger(p.progress) && p.progress >= 0 && p.progress <= 32767,
        'saved progress must be a non-negative Java short, got ' + p.progress);
    }
    if (p?.total != null) {
      assert.ok(Number.isInteger(p.total) && p.total >= 0 && p.total <= 32767,
        'saved totalTime must be a non-negative Java short, got ' + p.total);
    }
  });
  expectFlux('saved flux is readable again', 250);
  block('emitter survives the save', P.emitter);
  block('target pedestal survives the save', P.target);

  return {recipe: RECIPE, layout: LAYOUT, steps};
}

// ---------------------------------------------------------------------------
// Live runner.
// ---------------------------------------------------------------------------
const EXPECTED_BLOCKS = new Map([
  ['emitter is placed', EMITTER],
  ['target pedestal is placed', TARGET],
  ['result pedestal is placed', RESULT],
  ['source pedestal A is placed', SOURCE],
  ['source pedestal B is placed', SOURCE],
  ['emitter survives the save', EMITTER],
  ['target pedestal survives the save', TARGET],
]);

export async function runCustom(call, assert) {
  const rawCall = call;
  call = async argv => rawCall(argv[0] === 'data' ? ['command', ...argv] : argv);
  const plan = reformationPlan();
  selfTest(plan);
  const results = [];
  for (const step of plan.steps) {
    const reply = await call(step.args);
    const expected = EXPECTED_BLOCKS.get(step.name);
    if (expected) assert.equal(blockId(reply), expected, step.name + ': unexpected block ' + detail(reply));
    if (typeof step.check === 'function') step.check(reply, assert);
    results.push({name: step.name, args: step.args, reply});
    // Block/inventory writes land on the integrated server; give the headless
    // client a tick pair before the next read-back is meaningful.
    if (step.args[0] === 'setblock' || step.args[0] === 'container' || step.args[0] === 'data') {
      await call(['wait', '2']);
    }
  }
  return results;
}
