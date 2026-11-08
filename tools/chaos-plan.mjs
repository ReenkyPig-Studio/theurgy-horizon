// Trusted, bounded actions for the isolated vanilla Horizon lab only.
// These exercise game/control stability, NOT Theurgy machine parity.
export function chaosPlan(seed=0x26_03,steps=32) {
  if(!Number.isInteger(seed)||!Number.isInteger(steps)||steps<1||steps>64)
    throw new Error('Invalid chaos seed/step budget');
  let state=seed>>>0;
  const random=n=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state%n;};
  const blocks=['minecraft:stone','minecraft:glass','minecraft:chest','minecraft:hopper','minecraft:air'];
  const actions=[];
  for(let i=0;i<steps;i++) {
    const pos=[String(random(4)),'80',String(random(4))];
    const block=blocks[random(blocks.length)];
    actions.push({args:['setblock',...pos,block]});
    // Repeat writes, destroy and immediately replace; assert actual readback after every mutation.
    if(i%4===0)actions.push({args:['setblock',...pos,block]});
    actions.push({args:['block',...pos],expected:block});
    if(i%8===7)actions.push({args:['save']});
  }
  return {seed:seed>>>0,steps,actions};
}

// ---------------------------------------------------------------------------
// Machine chaos: adversarial, bounded plans against a Theurgy machine only.
// Unlike chaosPlan this is NOT run against vanilla blocks; it targets the
// `theurgy:distiller` placed by scenarios/machines.mjs and probes duplication,
// overflow, rollback and reload-mid-recipe. It never guesses results: every
// entry states only what MUST hold (no items created, no items lost).
// ---------------------------------------------------------------------------
export const MACHINE_POSITIONS = Object.freeze({
  main: [0, 80, 4],
  side: [2, 80, 4],
  reload: [4, 80, 4],
  destroy: [6, 80, 4],
});

const MACHINE = 'theurgy:distiller';
const VANILLA_INPUT = 'minecraft:stone';

function itemsPerCall(reply) {
  const s = reply && Array.isArray(reply.slots) ? reply.slots : null;
  return s ? s.filter(Boolean).reduce((n, x) => n + Number(x.count || 0), 0) : null;
}

export function machineChaosPlan(opts = {}) {
  const seed = opts.seed ?? 0x26_04;
  const steps = opts.steps ?? 24;
  if (!Number.isInteger(seed) || !Number.isInteger(steps) || steps < 1 || steps > 64)
    throw new Error('Invalid machine chaos seed/step budget');
  let state = seed >>> 0;
  const random = n => { state = (Math.imul(state, 1664525) + 1013904223) >>> 0; return state % n; };
  const actions = [];
  // Machine-chaos invariants: never create a stack out of nothing, never drop below zero.
  const sanity = {kind: 'invariant', name: 'no negative or created item counts'};
  const positions = Object.values(MACHINE_POSITIONS);
  // Every probed position must actually hold a distiller first, otherwise `container` replies
  // `no container there` and the probe would fail for the wrong reason. Place them all, then
  // assert each readback so a failed placement halts immediately.
  for (const p of positions) {
    // Reused isolated homes must start with both halves empty. Otherwise a
    // previous run's upper half can make the new lower placement pop back to
    // air before the first real chaos action.
    actions.push({name: 'clear upper fixture ' + p.join(','), kind: 'setblock',
      args: ['setblock', String(p[0]), String(p[1] + 1), String(p[2]), 'minecraft:air']});
    actions.push({name: 'clear lower fixture ' + p.join(','), kind: 'setblock',
      args: ['setblock', ...p.map(String), 'minecraft:air']});
    actions.push({name: 'place machine ' + p.join(','), kind: 'setblock',
      args: ['setblock', ...p.map(String), MACHINE], expect: MACHINE});
    actions.push({name: 'place upper fixture ' + p.join(','), kind: 'setblock',
      args: ['setblock', String(p[0]), String(p[1] + 1), String(p[2]), MACHINE + '[half=upper]'], expect: MACHINE});
    actions.push({name: 'confirm machine ' + p.join(','), kind: 'block',
      args: ['block', ...p.map(String)], expect: MACHINE});
  }
  const touched = new Set();
  for (let i = 0; i < steps; i++) {
    const p = positions[random(positions.length)];
    const pos = p.map(String);
    const key = pos.join(',');
    if (i % 3 === 0) {
      // Only a never-inserted-into machine is trustworthy as empty; otherwise prefer a
      // readback that cannot false-fail because a valid insert legitimately populated it.
      if (!touched.has(key)) {
        actions.push({name: `empty-machine read ${i}`, kind: 'container', args: ['container', ...pos],
          expectEmpty: true});
      } else {
        actions.push({name: `readback touched machine ${i}`, kind: 'container', args: ['container', ...pos],
          expectNonNegative: true});
      }
    } else if (i % 3 === 1) {
      // A vanilla stone has no distillation recipe: it may be stored, but must NEVER be
      // distilled. The output slot must stay empty for the whole probe.
      actions.push({name: `invalid insert ${i}`, kind: 'container-put', args: ['container', ...pos, VANILLA_INPUT, '1'],
        invalidInput: true});
      touched.add(key);
      actions.push({name: `invalid insert readback ${i}`, kind: 'container', args: ['container', ...pos],
        invalidInput: true});
    } else {
      actions.push({name: `repeat setblock ${i}`, kind: 'setblock',
        args: ['setblock', ...pos, MACHINE]});
      actions.push({name: `readback ${i}`, kind: 'block', args: ['block', ...pos], expect: MACHINE});
    }
    if (i % 8 === 7) actions.push({name: `save ${i}`, kind: 'save', args: ['save']});
    if (i % 12 === 11) actions.push({name: `reload ${i}`, kind: 'reload', args: []});
  }
  actions.push({name: 'destroy machine (chaos)', kind: 'setblock',
    args: ['setblock', ...MACHINE_POSITIONS.destroy.map(String), 'minecraft:air'],
    expect: 'minecraft:air'});
  return {seed, steps, positions: MACHINE_POSITIONS, actions, sanity};
}

export {itemsPerCall};
