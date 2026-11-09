// Real CodeDefinedGUI list filters on extractor+inserter, including components and fluids.
const FILTER='theurgy:list_filter',EXTRACTOR='theurgy:logistics_item_extractor',INSERTER='theurgy:logistics_item_inserter';
const SRC=[12,80,10],EX=[11,80,10],IN=[14,80,10],DST=[13,80,10],NEX=[12,80,12];
const TANK_SRC=[12,80,20],FLUID_EX=[11,80,20],FLUID_IN=[14,80,20],TANK_DST=[13,80,20],FLUID_NEX=[12,80,22];
const s=p=>p.map(String);
function slots(r){return Array.isArray(r?.slots)?r.slots:[];}
function count(r,id){return slots(r).reduce((n,v)=>{const x=v?.item&&typeof v.item==='object'?v.item:v;return n+(x?.id===id||x?.item===id?Number(x.count??v.count??0):0);},0);}
function held(r){return String(r?.held?.components??'');}
export async function runCustom(call,assert){
 const results=[];const step=async(name,args)=>{const r=await call(args);results.push({name,args,reply:r});if(['give','setblock','tp','container','command'].includes(args[0]))await call(['wait','2']);return r;};
 const place=async(name,p,id,state='')=>{await step(name+' clear old fixture',['setblock',...s(p),'minecraft:air']);return step(name,['setblock',...s(p),id+state]);};
 const nexus=async(p,id)=>step('nexus '+p,['command','data','merge','block',...s(p),'{nexusId:"'+id+'"}']);
 const click=async(name,p,sneak=false)=>{
   if(sneak)await step('disable for filter change',['command','data','merge','block',...s(p),'{enabled:0}']);
   if(sneak)await step('sneak down',['key','shift','down']);
   await step(name+' approach',['tp',String(p[0]),'82',String(p[2]-2)]);
   await step(name+' look',['look-at','block',...s(p)]);
   const r=await step(name,['interact',...s(p)]);
   if(sneak)await step('sneak up',['key','shift','up']);
   if(!sneak)await step('enable filtered connector',['command','data','merge','block',...s(p),'{enabled:1}']);
   return r;
 };
 const filter=async(mode,item,respect=false)=>{
   await step('clear returned filters',['command','clear','@s']);
   await step('new filter',['give',FILTER,'1']);
   const components='{"codedefinedgui:list_filter_contents":[{"slot":0,"item":{"id":"'+item+'","count":1}}],"codedefinedgui:list_filter_config":{"mode":"'+mode+'","respect_data_components":'+respect+'}}';
   await step('configure '+mode+' '+item,['command','item modify entity @s weapon.mainhand {"type":"minecraft:set_components","components":'+components+'}']);
   const serverStack=await step('configured filter server stack',['command','data get entity @s SelectedItem']);
   assert.ok(JSON.stringify(serverStack).includes('codedefinedgui:list_filter_contents'),'actual list_filter_contents must be on authoritative held stack: '+JSON.stringify(serverStack));
 };
 await step('filter matching lab',['tp','12','82','8']);await step('chunks',['wait','40']);
 await place('source',SRC,'minecraft:chest');await place('extractor',EX,EXTRACTOR,'[facing=west]');await place('inserter',IN,INSERTER,'[facing=east]');await place('dest',DST,'minecraft:chest');await place('nexus',NEX,'theurgy:logistics_nexus');
 for(const p of [EX,IN,NEX])await nexus(p,'matching-items');
 await filter('allow','minecraft:apple');await click('attach allow apple',EX);
 await step('seed apple',['container',...s(SRC),'minecraft:apple','1']);await step('seed cookie',['container',...s(SRC),'minecraft:cookie','1']);await step('allow wait',['wait','60']);
 assert.equal(count(await step('allow destination',['container',...s(DST)]),'minecraft:apple'),1);
 assert.equal(count(await step('cookie blocked',['container',...s(SRC)]),'minecraft:cookie'),1,'allow list rejects other item');
 await click('remove allow apple',EX,true);
 await filter('deny','minecraft:apple');await click('attach deny apple',EX);
 await step('deny reseed apple',['container',...s(SRC),'minecraft:apple','1']);
 const denyNbt=await step('deny filter saved NBT',['command','data get block 11 80 10']);
 assert.ok(JSON.stringify(denyNbt).includes('codedefinedgui:list_filter_config')&&JSON.stringify(denyNbt).includes('deny'),'deny filter components must persist in connector NBT: '+JSON.stringify(denyNbt));
 await step('deny wait',['wait','60']);
 assert.equal(count(await step('deny destination',['container',...s(DST)]),'minecraft:cookie'),1,'deny list permits other item');
 assert.equal(count(await step('apple denied',['container',...s(SRC)]),'minecraft:apple'),1);
 await click('remove deny apple',EX,true);
 await filter('allow','minecraft:cookie');await click('sink allow cookie',IN);
 await step('enable extractor for sink test',['command','data','merge','block',...s(EX),'{enabled:1}']);
 await step('sink seed apple',['container',...s(SRC),'minecraft:apple','1']);await step('sink seed cookie',['container',...s(SRC),'minecraft:cookie','1']);await step('sink wait',['wait','60']);
 assert.equal(count(await step('sink allowed cookie',['container',...s(DST)]),'minecraft:cookie'),2);
 assert.equal(count(await step('sink rejected apple',['container',...s(SRC)]),'minecraft:apple'),2,'inserter filter must independently reject');
 await click('remove sink filter',IN,true);
 await place('reset source',SRC,'minecraft:chest');await place('reset dest',DST,'minecraft:chest');
 await step('plain apple fixture',['command','item replace block 12 80 10 container.0 with minecraft:apple 1']);
 await step('named apple fixture',['command','item replace block 12 80 10 container.1 with minecraft:apple[minecraft:custom_name="Special"] 1']);
 await filter('allow','minecraft:apple',true);
 // Replace the filter entry with an exact named reference stack.
 await step('exact reference',['command','item modify entity @s weapon.mainhand {"type":"minecraft:set_components","components":{"codedefinedgui:list_filter_contents":[{"slot":0,"item":{"id":"minecraft:apple","count":1,"components":{"minecraft:custom_name":"Special"}}}]}}']);
 await click('attach exact apple',EX);
 await step('enable sink for exact test',['command','data','merge','block',...s(IN),'{enabled:1}']);
 const exactNbt=await step('exact saved NBT',['command','data get block 11 80 10']);
 assert.ok(JSON.stringify(exactNbt).includes('respect_data_components: 1b'),'respect flag must persist: '+JSON.stringify(exactNbt));
 await step('exact wait',['wait','60']);
 assert.equal(count(await step('exact named moved',['container',...s(DST)]),'minecraft:apple'),1);
 assert.equal(count(await step('exact plain rejected',['container',...s(SRC)]),'minecraft:apple'),1,'respect_data_components distinguishes named apple');
 await click('remove exact filter',EX,true);
 await place('fluid source',TANK_SRC,'theurgy:sal_ammoniac_tank');await place('fluid extractor',FLUID_EX,'theurgy:logistics_fluid_extractor','[facing=west]');await place('fluid inserter',FLUID_IN,'theurgy:logistics_fluid_inserter','[facing=east]');await place('fluid dest',TANK_DST,'theurgy:sal_ammoniac_tank');await place('fluid nexus',FLUID_NEX,'theurgy:logistics_nexus');
 for(const p of [FLUID_EX,FLUID_IN,FLUID_NEX])await nexus(p,'matching-fluids');
 await filter('allow','minecraft:water_bucket');await click('attach water filter',FLUID_EX);
 await step('seed water',['command','data','merge','block',...s(TANK_SRC),'{fluids:[{Fluid:"minecraft:water",Amount:1000}]}']);await step('water wait',['wait','60']);
 assert.ok(/Amount\D*1000/.test(JSON.stringify(await step('water destination',['command','data','get','block',...s(TANK_DST)]))),'water filter must allow transfer');
 await click('remove water filter',FLUID_EX,true);
 await filter('deny','minecraft:water_bucket');await click('attach deny water',FLUID_EX);
 await step('reseed water',['command','data','merge','block',...s(TANK_SRC),'{fluids:[{Fluid:"minecraft:water",Amount:1000}]}']);await step('deny water wait',['wait','60']);
 assert.ok(/Amount\D*1000/.test(JSON.stringify(await step('water denied at source',['command','data','get','block',...s(TANK_SRC)]))),'deny water must not drain');
 await click('remove deny water',FLUID_EX,true);
 return results;
}
export default {name:'filter-matching',runCustom};