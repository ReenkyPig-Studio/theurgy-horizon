// LogisticsGameTests + LogisticsCapabilityProxyGameTests network behaviours.
import {at, record, setblock, container, merge, wait, probe} from '../differential/harness.mjs';

const UUID = '12345678-1234-1234-1234-123456789abc';
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
async function placeNexus(call, o, locals) {
  // The Java fixtures use setBlock and then assign the same nexus id to both
  // BEs; they do not place a supporting stone below the nexus.
  for (const local of locals) {
    const pos = at(o, local);
    await setblock(call, pos, 'theurgy:logistics_nexus');
    await merge(call, pos, `{nexusId:"${UUID}"}`);
  }
  await wait(call, 2);
}
async function until(call, pos, predicate, description, timeout = 400) {
  for (let elapsed = 0; elapsed < timeout; elapsed += 10) {
    await wait(call, 10);
    if (predicate(await probe(call, pos))) return;
  }
  throw new Error('Timed out waiting for ' + description);
}
async function proxyCommand(call, command, pos, amount = null) {
  const reply = await call(['command', command, ...pos.map(String), ...(amount === null ? [] : [String(amount)])]);
  const text = reply.chat?.at(-1) ?? '';
  return JSON.parse(text.slice(text.indexOf('{')));
}
const fluid = (state, amount = 1) => state.nbt?.fluids?.some(tank => tank.Amount >= amount);
const energy = state => Number(state.nbt?.mercuryFluxHandler?.energy ?? 0);

