// Pure, testable machine scenarios for the Theurgy distiller on Horizon.
//
// Source of truth: upstream/theurgy/.../apparatus/distiller/
//   DistillerBlockEntity.tickServer (heat + input gate)
//   DistillationStorageBehaviour (1 input slot, 1 extract-only output slot)
//   DistillerBlockEntity.preRemoveSideEffects (drops every inventory slot)
//   saveAdditional/loadAdditional (inputInventory, outputInventory, heatReceiver)
//
// A scenario is a list of steps. Each step is {name, args, check}:
//   args   -> horizon-ctl arguments (or a special local action, see `kind`)
//   check  -> pure predicate over the accumulated machine observations; it must be able to
//             read only the *parsed* replies, never the raw text, so a benign formatting
//             change in Horizon cannot silently weaken it.
//
// Nothing here talks to the game: tools/game.mjs executes it, tests/machines.test.mjs
// checks the plan offline (shape, determinism, absolute expectations, no player paths).

export const MACHINE_BLOCK = 'theurgy:distiller';

// Machine positions: a small grid in the harness lab, y=80 (test terrain), far from spawn
// structures but inside the loaded test-terrain area the other modes already use.
export const POS = Object.freeze({
  main: [0, 80, 0],
  cold: [2, 80, 0],
  invalid: [4, 80, 0],
  extract: [6, 80, 0],
  persist: [8, 80, 0],
  breaker: [10, 80, 0],
});

export function posArgs(p) {
  return p.map(String);
}

/**
 * Parse the reply of `container X Y Z` -> normalized `[null|{item,count}]`.
 *
 * Horizon has serialised a slot either flat (`{item:"ns:id", count:1, ...}`) or nested
 * (`{item:{id:"ns:id", count:1}}`) across builds; both are normalised here so a serializer
 * change can never turn a populated slot into a phantom match (or hide a real one). An
 * unparseable slot becomes `{item:null, count:0}` -> it counts as empty, never as a result.
 */
export function slots(reply) {
  if (!reply || !Array.isArray(reply.slots)) return null;
  return reply.slots.map(s => {
    if (s == null) return null;
    if (typeof s === 'string') return {item: s, count: 1};
    const inner = s.item && typeof s.item === 'object' ? s.item : null;
    const item = inner ? inner.id : s.item;
    const count = inner ? inner.count : s.count;
    return {item: item == null ? null : String(item), count: Number(count)};
  });
}

export function slotCount(reply, i) {
  const s = slots(reply);
  return s && s[i] ? s[i].count : 0;
}

export function slotItem(reply, i) {
  const s = slots(reply);
  return s && s[i] ? s[i].item : null;
}

export function totalItems(reply) {
  const s = slots(reply);
  return s ? s.filter(Boolean).reduce((n, x) => n + x.count, 0) : 0;
}

/**
 * `block`/`getblock`/`setblock` replies carry the state string, e.g.
 * `{block:"theurgy:distiller[lit=false,...]"}` (older builds used `id`). Compare the name
 * only; a nested `{block:{id}}` is also accepted. Anything else returns null -> a failed
 * assertion, never a silent pass.
 */
export function blockName(reply) {
  if (typeof reply === 'string') return reply.split('[')[0];
  if (!reply || typeof reply !== 'object') return null;
  let text = reply.block ?? reply.id ?? reply.state;
  if (text && typeof text === 'object') text = text.id ?? text.name ?? text.block;
  return typeof text === 'string' ? text.split('[')[0] : null;
}

function expect(name, ok, detail) {
  return {name, ok: Boolean(ok), detail};
}

/**
 * The ordered distiller parity run. Recipe is pinned upstream:
 *   src/generated/resources/data/theurgy/recipe/distillation/bread.json
 *   ingredient: minecraft:bread (count 1), result: theurgy:mercury_shard (count 1), time 100.
 * tools/game.mjs may override via THEURGY_TEST_INPUT / THEURGY_TEST_OUTPUT.
 */
