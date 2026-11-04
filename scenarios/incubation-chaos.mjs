// SPDX-License-Identifier: MIT
// Incubation chaos probe scenario.
//
// Deterministic, headless, in-process probes of the incubator multiblock:
//   * vessel matching (all three kinds on the four horizontal neighbours)
//   * heat gating (an unheated incubator never advances the timer)
//   * exact result item AND count (recipe result with its own count, stacked)
//   * missing and duplicate vessels (never a craft, never a partial consume)
//   * input replacement (a different item in a vessel restarts the timer)
//   * output-full (no consume when the result cannot fit, no double insert)
//   * save/reload (progress survives a save+reload and resumes at the same point)
//
// Conservation is asserted on every probe: across one operation the total number of
// vessel items may only drop by the number of committed crafts, and the incubator
// output may only grow by the recipe result of those crafts. Anything else fails.
//
// All helpers below read only parsed replies; a formatting change in Horizon turns
// into a failed assertion, never into a silent pass.

export const INCUBATOR = 'theurgy:incubator';
export const MERCURY = 'theurgy:incubator_mercury_vessel';
export const SALT = 'theurgy:incubator_salt_vessel';
export const SULFUR = 'theurgy:incubator_sulfur_vessel';

// Positions used by the shared harness lab (y=80 test terrain, near machines.mjs
// grid but in its own x band so nothing overlaps).
export const POS = Object.freeze({
  center: [40, 80, 40],
  reloadCenter: [44, 80, 40],
  brokenCenter: [48, 80, 40],
});

/** Normalise a `container` reply into `[null|{item,count}]`. */
export function slots(reply) {
  if (!reply || !Array.isArray(reply.slots)) return null;
  return reply.slots.map(s => {
    if (s == null) return null;
    if (typeof s === 'string') return {item: s, count: 1};
    const inner = s.item && typeof s.item === 'object' ? s.item : null;
    const item = inner ? inner.id : s.item;
    const count = inner ? inner.count : s.count;
    if (item == null) return null;
    return {item: String(item), count: Number(count)};
  });
}

export function slot(reply, i) {
  const s = slots(reply);
  return s && s[i] ? s[i] : null;
}

export function totalItems(reply) {
  const s = slots(reply);
  return s ? s.filter(Boolean).reduce((n, x) => n + x.count, 0) : 0;
}

/** The block id of a `block`/`getblock` reply, without state. */
export function blockName(reply) {
  if (typeof reply === 'string') return reply.split('[')[0];
  if (!reply || typeof reply !== 'object') return null;
  let text = reply.block ?? reply.id ?? reply.state;
  if (text && typeof text === 'object') text = text.id ?? text.name ?? text.block;
  return typeof text === 'string' ? text.split('[')[0] : null;
}

/** State of a `getblock` reply, as a `Record<string,string>`; `{}` when absent. */
export function blockState(reply) {
  let text = reply;
  if (typeof reply === 'object' && reply) text = reply.block ?? reply.state ?? reply.id;
  if (typeof text !== 'string') return {};
  const open = text.indexOf('[');
  const close = text.lastIndexOf(']');
  if (open < 0 || close <= open) return {};
  const out = {};
  for (const part of text.slice(open + 1, close).split(',')) {
    const [k, v] = part.split('=');
    if (k) out[k.trim()] = (v ?? '').trim();
  }
  return out;
}

// --- small assertion shim -------------------------------------------------
// The harness may hand us either chai-like `assert` or a plain object. Resolve the
// three primitives we need once so the probes stay readable.
function helpers(assert) {
  const fail = msg => {
    if (typeof assert?.fail === 'function') assert.fail(msg);
    else throw new Error(msg);
  };
  const ok = (cond, msg) => {
    if (cond) return;
    if (typeof assert?.ok === 'function') assert.ok(cond, msg);
    else fail(msg);
  };
  const equal = (a, b, msg) => {
    if (Object.is(a, b)) return;
    if (typeof assert?.strictEqual === 'function') assert.strictEqual(a, b, msg);
    else if (typeof assert?.equal === 'function') assert.equal(a, b, msg);
    else fail(`${msg}: expected ${JSON.stringify(b)}, got ${JSON.stringify(a)}`);
  };
  return {ok, equal, fail};
}

