// Real native Hermetica client evidence: opening the item, category/entry
// navigation, page turns and screenshots in an isolated GUI world.
import fs from 'node:fs/promises';
import path from 'node:path';
import {root} from '../tools/common.mjs';
import {decodePng} from '../tools/png.mjs';

const shots = path.join(root, '.local/agent-book/shots');
// Distinct colours inside the page body (logical rect, screenshot scale).
async function bodyColours(file, body, scale) {
  const png = decodePng(await fs.readFile(file));
  const colours = new Set();
  for (let y = Math.round(body[1] * scale); y < Math.round((body[1] + body[3]) * scale); y++)
    for (let x = Math.round(body[0] * scale); x < Math.round((body[0] + body[2]) * scale); x++) {
      const i = (y * png.width + x) * 4;
      colours.add((png.pixels[i] << 16) | (png.pixels[i + 1] << 8) | png.pixels[i + 2]);
    }
  return colours.size;
}

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

  // Calcination Oven entry: spotlight, multiblock, text, crafting recipe, image
  // (order from the Modonomicon entry JSON). Graphic pages must draw items,
  // blocks or the picture; a text page is the baseline palette.
  const book = JSON.parse(await fs.readFile(path.join(root, 'content/theurgy/data/theurgy/hermetica.json'), 'utf8'));
  const apparatus = book.entries.filter(e => e.category === 'theurgy:apparatus' || e.category === 'apparatus');
  const row = apparatus.findIndex(e => e.id === 'theurgy:apparatus/calcination_oven');
  const kinds = apparatus[row].pages.map(p => p.type);
  assert.deepEqual(kinds, ['modonomicon:spotlight', 'modonomicon:multiblock', 'modonomicon:text', 'modonomicon:crafting_recipe', 'modonomicon:image']);
  assert.ok(row >= 0 && row < 9, 'calcination oven must be on the first list page, row ' + row);
  await step('oven entry', ['click-at', String(px + 60), String(py + 43 + row * 17 + 7)]);
  await fs.mkdir(shots, {recursive: true});
  const scale = 1280 / rect[2];
  const body = [px + 12, py + 62, w - 24, h - 90];
  const colours = {};
  for (let page = 0; page < kinds.length; page++) {
    if (page) await step('oven next page ' + page, ['click-at', String(px + w - 20), String(py + h - 15)]);
    await step('oven page frame ' + page, ['wait', '4']);
    const file = path.join(shots, `oven-${page}-${kinds[page].split(':')[1]}.png`);
    const shot = await step('oven page screenshot ' + page, ['screenshot', file]);
    assert.ok(shot?.ok !== false && shot?.width === 1280, JSON.stringify(shot));
    colours[kinds[page]] = await bodyColours(file, body, scale);
  }
  out.push({name: 'oven page colours', args: [], reply: colours});
  const textColours = colours['modonomicon:text'];
  for (const kind of ['modonomicon:spotlight', 'modonomicon:multiblock', 'modonomicon:crafting_recipe', 'modonomicon:image'])
    assert.ok(colours[kind] > Math.max(48, textColours * 2), `${kind} page draws no graphics: ${JSON.stringify(colours)}`);
  await step('close book', ['key', 'esc', 'down']);
  await step('close key release', ['key', 'esc', 'up']);
  await step('closed frame', ['wait', '2']);
  const closed = await step('gameplay restored', ['screen']);
  assert.ok(!JSON.stringify(closed).includes('"id":"mod"'), 'book did not close: ' + JSON.stringify(closed));
  return out;
}

export default {name: 'book', runCustom};
