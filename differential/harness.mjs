// Shared real Horizon probes for the pinned Java↔Rust differential matrix.
// Every record is bound to one upstream GameTest id; no synthetic SKIP state.

export function at(origin, local = [2, 2, 2]) {
  return origin.map((value, index) => value + local[index] - 2);
}

export async function probe(call, pos) {
  const reply = await call(['command', 'theurgy_probe', ...pos.map(String)]);
  const lines = reply?.chat ?? [];
  const text = lines.at(-1) ?? '';
  const start = text.indexOf('{');
  if (start < 0) throw new Error('theurgy_probe returned no JSON: ' + JSON.stringify(reply));
  return JSON.parse(text.slice(start));
}

export async function record(results, javaId, call, origin, locals = [[2, 2, 2]], captured = null) {
  const probes = [];
  for (const local of locals) {
    const pos = at(origin, local);
    probes.push({local, state: captured?.get?.(JSON.stringify(local)) ?? await probe(call, pos)});
  }
  results.push({name: `diff:${javaId}`, args: [], reply: {javaId, origin, probes}});
}

export function recordStack(results, javaId, outputStack) {
  results.push({name: `diff:${javaId}`, args: [], reply: {javaId, probes: [], outputStack}});
}

export async function wait(call, ticks) {
  return call(['wait', String(ticks)]);
}

export async function setblock(call, pos, id) {
  // Origin-spaced fixtures must never place into an unloaded chunk. Every
  // placement is preceded by a real teleport/chunk-load, not an air reply.
  await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 2)]);
  await wait(call, 2);
  const state = /^([^[]+)\[(.+)\]$/.exec(id);
  const reply = await call(['setblock', ...pos.map(String), state?.[1] ?? id]);
  if (state) {
    for (const pair of state[2].split(',')) {
      const [key, value] = pair.split('=');
      await call(['command', 'theurgy_state', ...pos.map(String), key, value]);
    }
  }
  await wait(call, 2);
  return reply;
}

export async function container(call, pos, item, count = 1) {
  let reply;
  try {
    reply = await call(['container', ...pos.map(String), item, String(count)]);
  } catch (error) {
    if (!String(error?.message ?? error).includes('container is full')) throw error;
    await wait(call, 2);
    const current = await call(['container', ...pos.map(String)]);
    const present = current?.slots?.some(stack => {
      const value = stack?.item && typeof stack.item === 'object' ? stack.item : stack;
      const id = value?.id ?? value?.item;
      return id === item && Number(value?.count ?? stack?.count ?? 0) >= count;
    });
    if (present) return current;
    reply = await call(['container', ...pos.map(String), item, String(count)]);
  }
  await wait(call, 2);
  return reply;
}

export async function merge(call, pos, snbt) {
  return call(['command', 'data', 'merge', 'block', ...pos.map(String), snbt]);
}
