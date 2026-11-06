export async function runCustom(call, assert) {
  const results = [];
  const step = async (name, args) => {
    const reply = await call(args);
    results.push({name, args, reply});
    return reply;
  };

  await step('worldgen chunks', ['wait', '60']);
  const ore = await step('sal ore', ['find-block', 'theurgy:sal_ammoniac_ore', '64']);
  assert.ok(ore.count > 0, JSON.stringify(ore));
  const first = ore.nearest[0].pos;
  assert.equal((await step('ore before', ['block', ...first.map(String)])).block,
    'theurgy:sal_ammoniac_ore');

  // Deepslate replaceables do not occur within 64 blocks above the surface.
  // Search from inside their actual height band instead of asserting zero there.
  await step('descend to deepslate band', ['tp', '10', '-20', '0']);
  await step('deepslate chunks', ['wait', '40']);
  const deep = await step('deepslate ore', ['find-block', 'theurgy:deepslate_sal_ammoniac_ore', '64']);
  const vanillaDeep = await step('vanilla deepslate probe', ['find-block', 'minecraft:deepslate', '64']);
  await step('vanilla stone probe', ['find-block', 'minecraft:stone', '64']);
  let deepFirst = null;
  if (vanillaDeep.count > 0) {
    assert.ok(deep.count > 0, JSON.stringify(deep));
    deepFirst = deep.nearest[0].pos;
    assert.equal((await step('deep ore before', ['block', ...deepFirst.map(String)])).block,
      'theurgy:deepslate_sal_ammoniac_ore');
  } else {
    // The pinned Horizon generator has no deepslate blocks at all in this
    // terrain. Record an explicit environment skip, never manufacture the
    // deepslate target by replacing stone contrary to ConfiguredFeatures.
    results.push({name: 'deepslate target unavailable in Horizon terrain', skipped: true,
      vanillaDeepslateCount: vanillaDeep.count});
  }

  await step('reload', ['reload']);
  await step('loaded', ['wait', '40']);
  if (deepFirst) {
    assert.equal((await step('deep ore after', ['block', ...deepFirst.map(String)])).block,
      'theurgy:deepslate_sal_ammoniac_ore');
  }
  await step('return to stone ore', ['tp', String(first[0]), String(first[1] + 2), String(first[2])]);
  await step('surface chunks reloaded', ['wait', '40']);
  assert.equal((await step('ore after', ['block', ...first.map(String)])).block,
    'theurgy:sal_ammoniac_ore');
  const again = await step('count after', ['find-block', 'theurgy:sal_ammoniac_ore', '64']);
  assert.ok(again.count > 0 && again.count <= ore.count, 'old chunks must not regenerate');
  return results;
}

export default {name: 'worldgen', runCustom};
