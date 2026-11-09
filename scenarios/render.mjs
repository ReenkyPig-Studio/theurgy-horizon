// Real GUI/native-renderer check. Every apparatus is shot from a fixed camera
// with the HUD hidden, then shot again with only that apparatus removed; the
// pixel difference is its rendered silhouette, compared (grid IoU, area and
// mean colour) with a manually accepted reference in scenarios/render-reference.
// A screenshot that merely exists is not evidence.
import path from 'node:path';
import fs from 'node:fs/promises';
import {root} from '../tools/common.mjs';
import {check} from '../tools/render-check.mjs';

const S=p=>p.map(String);
const shots=path.join(root,'.local/agent-render/shots');
const FIXTURES=[
 // name, position, block, options
 ['distiller-idle',[30,80,40],'theurgy:distiller',{upper:true,heat:false}],
 ['distiller-processing',[30,80,48],'theurgy:distiller',{upper:true,heat:true,input:'minecraft:bread',animated:true}],
 ['calcination-idle',[36,80,40],'theurgy:calcination_oven',{upper:true,heat:false}],
 ['calcination-processing',[36,80,48],'theurgy:calcination_oven',{upper:true,heat:true,input:'minecraft:cobblestone',animated:true}],
 ['liquefaction',[42,80,40],'theurgy:liquefaction_cauldron',{fluid:'{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:1000}]}'}],
 ['sal-tank',[48,80,40],'theurgy:sal_ammoniac_tank',{fluid:'{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:1000}]}'}],
 ['accumulator',[54,80,40],'theurgy:sal_ammoniac_accumulator',{}],
 ['nexus',[60,80,40],'theurgy:logistics_nexus',{}],
 ['mercury-vessel',[42,80,48],'theurgy:incubator_mercury_vessel',{}],
 ['salt-vessel',[48,80,48],'theurgy:incubator_salt_vessel',{}],
 ['sulfur-vessel',[54,80,48],'theurgy:incubator_sulfur_vessel',{}],
];

