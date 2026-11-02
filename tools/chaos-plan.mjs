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
