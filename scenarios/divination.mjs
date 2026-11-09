// Real headless divination: attune a rod to the sal ammoniac ore tag, then
// right-click air and require a concrete world result in the rod components.
const ROD = 'theurgy:divination_rod_t1';
const ORE = 'theurgy:sal_ammoniac_ore';

function held(inv) {
  const stack = inv?.held;
  return stack ? {item: stack.item, components: String(stack.components ?? ''), damage: Number(stack.damage ?? 0)} : null;
}

export async function runCustom(call, assert) {
  const results = [];
  const step = async (name, args, check) => {
    const reply = await call(args);
    if (check) check(reply);
    results.push({name, reply});
    if (['give', 'setblock', 'tp', 'command'].includes(args[0])) await call(['wait', '2']);
    return reply;
  };

  await step('reset divination game mode', ['command', 'gamemode', 'creative', '@s']);
  await step('clear divination inventory', ['command', 'clear', '@s']);
  await step('divination lab', ['tp', '40', '80', '36']);
  await step('divination chunks', ['wait', '40']);
  // The native worldgen hook can place real sal ammoniac ores in this reused
  // test world. Clear the fixture airspace so the Java nearest-result probe
  // sees only the intentionally placed source and deepslate variant.
  await step('clear divination fixture airspace', ['fill', '24', '64', '24', '55', '95', '55', 'minecraft:air']);
  await step('divination fixture floor', ['fill', '24', '79', '24', '55', '79', '55', 'minecraft:stone']);
  await step('fixture position', ['tp', '40', '80', '36']);
  await step('fixture chunks', ['wait', '20']);
  await step('ore', ['setblock', '40', '80', '40', ORE]);
  await step('give rod', ['give', ROD, '1']);
  const inv = await step('rod inventory', ['inventory']);
  assert.equal(held(inv)?.item, ROD, 'rod must be held');
  await step('approach ore', ['tp', '40', '80', '38']);
  await step('look ore', ['look-at', 'block', '40', '80', '40']);
  await step('sneak down', ['key', 'shift', 'down']);
  await step('attune ore', ['interact', '40', '80', '40']);
  await step('sneak up', ['key', 'shift', 'up']);
  const linked = await step('linked rod', ['inventory']);
  assert.ok(String(held(linked)?.components).includes('divination_linked_block'),
    'attunement must link the ore: ' + JSON.stringify(held(linked)));
  assert.ok(String(held(linked).components).includes(ORE), 'linked block must be the ore');

  // A normal-ore attunement must expand through c:ores/sal_ammoniac. Remove
  // the exact linked block and leave only its deepslate variant, high enough
  // that a 3D distance bucket would differ from Java's horizontal 2D bucket.
  await step('remove exact linked ore', ['setblock', '40', '80', '40', 'minecraft:air']);
  await step('deepslate variant only', ['setblock', '40', '88', '40', 'theurgy:deepslate_sal_ammoniac_ore']);
  await step('move away', ['tp', '40', '80', '40']);
  const freshForScan = await step('fresh rod before timed scan', ['inventory']);
  assert.equal(held(freshForScan)?.damage, 0, 'the timed durability probe must start with an undamaged rod');
  await step('survival durability fixture', ['command', 'gamemode', 'survival', '@s']);
  await step('look air', ['look', '180', '0']);
  await step('scan air for the full Java duration', ['use-item', '--hold', '3']);
  await call(['wait', '20']);
  const scanned = await step('scan inventory', ['inventory']);
  const components = String(held(scanned)?.components ?? '');
  assert.ok(components.includes('divination_pos'), 'scan must store divination_pos: ' + components);
  assert.ok(components.includes('divination_distance'), 'scan must store divination_distance: ' + components);
  assert.ok(/"theurgy:divination_pos"\s*:\s*\[\s*(?:I;)?\s*40\s*,\s*88\s*,\s*40\s*\]/.test(components),
    'ore-tag scan must identify the deepslate variant at 40,88,40: ' + components);
  assert.ok(components.includes('"theurgy:divination_distance":0.0f'),
    'Java distance is horizontal (2D), so vertical separation must not raise the bucket: ' + components);
  assert.equal(held(scanned)?.damage, 1, 'a completed survival scan must cost exactly one durability');

  // Early cancellation does not rescan or damage the rod; it restores the
  // previous result instead of leaving SEARCHING=8 in the component.
  await step('early release', ['use-item', '--hold', '0.25']);
  await call(['wait', '10']);
  const released = await step('release inventory', ['inventory']);
  const releasedComponents = String(held(released)?.components ?? '');
  assert.ok(releasedComponents.includes('"theurgy:divination_distance":0.0f'),
    'early release must restore the previous result, not SEARCHING/NOT_FOUND: ' + releasedComponents);
  assert.ok(releasedComponents.includes('divination_pos'), 'early release must preserve the previous position');
  assert.equal(held(released)?.damage, 1, 'early release must not cost additional durability');
  await step('restore creative fixture', ['command', 'gamemode', 'creative', '@s']);
  return results;
}

export default {name: 'divination', runCustom};