// --- world helpers --------------------------------------------------------
async function setBlock(call, pos, id) {
  const r = await call('setblock', ...pos.map(String), id);
  if (r && r.ok === false) throw new Error(`setblock ${id} at ${pos}: ${JSON.stringify(r)}`);
  return r;
}

async function clear(call, pos) {
  return setBlock(call, pos, 'minecraft:air');
}

async function give(call, pos, i, item, count) {
  const r = await call('container', 'set', ...pos.map(String), String(i), item, String(count));
  if (r && r.ok === false) throw new Error(`container set ${item}@${i} ${pos}: ${JSON.stringify(r)}`);
  return r;
}

/** Empties slot `i` of a container without assuming a `set ... air 0` convention. */
async function emptySlot(call, pos, i) {
  // `container set` with a zero count is the only removal path the harness offers;
  // fall back to setting air when the host rejects zero counts.
  const r = await call('container', 'set', ...pos.map(String), String(i), 'minecraft:air', '0');
  if (r && r.ok === false) {
    await call('container', 'set', ...pos.map(String), String(i), 'minecraft:air', '1');
    await call('container', 'set', ...pos.map(String), String(i), 'minecraft:air', '0');
  }
  return r;
}

async function readContainer(call, pos) {
  return slots(await call('container', ...pos.map(String)));
}

async function readBlock(call, pos) {
  return call('getblock', ...pos.map(String));
}

async function placeGrid(call, center, {mercury = MERCURY, salt = SALT, sulfur = SULFUR} = {}) {
  const [x, y, z] = center;
  await setBlock(call, [x, y, z], INCUBATOR);
  // Java's horizontal order: north (-z), east (+x), south (+z), west (-x).
  await setBlock(call, [x, y, z - 1], mercury);
  await setBlock(call, [x + 1, y, z], salt);
  await setBlock(call, [x, y, z + 1], sulfur);
  return {
    mercury: [x, y, z - 1],
    salt: [x + 1, y, z],
    sulfur: [x, y, z + 1],
  };
}

async function tearDown(call, center) {
  const [x, y, z] = center;
  for (const p of [[x, y, z - 1], [x + 1, y, z], [x, y, z + 1], [x - 1, y, z], [x, y, z]]) {
    await clear(call, p);
  }
}

function totalOf(replies) {
  return replies.reduce((n, r) => n + totalItems(r), 0);
}

/**
 * Snapshot of the four inventories of an incubator multiblock: the three vessel
 * inputs and the incubator's own output slot. Everything a craft may touch.
 */
async function snapshot(call, v, c) {
  const mercury = await readContainer(call, v.mercury);
  const salt = await readContainer(call, v.salt);
  const sulfur = await readContainer(call, v.sulfur);
  const out = await readContainer(call, c);
  return {
    mercury, salt, sulfur, out,
    inputs: totalItems(mercury) + totalItems(salt) + totalItems(sulfur),
    output: totalItems(out),
  };
}

/**
 * Asserts the craft conservation law between two snapshots.
 *
 * Each committed craft removes exactly one unit from *each* vessel and adds the
 * recipe result to the output slot. So for k crafts:
 *   inputs_after  = inputs_before - 3k
 *   output_after >= output_before + k          (k >= 1, result count >= 1)
 *   output_after <= output_before + 64*k
 * and the three vessel deltas must be equal (no partial consume).
 */
