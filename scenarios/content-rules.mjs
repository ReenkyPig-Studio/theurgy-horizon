// Runtime recipe-condition and tag evidence, not a recipe-count smoke test.
export async function runCustom(call, assert) {
  const out = [];
  const step = async (name, args) => {
    const reply = await call(args);
    out.push({name, args, reply});
    return reply;
  };
  await step('vat', ['setblock', '10', '80', '10', 'theurgy:fermentation_vat']);
  await step('log ingredient', ['container', '10', '80', '10', 'theurgy:alchemical_sulfur_porkchop', '1']);
  await step('oak log', ['container', '10', '80', '10', 'minecraft:oak_log', '1']);
  await step('water solvent', ['command', 'data', 'merge', 'block', '10', '80', '10', '{fluids:[{Fluid:"minecraft:water",Amount:125}]}']);
  await step('close lid', ['command', 'theurgy_state', '10', '80', '10', 'open', 'false']);
  await step('process conditioned recipe', ['wait', '210']);
  const result = await step('conditioned result', ['command', 'theurgy_probe', '10', '80', '10']);
  assert.ok(JSON.stringify(result).includes('alchemical_niter_animals_abundant'), 'runtime tags/conditions must produce niter: '+JSON.stringify(result));

  await step('invalid vat', ['setblock', '14', '80', '10', 'theurgy:fermentation_vat']);
  await step('invalid ingredient', ['container', '14', '80', '10', 'minecraft:diamond', '1']);
  await step('invalid solvent', ['command', 'data', 'merge', 'block', '14', '80', '10', '{fluids:[{Fluid:"minecraft:water",Amount:125}]}']);
  await step('invalid close lid', ['command', 'theurgy_state', '14', '80', '10', 'open', 'false']);
  await step('invalid wait', ['wait', '20']);
  const invalid = await step('invalid remains', ['command', 'theurgy_probe', '14', '80', '10']);
  assert.ok(JSON.stringify(invalid).includes('minecraft:diamond'), 'invalid ingredient must not be consumed: '+JSON.stringify(invalid));

  await step('brazier', ['setblock', '18', '80', '10', 'theurgy:pyromantic_brazier']);
  await step('nonfuel', ['container', '18', '80', '10', 'minecraft:diamond', '1']);
  const nonfuel = await step('nonfuel remains', ['command', 'theurgy_probe', '18', '80', '10']);
  assert.ok(JSON.stringify(nonfuel).includes('minecraft:diamond'), 'non-fuel must be rejected: '+JSON.stringify(nonfuel));
  return out;
}

export default {name: 'content-rules', runCustom};
