// Real native Hermetica client evidence: opening the item, category/entry
// navigation, page turns and screenshots in an isolated GUI world.
export async function runCustom(call, assert) {
  const out = [];
  const step = async (name, args) => {
    const reply = await call(args);
    out.push({name, args, reply});
    return reply;
  };
  await step('give Hermetica', ['give', 'theurgy:the_hermetica', '1']);
  await step('open Hermetica', ['use-item']);
  await step('open frame', ['wait', '4']);
  const initial = await step('native mod screen', ['screen']);
  assert.ok(JSON.stringify(initial).includes('mod'), 'native Hermetica screen did not open: ' + JSON.stringify(initial));
  await step('categories screenshot', ['screenshot', 'hermetica-categories.png']);

  const rect = initial.rect ?? initial.screen?.rect ?? [0, 0, 427, 240];
  // Use the same centred panel formula as native/theurgy/client.rs.
  const w = Math.min(360, Math.max(240, rect[2] - 16));
  const h = Math.min(240, Math.max(180, rect[3] - 16));
  const px = Math.max(4, (rect[2] - w) / 2);
  const py = Math.max(4, (rect[3] - h) / 2);
  await step('open first entry', ['click-at', String(px + 25), String(py + 50)]);
  await step('entry frame', ['wait', '4']);
  const entry = await step('entry screen', ['screen']);
  assert.ok(JSON.stringify(entry).includes('mod'));
  await step('entry screenshot', ['screenshot', 'hermetica-entry.png']);
  await step('next page', ['click-at', String(px + w - 20), String(py + h - 15)]);
  await step('page frame', ['wait', '4']);
  await step('page screenshot', ['screenshot', 'hermetica-page.png']);
  await step('back to entries', ['click-at', String(px + 25), String(py + 49)]);
  await step('apparatus category', ['click-at', String(px + 130), String(py + 30)]);
  await step('apparatus frame', ['wait', '4']);
  await step('apparatus screenshot', ['screenshot', 'hermetica-apparatus.png']);
  await step('close book', ['key', 'esc', 'down']);
  await step('close key release', ['key', 'esc', 'up']);
  await step('closed frame', ['wait', '2']);
  const closed = await step('gameplay restored', ['screen']);
  assert.ok(!JSON.stringify(closed).includes('"id":"mod"'), 'book did not close: ' + JSON.stringify(closed));
  return out;
}

export default {name: 'book', runCustom};
