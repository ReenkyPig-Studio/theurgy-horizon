// Real GUI/native-renderer smoke. Screenshots are inspected by the trusted
// integrator; a successful headless cargo build is not rendering evidence.
const S=p=>p.map(String);
export async function runCustom(call,assert){
 const out=[];
 const step=async(name,args)=>{const r=await call(args);out.push({name,args,reply:r});return r;};
 await step('render lab',['tp','30','82','36']);
 await step('chunks',['wait','40']);
 const blocks=[
  ['distiller',[30,80,40],'theurgy:distiller'],
  ['calcination',[34,80,40],'theurgy:calcination_oven'],
  ['liquefaction',[38,80,40],'theurgy:liquefaction_cauldron'],
  ['sal tank',[42,80,40],'theurgy:sal_ammoniac_tank'],
  ['accumulator',[46,80,40],'theurgy:sal_ammoniac_accumulator'],
  ['nexus',[50,80,40],'theurgy:logistics_nexus'],
  ['mercury vessel',[54,80,40],'theurgy:incubator_mercury_vessel'],
 ];
 for(const [name,pos,id] of blocks){
  await step('place '+name,['setblock',...S(pos),id]);
  if(name==='liquefaction'||name==='sal tank')await step('fill '+name,['command','data','merge','block',...S(pos),'{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:1000}]}']);
  await step('approach '+name,['tp',String(pos[0]),String(pos[1]+2),String(pos[2]-4)]);
  await step('look '+name,['look-at','block',...S(pos)]);
  await step('frame '+name,['wait','4']);
  const shot=await step('screenshot '+name,['screenshot','render-'+name.replaceAll(' ','-')+'.png']);
  assert.ok(shot&&shot.ok!==false,'screenshot failed for '+name+': '+JSON.stringify(shot));
 }
 return out;
}
export default {name:'render',runCustom};
