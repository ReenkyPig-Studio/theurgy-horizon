// Native JEI/EMI-style recipe catalog evidence.
export async function runCustom(call, assert) {
  const out = [];
  const step = async (name, args) => {
    const reply = await call(args);
    out.push({name, args, reply});
    return reply;
  };
  await step('open recipe catalog', ['key', 'r', 'down']);
  await step('release recipe key', ['key', 'r', 'up']);
  await step('catalog frame', ['wait', '4']);
  const catalog = await step('catalog screen', ['screen']);
  assert.ok(JSON.stringify(catalog).includes('mod'), 'recipe catalog did not open: ' + JSON.stringify(catalog));
  await step('catalog screenshot', ['screenshot', 'recipes-catalog.png']);
  const rect = catalog.rect ?? catalog.screen?.rect ?? [0, 0, 427, 240];
  const w = Math.min(360, Math.max(240, rect[2] - 16));
  const h = Math.min(240, Math.max(180, rect[3] - 16));
  const x = Math.max(4, (rect[2] - w) / 2), y = Math.max(4, (rect[3] - h) / 2);
  await step('open first recipe', ['click-at', String(x + 40), String(y + 48)]);
  await step('detail frame', ['wait', '4']);
  const detail = await step('detail screen', ['screen']);
  assert.ok(JSON.stringify(detail).includes('mod'));
  await step('detail screenshot', ['screenshot', 'recipes-detail.png']);
  await step('back to catalog', ['click-at', String(x + 20), String(y + 48)]);
  await step('cycle type', ['click-at', String(x + 80), String(y + 28)]);
  await step('type screenshot', ['screenshot', 'recipes-type.png']);
  await step('close catalog', ['close']);
  return out;
}

export default {name: 'recipes', runCustom};
