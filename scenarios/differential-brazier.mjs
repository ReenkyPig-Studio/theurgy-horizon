// PyromanticBrazierGameTests: fuel, heat and drop behaviours.
import {at, record, setblock, container, merge, wait, probe} from '../differential/harness.mjs';

const BLOCK = 'theurgy:pyromantic_brazier', POS = [2, 1, 2], ABOVE = [2, 2, 2];
async function until(call, pos, predicate, description, timeout = 400) {
  for (let elapsed = 0; elapsed < timeout; elapsed += 5) {
    await wait(call, 5);
    if (predicate(await probe(call, pos))) return;
  }
  throw new Error('Timed out waiting for ' + description);
}
async function interact(call, pos, item = null) {
  await call(['command', 'clear', '@s']);
  if (item) await call(['give', item, '1']);
  await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
  await call(['look-at', 'block', ...pos.map(String)]);
  await call(['interact', ...pos.map(String)]);
  await wait(call, 2);
}
async function breakAt(call, pos) {
  await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
  await call(['look-at', 'block', ...pos.map(String)]);
  const broken = await call(['break', ...pos.map(String)]);
  await wait(call, 4);
  return broken;
}
const has = (state, id, count) => state.items.some(stack => stack.id === id && (count === undefined || stack.count === count));

export async function runCustom(call) {
  const out = [];
  let index = 0;
  const origin = () => [(index++) * 16, 80, 0];
  const clear = async o => call(['fill', String(o[0] - 4), '78', '-4', String(o[0] + 4), '86', '4', 'minecraft:air']);
  {
    const o = origin(); await clear(o); await setblock(call, at(o, POS), BLOCK);
    await container(call, at(o, POS), 'minecraft:coal', 1);
    await until(call, at(o, POS), state => state.properties?.lit === 'true', 'brazier LIT');
    await record(out, 'theurgy:pyromantic_brazier_lit_state_updates', call, o, [POS]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, POS), BLOCK);
    await merge(call, at(o, POS), '{Items:[{Slot:0b,id:"minecraft:coal",count:1}]}');
    await record(out, 'theurgy:pyromantic_brazier_insert_fuel', call, o, [POS]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, POS), BLOCK);
    await merge(call, at(o, POS), '{Items:[{Slot:0b,id:"minecraft:diamond",count:1}]}');
    await record(out, 'theurgy:pyromantic_brazier_rejects_non_burnable', call, o, [POS]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, POS), BLOCK);
    await container(call, at(o, POS), 'minecraft:coal', 1);
    await until(call, at(o, POS), state => state.items.length === 0, 'coal depletion');
    await record(out, 'theurgy:pyromantic_brazier_fuel_consumed', call, o, [POS]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, POS), BLOCK);
    await container(call, at(o, POS), 'minecraft:coal', 2);
    await until(call, at(o, POS), state => has(state, 'minecraft:coal', 1) && state.properties?.lit === 'true', 'stacked coal shrink');
    await record(out, 'theurgy:pyromantic_brazier_stacked_fuel_shrinks', call, o, [POS]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, POS), BLOCK);
    await container(call, at(o, POS), 'minecraft:lava_bucket', 1);
    await until(call, at(o, POS), state => has(state, 'minecraft:bucket', 1) && state.properties?.lit === 'true', 'lava bucket remainder');
    await record(out, 'theurgy:pyromantic_brazier_fuel_leaves_remainder', call, o, [POS]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, POS), BLOCK);
    await container(call, at(o, POS), 'minecraft:coal', 64);
    await interact(call, at(o, POS));
    await record(out, 'theurgy:pyromantic_brazier_remove_fuel', call, o, [POS]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, POS), BLOCK);
    await interact(call, at(o, POS), 'minecraft:dirt');
    await interact(call, at(o, POS));
    await record(out, 'theurgy:pyromantic_brazier_rejected_placeable_dont_duplicate', call, o, [POS, ABOVE]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, POS), BLOCK);
    await container(call, at(o, POS), 'minecraft:coal', 3);
    const dropped = await breakAt(call, at(o, POS));
    await record(out, 'theurgy:pyromantic_brazier_drops_items_when_broken', call, o, [POS]);
    out.at(-1).reply.drops = dropped.drops;
    out.at(-1).reply.entities = await call(['entities']);
  }
  for (const [id, machine] of [
    ['theurgy:pyromantic_brazier_heat_calcination_oven', 'theurgy:calcination_oven'],
    ['theurgy:pyromantic_brazier_heat_distiller', 'theurgy:distiller'],
    ['theurgy:pyromantic_brazier_heat_liquefaction_cauldron', 'theurgy:liquefaction_cauldron'],
  ]) {
    const o = origin(); await clear(o); await setblock(call, at(o, POS), BLOCK);
    await setblock(call, at(o, ABOVE), machine);
    await container(call, at(o, POS), 'minecraft:coal', 64);
    await until(call, at(o, POS), state => state.properties?.lit === 'true', 'heat provider');
    await record(out, id, call, o, [POS, ABOVE]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, POS), BLOCK);
    await container(call, at(o, POS), 'minecraft:stick', 1);
    await until(call, at(o, POS), state => state.items.length === 0 && state.properties?.lit === 'false', 'heat depletion', 300);
    await record(out, 'theurgy:pyromantic_brazier_stops_heat_no_fuel', call, o, [POS]);
  }
  return out;
}

export default {name: 'differential-brazier', runCustom};
