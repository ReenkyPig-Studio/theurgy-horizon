// GUI check of the sal ammoniac LiquidBlock in the world (THEURGY_GUI=1):
// a source on a flat floor spreads as sal ammoniac and is drawn by the fluid
// renderer (native/theurgy/sal_fluid.rs), i.e. a sloped surface instead of
// one flat step per level. Java: FluidRegistry.SAL_AMMONIAC(_FLOWING),
// Theurgy.java onRegisterFluidModels. The shot is for a human to look at.
import path from 'node:path';
import fs from 'node:fs/promises';
import {root} from '../tools/common.mjs';

const shots=path.join(root,'.local/agent-sal-fluid/shots');

export async function runCustom(call,assert){
 const out=[];
 const step=async(name,args)=>{const r=await call(args);out.push({name,args,reply:r});return r;};
 await fs.mkdir(shots,{recursive:true});
 await step('clear',['fill','20','79','20','44','90','44','minecraft:air']);
 await step('floor',['fill','20','78','20','44','78','44','minecraft:stone']);
 await step('tp',['tp','32','82','25']);
 await step('chunks',['wait','40']);
 await step('weather',['command','weather','clear']);
 await step('time',['command','time','set','6000']);
 await step('source',['setblock','32','79','32','theurgy:sal_ammoniac']);
 await step('spread',['wait','120']);
 for(const [dx,level] of [[0,'0'],[1,'1'],[4,'4'],[7,'7']]){
  const block=await step('block +'+dx,['block',String(32+dx),'79','32']);
  assert.equal(block.block,'theurgy:sal_ammoniac','sal ammoniac must spread as itself at +'+dx);
  assert.equal(String(block.properties?.level??block.state?.level??level),level,'water spread levels at +'+dx);
 }
 await step('hide HUD',['key','f1','down']);await step('release HUD',['key','f1','up']);
 await step('look',['look-at','32.5','79.5','32.5']);
 await step('frame',['wait','10']);
 const shot=await step('screenshot',['screenshot',path.join(shots,'spread.png')]);
 assert.ok(shot&&shot.ok!==false,'screenshot failed');
 await step('low',['tp','32','80','23']);
 await step('look low',['look-at','32.5','79.3','32.5']);
 await step('frame low',['wait','10']);
 await step('screenshot low',['screenshot',path.join(shots,'spread-low.png')]);
 await step('restore HUD',['key','f1','down']);await step('release restored HUD',['key','f1','up']);
 return out;
}
