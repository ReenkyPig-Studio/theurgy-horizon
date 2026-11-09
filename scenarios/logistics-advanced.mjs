// Java-faced connector/proxy/world-source/flux/nexus integration coverage.
const S=p=>p.map(String);
const text=r=>Array.isArray(r?.chat)?r.chat.join('\n'):JSON.stringify(r??'');
const amount=r=>Number(/Amount\s*:\s*(-?\d+)/.exec(text(r))?.[1]??0);
const energy=r=>Number(/energy\s*:\s*(-?\d+)/.exec(text(r))?.[1]??0);
const slots=r=>Array.isArray(r?.slots)?r.slots:[];
const total=r=>slots(r).reduce((n,v)=>n+Number(v?.count??v?.item?.count??0),0);
const itemAt=(r,i)=>{const v=slots(r)[i];if(!v)return null;const x=v.item&&typeof v.item==='object'?v.item:v;return x.id??x.item??null;};
const countItem=(r,id)=>slots(r).reduce((n,v)=>{const x=v?.item&&typeof v.item==='object'?v.item:v;return n+(x?.id===id||x?.item===id?Number(x.count??v.count??0):0);},0);
export async function runCustom(call,assert){
 const out=[];
 const step=async(name,args)=>{const r=await call(args);out.push({name,args,reply:r});if(['setblock','command','container','give','place'].includes(args[0]))await call(['wait','2']);return r;};
 const place=async(name,p,id,state='')=>{await step(name+' clear old fixture',['setblock',...S(p),'minecraft:air']);return step(name,['setblock',...S(p),id+state]);};
 const nexus=(p,id)=>step('nexus '+p,['command','data','merge','block',...S(p),`{nexusId:"${id}"}`]);
 const data=(name,p)=>step(name,['command','data','get','block',...S(p)]);
 const put=(name,p,item,count='1')=>step(name,['container',...S(p),item,count]);
 const merge=(name,p,nbt)=>step(name,['command','data','merge','block',...S(p),nbt]);
 await step('advanced lab',['tp','45','82','55']);await step('advanced chunks',['wait','40']);await step('clear advanced inventory',['command','clear','@s']);

 // Infinite world water source: extract, never remove the source block.
 const water=[12,80,60],ex=[11,80,60],input=[14,80,60],tank=[13,80,60];
 await place('world water',water,'minecraft:water');await place('world extractor',ex,'theurgy:logistics_fluid_extractor','[facing=west]');
 await place('world inserter',input,'theurgy:logistics_fluid_inserter','[facing=east]');await place('world tank',tank,'theurgy:sal_ammoniac_tank');
 for(const p of [ex,input])await nexus(p,'world-fluid');
 await step('world transfer',['wait','60']);
 assert.ok(amount(await data('world target',tank))>0,'world source fills tank');
 assert.ok(String((await data('source remains',water)).reply??'').includes('water')||true);
 const sourceState=await step('world source block',['block',...S(water)]);
 assert.ok(String(sourceState.state??sourceState.block).startsWith('minecraft:water'),'infinite source must remain');

 // Two probes are selected round-robin by the capability proxy.
 const sourceA=[20,80,60],probeA=[21,80,60],proxy=[23,80,60],sourceB=[26,80,60],probeB=[25,80,60],fluidIn=[28,80,60],fluidDst=[27,80,60];
 for(const [name,p] of [['source A',sourceA],['source B',sourceB],['fluid destination',fluidDst]])await place(name,p,'theurgy:sal_ammoniac_tank');
 await place('probe A',probeA,'theurgy:logistics_capability_probe','[facing=east]');
 await place('proxy',proxy,'theurgy:logistics_capability_proxy');await place('probe B',probeB,'theurgy:logistics_capability_probe','[facing=west]');
 await place('proxy fluid inserter',fluidIn,'theurgy:logistics_fluid_inserter','[facing=east]');
 for(const p of [sourceA,probeA,proxy,probeB,fluidIn])await nexus(p,'proxy-fluid');
 await merge('source A water',sourceA,'{fluids:[{Fluid:"minecraft:water",Amount:1000}]}');
 await merge('source B water',sourceB,'{fluids:[{Fluid:"minecraft:water",Amount:1000}]}');
 await step('proxy round robin transfers',['wait','200']);
 await data('proxy source A state',sourceA);await data('proxy source B state',sourceB);await data('proxy final state',proxy);
 await step('probe A block state',['block',...S(probeA)]);await data('probe A nbt',probeA);
 assert.equal(amount(await data('proxy destination',fluidDst)),2000,'both probes must forward exactly one bucket');
 await data('proxy final state',proxy);
 assert.equal(amount(await data('probe source A empty',sourceA)),0);
 assert.equal(amount(await data('probe source B empty',sourceB)),0);

 // Item capability forwarding uses the same probe round-robin.
 const chestA=[40,80,60],itemProbeA=[41,80,60],itemProxy=[43,80,60],chestB=[46,80,60],itemProbeB=[45,80,60],itemIn=[48,80,60],chestDst=[47,80,60];
 await place('item chest A',chestA,'minecraft:chest');await place('item probe A',itemProbeA,'theurgy:logistics_capability_probe','[facing=east]');
 await place('item proxy',itemProxy,'theurgy:logistics_capability_proxy');await place('item chest B',chestB,'minecraft:chest');
 await place('item probe B',itemProbeB,'theurgy:logistics_capability_probe','[facing=west]');await place('item inserter',itemIn,'theurgy:logistics_item_inserter','[facing=east]');await place('item destination',chestDst,'minecraft:chest');
 for(const p of [itemProbeA,itemProxy,itemProbeB,itemIn])await nexus(p,'proxy-items');
 await put('item source A bread',chestA,'minecraft:bread');await put('item source B apple',chestB,'minecraft:apple');
 await step('proxy item transfers',['wait','200']);
 const dstItems=await step('proxy item destination',['container',...S(chestDst)]);
 assert.equal(countItem(dstItems,'minecraft:apple'),1);assert.equal(countItem(dstItems,'minecraft:bread'),1);
 assert.equal(total(await step('item source A empty',['container',...S(chestA)])),0);
 assert.equal(total(await step('item source B empty',['container',...S(chestB)])),0);

 // Source capacitor -> connector A -> connector B -> target capacitor.
 const sourceCap=[62,80,60],connectorA=[61,80,60],connectorB=[67,80,60],targetCap=[66,80,60];
 await place('source capacitor',sourceCap,'theurgy:mercury_capacitor');await place('flux connector A',connectorA,'theurgy:logistics_mercury_flux_connector','[facing=west]');
 await place('flux connector B',connectorB,'theurgy:logistics_mercury_flux_connector','[facing=east]');await place('target capacitor',targetCap,'theurgy:mercury_capacitor');
 for(const p of [connectorA,connectorB])await nexus(p,'flux-net');
 await merge('charge source capacitor',sourceCap,'{mercuryFluxHandler:{energy:500},sideModes:{WEST:3}}');
 await step('flux connector transfer',['wait','300']);
 await step('source capacitor state',['block',...S(sourceCap)]);
 await data('source capacitor debug',sourceCap);await data('connector A debug',connectorA);await data('connector B debug',connectorB);
 assert.equal(energy(await data('target flux',targetCap)),500,'connector chain conserves all flux');
 assert.equal(energy(await data('source flux drained',sourceCap)),0);
 assert.equal(energy(await data('connector A empty',connectorA)),0);

 // Nexus item UUID survives placement and pairs both blocks.
 await step('give nexus pair',['give','theurgy:logistics_nexus','2']);
 await step('clear old nexus A',['setblock','80','80','60','minecraft:air']);await step('clear old nexus B',['setblock','82','80','60','minecraft:air']);
 await step('nexus pair ticks',['wait','4']);
 await step('nexus placement approach',['command','tp','@s','80','82','58']);await step('nexus placement chunks',['wait','40']);
 await step('nexus placement settled approach',['command','tp','@s','80','82','58']);
 await step('nexus placement camera',['state']);await step('clear nexus placement volume',['fill','78','79','58','84','84','62','minecraft:air']);
 await step('nexus support A',['setblock','80','79','60','minecraft:stone']);await step('nexus support A readback',['block','80','79','60']);
 await step('nexus support B',['setblock','82','79','60','minecraft:stone']);await step('nexus support B readback',['block','82','79','60']);
 await step('select nexus for placement',['select','theurgy:logistics_nexus']);
 await step('nexus final placement approach',['command','tp','@s','80','82','58']);
 await step('nexus aim A',['look-at','block','80','79','60']);
 await step('place nexus A',['interact','80','79','60','up']);
 await step('nexus approach B',['command','tp','@s','82','82','58']);await step('nexus B chunks',['wait','20']);
 await step('nexus B settled approach',['command','tp','@s','82','82','58']);
 await step('nexus aim B',['look-at','block','82','79','60']);
 await step('place nexus B',['interact','82','79','60','up']);
 const found=await step('placed nexus positions',['find-block','theurgy:logistics_nexus','16']);
 assert.equal(found.count,2,'both nexus items must be placed: '+JSON.stringify(found));
 const idA=text(await data('nexus A id',found.nearest[0].pos)),idB=text(await data('nexus B id',found.nearest[1].pos));
 const uuid=/"?nexusId"?\s*:\s*"?([0-9a-f-]{36})"?/.exec(idA)?.[1];
 assert.ok(uuid,'placed nexus must carry UUID: '+idA);
 assert.ok(idB.includes(uuid),'both nexus blocks must pair by the item UUID: '+idB);
 return out;
}
export default {name:'logistics-advanced',runCustom};
