// The three recipe-only DivinationRodGameTests, performed through a real
// crafting table/menu. Semantic assembled stacks are traced on the Java side.
import {recordStack, setblock, wait} from '../differential/harness.mjs';

async function put(call, item, grids) {
  const inventory = await call(['inventory']);
  const source = inventory.hotbar.find(slot => slot.item === item)?.slot;
  if (source === undefined) throw new Error('Missing crafting ingredient ' + item);
  await call(['click-slot', String(source)]);
  for (const grid of grids) await call(['click-slot', String(grid), 'right']);
  await call(['click-slot', String(source)]);
}

async function craft(call, origin, outputId, ingredients) {
  await call(['command', 'clear', '@s']);
  await setblock(call, origin, 'minecraft:crafting_table');
  const counts = new Map();
  for (const [, item] of ingredients) counts.set(item, (counts.get(item) ?? 0) + 1);
  for (const [item, count] of counts) await call(['give', item, String(count)]);
  await call(['tp', String(origin[0]), '82', String(origin[2] - 3)]);
  await call(['look-at', 'block', ...origin.map(String)]);
  await call(['interact', ...origin.map(String)]);
  const grouped = new Map();
  for (const [grid, item] of ingredients) {
    if (!grouped.has(item)) grouped.set(item, []);
    grouped.get(item).push(grid);
  }
  for (const [item, grids] of grouped) await put(call, item, grids);
  await call(['click-slot', '200', 'shift']);
  await call(['close']);
  await wait(call, 2);
  const inventory = await call(['inventory']);
  const stack = [...inventory.hotbar, ...inventory.main].find(slot => slot.item === outputId);
  if (!stack) throw new Error('Crafting produced no ' + outputId + ': ' + JSON.stringify(inventory.counts));
  return stack;
}

export async function runCustom(call) {
  const out = [];
  {
    const stack = await craft(call, [0, 80, 0], 'theurgy:divination_rod_t1', [
      [101, 'minecraft:glass'], [102, 'minecraft:stick'], [104, 'minecraft:stick'],
      [105, 'minecraft:glass'], [106, 'minecraft:stick'],
    ]);
    recordStack(out, 'theurgy:divination_rod_craft_t1', stack);
  }
  {
    const stack = await craft(call, [16, 80, 0], 'theurgy:sulfur_attuned_divination_rod_abundant', [
      [101, 'minecraft:glass'], [102, 'theurgy:alchemical_sulfur_dirt'], [104, 'minecraft:stick'],
      [105, 'minecraft:glass'], [106, 'minecraft:stick'],
    ]);
    recordStack(out, 'theurgy:divination_rod_craft_abundant_dirt_sulfur', stack);
  }
  {
    const stack = await craft(call, [32, 80, 0], 'theurgy:sulfur_attuned_divination_rod_abundant', [
      [101, 'minecraft:glass'], [102, 'theurgy:alchemical_sulfur_copper'], [104, 'minecraft:stick'],
      [105, 'minecraft:glass'], [106, 'minecraft:stick'],
    ]);
    recordStack(out, 'theurgy:divination_rod_craft_abundant_copper_sulfur', stack);
  }
  return out;
}

export default {name: 'differential-divination-recipes', runCustom};
