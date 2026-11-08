// IncubatorGameTests: all 18 behaviours not already covered by placement defaults.
import {at, record, setblock, container, merge, wait, probe} from '../differential/harness.mjs';

const BLOCK = 'theurgy:incubator', BRAZIER = 'theurgy:pyromantic_brazier';
const LOWER = [2, 2, 2], UPPER = [2, 3, 2], HEAT = [2, 1, 2];
const MERCURY = [2, 2, 1], SALT = [3, 2, 2], SULFUR = [2, 2, 3];
const VESSELS = [MERCURY, SALT, SULFUR];
const ALL = [HEAT, LOWER, UPPER, ...VESSELS];
const WITHOUT_HEAT = [LOWER, UPPER, ...VESSELS];

async function incubator(call, o, vessels = false, heat = false) {
  if (heat) await setblock(call, at(o, HEAT), BRAZIER);
  await setblock(call, at(o, LOWER), BLOCK);
  await setblock(call, at(o, UPPER), BLOCK + '[half=upper]');
  if (heat) await container(call, at(o, HEAT), 'minecraft:coal', 64);
  if (vessels) {
    await setblock(call, at(o, MERCURY), 'theurgy:incubator_mercury_vessel');
    await setblock(call, at(o, SALT), 'theurgy:incubator_salt_vessel');
    await setblock(call, at(o, SULFUR), 'theurgy:incubator_sulfur_vessel');
  }
}
async function inputs(call, o) {
  await container(call, at(o, MERCURY), 'theurgy:mercury_shard', 1);
  await container(call, at(o, SALT), 'theurgy:alchemical_salt_creature', 1);
  await container(call, at(o, SULFUR), 'theurgy:alchemical_sulfur_bone', 1);
}
async function processing(call, o) {
  await incubator(call, o, true, true);
  await wait(call, 5);
  await inputs(call, o);
}
async function breakAt(call, pos) {
  await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
  await call(['look-at', 'block', ...pos.map(String)]);
  await call(['break', ...pos.map(String)]);
  await wait(call, 4);
}
// The Java helper calls useBlock directly; use the same authoritative native
// interaction hook when Horizon's raycast cannot see a double-high lower half.
async function extract(call, pos) {
  await call(['command', 'clear', '@s']);
  await call(['command', 'theurgy_interact', ...pos.map(String), 'north']);
  await wait(call, 2);
}
async function until(call, pos, predicate, description, timeout = 400) {
  for (let elapsed = 0; elapsed < timeout; elapsed += 10) {
    await wait(call, 10);
    const state = await probe(call, pos);
    if (predicate(state)) return state;
  }
  throw new Error('Timed out waiting for ' + description);
}

export async function runCustom(call) {
  const out = [];
  let index = 0;
  const origin = () => [(index++) * 16, 80, 0];
  const clear = async o => call(['fill', String(o[0] - 4), '78', '-4', String(o[0] + 4), '86', '4', 'minecraft:air']);
  {
    const o = origin(); await clear(o); await incubator(call, o); await breakAt(call, at(o, LOWER));
    await record(out, 'theurgy:incubator_break_lower_destroys_upper', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(o); await incubator(call, o); await breakAt(call, at(o, UPPER));
    await record(out, 'theurgy:incubator_break_upper_destroys_lower', call, o, [LOWER, UPPER]);
  }
  for (const [id, local, block, property] of [
    ['theurgy:incubator_mercury_vessel_connection', MERCURY, 'theurgy:incubator_mercury_vessel', 'north'],
    ['theurgy:incubator_salt_vessel_connection', SALT, 'theurgy:incubator_salt_vessel', 'east'],
    ['theurgy:incubator_sulfur_vessel_connection', SULFUR, 'theurgy:incubator_sulfur_vessel', 'south'],
  ]) {
    const o = origin(); await clear(o); await incubator(call, o);
    await setblock(call, at(o, local), block);
    await until(call, at(o, LOWER), state => state.properties?.[property] === 'true', 'vessel connection');
    await record(out, id, call, o, [LOWER, UPPER, local]);
  }
  {
    const o = origin(); await clear(o); await incubator(call, o);
    await setblock(call, at(o, MERCURY), 'theurgy:incubator_mercury_vessel');
    await wait(call, 5);
    await breakAt(call, at(o, MERCURY));
    await until(call, at(o, LOWER), state => state.properties?.north === 'false', 'vessel removal');
    await record(out, 'theurgy:incubator_removing_vessel_connection', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(o); await incubator(call, o, true);
    await record(out, 'theurgy:incubator_valid_multiblock', call, o, WITHOUT_HEAT);
  }
  {
    const o = origin(); await clear(o); await incubator(call, o, true);
    await container(call, at(o, MERCURY), 'theurgy:mercury_shard', 1);
    await record(out, 'theurgy:incubator_insert_mercury', call, o, WITHOUT_HEAT);
  }
  {
    const o = origin(); await clear(o); await incubator(call, o, true);
    await container(call, at(o, SALT), 'theurgy:alchemical_salt_creature', 1);
    await record(out, 'theurgy:incubator_insert_salt', call, o, WITHOUT_HEAT);
  }
  {
    const o = origin(); await clear(o); await incubator(call, o, true);
    await container(call, at(o, SULFUR), 'theurgy:alchemical_sulfur_bone', 1);
    await record(out, 'theurgy:incubator_insert_sulfur', call, o, WITHOUT_HEAT);
  }
  {
    const o = origin(); await clear(o); await incubator(call, o, true);
    await merge(call, at(o, MERCURY), '{Items:[{Slot:0b,id:"theurgy:mercury_shard",count:1}]}');
    await extract(call, at(o, MERCURY));
    await record(out, 'theurgy:incubator_extract_items', call, o, WITHOUT_HEAT);
  }
  {
    const o = origin(); await clear(o); await processing(call, o);
    await until(call, at(o, LOWER), state => Number(state.data?.[4]) === 1, 'incubator processing');
    await record(out, 'theurgy:incubator_processing_starts', call, o, ALL);
  }
  {
    const o = origin(); await clear(o); await processing(call, o);
    await until(call, at(o, LOWER), state => state.items.length > 0, 'incubation result');
    await record(out, 'theurgy:incubator_inputs_consumed_output_produced', call, o, ALL);
  }
  {
    const o = origin(); await clear(o); await incubator(call, o, true);
    await merge(call, at(o, LOWER), '{Items:[{Slot:0b,id:"minecraft:cobblestone",count:1}]}');
    await extract(call, at(o, LOWER));
    await record(out, 'theurgy:incubator_extract_output', call, o, WITHOUT_HEAT);
  }
  {
    const o = origin(); await clear(o); await incubator(call, o, true);
    await merge(call, at(o, LOWER), '{Items:[{Slot:0b,id:"minecraft:cobblestone",count:1}]}');
    await inputs(call, o);
    await wait(call, 3);
    await record(out, 'theurgy:incubator_blocked_output_does_not_consume_vessel_inputs', call, o, WITHOUT_HEAT);
  }
  {
    const o = origin(); await clear(o); await processing(call, o);
    await wait(call, 25);
    await setblock(call, at(o, HEAT), 'minecraft:air');
    await until(call, at(o, LOWER), state => Number(state.data?.[4]) === 0, 'incubator heat removal');
    await record(out, 'theurgy:incubator_stops_without_heat', call, o, ALL);
  }
  return out;
}

export default {name: 'differential-incubator', runCustom};
