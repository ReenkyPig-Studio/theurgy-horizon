// Real headless logistics slice: one extractor, one inserter and a shared
// nexus id move actual items/fluids between real containers. No wire rendering
// or GUI is needed; the network id and facing states are the same NBT/state the
// Java block entities persist.
const s = p => p.map(String);
const ITEM_SOURCE = 'theurgy:logistics_item_extractor';
const ITEM_SINK = 'theurgy:logistics_item_inserter';
const FLUID_SOURCE = 'theurgy:logistics_fluid_extractor';
const FLUID_SINK = 'theurgy:logistics_fluid_inserter';
const NEXUS = 'theurgy:logistics_nexus';
const SRC_CHEST = [12, 80, 10], ITEM_EX = [11, 80, 10];
const ITEM_IN = [14, 80, 10], DST_CHEST = [13, 80, 10], ITEM_NEXUS = [12, 80, 12];
const SRC_TANK = [12, 80, 20], FLUID_EX = [11, 80, 20];
const FLUID_IN = [14, 80, 20], DST_TANK = [13, 80, 20], FLUID_NEXUS = [12, 80, 22];

function nbtText(reply) {
  if (Array.isArray(reply?.chat)) return reply.chat.join('\n');
  return typeof reply === 'string' ? reply : JSON.stringify(reply ?? '');
}
function slots(reply) { return Array.isArray(reply?.slots) ? reply.slots : null; }
function itemAt(reply, i) {
  const v = slots(reply)?.[i]; if (!v) return null;
  const inner = v.item && typeof v.item === 'object' ? v.item : v;
  return inner.id ?? inner.item ?? null;
}
function countAt(reply, i) {
  const v = slots(reply)?.[i]; if (!v) return 0;
  const inner = v.item && typeof v.item === 'object' ? v.item : v;
  return Number(inner.count ?? v.count ?? 0);
}
function total(reply) { return slots(reply)?.reduce((n, _, i) => n + countAt(reply, i), 0) ?? 0; }
function fluidAmount(reply) {
  const m = /Amount\s*:\s*(-?\d+)/.exec(nbtText(reply));
  return m ? Number(m[1]) : 0;
}
function fluidId(reply) {
  const m = /Fluid\s*:\s*"([^"]+)"/.exec(nbtText(reply));
  return m ? m[1] : null;
}

export async function runCustom(call, assert) {
  const results = [];
  const step = async (name, args, check) => {
    const reply = await call(args);
    if (check) check(reply);
    results.push({name, args, reply});
    if (['setblock', 'container', 'command'].includes(args[0])) await call(['wait', '2']);
    return reply;
  };
  const place = (name, p, id, state = '') => step(name, ['setblock', ...s(p), id + state],
    r => assert.ok(String(r?.state ?? r?.block ?? '').startsWith(id), name + ': ' + JSON.stringify(r)));
  const link = async (p, nexus) => step('link ' + p.join(','),
    ['command', 'data', 'merge', 'block', ...s(p), `{nexusId:"${nexus}"}`]);

  await step('logistics lab', ['tp', '12', '80', '16']);
  await step('logistics chunks', ['wait', '40']);

  // --- item network -------------------------------------------------------
  await place('source chest', SRC_CHEST, 'minecraft:chest');
  await place('item extractor', ITEM_EX, ITEM_SOURCE, '[facing=west]');
  await place('item inserter', ITEM_IN, ITEM_SINK, '[facing=east]');
  await place('target chest', DST_CHEST, 'minecraft:chest');
  await place('item nexus', ITEM_NEXUS, NEXUS);
  for (const p of [ITEM_EX, ITEM_IN, ITEM_NEXUS]) await link(p, 'item-net');
  await step('seed source chest', ['container', ...s(SRC_CHEST), 'minecraft:bread', '3'],
    r => assert.equal(itemAt(r, 0), 'minecraft:bread'));
  await step('item transfer wait', ['wait', '60']);
  const src = await step('source after items', ['container', ...s(SRC_CHEST)]);
  const dst = await step('target after items', ['container', ...s(DST_CHEST)]);
  assert.equal(total(src), 0, 'extractor must remove the moved items from its source');
  assert.equal(itemAt(dst, 0), 'minecraft:bread', 'inserter must place bread in the target');
  assert.equal(countAt(dst, 0), 3, 'all three bread must move, never duplicate');
  results.push({name: 'item conservation', src: total(src), dst: total(dst)});

  await step('disable extractor', ['command', 'data', 'merge', 'block', ...s(ITEM_EX), '{enabled:0}']);
  await step('seed again', ['container', ...s(SRC_CHEST), 'minecraft:apple', '1'],
    r => assert.equal(itemAt(r, 0), 'minecraft:apple'));
  await step('disabled wait', ['wait', '60']);
  const disabledSrc = await step('disabled source', ['container', ...s(SRC_CHEST)]);
  assert.equal(itemAt(disabledSrc, 0), 'minecraft:apple', 'disabled extractor must not move items');

  // --- fluid network ------------------------------------------------------
  await place('fluid source tank', SRC_TANK, 'theurgy:sal_ammoniac_tank');
  await place('fluid extractor', FLUID_EX, FLUID_SOURCE, '[facing=west]');
  await place('fluid inserter', FLUID_IN, FLUID_SINK, '[facing=east]');
  await place('fluid target tank', DST_TANK, 'theurgy:sal_ammoniac_tank');
  await place('fluid nexus', FLUID_NEXUS, NEXUS);
  for (const p of [FLUID_EX, FLUID_IN, FLUID_NEXUS]) await link(p, 'fluid-net');
  await step('seed source tank', ['command', 'data', 'merge', 'block', ...s(SRC_TANK),
    '{fluids:[{Fluid:"minecraft:water",Amount:1000}]}']);
  await step('fluid transfer wait', ['wait', '60']);
  const srcTank = await step('source tank after', ['command', 'data', 'get', 'block', ...s(SRC_TANK)]);
  const dstTank = await step('target tank after', ['command', 'data', 'get', 'block', ...s(DST_TANK)]);
  assert.equal(fluidId(dstTank), 'minecraft:water', 'fluid inserter must fill the target tank with water');
  assert.equal(fluidAmount(dstTank), 1000, 'the full bucket must move, never duplicate');
  assert.equal(fluidAmount(srcTank), 0, 'fluid extractor must drain its source');
  results.push({name: 'fluid conservation', src: fluidAmount(srcTank), dst: fluidAmount(dstTank)});

  return results;
}

export default {name: 'logistics', runCustom};
