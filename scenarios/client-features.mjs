// Native rendering/config integration evidence: wire geometry, held-stack-fit
// outlines, derivative overlay settings, target HUD, and colored particles.
export async function runCustom(call, assert) {
  const out = [];
  const step = async (name, args) => {
    const reply = await call(args);
    out.push({name, args, reply});
    return reply;
  };
  await step('give sulfur', ['give', 'theurgy:alchemical_sulfur_iron', '1']);
  await step('open creative inventory', ['open-inventory']);
  await step('inventory frame', ['wait', '4']);
  await step('derivative source disabled screenshot', ['screenshot', 'client-features-derivative.png']);
  await step('close inventory', ['close']);

  const a = [30, 80, 40], b = [34, 80, 40];
  await step('connector A', ['setblock', ...a.map(String), 'theurgy:logistics_item_inserter[facing=east]']);
  await step('connector B', ['setblock', ...b.map(String), 'theurgy:logistics_item_extractor[facing=west]']);
  await step('give wire', ['give', 'theurgy:copper_wire', '1']);
  for (const pos of [a, b]) {
    await step('wire approach', ['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
    await step('wire look', ['look-at', 'block', ...pos.map(String)]);
    await step('wire endpoint', ['interact', ...pos.map(String)]);
    await step('wire settle', ['wait', '2']);
  }
  await step('wire view approach', ['tp', '30', '82', '37']);
  await step('wire view look', ['look-at', 'block', '30', '80', '40']);
  await step('wire geometry screenshot', ['screenshot', 'client-features-wires.png']);

  await step('brazier', ['setblock', '30', '79', '40', 'theurgy:pyromantic_brazier']);
  await step('oven', ['setblock', '30', '80', '40', 'theurgy:calcination_oven']);
  await step('fuel', ['container', '30', '79', '40', 'minecraft:coal', '4']);
  await step('input', ['container', '30', '80', '40', 'minecraft:cobblestone', '1']);
  await step('held coal', ['give', 'minecraft:coal', '1']);
  await step('fit view approach', ['tp', '30', '82', '37']);
  await step('fit view look', ['look-at', 'block', '30', '80', '40']);
  await step('fit outline screenshot', ['screenshot', 'client-features-fit.png']);
  await step('processing and particles', ['wait', '110']);
  await step('processing probe', ['command', 'theurgy_probe', '30', '80', '40']);
  await step('particle view approach', ['tp', '30', '83', '38']);
  await step('particle view look', ['look-at', 'block', '30', '80', '40']);
  await step('particle screenshot', ['screenshot', 'client-features-particles.png']);

  await step('capacitor', ['setblock', '36', '80', '40', 'theurgy:mercury_capacitor']);
  await step('capacitor flux', ['command', 'data', 'merge', 'block', '36', '80', '40', '{mercuryFluxHandler:{energy:500}}']);
  await step('target approach', ['tp', '36', '82', '37']);
  await step('target look', ['look-at', 'block', '36', '80', '40']);
  await step('target HUD screenshot', ['screenshot', 'client-features-target.png']);
  return out;
}

export default {name: 'client-features', runCustom};
