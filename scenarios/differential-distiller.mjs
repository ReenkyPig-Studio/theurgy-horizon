// Java DistillerGameTests: all 11 behaviours with real Horizon state probes.
import {at, record, setblock, container, merge, wait, probe} from '../differential/harness.mjs';

const MACHINE = 'theurgy:distiller';
const BRAZIER = 'theurgy:pyromantic_brazier';
const OUTPUT = 'theurgy:mercury_shard';
const LOWER = [2, 2, 2], UPPER = [2, 3, 2], HEAT = [2, 1, 2];

async function until(call, pos, predicate, description, timeout = 400) {
  for (let elapsed = 0; elapsed < timeout; elapsed += 1) {
    await wait(call, 1); // GameTestHelper.succeedWhen polls every tick
    if (predicate(await probe(call, pos))) return;
  }
  throw new Error('Timed out waiting for ' + description);
}
async function clear(call, o) {
  await call(['fill', String(o[0] - 4), '78', '-4', String(o[0] + 4), '86', '4', 'minecraft:air']);
}
async function machine(call, o, heat = false, fuel = 'minecraft:coal') {
  if (heat) await setblock(call, at(o, HEAT), BRAZIER);
  await setblock(call, at(o, LOWER), MACHINE);
  await setblock(call, at(o, UPPER), MACHINE + '[half=upper]');
  if (heat) await container(call, at(o, HEAT), fuel, fuel === 'minecraft:coal' ? 64 : 1);
}
async function breakAt(call, o, local) {
  const pos = at(o, local);
  await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
  await call(['look-at', 'block', ...pos.map(String)]);
  const broken = await call(['break', ...pos.map(String)]);
  await wait(call, 4);
  return broken;
}
const has = (state, id) => state.items.some(stack => stack.id === id);

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
    await record(out, 'theurgy:distiller_placement', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(call, o); await machine(call, o);
    await record(out, 'theurgy:distiller_be_only_lower', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(call, o); await machine(call, o); await breakAt(call, o, LOWER);
    await record(out, 'theurgy:distiller_break_lower_destroys_upper', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(call, o); await machine(call, o); await breakAt(call, o, UPPER);
    await record(out, 'theurgy:distiller_break_upper_destroys_lower', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(call, o); await machine(call, o);
    await merge(call, at(o, LOWER), '{Items:[{Slot:0b,id:"minecraft:oak_log",count:3}]}');
    const dropped = await breakAt(call, o, LOWER);
    await record(out, 'theurgy:distiller_drops_items_when_broken', call, o, [LOWER, UPPER]);
    out.at(-1).reply.drops = dropped.drops;
    out.at(-1).reply.entities = await call(['entities']);
  }
  {
    const o = origin(); await clear(call, o); await machine(call, o, true);
    await container(call, at(o, LOWER), 'minecraft:oak_log', 1);
    await record(out, 'theurgy:distiller_insert_input', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(call, o); await machine(call, o, true);
    await merge(call, at(o, LOWER), `{Items:[{Slot:1b,id:"${OUTPUT}",count:1}]}`);
    await call(['command', 'clear', '@s']);
    await call(['tp', String(o[0]), '82', '-3']);
    await call(['look-at', 'block', ...at(o, LOWER).map(String)]);
    await call(['interact', ...at(o, LOWER).map(String)]);
    await record(out, 'theurgy:distiller_extract_output', call, o, [LOWER, UPPER, HEAT]);
  }
  for (const kind of ['processing', 'lit', 'complete']) {
    const o = origin(); await clear(call, o); await machine(call, o, true);
    await container(call, at(o, LOWER), 'minecraft:oak_log', 2);
    if (kind === 'processing') {
      await until(call, at(o, LOWER), state => Number(state.data?.[4]) === 1, 'distiller processing');
      await record(out, 'theurgy:distiller_processing_starts', call, o, [LOWER, UPPER, HEAT]);
    } else if (kind === 'lit') {
      await until(call, at(o, LOWER), state => state.properties?.lit === 'true', 'distiller LIT');
      await record(out, 'theurgy:distiller_lit_during_processing', call, o, [LOWER, UPPER, HEAT]);
    } else {
      await until(call, at(o, LOWER), state => has(state, OUTPUT), 'distiller output');
      await record(out, 'theurgy:distiller_input_consumed_output_produced', call, o, [LOWER, UPPER, HEAT]);
    }
  }
  {
    const o = origin(); await clear(call, o); await machine(call, o);
    await setblock(call, at(o, HEAT), BRAZIER);
    await record(out, 'theurgy:distiller_stops_without_heat', call, o, [LOWER, UPPER, HEAT]);
  }
  return out;
}

export default {name: 'differential-distiller', runCustom};
