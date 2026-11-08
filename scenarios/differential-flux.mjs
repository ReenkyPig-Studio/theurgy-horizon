// Mercury catalyst/capacitor/connector/emitter differential behaviours.
import {at, record, setblock, container, merge, wait, probe} from '../differential/harness.mjs';

async function until(call, pos, predicate, description, timeout = 400) {
  for (let elapsed = 0; elapsed < timeout; elapsed += 10) {
    await wait(call, 10);
    if (predicate(await probe(call, pos))) return;
  }
  throw new Error('Timed out waiting for ' + description);
}
const energy = (state, key = 'mercuryFluxHandler') => Number(state.nbt?.[key]?.energy ?? 0);
async function interact(call, pos, item = null) {
  await call(['command', 'clear', '@s']);
  if (item) await call(['give', item, '1']);
  await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
  await call(['look-at', 'block', ...pos.map(String)]);
  await call(['interact', ...pos.map(String)]);
  await wait(call, 2);
}
// Mirrors GameTestHelper.useBlock's explicit UP hit through the authoritative
// native interaction probe, not through client-side inventory/NBT edits.
async function interactTop(call, pos, item = null) {
  await call(['command', 'clear', '@s']);
  if (item) await call(['give', item, '1']);
  await call(['command', 'theurgy_interact', ...pos.map(String), 'up']);
  await wait(call, 2);
}
async function wire(call, from, to) {
  await call(['command', 'clear', '@s']);
  await call(['give', 'theurgy:copper_wire', '1']);
  for (const pos of [from, to]) {
    await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
    await call(['look-at', 'block', ...pos.map(String)]);
    await call(['interact', ...pos.map(String)]);
    await wait(call, 2);
  }
}