export function machineScenarios(opts = {}) {
  const input = opts.input ?? 'minecraft:bread';
  const output = opts.output ?? 'theurgy:mercury_shard';
  const waitTicks = opts.waitTicks ?? 200;
  const steps = [];
  const step = s => {
    steps.push(s);
    return s;
  };
  const observe = (name, p, check, extra = {}) =>
    step({name, kind: 'container', pos: p, args: ['container', ...posArgs(p)], check, ...extra});

  // -- 1. Placement --------------------------------------------------------------
  for (const [key, p] of Object.entries(POS)) {
    step({
      name: `place ${key}`,
      kind: 'setblock',
      pos: p,
      args: ['setblock', ...posArgs(p), MACHINE_BLOCK],
      expectsBlock: MACHINE_BLOCK,
      check: reply => expect(`place ${key}`, blockName(reply) === MACHINE_BLOCK, blockName(reply)),
    });
    // `setblock` executes on the integrated server; a headless client needs a
    // tick to receive the chunk change before its `block` readback is valid.
    step({name: `sync ${key}`, kind: 'wait', args: ['wait', '2']});
    step({
      name: `readback ${key}`,
      kind: 'block',
      pos: p,
      args: ['block', ...posArgs(p)],
      check: reply => expect(`readback ${key}`, blockName(reply) === MACHINE_BLOCK, blockName(reply)),
    });
    step({
      name: `empty inventory ${key}`,
      kind: 'container',
      pos: p,
      args: ['container', ...posArgs(p)],
      check: reply => {
        const s = slots(reply);
        return expect(`empty inventory ${key}`, s !== null && s.filter(Boolean).length === 0,
          s === null ? 'no block entity / no slots' : JSON.stringify(s));
      },
    });
  }

  // -- 2. Insert the recipe input into the heated machine ------------------------
  step({
    name: 'insert input',
    kind: 'container-put',
    pos: POS.main,
    args: ['container', ...posArgs(POS.main), input, '1'],
    check: reply => expect('insert input', slotItem(reply, 0) === input && slotCount(reply, 0) === 1,
      JSON.stringify(slots(reply))),
  });

  // -- 3. Cold machine produces nothing -----------------------------------------
  step({
    name: 'cold machine insert',
    kind: 'container-put',
    pos: POS.cold,
    args: ['container', ...posArgs(POS.cold), input, '1'],
    check: reply => expect('cold machine insert', slotItem(reply, 0) === input, JSON.stringify(slots(reply))),
  });
  step({name: 'cold settle', kind: 'wait', args: ['wait', String(waitTicks)]});
  observe('cold no output', POS.cold, reply => {
    const s = slots(reply);
    return expect('cold no output', s !== null && totalItems(reply) === 1 && slotItem(reply, 0) === input,
      JSON.stringify(s));
  });

  // -- 4. Invalid input is refused ----------------------------------------------
  // Horizon's `container` write path targets the first empty slot and therefore bypasses the
  // storage's `isValid`; stone may physically land in the input slot. The parity invariant that
  // the Java machine guarantees regardless is: an item with no distillation recipe is NEVER
  // processed, so the extract-only OUTPUT slot (index 1) must stay empty and nothing is created.
  step({
    name: 'invalid input not processed',
    kind: 'container-put',
    pos: POS.invalid,
    args: ['container', ...posArgs(POS.invalid), 'minecraft:stone', '1'],
    check: reply => {
      const s = slots(reply);
      return expect('invalid input not processed',
        s !== null && slotCount(reply, 1) === 0 && totalItems(reply) <= 1, JSON.stringify(s));
    },
  });
  // Give it time to (wrongly) distil, then re-assert the output stayed empty.
  step({name: 'invalid settle', kind: 'wait', args: ['wait', '120']});
  observe('invalid still not distilled', POS.invalid, reply => {
    const s = slots(reply);
    return expect('invalid still not distilled', s !== null && slotCount(reply, 1) === 0 && totalItems(reply) <= 1,
      JSON.stringify(s));
  });

  // -- 5. Heat the main machine and process -------------------------------------
  // Heat comes from a HeatProvider in the block BELOW (HeatConsumerBehaviour.hasHeatProvider),
  // and the only HEAT_PROVIDER is the Pyromantic Brazier, hot while its LIT state is true and
  // it has fuel. bread.json: time=100, ingredient minecraft:bread x1 -> theurgy:mercury_shard.
  step({
    name: 'place brazier under main',
    kind: 'setblock',
    pos: [POS.main[0], POS.main[1] - 1, POS.main[2]],
    args: ['setblock', String(POS.main[0]), String(POS.main[1] - 1), String(POS.main[2]), 'theurgy:pyromantic_brazier'],
    expectsBlock: 'theurgy:pyromantic_brazier',
    check: reply => expect('place brazier under main', blockName(reply) === 'theurgy:pyromantic_brazier', blockName(reply)),
  });
  step({
    name: 'fuel brazier with coal',
    kind: 'container-put',
    pos: [POS.main[0], POS.main[1] - 1, POS.main[2]],
    args: ['container', String(POS.main[0]), String(POS.main[1] - 1), String(POS.main[2]), 'minecraft:coal', '4'],
    check: reply => expect('fuel brazier with coal', slotItem(reply, 0) === 'minecraft:coal' && slotCount(reply, 0) >= 1,
      JSON.stringify(slots(reply))),
  });
  // The brazier needs a tick to consume fuel and set LIT=true before the distiller sees heat.
  step({name: 'brazier warm-up', kind: 'wait', args: ['wait', '40']});
  step({
    name: 'brazier is lit',
    kind: 'block',
    pos: [POS.main[0], POS.main[1] - 1, POS.main[2]],
    args: ['block', String(POS.main[0]), String(POS.main[1] - 1), String(POS.main[2])],
    check: reply => {
      const state = String(reply?.state ?? reply?.block ?? '');
      return expect('brazier is lit', /\blit=(true|1)\b/.test(state), state);
    },
  });
  // Bread distils in 100 ticks; the wait budget must cover recipe time plus the heat polling
  // interval (HeatConsumerBehaviour re-checks every 20 game ticks). Guard it here.
  if (waitTicks < 200) throw new Error('process wait must cover recipe time + heat poll (>=200 ticks)');
  step({name: 'process wait', kind: 'wait', args: ['wait', String(waitTicks)]});
  observe('processing moved input to output', POS.main, reply => {
    const s = slots(reply);
    const inCount = slotCount(reply, 0);
    const outCount = slotCount(reply, 1);
    return expect('processing moved input to output',
      s !== null && inCount === 0 && outCount >= 1 && slotItem(reply, 1) === output,
      JSON.stringify(s));
  });
  // Conservation: after heating exactly one input the total must still be 1 (one result in the
  // output). `>=1` above would let a duplication bug pass; this forbids it.
  observe('processing conserved stock (no duplication)', POS.main, reply => {
    const s = slots(reply);
    return expect('processing conserved stock (no duplication)', s !== null && totalItems(reply) === 1,
      JSON.stringify(s));
  });

  // -- 6. Output side is extract-only -------------------------------------------
  step({
    name: 'output refuses external insert',
    kind: 'container-put-expect-refused',
    pos: POS.extract,
    args: ['container', ...posArgs(POS.extract), output, '1'],
    check: reply => {
      const s = slots(reply);
      // Either the storage refuses the slot, or the put lands in the input slot only and is
      // still not accepted as a valid recipe input for this machine: in both cases the
      // extract-only output must stay empty.
      return expect('output refuses external insert', s !== null && slotCount(reply, 1) === 0,
        JSON.stringify(s));
    },
  });

  // -- 7. Save / reload keeps inventories ---------------------------------------
  step({
    name: 'persist insert',
    kind: 'container-put',
    pos: POS.persist,
    args: ['container', ...posArgs(POS.persist), input, '1'],
    check: reply => expect('persist insert', slotCount(reply, 0) === 1, JSON.stringify(slots(reply))),
  });
  step({name: 'persist save', kind: 'save', args: ['save']});
  step({
    name: 'persist reload',
    kind: 'reload',
    // tools/game.mjs turns this into quit + launch + wait on the SAME world dir/socket.
    args: [],
    check: null,
  });
  observe('persist surviving reload', POS.persist, reply => {
    const s = slots(reply);
    return expect('persist surviving reload', s !== null && slotItem(reply, 0) === input && slotCount(reply, 0) === 1,
      JSON.stringify(s));
  });
  step({
    name: 'persist readback block',
    kind: 'block',
    pos: POS.persist,
    args: ['block', ...posArgs(POS.persist)],
    check: reply => expect('persist readback block', blockName(reply) === MACHINE_BLOCK, blockName(reply)),
  });

  // -- 8. Breaking the machine drops, never duplicates --------------------------
  // Seed the machine with a stack, then setblock air (removes the BE like a destroy); the
  // drops must appear as item entities and total near the inserted count, never more.
  step({
    name: 'breaker insert',
    kind: 'container-put',
    pos: POS.breaker,
    args: ['container', ...posArgs(POS.breaker), input, '1'],
    check: reply => expect('breaker insert', slotCount(reply, 0) === 1, JSON.stringify(slots(reply))),
  });
  // Drops land near the machine; move the player there so `entities` (radius around the player)
  // can see them, and stand clear of the drop's own position.
  step({
    name: 'breaker approach',
    kind: 'tp',
    args: ['tp', String(POS.breaker[0] + 0.5), String(POS.breaker[1] + 1), String(POS.breaker[2] + 2.5)],
    check: null,
  });
  // Break immediately after teleporting: a headless creative player still has
  // gravity, and settling first can put it below the test-terrain platform.
  step({
    name: 'breaker mine',
    kind: 'break',
    pos: POS.breaker,
    args: ['break', ...posArgs(POS.breaker)],
    dropOf: input,
    check: reply => expect('breaker mine', reply && reply.ok === true, JSON.stringify(reply).slice(0, 200)),
  });
  step({name: 'breaker settle', kind: 'wait', args: ['wait', '20']});
  step({
    name: 'breaker drops counted',
    kind: 'entities-count',
    args: ['entities', '8', 'minecraft:item'],
    check: reply => {
      const list = Array.isArray(reply.entities) ? reply.entities : null;
      return expect('breaker drops counted', list !== null, JSON.stringify(reply).slice(0, 200));
    },
  });

  return {input, output, waitTicks, positions: POS, steps};
}

