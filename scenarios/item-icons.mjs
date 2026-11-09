// GUI check that block items (including the baked GeckoLib ones) draw an icon
// in the hotbar, the same item renderer the creative tab uses.
import path from 'node:path';
import fs from 'node:fs/promises';
import {root} from '../tools/common.mjs';

const ITEMS=['theurgy:calcination_oven','theurgy:sal_ammoniac_tank','theurgy:sal_ammoniac_accumulator',
 'theurgy:distiller','theurgy:logistics_nexus','theurgy:incubator_mercury_vessel',
 'theurgy:incubator_salt_vessel','theurgy:incubator_sulfur_vessel','theurgy:pyromantic_brazier'];

export async function runCustom(call,assert){
 const out=[];
 const step=async(name,args)=>{const r=await call(args);out.push({name,args,reply:r});return r;};
 const shots=path.join(root,'.local/agent-item-icons/shots');
 await fs.mkdir(shots,{recursive:true});
 await step('chunks',['wait','40']);
 await step('clear',['command','clear','@s']);
 for(const item of ITEMS)await step('give '+item,['give',item,'1']);
 await step('settle',['wait','10']);
 const shot=await step('screenshot hotbar',['screenshot',path.join(shots,'hotbar.png')]);
 assert.ok(shot&&shot.ok!==false,'screenshot failed: '+JSON.stringify(shot));
 return out;
}
