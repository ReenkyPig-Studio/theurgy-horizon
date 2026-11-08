// ServerConfig.sulfurSourceToBlockMapping evidence using the same attunement
// fixture as scenarios/divination.mjs, with an isolated config override.
export async function runCustom(call, assert) {
  const out = [];
  const step = async (name, args) => {
    const reply = await call(args);
    out.push({name, args, reply});
    return reply;
  };
  await step('lab', ['tp', '40', '80', '36']);
  await step('chunks', ['wait', '40']);
  await step('ore', ['setblock', '40', '80', '40', 'theurgy:sal_ammoniac_ore']);
  await step('rod', ['give', 'theurgy:divination_rod_t1', '1']);
  await step('approach', ['tp', '40', '80', '38']);
  await step('look', ['look-at', 'block', '40', '80', '40']);
  await step('sneak down', ['key', 'shift', 'down']);
  await step('attune', ['interact', '40', '80', '40']);
  await step('sneak up', ['key', 'shift', 'up']);
  const inventory = await step('inventory', ['inventory']);
  const components = JSON.stringify(inventory);
  assert.ok(components.includes('divination_linked_block'), 'attunement must link the ore: '+components);
  assert.ok(components.includes('minecraft:stone'), 'config override must map the ore to stone: '+components);
  return out;
}

export default {name: 'config-attunement', runCustom};