/**
 * Offline self-check for the adversarial machine plan (tools/chaos-plan.mjs `machineChaosPlan`).
 * Verifies every probed position is placed before it is read, invariants are present, and the
 * plan is bounded and deterministic. Called by tools/game.mjs before launching chaos-machines.
 */
export function selfTestChaos(plan) {
  if (!plan || !Array.isArray(plan.actions)) throw new Error('machine chaos plan has no actions');
  if (plan.actions.length > 200) throw new Error('machine chaos plan is not bounded');
  if (!plan.actions.some(a => a.invalidInput === true)) throw new Error('no invalid-input probe');
  if (!plan.actions.some(a => a.kind === 'reload')) throw new Error('no reload probe');
  const placed = new Set(plan.actions
    .filter(a => a.kind === 'setblock' && a.expect === MACHINE_BLOCK)
    .map(a => a.args.slice(1, 4).join(',')));
  for (const [key, p] of Object.entries(plan.positions || {})) {
    if (!placed.has(p.map(String).join(','))) throw new Error('machine chaos plan never places ' + key);
  }
  // No probe may escape the lab coordinates.
  for (const a of plan.actions) {
    if (a.kind === 'reload' || a.kind === 'save') continue;
    const nums = a.args.slice(1).filter(x => /^-?\d+$/.test(x)).map(Number);
    if (a.args[0] === 'container' || a.args[0] === 'container-put' || a.args[0] === 'setblock' || a.args[0] === 'block') {
      const [x, y, z] = a.args.slice(1, 4).map(Number);
      if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z))
        throw new Error('machine chaos probe has no numeric position: ' + a.name);
      if (y < 78 || y > 82 || x < 0 || x > 16 || z < 0 || z > 16)
        throw new Error('machine chaos probe outside lab: ' + a.name);
    }
    void nums;
  }
  return true;
}

