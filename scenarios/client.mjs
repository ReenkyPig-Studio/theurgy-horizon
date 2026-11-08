// GUI evidence for derivative item overlays and the wand ItemHUDProvider.
export async function runCustom(call, assert) {
  const out = [];
  const step = async (name, args) => {
    const reply = await call(args);
    out.push({name, args, reply});
    return reply;
  };
  await step('give sulfur', ['give', 'theurgy:alchemical_sulfur_iron', '1']);
  await step('give niter', ['give', 'theurgy:alchemical_niter_metals_common', '1']);
  await step('open inventory', ['open-inventory']);
  await step('inventory frame', ['wait', '4']);
  const inventory = await step('inventory screen', ['screen']);
  assert.ok(JSON.stringify(inventory).includes('inventory') || JSON.stringify(inventory).includes('creative'));
  await step('derivative jars screenshot', ['screenshot', 'client-derivative-jars.png']);
  await step('shift down', ['key', 'shift', 'down']);
  await step('source frame', ['wait', '4']);
  await step('derivative sources screenshot', ['screenshot', 'client-derivative-sources.png']);
  await step('shift up', ['key', 'shift', 'up']);
  await step('close inventory', ['close']);
  await step('give wand', ['give', 'theurgy:mercurial_wand', '1']);
  await step('select wand', ['select', 'theurgy:mercurial_wand']);
  await step('hud frame', ['wait', '4']);
  await step('wand hud screenshot', ['screenshot', 'client-wand-hud.png']);
  return out;
}

export default {name: 'client', runCustom};
