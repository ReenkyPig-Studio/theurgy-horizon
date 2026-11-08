// All 14 LiquefactionCauldronGameTests behaviours.
import {at, record, setblock, container, merge, wait, probe} from '../differential/harness.mjs';

const BLOCK = 'theurgy:liquefaction_cauldron', HEAT_BLOCK = 'theurgy:pyromantic_brazier';
const LOWER = [2, 2, 2], UPPER = [2, 3, 2], HEAT = [2, 1, 2];
const OUTPUT = 'theurgy:alchemical_sulfur_bone';

async function machine(call, o, heat = false) {
  if (heat) await setblock(call, at(o, HEAT), HEAT_BLOCK);
  await setblock(call, at(o, LOWER), BLOCK);
  await setblock(call, at(o, UPPER), BLOCK + '[half=upper]');
  if (heat) await container(call, at(o, HEAT), 'minecraft:coal', 64);
}
async function processing(call, o) {
  await machine(call, o, true);
  await container(call, at(o, LOWER), 'minecraft:bone', 1);
  await merge(call, at(o, LOWER), '{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:1000}]}');
}
async function breakAt(call, pos) {
  await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
  await call(['look-at', 'block', ...pos.map(String)]);
  const broken = await call(['break', ...pos.map(String)]);
  await wait(call, 4);
  return broken;
}
async function interact(call, pos, item = null) {
  await call(['command', 'clear', '@s']);
  if (item) await call(['give', item, '1']);
  await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
  await call(['look-at', 'block', ...pos.map(String)]);
  await call(['interact', ...pos.map(String)]);
  await wait(call, 2);
}
async function until(call, pos, predicate, description, timeout = 400) {
  for (let elapsed = 0; elapsed < timeout; elapsed += 1) {
    await wait(call, 1); // GameTestHelper.succeedWhen polls every tick
    if (predicate(await probe(call, pos))) return;
  }
  throw new Error('Timed out waiting for ' + description);
}
const has = (state, id) => state.items.some(stack => stack.id === id);
const fluid = (state, id, amount) => state.nbt?.fluids?.some(tank => tank.Fluid === id && tank.Amount === amount);

export async function runCustom(call) {
  const out = [];
  let index = 0;
  const origin = () => [(index++) * 16, 80, 0];
  const clear = async o => call(['fill', String(o[0] - 4), '78', '-4', String(o[0] + 4), '86', '4', 'minecraft:air']);
  {
    const o = origin(); await clear(o); await machine(call, o);
    await record(out, 'theurgy:lc_placement_creates_two_block_structure', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(o); await machine(call, o);
    await record(out, 'theurgy:lc_block_entity_only_on_lower_half', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(o); await machine(call, o); await breakAt(call, at(o, LOWER));
    await record(out, 'theurgy:lc_breaking_lower_destroys_upper', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(o); await machine(call, o); await breakAt(call, at(o, UPPER));
    await record(out, 'theurgy:lc_breaking_upper_destroys_lower', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(o); await machine(call, o);
    await merge(call, at(o, LOWER), '{Items:[{Slot:0b,id:"minecraft:bone",count:3}]}');
    const dropped = await breakAt(call, at(o, LOWER));
    await record(out, 'theurgy:lc_drops_items_when_broken', call, o, [LOWER, UPPER]);
    out.at(-1).reply.drops = dropped.drops;
    out.at(-1).reply.entities = await call(['entities']);
  }
  {
    const o = origin(); await clear(o); await machine(call, o);
    await container(call, at(o, LOWER), 'minecraft:bone', 1);
    await record(out, 'theurgy:lc_insert_input_item', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(o); await machine(call, o);
    await merge(call, at(o, LOWER), '{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:500}]}');
    await record(out, 'theurgy:lc_insert_solvent_fluid', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(o); await machine(call, o);
    await merge(call, at(o, LOWER), '{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:1000}]}');
    await interact(call, at(o, LOWER), 'minecraft:bucket');
    await record(out, 'theurgy:lc_extract_solvent_fluid', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(o); await machine(call, o);
    await merge(call, at(o, LOWER), `{Items:[{Slot:1b,id:"${OUTPUT}",count:1}]}`);
    await interact(call, at(o, LOWER));
    await record(out, 'theurgy:lc_extract_output_item', call, o, [LOWER, UPPER]);
  }
  {
    const o = origin(); await clear(o); await processing(call, o);
    await until(call, at(o, LOWER), state => Number(state.data?.[4]) === 1, 'cauldron processing');
    await record(out, 'theurgy:lc_processing_starts', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(o); await processing(call, o);
    await until(call, at(o, LOWER), state => state.properties?.lit === 'true', 'cauldron LIT');
    await record(out, 'theurgy:lc_lit_state_true_during_processing', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(o); await processing(call, o);
    await until(call, at(o, LOWER), state => has(state, OUTPUT) && fluid(state, 'theurgy:sal_ammoniac', 990), 'cauldron output');
    await record(out, 'theurgy:lc_input_consumed_output_produced', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(o); await processing(call, o);
    await wait(call, 25);
    await breakAt(call, at(o, HEAT));
    await until(call, at(o, LOWER), state => Number(state.data?.[4]) === 0, 'cauldron heat removal');
    await record(out, 'theurgy:lc_processing_stops_when_heat_removed', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(o); await machine(call, o, true);
    await merge(call, at(o, LOWER), '{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:1000}]}');
    await wait(call, 50);
    await record(out, 'theurgy:lc_processing_requires_input', call, o, [LOWER, UPPER, HEAT]);
  }
  {
    const o = origin(); await clear(o); await machine(call, o, true);
    await container(call, at(o, LOWER), 'minecraft:bone', 1);
    await wait(call, 50);
    await record(out, 'theurgy:lc_processing_requires_solvent', call, o, [LOWER, UPPER, HEAT]);
  }
  return out;
}

export default {name: 'differential-liquefaction', runCustom};