export async function runCustom(call) {
  const out = [];
  let index = 0;
  const origin = () => [(index++) * 24, 80, 0];
  const clear = async o => call(['fill', String(o[0] - 7), '78', '-6', String(o[0] + 7), '86', '6', 'minecraft:air']);
  {
    const o = origin(); await clear(o);
    const tank = [2, 2, 2], probePos = [1, 2, 2], proxy = [3, 2, 2];
    await setblock(call, at(o, tank), 'theurgy:sal_ammoniac_tank');
    await setblock(call, at(o, probePos), 'theurgy:logistics_capability_probe[facing=west]');
    await setblock(call, at(o, proxy), 'theurgy:logistics_capability_proxy');
    await wire(call, at(o, proxy), at(o, probePos));
    await merge(call, at(o, tank), '{fluids:[{Fluid:"minecraft:water",Amount:1000}]}');
    const drained = await proxyCommand(call, 'theurgy_proxy_drain', at(o, proxy), 500);
    await record(out, 'theurgy:logistics_capability_proxy_fluid_forwarding', call, o, [tank, probePos, proxy]);
    out.at(-1).reply.proxyDrained = drained.drained;
  }
  {
    const o = origin(); await clear(o);
    const proxy = [1, 2, 2], probe1 = [2, 2, 1], probe2 = [2, 2, 3], tank1 = [3, 2, 1], tank2 = [3, 2, 3];
    for (const pos of [tank1, tank2]) await setblock(call, at(o, pos), 'theurgy:sal_ammoniac_tank');
    for (const pos of [probe1, probe2]) await setblock(call, at(o, pos), 'theurgy:logistics_capability_probe[facing=west]');
    await setblock(call, at(o, proxy), 'theurgy:logistics_capability_proxy');
    await wire(call, at(o, proxy), at(o, probe1));
    await wire(call, at(o, proxy), at(o, probe2));
    await merge(call, at(o, tank1), '{fluids:[{Fluid:"minecraft:water",Amount:1000}]}');
    await merge(call, at(o, tank2), '{fluids:[{Fluid:"minecraft:water",Amount:2000}]}');
    const first = await proxyCommand(call, 'theurgy_proxy_read', at(o, proxy));
    const second = await proxyCommand(call, 'theurgy_proxy_read', at(o, proxy));
    await record(out, 'theurgy:logistics_capability_proxy_fluid_round_robin', call, o, [proxy, probe1, probe2, tank1, tank2]);
    out.at(-1).reply.proxyReads = [first.fluid, second.fluid];
  }
  for (const fluidKind of [false, true]) {
    const o = origin(); await clear(o);
    const pos = fluidKind ? [4, 2, 2] : [3, 2, 2];
    const block = fluidKind ? 'theurgy:logistics_fluid_inserter[facing=west]' : 'theurgy:logistics_item_inserter[facing=east]';
    await setblock(call, at(o, pos), block);
    await merge(call, at(o, pos), '{targets:[]}');
    await wait(call, 2);
    await record(out, fluidKind ? 'theurgy:logistics_fluid_connector_restores_attached_target_on_load' : 'theurgy:logistics_item_connector_restores_attached_target_on_load', call, o, [pos]);
  }
  for (const reversed of [false, true]) {
    const o = origin(); await clear(o);
    const target = [2, 2, 2], inserter = [3, 2, 2], extractor = [5, 2, 2], source = [6, 2, 2];
    const placements = reversed ? [[extractor, 'theurgy:logistics_item_extractor[facing=west]'], [inserter, 'theurgy:logistics_item_inserter[facing=east]']]
      : [[inserter, 'theurgy:logistics_item_inserter[facing=east]'], [extractor, 'theurgy:logistics_item_extractor[facing=west]']];
    await setblock(call, at(o, target), 'minecraft:chest');
    for (const [pos, block] of placements) await setblock(call, at(o, pos), block);
    await setblock(call, at(o, source), 'minecraft:chest');
    await merge(call, at(o, inserter), '{directionOverride:5,targets:[]}');
    await merge(call, at(o, extractor), '{directionOverride:4,targets:[]}');
    await wire(call, at(o, inserter), at(o, extractor));
    await record(out, reversed ? 'theurgy:logistics_item_extractor_targets_in_any_registration_order_reversed' : 'theurgy:logistics_item_extractor_targets_in_any_registration_order', call, o, [target, inserter, extractor, source]);
  }
  {
    const o = origin(); await clear(o);
    const target = [2, 2, 2], inserter = [3, 2, 2], extractor = [5, 2, 2], source = [6, 2, 2];
    await setblock(call, at(o, target), 'minecraft:chest');
    await setblock(call, at(o, inserter), 'theurgy:logistics_item_inserter[facing=east]');
    await setblock(call, at(o, extractor), 'theurgy:logistics_item_extractor[facing=west]');
    await setblock(call, at(o, source), 'minecraft:chest');
    await container(call, at(o, source), 'theurgy:purified_gold', 1);
    await merge(call, at(o, inserter), '{directionOverride:5,targets:[]}');
    await merge(call, at(o, extractor), '{directionOverride:4,targets:[]}');
    await wire(call, at(o, inserter), at(o, extractor));
    await until(call, at(o, target), state => state.items.some(stack => stack.id === 'theurgy:purified_gold'), 'item logistics transfer');
    await record(out, 'theurgy:logistics_item_extractor_transfers_items', call, o, [target, inserter, extractor, source]);
  }
  {
    const o = origin(); await clear(o);
    await placeNexus(call, o, [[2, 2, 2]]);
    await record(out, 'theurgy:logistics_nexus_be_accepts_item_uuid', call, o);
  }
  {
    const o = origin(); await clear(o);
    await placeNexus(call, o, [[2, 2, 2], [6, 2, 2]]);
    await record(out, 'theurgy:logistics_nexus_pairs_across_placement', call, o, [[2, 2, 2], [6, 2, 2]]);
  }
  {
    const o = origin(); await clear(o);
    await setblock(call, at(o), 'theurgy:logistics_nexus');
    await setblock(call, at(o, [2, 2, 1]), 'theurgy:logistics_connector_node[facing=north]');
    await setblock(call, at(o, [1, 2, 2]), 'theurgy:logistics_connector_node[facing=up]');
    await wait(call, 4);
    await record(out, 'theurgy:logistics_node_only_connects_when_attached_to_nexus', call, o, [[2, 2, 2], [2, 2, 1], [1, 2, 2]]);
  }
  {
    const o = origin(); await clear(o);
    const source = [1, 2, 2], extractor = [2, 2, 2], inserter = [4, 2, 2], tank = [5, 2, 2];
    const stones = [[0, 2, 2], [1, 2, 1], [1, 2, 3], [1, 3, 2], [1, 1, 2]];
    for (const pos of stones) await setblock(call, at(o, pos), 'minecraft:stone');
    await setblock(call, at(o, source), 'minecraft:water');
    await setblock(call, at(o, extractor), 'theurgy:logistics_fluid_extractor[facing=east]');
    await setblock(call, at(o, inserter), 'theurgy:logistics_fluid_inserter[facing=west]');
    await setblock(call, at(o, tank), 'theurgy:sal_ammoniac_tank');
    const packed = pos => BigInt.asIntN(64, (BigInt(pos[0]) << 38n) | (BigInt(pos[2]) << 12n) | BigInt(pos[1] & 0xfff));
    const tankPos = at(o, tank), packedTank = packed(tankPos);
    await merge(call, at(o, inserter), `{targets:[${packedTank}L,${packedTank}L]}`);
    await wire(call, at(o, extractor), at(o, inserter));
    await until(call, at(o, tank), state => fluid(state), 'world fluid extraction');
    await record(out, 'theurgy:logistics_fluid_extracts_world_source', call, o, [...stones, source, extractor, inserter, tank]);
  }
  return out;
}

export default {name: 'differential-logistics', runCustom};
