// Java CalcinationOvenGameTests: all 13 behaviours, using the same fixture
// positions and real Horizon state probes. Expected values are compared with
// the pinned Java traces by tools/differential.mjs, not retyped here.
import {at, record, setblock, container, merge, wait, probe} from '../differential/harness.mjs';

const MACHINE = 'theurgy:calcination_oven';
const BRAZIER = 'theurgy:pyromantic_brazier';
const SALT = 'theurgy:alchemical_salt_strata';
const LOWER = [2, 2, 2];
const UPPER = [2, 3, 2];
const HEAT = [2, 1, 2];

function has(items, id, count) {
  return items.some(stack => stack.id === id && (count === undefined || stack.count === count));
}

async function until(call, pos, predicate, description, timeout = 400) {
  for (let elapsed = 0; elapsed < timeout; elapsed += 10) {
    await wait(call, 10);
    const value = await probe(call, pos);
    if (predicate(value)) return value;
  }
  throw new Error('Timed out waiting for ' + description);
}

async function clear(call, origin) {
  await call(['fill', String(origin[0] - 4), '78', '-4', String(origin[0] + 4), '86', '4', 'minecraft:air']);
}

async function oven(call, origin, heat = false, fuel = 'minecraft:coal') {
  if (heat) await setblock(call, at(origin, HEAT), BRAZIER);
  await setblock(call, at(origin, LOWER), MACHINE);
  await setblock(call, at(origin, UPPER), MACHINE + '[half=upper]');
  if (heat) await container(call, at(origin, HEAT), fuel, fuel === 'minecraft:coal' ? 64 : 1);
}

async function breakAt(call, origin, local) {
  const pos = at(origin, local);
  await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
  await call(['look-at', 'block', ...pos.map(String)]);
  const broken = await call(['break', ...pos.map(String)]);
  await wait(call, 4);
  return broken;
}

export async function runCustom(call) {
  const out = [];
  let index = 0;
  const origin = () => [(index++) * 16, 80, 0];

  {
    const o = origin(); await clear(call, o);
    await setblock(call, at(o, LOWER), 'minecraft:air');
    await setblock(call, at(o, UPPER), 'minecraft:air');
    await setblock(call, at(o, LOWER), MACHINE);
    await setblock(call, at(o, UPPER), MACHINE + '[half=upper]');
    await record(out, 'theurgy:calcination_oven_placement', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(call, o); await oven(call, o);
    await record(out, 'theurgy:calcination_oven_be_only_lower', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(call, o); await oven(call, o);
    await breakAt(call, o, LOWER);
    await record(out, 'theurgy:calcination_oven_break_lower_destroys_upper', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(call, o); await oven(call, o);
    await breakAt(call, o, UPPER);
    await record(out, 'theurgy:calcination_oven_break_upper_destroys_lower', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(call, o); await oven(call, o);
    await merge(call, at(o, LOWER), '{Items:[{Slot:0b,id:"minecraft:cobblestone",count:3}]}');
    const dropped = await breakAt(call, o, LOWER);
    await record(out, 'theurgy:calcination_oven_drops_items_when_broken', call, o, [LOWER, UPPER]);
    out.at(-1).reply.drops = dropped.drops;
    out.at(-1).reply.entities = await call(['entities']);
  }
  {
    const o = origin(); await clear(call, o); await oven(call, o, true);
    await container(call, at(o, LOWER), 'minecraft:cobblestone', 1);
    await record(out, 'theurgy:calcination_oven_insert_input', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(call, o); await oven(call, o, true);
    await merge(call, at(o, LOWER), `{Items:[{Slot:1b,id:"${SALT}",count:1}]}`);
    await call(['command', 'clear', '@s']);
    await call(['tp', String(o[0]), '82', '-3']);
    await call(['look-at', 'block', ...at(o, LOWER).map(String)]);
    await call(['interact', ...at(o, LOWER).map(String)]);
    await record(out, 'theurgy:calcination_oven_extract_output', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(call, o); await oven(call, o, true);
    await container(call, at(o, LOWER), 'minecraft:cobblestone', 1);
    await until(call, at(o, LOWER), state => Number(state.data?.[4]) === 1, 'calcination processing start');
    await record(out, 'theurgy:calcination_oven_processing_starts', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(call, o); await oven(call, o, true);
    await container(call, at(o, LOWER), 'minecraft:cobblestone', 1);
    await until(call, at(o, LOWER), state => state.properties?.lit === 'true', 'calcination LIT');
    await record(out, 'theurgy:calcination_oven_lit_during_processing', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(call, o); await oven(call, o, true);
    await container(call, at(o, LOWER), 'minecraft:cobblestone', 1);
    await until(call, at(o, LOWER), state => has(state.items, SALT), 'calcination output');
    await record(out, 'theurgy:calcination_oven_input_consumed_output_produced', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(call, o); await oven(call, o, true);
    await merge(call, at(o, LOWER), `{Items:[{Slot:0b,id:"minecraft:cobblestone",count:1},{Slot:1b,id:"${SALT}",count:1}]}`);
    await wait(call, 4);
    await record(out, 'theurgy:calcination_oven_blocked_output_does_not_consume_input', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(call, o); await oven(call, o);
    await setblock(call, at(o, HEAT), BRAZIER);
    await record(out, 'theurgy:calcination_oven_stops_without_heat', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(call, o); await oven(call, o, true);
    await setblock(call, at(o, HEAT), BRAZIER);
    await record(out, 'theurgy:calcination_oven_stops_without_input', call, o, [LOWER, UPPER, HEAT]);
  }
  return out;
}

export default {name: 'differential-calcination', runCustom};