export async function runCustom(call,assert){
 const out=[];
 const step=async(name,args)=>{const r=await call(args);out.push({name,args,reply:r});return r;};
 await fs.mkdir(shots,{recursive:true});
 await step('quiet fixture',['command','gamerule','send_command_feedback','false']);
 await step('clear render lab',['fill','24','79','34','66','88','56','minecraft:air']);
 await step('render lab floor',['fill','24','78','34','66','78','56','minecraft:stone']);
 await step('render lab',['tp','30','82','36']);
 await step('chunks',['wait','40']);
 await step('clear weather',['command','weather','clear']);
 await step('hide HUD',['key','f1','down']);await step('release HUD toggle',['key','f1','up']);
 const results=[];
 for(const [name,pos,id,options] of FIXTURES){
  const upper=[pos[0],pos[1]+1,pos[2]],below=[pos[0],pos[1]-1,pos[2]];
  await step('heat source '+name,['setblock',...S(below),options.upper?'theurgy:pyromantic_brazier':'minecraft:stone']);
  await step('place '+name,['setblock',...S(pos),id]);
  if(options.upper)await step('place upper '+name,['setblock',...S(upper),id+'[half=upper]']);
  if(options.heat)await step('fuel '+name,['container',...S(below),'minecraft:coal','4']);
  if(options.input)await step('input '+name,['container',...S(pos),options.input,'1']);
  if(options.fluid)await step('fill '+name,['command','data','merge','block',...S(pos),options.fluid]);
  if(options.heat)await step('processing '+name,['wait','40']);
  // Fixed camera: 4 blocks south-west-ish in front, eye 2 above the base.
  await step('approach '+name,['tp',String(pos[0]),String(pos[1]+1),String(pos[2]-3)]);
  await step('settle '+name,['wait','20']);
  await step('look '+name,['look-at',String(pos[0]+0.5),String(pos[1]+0.8),String(pos[2]+0.5)]);
  await step('time '+name,['command','time','set','6000']);
  await step('frame '+name,['wait','6']);
  const present=await step('present '+name,['block',...S(pos)]);
  assert.equal(present.block,id,'render target must still exist for '+name);
  if(options.animated){
   const probe=await step('processing probe '+name,['command','theurgy_probe',...S(pos)]);
   const state=JSON.parse(probe?.chat?.at(-1)?.slice(probe.chat.at(-1).indexOf('{'))??'{}');
   assert.ok(Number(state.data?.[4]??0)===1,name+' must be processing (animated) for the shot: '+JSON.stringify(state.data));
  }
  const scene=path.join(shots,name+'.png'),background=path.join(shots,name+'-background.png');
  const shot=await step('screenshot '+name,['screenshot',scene]);
  assert.ok(shot&&shot.ok!==false,'screenshot failed for '+name+': '+JSON.stringify(shot));
  if(options.upper)await step('remove upper '+name,['setblock',...S(upper),'minecraft:air']);
  await step('remove '+name,['setblock',...S(pos),'minecraft:air']);
  await step('time background '+name,['command','time','set','6000']);
  await step('background frame '+name,['wait','6']);
  const bg=await step('screenshot background '+name,['screenshot',background]);
  assert.ok(bg&&bg.ok!==false,'background screenshot failed for '+name);
  const result=await check(name,scene,background,{minIou:options.animated?0.75:0.85});
  results.push(result);
  out.push({name:'render-check '+name,args:[],reply:result});
 }
 await step('restore HUD',['key','f1','down']);await step('release restored HUD toggle',['key','f1','up']);
 for(const result of results)
  assert.ok(result.accepted||result.ok,'render silhouette mismatch: '+JSON.stringify(result));

 // Reformation's Java GlowTrailProjectile chain: emitter -> target ->
 // sources -> result, while the real recipe is processing.
 const emitter=[58,80,52],target=[58,80,54],result=[58,80,56],sourceA=[56,80,54],sourceB=[56,80,56];
 for(const [name,pos,id] of [
  ['reformation emitter',emitter,'theurgy:sulfuric_flux_emitter'],
  ['reformation target',target,'theurgy:reformation_target_pedestal'],
  ['reformation result',result,'theurgy:reformation_result_pedestal'],
  ['reformation source A',sourceA,'theurgy:reformation_source_pedestal'],
  ['reformation source B',sourceB,'theurgy:reformation_source_pedestal'],
 ])await step(name,['setblock',...S(pos),id]);
 const point=(pos,type)=>`{blockPos:[I;${S(pos).join(',')}],type:"${type}"}`;
 await step('reformation selection',['command','data','merge','block',...S(emitter),
  `{sourcePedestals:[${point(sourceA,'SOURCE')},${point(sourceB,'SOURCE')}],targetPedestal:${point(target,'TARGET')},resultPedestal:${point(result,'RESULT')}}`]);
 await step('reformation flux',['command','data','merge','block',...S(emitter),'{mercuryFluxHandler:{energy:500}}']);
 await step('reformation target item',['container',...S(target),'theurgy:alchemical_niter_animals_abundant','1']);
 await step('reformation source A item',['container',...S(sourceA),'theurgy:alchemical_niter_crops_abundant','1']);
 await step('reformation source B item',['container',...S(sourceB),'theurgy:alchemical_niter_crops_abundant','1']);
 await step('reformation processing window',['wait','40']);
 const processing=await step('reformation processing',['command','theurgy_probe',...S(emitter)]);
 const processingProbe=JSON.parse(processing?.chat?.[0]??'{}');
 assert.ok(Number(processingProbe.nbt?.progress??0)>0,'reformation must be processing for trail evidence: '+JSON.stringify(processing));
 await step('approach reformation',['command','tp','@s','52','82','56']);
 await step('look reformation',['look-at','57','81','54']);
 await step('reformation trail frame',['wait','4']);
 await step('hide HUD for reformation trails',['key','f1','down']);await step('release HUD toggle',['key','f1','up']);
 const reformationShot=await step('screenshot reformation',['screenshot',path.join(shots,'reformation.png')]);
 assert.ok(reformationShot&&reformationShot.ok!==false,'reformation screenshot failed');
 await step('restore HUD after reformation',['key','f1','down']);await step('release restored HUD toggle',['key','f1','up']);
 await step('restore command feedback',['command','gamerule','send_command_feedback','true']);
 return out;
}
export default {name:'render',runCustom};
