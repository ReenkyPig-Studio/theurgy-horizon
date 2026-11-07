// Real native filter GUI and authoritative component edits.
export async function runCustom(call, assert) {
  const out = [];
  const step = async (name, args) => {
    const reply = await call(args);
    out.push({name, args, reply});
    return reply;
  };
  const panel = async () => {
    const screen = await call(['screen']);
    const rect = screen.rect ?? screen.screen?.rect ?? [0, 0, 427, 240];
    const w = Math.min(360, Math.max(240, rect[2] - 16));
    const h = Math.min(240, Math.max(180, rect[3] - 16));
    return {screen, w, h, x: Math.max(4, (rect[2] - w) / 2), y: Math.max(4, (rect[3] - h) / 2)};
  };
  await step('source apple', ['give', 'minecraft:apple', '1']);
  await step('list filter', ['give', 'theurgy:list_filter', '1']);
  await step('select list filter', ['select', 'theurgy:list_filter']);
  await step('aim at sky', ['look', '0', '-90']);
  await step('open list filter', ['use-item']);
  await step('list frame', ['wait', '4']);
  const initial = await panel();
  out.push({name: 'list screen', reply: initial.screen});
  assert.ok(JSON.stringify(initial.screen).includes('mod'));
  await step('list screenshot', ['screenshot', 'filter-list.png']);
  await step('deny mode', ['click-at', String(initial.x + 75), String(initial.y + 30)]);
  await step('add apple ghost', ['click-at', String(initial.x + 45), String(initial.y + initial.h - 35)]);
  await step('components mode', ['click-at', String(initial.x + 190), String(initial.y + 30)]);
  await step('edited list settle', ['wait', '24']);
  await step('edited list screenshot', ['screenshot', 'filter-list-edited.png']);
  const listNbt = await step('list authoritative NBT', ['command', 'data', 'get', 'entity', '@s', 'SelectedItem']);
  const listText = JSON.stringify(listNbt);
  assert.ok(listText.includes('minecraft:apple'), 'ghost apple must reach server filter contents: ' + listText);
  assert.ok(listText.includes('deny'), 'deny mode must be authoritative: ' + listText);
  assert.ok(listText.includes('respect_data_components'), 'component mode must be authoritative: ' + listText);
  await step('remove apple ghost', ['click-at', String(initial.x + 18), String(initial.y + 55)]);
  await step('remove settle', ['wait', '24']);
  const removed = await step('removed authoritative NBT', ['command', 'data', 'get', 'entity', '@s', 'SelectedItem']);
  assert.ok(!JSON.stringify(removed).includes('minecraft:apple'), 'ghost removal must be authoritative');
  await step('close list', ['close']);

  await step('attribute filter', ['give', 'theurgy:attribute_filter', '1']);
  await step('select attribute filter', ['select', 'theurgy:attribute_filter']);
  await step('aim at sky again', ['look', '0', '-90']);
  await step('open attribute filter', ['use-item']);
  await step('attribute frame', ['wait', '4']);
  const attribute = await panel();
  assert.ok(JSON.stringify(attribute.screen).includes('mod'));
  await step('attribute screenshot', ['screenshot', 'filter-attribute.png']);
  await step('add renamed rule', ['click-at', String(attribute.x + 42), String(attribute.y + attribute.h - 35)]);
  await step('add item tag rule', ['click-at', String(attribute.x + 145), String(attribute.y + attribute.h - 35)]);
  await step('attribute settle', ['wait', '24']);
  const attributeNbt = await step('attribute authoritative NBT', ['command', 'data', 'get', 'entity', '@s', 'SelectedItem']);
  const attributeText = JSON.stringify(attributeNbt);
  assert.ok(attributeText.includes('renamed'), 'renamed rule must be authoritative: ' + attributeText);
  assert.ok(attributeText.includes('in_tag'), 'item tag rule must be authoritative: ' + attributeText);
  await step('edited attribute screenshot', ['screenshot', 'filter-attribute-edited.png']);
  await step('open advanced editor', ['click-at', String(attribute.x + 305), String(attribute.y + attribute.h - 35)]);
  await step('type advanced rule', ['type', 'codedefinedgui:standard {"value":"enchanted"}']);
  await step('apply advanced rule', ['key', 'enter', 'down']);
  await step('release enter', ['key', 'enter', 'up']);
  await step('advanced settle', ['wait', '24']);
  const advancedNbt = await step('advanced authoritative NBT', ['command', 'data', 'get', 'entity', '@s', 'SelectedItem']);
  assert.ok(JSON.stringify(advancedNbt).includes('enchanted'), 'advanced JSON rule must be authoritative');
  await step('advanced screenshot', ['screenshot', 'filter-attribute-advanced.png']);
  await step('close attribute', ['close']);
  return out;
}

export default {name: 'filter-ui', runCustom};
