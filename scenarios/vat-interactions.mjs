// GenericVatInteractionBehaviour.useItemOn for both vat types, through the
// authoritative native interaction path. No client-side state is fabricated.
const S=p=>p.map(String);
const text=r=>[r?.actionbar,...(Array.isArray(r?.chat)?r.chat:[])].filter(Boolean).join('\n')||JSON.stringify(r??'');
const state=r=>String(r?.state??r?.block??'');
const slots=r=>Array.isArray(r?.slots)?r.slots:[];
const total=r=>slots(r).reduce((n,v)=>n+Number(v?.count??v?.item?.count??0),0);
const itemAt=(r,i)=>{const v=slots(r)[i];if(!v)return null;const x=v.item&&typeof v.item==='object'?v.item:v;return x.id??x.item??null;};

export async function runCustom(call,assert){
 const out=[];
 const step=async(name,args)=>{const r=await call(args);out.push({name,args,reply:r});if(['setblock','container','give','command','tp'].includes(args[0]))await call(['wait','2']);return r;};
 const place=async(name,p,id)=>{await step(name+' clear',['setblock',...S(p),'minecraft:air']);return step(name,['setblock',...S(p),id]);};
 const interact=async(name,p)=>step(name,['command','theurgy_interact',...S(p),'north']);
 const shiftInteract=async(name,p)=>{await step(name+' shift down',['key','shift','down']);const r=await interact(name,p);await step(name+' shift up',['key','shift','up']);return r;};
 const data=async(name,p)=>step(name,['command','data','get','block',...S(p)]);
 const messages=async name=>step(name,['messages','30']);
 const cases=[
  {kind:'fermentation',pos:[20,80,20],fluid:'minecraft:water',items:['theurgy:alchemical_sulfur_beef','minecraft:oak_log']},
  {kind:'digestion',pos:[24,80,20],fluid:'theurgy:sal_ammoniac',items:['theurgy:alchemical_niter_animals_common']},
 ];
 await step('vat interaction lab',['command','tp','@s','22','80','16']);await step('vat interaction chunks',['wait','40']);
 for(const fixture of cases){
  const {kind,pos,fluid,items}=fixture,block=`theurgy:${kind}_vat`;
  await place(`${kind} valid vat`,pos,block);
  await step(`${kind} solvent`,['command','data','merge','block',...S(pos),`{fluids:[{Fluid:"${fluid}",Amount:1000}]}`]);
  await step(`${kind} empty hand before open insertion`,['command','clear','@s']);
  await step(`${kind} held input for open insertion`,['give',items[0],'1']);
  await interact(`${kind} insert item while open`,pos);
  assert.equal(itemAt(await step(`${kind} open insertion readback`,['container',...S(pos)]),0),items[0],kind+' open vat must accept input');
  for(const item of items.slice(1))await step(`${kind} input ${item}`,['container',...S(pos),item,'1']);
  const outputItems=items.map((item,index)=>`{Slot:${index}b,id:"${item}",count:1}`).join(',')+', {Slot:3b,id:"minecraft:cobblestone",count:1}';
  await step(`${kind} seed open output`,['command','data','merge','block',...S(pos),`{Items:[${outputItems}]}`]);
  await step(`${kind} empty hand before open extraction`,['command','clear','@s']);
  await interact(`${kind} extract output while open`,pos);
  assert.equal(itemAt(await step(`${kind} open extraction readback`,['container',...S(pos)]),3),null,kind+' open vat must return output');
  assert.equal(itemAt(await step(`${kind} input survives open extraction`,['container',...S(pos)]),0),items[0],kind+' extraction must not remove input');
  await step('clear vat interaction hand',['command','clear','@s']);
  await shiftInteract(`${kind} close valid vat`,pos);
  assert.ok(state(await step(`${kind} closed state`,['block',...S(pos)])).includes('open=false'),kind+' valid recipe must close');
  await step(`${kind} processing window`,['wait','30']);
  assert.ok(/progress:\s*[1-9]\d*s?/.test(text(await data(`${kind} real progress`,pos))),kind+' must really process before reopening');

  const before=total(await step(`${kind} closed inventory before`,['container',...S(pos)]));
  await step(`${kind} held item for closed rejection`,['give','minecraft:oak_log','1']);
  await interact(`${kind} reject item while closed`,pos);
  const after=total(await step(`${kind} closed inventory after`,['container',...S(pos)]));
  assert.equal(after,before,kind+' closed vat must not accept or return items');
  assert.ok(text(await messages(`${kind} closed message`)).includes('Cannot add or remove items or fluids'),
    kind+' closed message missing');

  await step(`${kind} empty hand before open`,['command','clear','@s']);
  await shiftInteract(`${kind} reopen vat`,pos);
  assert.ok(state(await step(`${kind} reopened state`,['block',...S(pos)])).includes('open=true'),kind+' shift-empty must open');
  assert.ok(/progress:\s*0s/.test(text(await data(`${kind} progress reset`,pos))),kind+' opening must stop and reset processing');

  await place(`${kind} invalid vat`,pos,block);
  await step(`${kind} invalid solvent`,['command','data','merge','block',...S(pos),`{fluids:[{Fluid:"${fluid}",Amount:1000}]}`]);
  await step(`${kind} invalid ingredient`,['container',...S(pos),'minecraft:diamond','1']);
  await step(`${kind} empty hand invalid close`,['command','clear','@s']);
  await shiftInteract(`${kind} reject invalid close`,pos);
  assert.ok(state(await step(`${kind} invalid stays open`,['block',...S(pos)])).includes('open=true'),kind+' invalid recipe must not close');
  assert.ok(text(await messages(`${kind} no recipe message`)).includes('Cannot close vat'),kind+' no-recipe message missing');
 }
 return out;
}
export default {name:'vat-interactions',runCustom};
