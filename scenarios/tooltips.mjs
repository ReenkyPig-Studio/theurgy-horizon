// TooltipHandler evidence in a real survival inventory (creative inventory
// uses a different screen and does not exercise item tooltip providers).
export async function runCustom(call, assert) {
  const out = [];
  const step = async (name, args) => {
    const reply = await call(args);
    out.push({name, args, reply});
    return reply;
  };
  await step('reset creative fixture', ['command', 'gamemode', 'creative', '@s']);
  await step('clear tooltip inventory', ['command', 'clear', '@s']);
  await step('safe tooltip floor', ['fill', '38', '79', '34', '42', '79', '42', 'minecraft:stone']);
  await step('lab', ['tp', '40', '80', '36']);
  await step('chunks', ['wait', '40']);
  await step('ore', ['setblock', '40', '80', '40', 'theurgy:sal_ammoniac_ore']);
  await step('rod', ['give', 'theurgy:divination_rod_t1', '1']);
  await step('approach', ['tp', '40', '80', '38']);
  await step('look', ['look-at', 'block', '40', '80', '40']);
  await step('sneak down', ['key', 'shift', 'down']);
  await step('attune', ['interact', '40', '80', '40']);
  await step('sneak up', ['key', 'shift', 'up']);
  await step('survival', ['command', 'gamemode', 'survival', '@s']);
  await step('open inventory', ['open-inventory']);
  const screen = await step('inventory screen', ['screen']);
  assert.ok(JSON.stringify(screen).includes('inventory'), 'survival inventory required for tooltip evidence');
  await step('hover rod', ['drag-at', '20', '20', '137', '192']);
  await step('tooltip frame', ['wait', '4']);
  await step('tooltip screenshot', ['screenshot', 'client-features-tooltip.png']);
  await step('close inventory', ['close']);
  await step('restore creative fixture', ['command', 'gamemode', 'creative', '@s']);
  return out;
}

export default {name: 'tooltips', runCustom};