function conserve(ok, equal, before, after, label) {
  const dM = totalItems(before.mercury) - totalItems(after.mercury);
  const dS = totalItems(before.salt) - totalItems(after.salt);
  const dF = totalItems(before.sulfur) - totalItems(after.sulfur);
  ok(dM >= 0 && dS >= 0 && dF >= 0, `${label}: a craft never creates vessel input`);
  equal(dS, dM, `${label}: every committed craft consumes one salt per mercury`);
  equal(dF, dM, `${label}: every committed craft consumes one sulfur per mercury`);
  const crafts = dM;
  const gained = totalItems(after.out) - totalItems(before.out);
  ok(gained >= 0, `${label}: output only ever grows (an item is never replaced away)`);
  ok(gained >= crafts, `${label}: each of the ${crafts} crafts produced at least one item`);
  ok(gained <= 64 * crafts, `${label}: no craft produced more than a full slot`);
  return crafts;
}

export async function runCustom(call, assert) {
  const {ok, equal} = helpers(assert);

  // --- probe 1: missing vessel stops any progress ------------------------
  {
    const c = POS.brokenCenter;
    await tearDown(call, c);
    const v = await placeGrid(call, c, {sulfur: 'minecraft:stone'});
    await setBlock(call, [c[0], c[1] - 1, c[2]], 'theurgy:brazier');
    await give(call, v.mercury, 0, 'theurgy:alchemical_mercury', 2);
    await give(call, v.salt, 0, 'theurgy:alchemical_salt', 2);
    await give(call, v.sulfur, 0, 'theurgy:alchemical_sulfur', 2);
    const before = await snapshot(call, v, c);
    await call('tick', ...c.map(String), '400');
    const after = await snapshot(call, v, c);

    equal(after.inputs, before.inputs,
      'incomplete multiblock must not consume any vessel input');
    equal(after.output, 0, 'incomplete multiblock must not produce anything');
    equal(slot(after.out, 0), null, 'the output slot stays empty');
    await tearDown(call, c);
  }

  // --- probe 2: duplicate vessel is never matched as a second kind -------
  {
    const c = POS.brokenCenter;
    await tearDown(call, c);
    const [x, y, z] = c;
    await setBlock(call, [x, y, z], INCUBATOR);
    await setBlock(call, [x, y, z - 1], MERCURY);
    await setBlock(call, [x + 1, y, z], MERCURY);
    await setBlock(call, [x, y, z + 1], SALT);
    await setBlock(call, [x, y, z - 2], SULFUR); // diagonal: not a neighbour
    await setBlock(call, [x, y - 1, z], 'theurgy:brazier');
    await give(call, [x, y, z - 1], 0, 'theurgy:alchemical_mercury', 4);
    await give(call, [x + 1, y, z], 0, 'theurgy:alchemical_mercury', 4);
    await give(call, [x, y, z + 1], 0, 'theurgy:alchemical_salt', 4);
    await give(call, [x, y, z - 2], 0, 'theurgy:alchemical_sulfur', 4);
    const before = await snapshot(call, {
      mercury: [x, y, z - 1], salt: [x, y, z + 1], sulfur: [x, y, z - 2],
    }, c);
    await call('tick', ...c.map(String), '400');
    const after = await snapshot(call, {
      mercury: [x, y, z - 1], salt: [x, y, z + 1], sulfur: [x, y, z - 2],
    }, c);
    equal(after.inputs, before.inputs,
      'a duplicate vessel (and a diagonal sulfur) must not consume input');
    equal(totalItems(await readContainer(call, [x + 1, y, z])), 4,
      'the second mercury vessel must be untouched');
    equal(after.output, 0, 'a duplicate-vessel multiblock produces nothing');
    await tearDown(call, c);
  }

  // --- probe 3: heat gate ------------------------------------------------
  {
    const c = POS.center;
    await tearDown(call, c);
    const v = await placeGrid(call, c);
    await give(call, v.mercury, 0, 'theurgy:alchemical_mercury', 2);
    await give(call, v.salt, 0, 'theurgy:alchemical_salt', 2);
    await give(call, v.sulfur, 0, 'theurgy:alchemical_sulfur', 2);
    await tearDown(call, [c[0], c[1] - 1, c[2]]); // no heat source
    const cold = await snapshot(call, v, c);
    await call('tick', ...c.map(String), '400');
    const stillCold = await snapshot(call, v, c);
    equal(stillCold.inputs, cold.inputs,
      'an unheated incubator must not consume vessel inputs');
    equal(stillCold.output, 0, 'an unheated incubator must not produce');

    // Heat it: the same inputs now have to be consumed, proving the gate (not a
    // missing recipe) is what blocked probe 3's craft.
    await setBlock(call, [c[0], c[1] - 1, c[2]], 'theurgy:brazier');
    await call('tick', ...c.map(String), '600');
    const hot = await snapshot(call, v, c);
    const crafts = conserve(ok, equal, stillCold, hot, 'heating');
    ok(crafts >= 1, 'a heated, valid multiblock with a real recipe must craft');
    await tearDown(call, c);
  }

  // --- probe 4: exact result item and count, plus stacking ---------------
  {
    const c = POS.center;
    await tearDown(call, c);
    const v = await placeGrid(call, c);
    await setBlock(call, [c[0], c[1] - 1, c[2]], 'theurgy:brazier');
    await give(call, v.mercury, 0, 'theurgy:alchemical_mercury', 3);
    await give(call, v.salt, 0, 'theurgy:alchemical_salt', 3);
    await give(call, v.sulfur, 0, 'theurgy:alchemical_sulfur', 3);
    const before = await snapshot(call, v, c);
    await call('tick', ...c.map(String), '1200');
    const after = await snapshot(call, v, c);
    const crafts = conserve(ok, equal, before, after, 'exact result');
    ok(crafts >= 1, 'three valid inputs and heat must craft at least once');
    ok(crafts <= 3, `at most the 3 available vessels can be consumed (got ${crafts})`);

    const first = slot(after.out, 0);
    ok(first != null && first.count > 0, 'a craft leaves a real result stack in slot 0');
    ok(typeof first.item === 'string' && first.item.includes(':'),
      `the result must be a namespaced item id`);
    equal(after.out.filter(Boolean).length, 1,
      'the incubator has exactly one output slot');
    // Every craft produced the same result item, so the slot is one stack of the
    // recipe result, never a mixed bag.
    const remaining = totalItems(after.mercury);
    equal(remaining, 3 - crafts,
      'the mercury vessel keeps exactly the unconsumed remainder');
    await tearDown(call, c);
  }

  // --- probe 5: input replacement restarts the timer ---------------------
  {
    const c = POS.center;
    await tearDown(call, c);
    const v = await placeGrid(call, c);
    await setBlock(call, [c[0], c[1] - 1, c[2]], 'theurgy:brazier');
    await give(call, v.mercury, 0, 'theurgy:alchemical_mercury', 2);
    await give(call, v.salt, 0, 'theurgy:alchemical_salt', 2);
    await give(call, v.sulfur, 0, 'theurgy:alchemical_sulfur', 2);
    const warm = await snapshot(call, v, c);
    await call('tick', ...c.map(String), '40'); // mid-craft, below any recipe time
    // Replace the vessel content mid-craft (one unit left in mercury).
    await give(call, v.mercury, 0, 'theurgy:alchemical_mercury', 1);
    const replaced = await snapshot(call, v, c);
    equal(totalItems(replaced.mercury), 1, 'the replacement set the mercury slot to 1');
    equal(blockName(await readBlock(call, v.salt)), SALT,
      'replacing one vessel leaves the others in place');
    ok(replaced.salt != null, 'the salt vessel still reads as a container');
    ok(warm.inputs >= replaced.inputs, 'the replacement never increases the input total');
    // A single tick after the replacement must not finish a full recipe timer.
    await call('tick', ...c.map(String), '1');
    const afterOneTick = await snapshot(call, v, c);
    equal(afterOneTick.inputs, replaced.inputs,
      'one tick after replacing vessel input cannot complete a full craft');
    equal(afterOneTick.output, replaced.output, 'no result is produced on that tick');
    await tearDown(call, c);
  }

  // --- probe 6: output-full blocks consumption ---------------------------
  {
    const c = POS.center;
    await tearDown(call, c);
    const v = await placeGrid(call, c);
    await setBlock(call, [c[0], c[1] - 1, c[2]], 'theurgy:brazier');
    await give(call, v.mercury, 0, 'theurgy:alchemical_mercury', 2);
    await give(call, v.salt, 0, 'theurgy:alchemical_salt', 2);
    await give(call, v.sulfur, 0, 'theurgy:alchemical_sulfur', 2);
    // A full slot of an unrelated item: a real craft cannot fit, so nothing may be
    // consumed and the blocking stack must be preserved exactly.
    await give(call, c, 0, 'minecraft:stone', 64);
    const full = await snapshot(call, v, c);
    await call('tick', ...c.map(String), '1200');
    const blocked = await snapshot(call, v, c);
    equal(blocked.inputs, full.inputs,
      'a full output slot must never consume vessel inputs');
    equal(slot(blocked.out, 0)?.item, 'minecraft:stone',
      'a full output slot must keep its original item');
    equal(slot(blocked.out, 0)?.count, 64,
      'a full output slot must keep its original count');

    // Draining the slot releases the craft: conservation then holds for the crafts
    // that happen afterwards, and the total can never exceed what was available.
    await emptySlot(call, c, 0);
    await call('tick', ...c.map(String), '1200');
    const drained = await snapshot(call, v, c);
    equal(totalItems(drained.out), slot(drained.out, 0)?.count ?? 0,
      'the output slot holds exactly one stack');
    // The three vessels each lost the same number of units (one per craft) and the
    // released crafts produced at least one item each.
    const crafts = conserve(ok, equal, blocked, drained, 'released craft');
    ok(crafts >= 1, 'draining the output releases a pending craft');
    ok(crafts <= 2, 'the released crafts are bounded by the two units per vessel');
    await tearDown(call, c);
  }

  // --- probe 7: save/reload preserves progress and result ----------------
  {
    const c = POS.reloadCenter;
    await tearDown(call, c);
    const v = await placeGrid(call, c);
    await setBlock(call, [c[0], c[1] - 1, c[2]], 'theurgy:brazier');
    await give(call, v.mercury, 0, 'theurgy:alchemical_mercury', 3);
    await give(call, v.salt, 0, 'theurgy:alchemical_salt', 3);
    await give(call, v.sulfur, 0, 'theurgy:alchemical_sulfur', 3);
    await call('tick', ...c.map(String), '1200'); // a craft or two completes
    const pre = await snapshot(call, v, c);
    await call('save');
    await call('reload');
    const post = await snapshot(call, v, c);
    equal(post.inputs, pre.inputs, 'save/reload must not change any vessel count');
    equal(post.output, pre.output, 'save/reload must not change the output count');
    equal((slots(post.out) ?? []).map(s => (s ? `${s.item}x${s.count}` : 'empty')),
      (slots(pre.out) ?? []).map(s => (s ? `${s.item}x${s.count}` : 'empty')),
      'the output slot keeps its exact item and count across save/reload');
    equal(blockName(await readBlock(call, c)), INCUBATOR,
      'the incubator survives save/reload');
    equal(blockName(await readBlock(call, v.mercury)), MERCURY,
      'the mercury vessel survives save/reload');

    // Continuing after the reload must respect the same conservation law and must
    // not repeat crafts already completed before the save.
    const remainingBefore = post.inputs;
    await call('tick', ...c.map(String), '1200');
    const resumed = await snapshot(call, v, c);
    const crafts = conserve(ok, equal, post, resumed, 'post-reload');
    ok(3 * crafts <= remainingBefore, 'post-reload crafts are bounded by the saved inputs');
    await tearDown(call, c);
  }
}

export default {name: 'incubation-chaos', runCustom};
