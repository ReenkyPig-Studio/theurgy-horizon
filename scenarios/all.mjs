// Isolated integration checks for the non-distiller apparatus. Every step uses
// the real game through horizon-ctl; no world or item is touched outside
// `.local/all-machines`.
const P = (x, y, z) => [x, y, z];
const s = p => p.map(String);

function blockState(reply) {
  return String(reply?.state ?? reply?.block ?? '');
}
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
function mergeFluid(p, fluid, amount) {
  const tag = `{fluids:[{Fluid:"${fluid}",Amount:${amount}}]}`;
  return ['command', 'data', 'merge', 'block', ...s(p), tag];
}
function mergeInt(p, key, value) {
  return ['command', 'data', 'merge', 'block', ...s(p), `{${key}:${value}}`];
}
function has(reply, item) {
  return itemAt(reply, 0) === item;
}

export async function runAll(call, assert) {
  const results = [];
  const step = async (name, args, check) => {
    const reply = await call(args);
    if (check) check(reply);
    results.push({name, reply});
    // Block/BE changes are made by the integrated server. Give its headless
    // client one tick pair to consume them before container/client queries.
    if (args[0] === 'setblock') await call(['wait', '2']);
    return reply;
  };

  // Keep the whole 0..50 test line in loaded chunks regardless of the random
  // spawn position. Otherwise the last apparatus can be outside render distance.
  await step('lab center', ['tp', '25', '80', '8']);
  await step('lab chunks', ['wait', '40']);

  // ---- Calcination oven: heat + item -> salt -------------------------------
  const calc = P(0, 80, 0), calcHeat = P(0, 79, 0);
  await step('calcination place', ['setblock', ...s(calc), 'theurgy:calcination_oven'],
    r => assert.ok(blockState(r).startsWith('theurgy:calcination_oven')));
  await step('calcination heat', ['setblock', ...s(calcHeat), 'theurgy:pyromantic_brazier'],
    r => assert.ok(blockState(r).startsWith('theurgy:pyromantic_brazier')));
  await step('calcination fuel', ['container', ...s(calcHeat), 'minecraft:coal', '4'],
    r => assert.equal(itemAt(r, 0), 'minecraft:coal'));
  await step('calcination input', ['container', ...s(calc), 'minecraft:armadillo_scute', '1'],
    r => assert.equal(itemAt(r, 0), 'minecraft:armadillo_scute'));
  await step('calcination wait', ['wait', '260']);
  const calcOut = await step('calcination output', ['container', ...s(calc)]);
  assert.equal(itemAt(calcOut, 0), null, 'calcination input should be consumed');
  assert.ok(itemAt(calcOut, 1), 'calcination produced no output');
  results.push({name: 'calcination result', item: itemAt(calcOut, 1)});

  // ---- Liquefaction cauldron: solvent + input -> sulfur --------------------
  const liq = P(8, 80, 0), liqHeat = P(8, 79, 0);
  await step('liquefaction place', ['setblock', ...s(liq), 'theurgy:liquefaction_cauldron'],
    r => assert.ok(blockState(r).startsWith('theurgy:liquefaction_cauldron')));
  await step('liquefaction solvent', mergeFluid(liq, 'theurgy:sal_ammoniac', 1000));
  await step('liquefaction heat', ['setblock', ...s(liqHeat), 'theurgy:pyromantic_brazier'],
    r => assert.ok(blockState(r).startsWith('theurgy:pyromantic_brazier')));
  await step('liquefaction fuel', ['container', ...s(liqHeat), 'minecraft:coal', '4'],
    r => assert.equal(itemAt(r, 0), 'minecraft:coal'));
  await step('liquefaction input', ['container', ...s(liq), 'minecraft:acacia_log', '1'],
    r => assert.equal(itemAt(r, 0), 'minecraft:acacia_log'));
  await step('liquefaction wait', ['wait', '260']);
  const liqOut = await step('liquefaction output', ['container', ...s(liq)]);
  assert.ok(itemAt(liqOut, 1), 'liquefaction produced no output');
  results.push({name: 'liquefaction result', item: itemAt(liqOut, 1)});

  // ---- Fermentation vat: water + sulfur + crop -> niter --------------------
  const ferm = P(16, 80, 0);
  await step('fermentation place', ['setblock', ...s(ferm), 'theurgy:fermentation_vat'],
    r => assert.ok(blockState(r).startsWith('theurgy:fermentation_vat')));
  await step('fermentation water', mergeFluid(ferm, 'minecraft:water', 1000));
  await step('fermentation nbt', ['command', 'data', 'get', 'block', ...s(ferm)]);
  await step('fermentation sulfur', ['container', ...s(ferm), 'theurgy:alchemical_sulfur_beef', '1'],
    r => assert.equal(itemAt(r, 0), 'theurgy:alchemical_sulfur_beef'));
  await step('fermentation crop', ['container', ...s(ferm), 'minecraft:oak_log', '1'],
    r => assert.ok(itemAt(r, 1) === 'minecraft:oak_log' || itemAt(r, 0) === 'minecraft:oak_log'));
  await step('fermentation wait', ['wait', '260']);
  const fermOut = await step('fermentation output', ['container', ...s(ferm)]);
  assert.ok(itemAt(fermOut, 3), 'fermentation produced no output: ' + JSON.stringify(slots(fermOut)));
  results.push({name: 'fermentation result', item: itemAt(fermOut, 3)});

  // ---- Digestion vat: sal ammoniac + niter -> higher niter ------------------
  const dig = P(24, 80, 0);
  await step('digestion place', ['setblock', ...s(dig), 'theurgy:digestion_vat'],
    r => assert.ok(blockState(r).startsWith('theurgy:digestion_vat')));
  await step('digestion solvent', mergeFluid(dig, 'theurgy:sal_ammoniac', 1000));
  await step('digestion input', ['container', ...s(dig), 'theurgy:alchemical_niter_animals_common', '1'],
    r => assert.ok(itemAt(r, 0) === 'theurgy:alchemical_niter_animals_common'));
  await step('digestion wait', ['wait', '260']);
  const digOut = await step('digestion output', ['container', ...s(dig)]);
  assert.ok(itemAt(digOut, 3), 'digestion produced no output: ' + JSON.stringify(slots(digOut)));
  results.push({name: 'digestion result', item: itemAt(digOut, 3)});

  // ---- Sal ammoniac accumulator: water + crystallized lava -> tank below ---
  const acc = P(32, 80, 0), tank = P(32, 79, 0);
  await step('accumulator place', ['setblock', ...s(acc), 'theurgy:sal_ammoniac_accumulator'],
    r => assert.ok(blockState(r).startsWith('theurgy:sal_ammoniac_accumulator')));
  await step('accumulator tank', ['setblock', ...s(tank), 'theurgy:sal_ammoniac_tank'],
    r => assert.ok(blockState(r).startsWith('theurgy:sal_ammoniac_tank')));
  await step('accumulator water', mergeFluid(acc, 'minecraft:water', 1000));
  await step('accumulator input', ['container', ...s(acc), 'theurgy:crystallized_lava', '1'],
    r => assert.equal(itemAt(r, 0), 'theurgy:crystallized_lava'));
  await step('accumulator wait', ['wait', '260']);
  const accOut = await step('accumulator output', ['container', ...s(acc)]);
  assert.equal(itemAt(accOut, 0), null, 'accumulator solute should be consumed');
  const tankData = await call(['block', ...s(tank)]);
  results.push({name: 'accumulator tank state', reply: tankData});

  // ---- Reformation array: target + source + flux -> result pedestal --------
  const emitter = P(40, 80, 0), target = P(40, 82, 0), source = P(38, 82, 0), source2 = P(39, 82, 0), result = P(43, 82, 0);
  await step('reformation emitter', ['setblock', ...s(emitter), 'theurgy:sulfuric_flux_emitter'],
    r => assert.ok(blockState(r).startsWith('theurgy:sulfuric_flux_emitter')));
  await step('reformation target', ['setblock', ...s(target), 'theurgy:reformation_target_pedestal'],
    r => assert.ok(blockState(r).startsWith('theurgy:reformation_target_pedestal')));
  await step('reformation source', ['setblock', ...s(source), 'theurgy:reformation_source_pedestal'],
    r => assert.ok(blockState(r).startsWith('theurgy:reformation_source_pedestal')));
  await step('reformation source2', ['setblock', ...s(source2), 'theurgy:reformation_source_pedestal'],
    r => assert.ok(blockState(r).startsWith('theurgy:reformation_source_pedestal')));
  await step('reformation result', ['setblock', ...s(result), 'theurgy:reformation_result_pedestal'],
    r => assert.ok(blockState(r).startsWith('theurgy:reformation_result_pedestal')));
  await step('reformation flux', mergeInt(emitter, 'mercuryFlux', 500));
  await step('reformation target item', ['container', ...s(target), 'theurgy:alchemical_niter_animals_abundant', '1'],
    r => assert.equal(itemAt(r, 0), 'theurgy:alchemical_niter_animals_abundant'));
  await step('reformation source item', ['container', ...s(source), 'theurgy:alchemical_niter_crops_abundant', '1'],
    r => assert.equal(itemAt(r, 0), 'theurgy:alchemical_niter_crops_abundant'));
  await step('reformation source2 item', ['container', ...s(source2), 'theurgy:alchemical_niter_crops_abundant', '1'],
    r => assert.equal(itemAt(r, 0), 'theurgy:alchemical_niter_crops_abundant'));
  await step('reformation wait', ['wait', '260']);
  const reformationOut = await step('reformation output', ['container', ...s(result)]);
  assert.ok(itemAt(reformationOut, 0), 'reformation produced no output: ' + JSON.stringify(slots(reformationOut)));
  results.push({name: 'reformation result', item: itemAt(reformationOut, 0)});

  // ---- Incubator: three horizontal vessels ---------------------------------
  const inc = P(50, 80, 0);
  const vessels = [[50, 80, -1, 'theurgy:incubator_mercury_vessel'],
    [51, 80, 0, 'theurgy:incubator_salt_vessel'], [50, 80, 1, 'theurgy:incubator_sulfur_vessel']];
  await step('incubator place', ['setblock', ...s(inc), 'theurgy:incubator'],
    r => assert.ok(blockState(r).startsWith('theurgy:incubator')));
  for (const [x, y, z, id] of vessels) {
    await step('incubator vessel ' + id, ['setblock', String(x), String(y), String(z), id],
      r => assert.ok(blockState(r).startsWith(id)));
  }
  await step('incubator heat', ['setblock', '50', '79', '0', 'theurgy:pyromantic_brazier']);
  await step('incubator brazier inspect', ['container', '50', '79', '0']);
  await step('incubator fuel', ['container', '50', '79', '0', 'minecraft:coal', '4']);
  await step('incubator mercury', ['container', '50', '80', '-1', 'theurgy:mercury_shard', '1'],
    r => assert.equal(itemAt(r, 0), 'theurgy:mercury_shard'));
  await step('incubator salt', ['container', '51', '80', '0', 'theurgy:alchemical_salt_plant', '1'],
    r => assert.equal(itemAt(r, 0), 'theurgy:alchemical_salt_plant'));
  await step('incubator sulfur', ['container', '50', '80', '1', 'theurgy:alchemical_sulfur_acacia_log', '1'],
    r => assert.equal(itemAt(r, 0), 'theurgy:alchemical_sulfur_acacia_log'));
  await step('incubator wait', ['wait', '260']);
  const incOut = await step('incubator output', ['container', ...s(inc)]);
  assert.ok(itemAt(incOut, 0), 'incubator produced no output: ' + JSON.stringify(slots(incOut)));
  results.push({name: 'incubator result', item: itemAt(incOut, 0)});

  return results;
}
