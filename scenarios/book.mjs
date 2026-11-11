// Real native Hermetica client evidence in an isolated GUI world: the
// Modonomicon-style node category screen, category switching, dragging the
// map, opening entries and flipping double pages, with screenshots of each.
// Layout mirrors native/theurgy/book.rs (Modonomicon BookParentNodeScreen /
// BookCategoryNodeScreen / BookPaginatedScreen constants).
import fs from 'node:fs/promises';
import path from 'node:path';
import {root} from '../tools/common.mjs';

const shots = path.join(root, '.local/agent-book/shots');

function layout(rect) {
  const [, , W, H] = rect;
  const fw = Math.floor(W - 60), fh = Math.floor(H - 20);
  const fx = Math.floor((W - fw) / 2), fy = Math.floor((H - fh) / 2);
  const inner = [fx + 7, fy + 7, fw - 14, fh - 14];
  const button = i => [fx - 25 + 12, fy - 14 + 32 + 22 * i + 10];
  const book = [Math.floor((W - 272) / 2), Math.floor((H - 178) / 2)];
  // BookSideButtonRenderer: 28 px of a 44 px button right of the frame.
  const side = y => [fx + fw + 10, y + 10];
  return {W, H, inner, button, book,
    research: side(fy + 18), search: side(fh - 26),
    next: [book[0] + 258 + 9, book[1] + 174 + 5],
    exit: [book[0] + 262 + 6, book[1] - 2 + 6]};
}

export async function runCustom(call, assert) {
  const out = [];
  const step = async (name, args) => {
    const reply = await call(args);
    out.push({name, args, reply});
    return reply;
  };
  await fs.mkdir(shots, {recursive: true});
  const shot = async name => {
    await step('frame ' + name, ['wait', '4']);
    const reply = await step('screenshot ' + name, ['screenshot', path.join(shots, name + '.png')]);
    assert.ok(reply?.ok !== false, JSON.stringify(reply));
  };
  const book = JSON.parse(await fs.readFile(path.join(root, 'content/theurgy/data/theurgy/hermetica.json'), 'utf8'));

  await step('give Hermetica', ['give', 'theurgy:the_hermetica', '1']);
  await step('open Hermetica', ['use-item']);
  await step('open frame', ['wait', '4']);
  const initial = await step('native mod screen', ['screen']);
  assert.ok(JSON.stringify(initial).includes('mod'), 'native Hermetica screen did not open: ' + JSON.stringify(initial));
  const L = layout(initial.rect ?? initial.screen?.rect ?? [0, 0, 427, 240]);
  await shot('category-getting-started');
  // A new player sees locked entries: Shift-click the research progress
  // button (ResearchProgressButton) to mark every entry viewed once.
  await step('shift down', ['key', 'shift', 'down']);
  await step('unlock all', ['click-at', ...L.research.map(String)]);
  await step('shift up', ['key', 'shift', 'up']);
  await shot('category-unlocked');
  await step('open search', ['click-at', ...L.search.map(String)]);
  await shot('search');
  await step('close search', ['key', 'esc', 'down']);
  await step('close search release', ['key', 'esc', 'up']);

  // Scroll state per category; zoom starts at 0.7 (BookCategoryNodeScreen).
  const zoom = 0.7;
  const scroll = {};
  const where = (category, entry) => {
    const s = scroll[category] ?? [0, 0];
    const [ix, iy, iw, ih] = L.inner;
    const x = (entry.x * 30 + 2 + iw / 2 / zoom - s[0] / 2) * zoom + 13 * zoom;
    const y = (entry.y * 30 + 2 + ih / 2 / zoom - s[1] / 2) * zoom + 13 * zoom;
    return [x, y];
  };
  const openEntry = async (categoryIndex, id) => {
    const category = book.categories[categoryIndex].id;
    await step('category ' + category, ['click-at', ...L.button(categoryIndex).map(String)]);
    const entry = book.entries.find(e => e.id === id);
    assert.ok(entry, 'no entry ' + id);
    // Drag the map until the entry is well inside the frame (scroll -= 1.5 * drag).
    const [ix, iy, iw, ih] = L.inner;
    const centre = [ix + iw / 2, iy + ih / 2];
    for (let i = 0; i < 6; i++) {
      const [x, y] = where(category, entry);
      const d = [centre[0] - x, centre[1] - y];
      if (Math.abs(d[0]) < iw / 3 && Math.abs(d[1]) < ih / 3) break;
      const drag = d.map(v => Math.max(-90, Math.min(90, v / (0.75 * zoom))));
      await step('drag map', ['drag-at', String(centre[0]), String(centre[1]), String(centre[0] + drag[0]), String(centre[1] + drag[1])]);
      const s = scroll[category] ?? [0, 0];
      scroll[category] = [s[0] - drag[0] * 1.5, s[1] - drag[1] * 1.5];
    }
    await shot('category-' + category);
    await step('open ' + id, ['click-at', ...where(category, entry).map(v => String(Math.round(v)))]);
    for (let spread = 0; spread < entry.pages.length; spread += 2) {
      if (spread) await step('next spread', ['click-at', ...L.next.map(String)]);
      await shot(`${id.split('/')[1]}-${spread / 2}`);
    }
    await step('exit entry', ['click-at', ...L.exit.map(String)]);
  };
  // liquefaction_cauldron: spotlight, multiblock, text, crafting, image pages.
  await openEntry(1, 'theurgy:apparatus/liquefaction_cauldron');
  const calcination = book.entries.find(e => e.pages.some(p => p.type === 'theurgy:calcination_recipe'));
  await openEntry(book.categories.findIndex(c => calcination.category.endsWith(c.id)), calcination.id);
  await openEntry(2, 'theurgy:logistics/mercurial_wand');

  await step('close book', ['key', 'esc', 'down']);
  await step('close key release', ['key', 'esc', 'up']);
  await step('closed frame', ['wait', '2']);
  const closed = await step('gameplay restored', ['screen']);
  assert.ok(!JSON.stringify(closed).includes('"id":"mod"'), 'book did not close: ' + JSON.stringify(closed));
  return out;
}

export default {name: 'book', runCustom};
