// Real headless divination: attune a rod to the sal ammoniac ore tag, then
// right-click air and require a concrete world result in the rod components.
const ROD = 'theurgy:divination_rod_t1';
const ORE = 'theurgy:sal_ammoniac_ore';

function held(inv) {
  const stack = inv?.held;
  return stack ? {item: stack.item, components: String(stack.components ?? '')} : null;
}

export async function runCustom(call, assert) {
  const results = [];
  const step = async (name, args, check) => {
    const reply = await call(args);
    if (check) check(reply);
    results.push({name, reply});
    if (['give', 'setblock', 'tp'].includes(args[0])) await call(['wait', '2']);
    return reply;
  };

  await step('divination lab', ['tp', '40', '80', '36']);
  await step('divination chunks', ['wait', '40']);
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

  await step('move away', ['tp', '40', '80', '36']);
  await step('look air', ['look', '180', '0']);
  await step('scan air', ['use-item']);
  await call(['wait', '20']);
  const scanned = await step('scan inventory', ['inventory']);
  const components = String(held(scanned)?.components ?? '');
  assert.ok(components.includes('divination_pos'), 'scan must store divination_pos: ' + components);
  assert.ok(components.includes('divination_distance'), 'scan must store divination_distance: ' + components);
  assert.ok(/"theurgy:divination_pos"\s*:\s*\[\s*40\s*,\s*80\s*,\s*40\s*\]/.test(components),
    'scan result must identify the ore at 40,80,40: ' + components);
  return results;
}

export default {name: 'divination', runCustom};
