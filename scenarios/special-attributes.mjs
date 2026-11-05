const FILTER='theurgy:attribute_filter',EXTRACTOR='theurgy:logistics_item_extractor',INSERTER='theurgy:logistics_item_inserter';
const SRC=[70,80,70],EX=[71,80,70],IN=[74,80,70],DST=[75,80,70],NEX=[72,80,72];const s=p=>p.map(String);
function count(r,id){return (r?.slots??[]).reduce((n,v)=>{const x=v?.item&&typeof v.item==='object'?v.item:v;return n+(x?.id===id||x?.item===id?Number(x.count??v.count??0):0);},0);}
const cases=[
 ['added_by','theurgy','theurgy:mercury_shard','minecraft:apple'],
 ['enchantment','minecraft:sharpness','minecraft:wooden_sword[minecraft:enchantments={"minecraft:sharpness":1}]','minecraft:wooden_sword'],
 ['fluid_contents','minecraft:water','minecraft:water_bucket','minecraft:lava_bucket'],
 ['item_name','Special','minecraft:apple[minecraft:item_name="Special"]','minecraft:apple'],
];
export async function runCustom(call,assert){const results=[];const step=async(name,args)=>{const r=await call(args);results.push({name,args,reply:r});if(['give','setblock','tp','container','command'].includes(args[0]))await call(['wait','2']);return r;};
const place=(n,p,id,state='')=>step(n,['setblock',...s(p),id+state]);const nexus=p=>step('nexus '+p,['command','data','merge','block',...s(p),'{nexusId:"special-net"}']);
const click=async(name,p,sneak=false)=>{if(sneak)await step('disable transition',['command','data','merge','block',...s(p),'{enabled:0}']);if(sneak)await step('sneak down',['key','shift','down']);await step(name+' approach',['tp',String(p[0]),'82',String(p[2]-2)]);await step(name+' look',['look-at','block',...s(p)]);const r=await step(name,['interact',...s(p)]);if(sneak)await step('sneak up',['key','shift','up']);if(!sneak)await step('enable connector',['command','data','merge','block',...s(p),'{enabled:1}']);return r;};
await step('special lab',['tp','72','82','68']);await step('chunks',['wait','40']);await place('source',SRC,'minecraft:chest');await place('extractor',EX,EXTRACTOR,'[facing=west]');await place('inserter',IN,INSERTER,'[facing=east]');await place('dest',DST,'minecraft:chest');await place('nexus',NEX,'theurgy:logistics_nexus');for(const p of [EX,IN,NEX])await nexus(p);
for(const [kind,value,yes,no] of cases){
 await place('reset source '+kind,SRC,'minecraft:chest');await place('reset dest '+kind,DST,'minecraft:chest');await step('new special filter '+kind,['give',FILTER,'1']);
 const payload=kind==='item_name'?{value}:{id:value};
 const config=JSON.stringify({mode:'match_any',rules:[{type:'codedefinedgui:'+kind,payload,inverted:false}]});
 await step('configure '+kind,['command','item modify entity @s weapon.mainhand {"type":"minecraft:set_components","components":{"codedefinedgui:attribute_filter_config":'+config+'}}']);
 await click('attach '+kind,EX);const yesId=yes.split('[')[0],noId=no.split('[')[0];
 await step('accepted '+kind,['command','item replace block 70 80 70 container.0 with '+yes+' 1']);await step('rejected '+kind,['command','item replace block 70 80 70 container.1 with '+no+' 1']);await step('wait '+kind,['wait','30']);
 assert.ok(count(await step('accepted destination '+kind,['container',...s(DST)]),yesId)>=1,kind+' accepts');assert.equal(count(await step('rejected source '+kind,['container',...s(SRC)]),noId),1,kind+' rejects');await click('remove '+kind,EX,true);
}
return results;}
export default {name:'special-attributes',runCustom};