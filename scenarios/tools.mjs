// Real headless wire/endpoint behavior: store a WireEndPoint, connect two
// logistics nodes, consume exactly one wire, and never write to non-endpoints.
const WIRE = 'theurgy:copper_wire';
const NODE = 'theurgy:logistics_connector_node';

function held(inv) {
  const stack = inv?.held;
  return stack ? {item: stack.item, components: String(stack.components ?? '')} : null;
}
function count(inv, item) {
  return Number(inv?.counts?.[item] ?? 0);
}
function hasWirePoint(inv) {
  return String(held(inv)?.components ?? '').includes('wire_end_point');
}

export async function runCustom(call, assert) {
  const results = [];
  const step = async (name, args, check) => {
    const reply = await call(args);
    if (check) check(reply);
    results.push({name, reply});
    if (['give', 'setblock', 'tp', 'command'].includes(args[0])) await call(['wait', '2']);
    return reply;
  };

  await step('tools lab', ['command', 'tp', '@s', '30', '80', '26']);
  // Fresh worlds may have terrain above y=79 between the player and the nodes.
  await step('tools clear air', ['fill', '28', '80', '22', '32', '85', '36', 'minecraft:air']);
  await step('tools safe floor', ['fill', '28', '79', '22', '32', '79', '36', 'minecraft:stone']);
  await step('tools chunks', ['wait', '40']);
  await step('clear test inventory', ['command', 'clear', '@s']);
  await step('wire conservation in survival', ['command', 'gamemode', 'survival', '@s']);
  await step('node a', ['setblock', '30', '80', '28', NODE]);
  await step('node b', ['setblock', '30', '80', '34', NODE]);

  await step('give wire', ['give', WIRE, '1']);
  assert.equal(count(await step('wire inventory', ['inventory']), WIRE), 1);
  await step('look node a', ['look-at', 'block', '30', '80', '28']);
  await step('store endpoint', ['interact', '30', '80', '28']);
  const stored = await step('endpoint inventory', ['inventory']);
  assert.ok(hasWirePoint(stored), 'first endpoint click must store wire_end_point: ' + JSON.stringify(held(stored)));
  assert.ok(String(held(stored).components).includes('minecraft:overworld'), 'wire endpoint must carry its dimension');
  const packed = String(held(stored).components).match(/pos:(-?\d+)L?/);
  const expected = ((30n & 0x3ffffffn) << 38n) | ((28n & 0x3ffffffn) << 12n) | (80n & 0xfffn);
  assert.ok(packed && BigInt(packed[1]) === expected,
    'wire_end_point.pos must use Java BlockPos.asLong packing, got ' + (packed?.[1] ?? 'missing'));

  await step('approach node b', ['command', 'tp', '@s', '30', '80', '32']);
  await step('look node b', ['look-at', 'block', '30', '80', '34']);
  await step('connect endpoint', ['interact', '30', '80', '34']);
  const connected = await step('connected inventory', ['inventory']);
  assert.equal(count(connected, WIRE), 0, 'connecting two nodes must consume exactly one wire');
  assert.ok(!hasWirePoint(connected), 'connect must clear wire_end_point');

  await step('give wire for self', ['give', WIRE, '1']);
  await step('approach node a', ['command', 'tp', '@s', '30', '80', '26']);
  await step('look node a again', ['look-at', 'block', '30', '80', '28']);
  await step('store endpoint again', ['interact', '30', '80', '28']);
  assert.ok(hasWirePoint(await step('self endpoint inventory', ['inventory'])), 'second wire must store an endpoint');
  await step('same node', ['interact', '30', '80', '28']);
  const self = await step('self inventory', ['inventory']);
  assert.equal(count(self, WIRE), 1, 'self-connection must not consume the wire');
  assert.ok(hasWirePoint(self), 'self-connection must keep the endpoint');

  await step('sneak down', ['key', 'shift', 'down']);
  await step('clear endpoint', ['interact', '30', '80', '28']);
  await step('sneak up', ['key', 'shift', 'up']);
  assert.ok(!hasWirePoint(await step('cleared inventory', ['inventory'])), 'sneak-use on an endpoint must clear the endpoint');

  await step('stone', ['setblock', '30', '80', '24', 'minecraft:stone']);
  await step('approach stone', ['command', 'tp', '@s', '30', '80', '26']);
  const beforeStone = count(await step('before stone inventory', ['inventory']), WIRE);
  await step('look stone', ['look-at', 'block', '30', '80', '24']);
  await step('wire on stone', ['interact', '30', '80', '24']);
  const stone = await step('stone inventory', ['inventory']);
  assert.equal(count(stone, WIRE), beforeStone, 'wire must not be consumed on a non-endpoint');
  assert.ok(!hasWirePoint(stone), 'wire must not store an endpoint on a non-endpoint');
  return results;
}

export default {name: 'tools', runCustom};
