// Real headless regressions for the Java 26.3 review findings. All operations
// use the isolated integrated server; selection tests use actual right clicks.
const S = p => p.map(String);
const slots = r => Array.isArray(r?.slots) ? r.slots : [];
const inner = v => v?.item && typeof v.item === 'object' ? v.item : v;
const itemAt = (r, i) => inner(slots(r)[i])?.id ?? inner(slots(r)[i])?.item ?? null;
const countAt = (r, i) => Number(inner(slots(r)[i])?.count ?? slots(r)[i]?.count ?? 0);
const total = r => slots(r).reduce((n, v) => n + Number(inner(v)?.count ?? v?.count ?? 0), 0);
const text = r => Array.isArray(r?.chat) ? r.chat.join('\n') : JSON.stringify(r ?? '');

export async function runCustom(call, assert) {
  const observations = [];
  const step = async (name, args) => {
    const reply = await call(args);
    observations.push({name, args, reply});
    return reply;
  };
  const place = async (name, pos, id, state = '') => {
    await step(name + ' clear old fixture', ['setblock', ...S(pos), 'minecraft:air']);
    await call(['wait', '2']);
    const reply = await step(name, ['setblock', ...S(pos), id + state]);
    await call(['wait', '2']);
    return reply;
  };
  const click = async (name, pos) => {
    await step(name + ' approach', ['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 2)]);
    await step(name + ' aim', ['look-at', 'block', ...S(pos)]);
    return step(name, ['interact', ...S(pos)]);
  };
  const put = (name, pos, item, count = '1') => step(name, ['container', ...S(pos), item, count]);
  const merge = (name, pos, nbt) => step(name, ['command', 'data', 'merge', 'block', ...S(pos), nbt]);
  const data = (name, pos) => step(name, ['command', 'data', 'get', 'block', ...S(pos)]);
  const energy = reply => {
    const match = /mercuryFluxHandler\s*:\s*\{[^{}]*energy\s*:\s*(-?\d+)/.exec(text(reply));
    return match ? Number(match[1]) : 0;
  };

  // ---- No invented apparatus menu: Java's direct useItemOn ----------------
  const direct = [60, 80, 60];
  await step('direct interaction lab', ['tp', '60', '82', '58']);
  await step('clear review fixture inventory', ['command', 'clear', '@s']);
  await place('direct distiller', direct, 'theurgy:distiller');
  await step('bread for direct insertion', ['give', 'minecraft:bread', '2']);
  await step('bread inventory sync', ['wait', '2']);
  // The isolated world is intentionally reused for save/reload coverage. A
  // previous selection scenario can leave an emitter in the active hotbar slot,
  // and creative `give` does not replace that stack. Select the actual input so
  // this regression tests the machine hook rather than leaked player state.
  await step('hold bread for direct insertion', ['select', 'minecraft:bread']);
  await click('insert bread with right click', direct);
  const inserted = await step('direct insertion readback', ['container', ...S(direct)]);
  assert.equal(itemAt(inserted, 0), 'minecraft:bread');
  assert.equal(countAt(inserted, 0), 2);
  assert.equal((await step('still playing after insertion', ['state'])).screen, 'playing',
    'Theurgy 26.3 has no distiller menu');
  await click('empty hand takes input back', direct);
  assert.equal(countAt(await step('direct extraction readback', ['container', ...S(direct)]), 0), 0);
  assert.equal((await step('still playing after extraction', ['state'])).screen, 'playing');

  // ---- CapabilityRegistry sided handlers: top input, bottom output --------
  const machine = [64, 80, 60], above = [64, 81, 60], feeder = [64, 82, 60];
  const below = [64, 79, 60], receiver = [65, 79, 60];
  await place('sided distiller', machine, 'theurgy:distiller');
  await place('top hopper', above, 'minecraft:hopper', '[facing=down]');
  await place('top feeder chest', feeder, 'minecraft:chest');
  await place('bottom hopper', below, 'minecraft:hopper', '[facing=east]');
  await place('bottom receiver chest', receiver, 'minecraft:chest');
  await put('feeder bread', feeder, 'minecraft:bread', '4');
  await step('top hopper transfer', ['wait', '80']);
  const topFed = await step('top fed input', ['container', ...S(machine)]);
  assert.equal(itemAt(topFed, 0), 'minecraft:bread');
  assert.equal(countAt(topFed, 0), 4);
  assert.equal(total(await step('bottom cannot pull input', ['container', ...S(receiver)])), 0,
    'a bottom hopper must never pull the distiller input');
  await merge('seed output without changing input', machine,
    '{Items:[{Slot:0b,id:"minecraft:bread",count:4},{Slot:1b,id:"theurgy:mercury_shard",count:3}]}');
  await step('bottom output transfer', ['wait', '60']);
  const afterOutput = await step('input survives bottom extraction', ['container', ...S(machine)]);
  assert.equal(itemAt(afterOutput, 0), 'minecraft:bread');
  assert.equal(countAt(afterOutput, 0), 4);
  const received = await step('bottom extracted only output', ['container', ...S(receiver)]);
  assert.equal(itemAt(received, 0), 'theurgy:mercury_shard');
  assert.equal(countAt(received, 0), 3);

  // ---- TagRecipeResult.getStack / TagUtil first item ----------------------
  const incubator = [68, 80, 60];
  const mercury = [68, 80, 59], salt = [69, 80, 60], sulfur = [68, 80, 61];
  await place('tag result incubator', incubator, 'theurgy:incubator');
  await place('tag result mercury vessel', mercury, 'theurgy:incubator_mercury_vessel');
  await place('tag result salt vessel', salt, 'theurgy:incubator_salt_vessel');
  await place('tag result sulfur vessel', sulfur, 'theurgy:incubator_sulfur_vessel');
  await place('tag result brazier', [68, 79, 60], 'theurgy:pyromantic_brazier');
  await put('tag result fuel', [68, 79, 60], 'minecraft:coal', '4');
  await put('tag result mercury', mercury, 'theurgy:mercury_shard');
  await put('tag result salt', salt, 'theurgy:alchemical_salt_mineral');
  await put('tag result sulfur', sulfur, 'theurgy:alchemical_sulfur_sal_ammoniac');
  await step('tag result craft window', ['wait', '220']);
  const tagOutput = await step('tag result output', ['container', ...S(incubator)]);
  assert.equal(itemAt(tagOutput, 0), 'theurgy:sal_ammoniac_crystal');
  assert.equal(countAt(tagOutput, 0), 1);

  // ---- Mercury selection: right click + placement, no neighbour broadcast -
  const cap = [72, 80, 60], otherCap = [73, 80, 62], emitter = [72, 80, 62];
  await place('mercury selected capacitor', cap, 'theurgy:mercury_capacitor');
  await place('mercury unselected capacitor', otherCap, 'theurgy:mercury_capacitor');
  await step('mercury emitter item', ['give', 'theurgy:mercury_flux_emitter', '1']);
  await step('mercury emitter inventory', ['inventory']);
  await step('hold mercury emitter for selection', ['select', 'theurgy:mercury_flux_emitter']);
  await click('select mercury capacitor', cap);
  await place('mercury placement support', [72, 79, 62], 'minecraft:stone');
  await step('mercury placement approach', ['tp', '72', '82', '61']);
  await step('clear old mercury emitter target', ['setblock', ...S(emitter), 'minecraft:air']);
  await call(['wait', '2']);
  await step('place selected mercury emitter', ['place', ...S(emitter), '--item', 'theurgy:mercury_flux_emitter']);
  const emitterSelection = await data('mercury selection persisted', emitter);
  assert.ok(text(emitterSelection).includes('selectedPoints') && text(emitterSelection).includes('blockPos'),
    'placement must persist the actual selected point: ' + text(emitterSelection));
  await merge('charge selected mercury emitter', emitter, '{mercuryFluxHandler:{energy:100}}');
  await step('selected mercury transfer window', ['wait', '60']);
  assert.ok(energy(await data('selected capacitor charged', cap)) > 0);
  assert.equal(energy(await data('unselected capacitor unchanged', otherCap)), 0);

  // ---- Reformation selection: explicit sources/target/result --------------
  const re = [80, 80, 60], target = [80, 80, 62], result = [80, 80, 58];
  const sourceA = [78, 80, 60], sourceB = [82, 80, 60], unselected = [84, 80, 60];
  await place('reformation target', target, 'theurgy:reformation_target_pedestal');
  await place('reformation result', result, 'theurgy:reformation_result_pedestal');
  await place('reformation source A', sourceA, 'theurgy:reformation_source_pedestal');
  await place('reformation source B', sourceB, 'theurgy:reformation_source_pedestal');
  await place('unselected valid source', unselected, 'theurgy:reformation_source_pedestal');
  await put('selected target item', target, 'theurgy:alchemical_niter_animals_abundant');
  await put('selected source A item', sourceA, 'theurgy:alchemical_niter_crops_abundant');
  await put('selected source B item', sourceB, 'theurgy:alchemical_niter_crops_abundant');
  await put('unselected source item', unselected, 'theurgy:alchemical_niter_crops_abundant');
  await step('sulfuric emitter item', ['give', 'theurgy:sulfuric_flux_emitter', '1']);
  await step('sulfuric emitter inventory sync', ['wait', '2']);
  await step('hold sulfuric emitter for selection', ['select', 'theurgy:sulfuric_flux_emitter']);
  await click('select source A', sourceA);
  await click('select source B', sourceB);
  await click('select reformation target', target);
  await click('select reformation result', result);
  await place('reformation placement support', [80, 79, 60], 'minecraft:stone');
  await step('reformation placement approach', ['tp', '80', '82', '59']);
  await step('clear old sulfuric emitter target', ['setblock', ...S(re), 'minecraft:air']);
  await call(['wait', '2']);
  await step('place selected sulfuric emitter', ['place', ...S(re), '--item', 'theurgy:sulfuric_flux_emitter']);
  const reSelection = await data('reformation selection persisted', re);
  assert.ok(text(reSelection).includes('sourcePedestals') && text(reSelection).includes('targetPedestal')
    && text(reSelection).includes('resultPedestal'), text(reSelection));
  await merge('reformation flux', re, '{mercuryFluxHandler:{energy:100}}');
  await step('selected reformation craft window', ['wait', '220']);
  assert.equal(itemAt(await step('selected reformation output', ['container', ...S(result)]), 0),
    'theurgy:alchemical_niter_animals_abundant');
  assert.equal(countAt(await step('unselected source untouched', ['container', ...S(unselected)]), 0), 1,
    'a nearby unselected pedestal must never join the array');

  // ---- canSurvive: attachments break with their support (Java updateShape /
  // neighborChanged). Connector: any non-air support; emitter: sturdy face.
  const blockAt = async pos => (await call(['block', ...S(pos)])).block;
  await step('support lab clear', ['fill', '96', '79', '56', '104', '84', '64', 'minecraft:air']);
  await step('support lab floor', ['fill', '96', '78', '56', '104', '78', '64', 'minecraft:stone']);
  await step('support lab approach', ['tp', '100', '80', '56']);
  for (const [label, id, facing] of [
    ['connector', 'theurgy:logistics_fluid_inserter', 'west'],
    ['emitter', 'theurgy:caloric_flux_emitter', 'west'],
  ]) {
    const pos = [100, 80, 60], support = [101, 80, 60];
    await place(label + ' support', support, 'minecraft:stone');
    await place(label + ' attached', pos, id, `[facing=${facing}]`);
    // A bottom slab is not air but its west face is not sturdy.
    await step(label + ' weaken support', ['setblock', ...S(support), 'minecraft:oak_slab']);
    await call(['wait', '4']);
    if (label === 'connector') {
      assert.equal(await blockAt(pos), id, 'connector must stay on a non-air (even non-sturdy) support');
      await step(label + ' remove support', ['setblock', ...S(support), 'minecraft:air']);
    }
    await call(['wait', '4']);
    assert.equal(await blockAt(pos), 'minecraft:air', label + ' must break once its support no longer holds it');
    const drops = await step(label + ' drop', ['entities']);
    assert.ok(JSON.stringify(drops).includes(id), label + ' must drop itself: ' + JSON.stringify(drops).slice(0, 400));
    await step(label + ' cleanup', ['command', 'kill', '@e[type=item]']);
  }

  return observations;
}

export default {name: 'review-fixes', runCustom};