export async function runCustom(call) {
  const out = [];
  let index = 0;
  const origin = () => [(index++) * 24, 80, 0];
  const clear = async o => call(['fill', String(o[0] - 7), '78', '-6', String(o[0] + 7), '86', '6', 'minecraft:air']);
  {
    const o = origin(); await clear(o);
    await setblock(call, at(o), 'theurgy:caloric_flux_emitter');
    await setblock(call, at(o, [2, 1, 2]), 'minecraft:redstone_block');
    await until(call, at(o), state => state.properties?.enabled === 'false', 'redstone disable');
    await record(out, 'theurgy:caloric_flux_emitter_redstone', call, o, [[2, 2, 2], [2, 1, 2]]);
  }
  {
    const o = origin(); await clear(o);
    await setblock(call, at(o, [2, 1, 2]), 'theurgy:mercury_capacitor');
    await setblock(call, at(o, [3, 1, 2]), 'theurgy:mercury_catalyst');
    await container(call, at(o, [3, 1, 2]), 'theurgy:mercury_shard', 1);
    await until(call, at(o, [2, 1, 2]), state => energy(state) > 0, 'capacitor receives flux');
    await record(out, 'theurgy:mercury_capacitor_receives_flux', call, o, [[2, 1, 2], [3, 1, 2]]);
  }
  {
    const o = origin(); await clear(o);
    await setblock(call, at(o, [2, 1, 2]), 'theurgy:mercury_capacitor[enabled=false]');
    await setblock(call, at(o, [3, 1, 2]), 'theurgy:mercury_catalyst');
    await record(out, 'theurgy:mercury_capacitor_disabled_does_not_receive_flux', call, o, [[2, 1, 2], [3, 1, 2]]);
  }
  {
    const o = origin(); await clear(o);
    await setblock(call, at(o, [2, 1, 2]), 'theurgy:mercury_capacitor');
    await setblock(call, at(o, [3, 1, 2]), 'theurgy:mercury_capacitor');
    await record(out, 'theurgy:mercury_capacitor_no_ping_pong', call, o, [[2, 1, 2], [3, 1, 2]]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, [2, 1, 2]), 'theurgy:mercury_catalyst');
    await container(call, at(o, [2, 1, 2]), 'theurgy:mercury_shard', 1);
    await until(call, at(o, [2, 1, 2]), state => energy(state) > 0, 'catalyst flux');
    await record(out, 'theurgy:mercury_catalyst_shard_generates_flux', call, o, [[2, 1, 2]]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, [2, 1, 2]), 'theurgy:mercury_catalyst');
    await container(call, at(o, [2, 1, 2]), 'theurgy:mercury_shard', 1);
    await until(call, at(o, [2, 1, 2]), state => state.items.length === 0, 'catalyst shard consumption');
    await record(out, 'theurgy:mercury_catalyst_shard_is_consumed', call, o, [[2, 1, 2]]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, [2, 1, 2]), 'theurgy:mercury_catalyst');
    await interactTop(call, at(o, [2, 1, 2]), 'minecraft:dirt');
    await interactTop(call, at(o, [2, 1, 2]));
    await record(out, 'theurgy:mercury_catalyst_rejected_placeable_dont_duplicate', call, o, [[2, 1, 2], [2, 2, 2]]);
  }
  {
    const o = origin(); await clear(o); await setblock(call, at(o, [2, 1, 2]), 'theurgy:mercury_catalyst[enabled=false]');
    await container(call, at(o, [2, 1, 2]), 'theurgy:mercury_shard', 1);
    await wait(call, 60);
    await record(out, 'theurgy:mercury_catalyst_disabled_still_generates_flux', call, o, [[2, 1, 2]]);
  }
  for (const multiple of [false, true]) {
    const o = origin(); await clear(o);
    const catalyst = [1, 2, 2], a = [2, 2, 2], b = [4, 2, 2], cap1 = [5, 2, 2], c = [4, 2, 4], cap2 = [5, 2, 4];
    await setblock(call, at(o, catalyst), 'theurgy:mercury_catalyst');
    await setblock(call, at(o, a), 'theurgy:logistics_mercury_flux_connector[facing=east]');
    await setblock(call, at(o, b), 'theurgy:logistics_mercury_flux_connector[facing=west]');
    await setblock(call, at(o, cap1), 'theurgy:mercury_capacitor');
    if (multiple) {
      await setblock(call, at(o, c), 'theurgy:logistics_mercury_flux_connector[facing=west]');
      await setblock(call, at(o, cap2), 'theurgy:mercury_capacitor');
    }
    await container(call, at(o, catalyst), 'theurgy:mercury_shard', 1);
    await wire(call, at(o, a), at(o, b));
    if (multiple) await wire(call, at(o, a), at(o, c));
    await until(call, at(o, cap1), state => energy(state) > 0, 'connector flux forwarding');
    if (multiple) await until(call, at(o, cap2), state => energy(state) > 0, 'connector second sink');
    const locals = multiple ? [catalyst, a, b, cap1, c, cap2] : [catalyst, a, b, cap1];
    await record(out, multiple ? 'theurgy:mercury_flux_connector_forwards_to_multiple' : 'theurgy:mercury_flux_connector_forwards_flux', call, o, locals);
  }
  for (const transfer of [false, true]) {
    const o = origin(); await clear(o);
    const catalyst = [2, 1, 2], emitter = [2, 2, 2], capacitor = [5, 1, 2];
    await setblock(call, at(o, catalyst), 'theurgy:mercury_catalyst');
    await setblock(call, at(o, emitter), 'theurgy:mercury_flux_emitter[facing=up]');
    await setblock(call, at(o, capacitor), 'theurgy:mercury_capacitor');
    const target = at(o, capacitor);
    await merge(call, at(o, emitter), `{selectedPoints:[{blockPos:[I;${target.join(',')}]}]}`);
    if (transfer) {
      await container(call, at(o, catalyst), 'theurgy:mercury_shard', 1);
      await until(call, at(o, capacitor), state => energy(state) > 0, 'emitter flux transfer');
    }
    await record(out, transfer ? 'theurgy:mercury_flux_emitter_flux_transfer' : 'theurgy:mercury_flux_emitter_linking', call, o, [catalyst, emitter, capacitor]);
  }
  return out;
}

export default {name: 'differential-flux', runCustom};
