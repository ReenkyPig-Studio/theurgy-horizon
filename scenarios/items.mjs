// Real headless item checks: registered components and the divination rod's
// sneak-use attunement, through actual inventory and right-click control.
const ROD = 'theurgy:divination_rod_t1';
const ROD_ATTUNED = 'theurgy:sulfur_attuned_divination_rod_abundant';

function held(inv) {
  const stack = inv?.held;
  if (!stack) return null;
  return {item: stack.item, components: String(stack.components ?? '')};
}
function count(inv, item) {
  return Number(inv?.counts?.[item] ?? 0);
}

export async function runCustom(call, assert) {
  const results = [];
  const step = async (name, args, check) => {
    const reply = await call(args);
    if (check) check(reply);
    results.push({name, reply});
    if (['give', 'setblock'].includes(args[0])) await call(['wait', '2']);
    return reply;
  };

  await step('items lab', ['tp', '20', '80', '20']);
  await step('items chunks', ['wait', '40']);
  await step('give rod', ['give', ROD, '1']);
  const inv = await step('inventory rod', ['inventory']);
  const rod = held(inv);
  assert.ok(rod && rod.item === ROD, 'divination rod must be the held item');

  await step('attunement block', ['setblock', '20', '80', '22', 'minecraft:stone']);
  await step('approach attunement', ['tp', '20', '80', '24']);
  await step('look at attunement', ['look-at', 'block', '20', '80', '22']);
  await step('sneak down', ['key', 'shift', 'down']);
  await step('attune', ['interact', '20', '80', '22']);
  await step('sneak up', ['key', 'shift', 'up']);
  const attuned = await step('inventory attuned', ['inventory']);
  const after = held(attuned);
  assert.ok(after.components.includes('divination_linked_block'),
    'sneak-use must write the linked block component: ' + after.components);
  assert.ok(after.components.includes('minecraft:stone'),
    'linked block must be the clicked stone: ' + after.components);

  await step('give attuned rod', ['give', ROD_ATTUNED, '1']);
  const inv2 = await step('inventory attuned rod', ['inventory']);
  assert.ok(count(inv2, ROD_ATTUNED) >= 1, 'attuned rod must register');
  assert.ok(JSON.stringify(inv2).includes('"item":"' + ROD_ATTUNED + '"'), 'attuned rod must be in inventory');

  await step('give wand', ['give', 'theurgy:mercurial_wand', '1']);
  await step('give wire', ['give', 'theurgy:copper_wire', '8']);
  const inv3 = await step('inventory tools', ['inventory']);
  assert.ok(count(inv3, 'theurgy:mercurial_wand') >= 1, 'wand must register');
  assert.ok(count(inv3, 'theurgy:copper_wire') >= 8, 'wire stack must register');
  return results;
}

export default {name: 'items', runCustom};