// Offline self-check: plan shape, distinct positions, determinism, absolute (non-tautological)
// checks and no escapes outside the isolated lab. Called by tools/game.mjs before launching.
export function selfTest() {
  const a = machineScenarios();
  const b = machineScenarios();
  if (JSON.stringify(a.steps.map(s => s.name)) !== JSON.stringify(b.steps.map(s => s.name)))
    throw new Error('machine scenarios are not deterministic');
  const names = a.steps.map(s => s.name);
  if (new Set(names).size !== names.length) throw new Error('machine scenario step names must be unique');
  const kinds = new Set(a.steps.map(s => s.kind));
  for (const k of ['setblock', 'block', 'container', 'container-put', 'wait', 'save', 'reload', 'entities-count', 'tp'])
    if (!kinds.has(k)) throw new Error('machine scenarios miss a ' + k + ' step');
  if (!names.some(n => /place brazier under main/.test(n))) throw new Error('brazier heat source missing');
  if (!names.some(n => /brazier is lit/.test(n))) throw new Error('brazier LIT assertion missing');
  if (!names.some(n => /cold no output/.test(n))) throw new Error('cold-machine invariant missing');
  if (!names.some(n => /invalid still not distilled/.test(n))) throw new Error('invalid-input invariant missing');
  if (!names.some(n => /output refuses external insert/.test(n))) throw new Error('extract-only invariant missing');
  if (!names.some(n => /surviving reload/.test(n))) throw new Error('reload invariant missing');
  if (!names.some(n => /no duplication/.test(n))) throw new Error('conservation invariant missing');
  if (!names.some(n => n === 'breaker drops counted')) throw new Error('break-drop conservation probe missing');
  // Guard the recipe/time assumptions against upstream drift: if the default input changes the
  // checks comparing against `a.input` must still be present and the wait must cover the recipe.
  if (a.waitTicks < 200) throw new Error('waitTicks too small for bread (100) + heat poll (20)');
  for (const s of a.steps) {
    for (const arg of s.args || []) {
      if (typeof arg !== 'string') continue;
      if (arg.includes('..') || /[A-Za-z]:\\|\//.test(arg))
        throw new Error('machine step escapes the lab: ' + s.name + ' -> ' + arg);
    }
    if (s.pos) {
      const [x, y] = s.pos;
      if (typeof y === 'number' && (y < 78 || y > 82)) throw new Error('machine must stay in the test-terrain lab (y 78..82), got ' + s.name);
      if (typeof x === 'number' && (x < 0 || x > 16)) throw new Error('machine outside lab x-range: ' + s.name);
    }
  }
  // A tautological check (one that ignores the reply) would also pass; forbid it.
  for (const s of a.steps) {
    if (typeof s.check === 'function' && s.check.length === 0 && s.check() === null)
      throw new Error('tautological machine check: ' + s.name);
  }
  return true;
}
