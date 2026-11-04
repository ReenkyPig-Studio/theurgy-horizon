// Isolated integration checks for the mercury flux apparatus. Every step goes
// through the real integrated server (headless client + horizon-ctl); nothing
// outside the lab line is touched.
const P = (x, y, z) => [x, y, z];
const s = p => p.map(String);

function slots(reply) {
  return Array.isArray(reply?.slots) ? reply.slots : null;
}
function innerAt(reply, i) {
  const v = slots(reply)?.[i];
  if (!v) return null;
  return v.item && typeof v.item === 'object' ? v.item : v;
}
function itemAt(reply, i) {
  const inner = innerAt(reply, i);
  return inner?.id ?? inner?.item ?? null;
}
function countAt(reply, i) {
  const v = slots(reply)?.[i];
  if (!v) return 0;
  return Number(innerAt(reply, i)?.count ?? v.count ?? 0);
}
function blockState(reply) {
  return String(reply?.state ?? reply?.block ?? '');
}
function mergeInt(p, key, value) {
  return ['command', 'data', 'merge', 'block', ...s(p), `{${key}:${value}}`];
}
function mergeEnergy(p, value) {
  return ['command', 'data', 'merge', 'block', ...s(p),
    `{mercuryFluxHandler:{energy:${value}}}`];
}
function nbtText(reply) {
  if (Array.isArray(reply?.chat)) return reply.chat.join('\n');
  return typeof reply === 'string' ? reply : JSON.stringify(reply ?? '');
}
function energyOf(reply) {
  const text = nbtText(reply);
  const m = /mercuryFluxHandler\s*:\s*\{[^{}]*energy\s*:\s*(-?\d+)/.exec(text);
  return m ? Number(m[1]) : null;
}
function fluxOf(reply) {
  const text = nbtText(reply);
  const m = /mercuryFlux\s*:\s*(-?\d+)/.exec(text);
  return m ? Number(m[1]) : null;
}

export async function runCustom(call, assert) {
  const results = [];
  const step = async (name, args) => {
    const reply = await call(args);
    results.push({name, reply});
    if (args[0] === 'setblock' || args[0] === 'command') await call(['wait', '2']);
    return reply;
  };
  const place = async (name, p, id) => {
    const r = await step(name, ['setblock', ...s(p), id]);
    assert.ok(blockState(r).startsWith(id), `${name}: expected ${id}, got ${blockState(r)}`);
    return r;
  };
  const give = async (name, p, item, count) => {
    const r = await step(name, ['container', ...s(p), item, String(count)]);
    assert.equal(itemAt(r, 0), item, `${name}: slot 0 should hold ${item}`);
    return r;
  };
  // Valid source pedestal with any item, and an empty target pedestal: this is
  // the selection the caloric emitter is allowed to heat.
  const pedestalTarget = async (name, p, withItem) => {
    await place(name, p, 'theurgy:reformation_target_pedestal');
    if (withItem) {
      const r = await step(name + ' item', ['container', ...s(p), 'theurgy:alchemical_sulfur_acacia_log', '1']);
      assert.ok(itemAt(r, 0), name + ': target pedestal item missing');
    }
  };

  await step('flux lab center', ['tp', '25', '90', '40']);
  await step('flux lab chunks', ['wait', '40']);

  // ---- Capacitor: persisted buffer + side modes ---------------------------
  const cap = P(4, 90, 40);
  await place('capacitor place', cap, 'theurgy:mercury_capacitor');
  await step('capacitor buffer', mergeEnergy(cap, 640));
  const capData = await step('capacitor data', ['command', 'data', 'get', 'block', ...s(cap)]);
  assert.equal(energyOf(capData), 640, 'capacitor must persist its buffer');
  results.push({name: 'capacitor state', reply: capData});

  // ---- Capacity split: emitter drains into a capacitor through a target ---
  const emitter = P(8, 90, 40), drainCap = P(9, 90, 40);
  await place('emitter place', emitter, 'theurgy:mercury_flux_emitter');
  await place('drain capacitor place', drainCap, 'theurgy:mercury_capacitor');
  await step('emitter buffer', mergeEnergy(emitter, 100));
  const before = await step('drain capacitor before', ['command', 'data', 'get', 'block', ...s(drainCap)]);
  results.push({name: 'drain capacitor before', reply: before});
  await step('emitter run', ['wait', '40']);
  await step('emitter sync', ['wait', '2']);
  const after = await step('drain capacitor after', ['command', 'data', 'get', 'block', ...s(drainCap)]);
  const beforeEnergy = energyOf(before) ?? 0, afterEnergy = energyOf(after) ?? 0;
  assert.ok(afterEnergy > beforeEnergy, 'emitter must push flux into the adjacent capacitor');
  const emitterAfter = await step('emitter after push', ['command', 'data', 'get', 'block', ...s(emitter)]);
  const left = energyOf(emitterAfter) ?? 0;
  assert.ok(afterEnergy - beforeEnergy + left === 100,
    `flux must be conserved (capacitor ${beforeEnergy}->${afterEnergy}, emitter left ${left})`);
  results.push({name: 'drain capacitor after', reply: after});

  // ---- Caloric emitter: heats a valid selected target ---------------------
  const caloric = P(12, 90, 40), target = P(12, 92, 40);
  await place('caloric place', caloric, 'theurgy:caloric_flux_emitter');
  await pedestalTarget('caloric target', target, true);
  await step('caloric buffer', mergeEnergy(caloric, 500));
  await step('caloric selection', ['command', 'data', 'merge', 'block', ...s(caloric),
    `{selectedPoints:[{x:${target[0]},y:${target[1]},z:${target[2]}}]}`]);
  const targetBefore = await step('caloric target before', ['command', 'data', 'get', 'block', ...s(target)]);
  results.push({name: 'caloric target before', reply: targetBefore});
  await step('caloric run', ['wait', '60']);
  const targetAfter = await step('caloric target after', ['command', 'data', 'get', 'block', ...s(target)]);
  assert.ok(/hotUntil\s*:\s*[1-9]/.test(nbtText(targetAfter)), 'caloric emitter must heat its selected target');
  const caloricAfter = await step('caloric after heat', ['command', 'data', 'get', 'block', ...s(caloric)]);
  assert.equal(energyOf(caloricAfter), 400, 'one heat pulse must consume exactly 100 flux');
  results.push({name: 'caloric target after', reply: targetAfter});

  // ---- Caloric emitter: no valid target means no heat / no drain ----------
  const lonely = P(16, 90, 40);
  await place('lonely caloric place', lonely, 'theurgy:caloric_flux_emitter');
  await step('lonely caloric buffer', mergeEnergy(lonely, 500));
  await step('lonely caloric run', ['wait', '40']);
  const lonelyState = await step('lonely caloric state', ['command', 'data', 'get', 'block', ...s(lonely)]);
  assert.equal(energyOf(lonelyState), 500, 'no target must consume no flux');
  results.push({name: 'lonely caloric state', reply: lonelyState});

  return results;
}
