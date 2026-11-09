// Real copper-wire graph: no nexusId shortcut. Connect two endpoints, persist
// their Java Wire records, transfer across the graph, disconnect/reconnect and
// isolate frequencies while checking exact item/wire conservation.
const WIRE='theurgy:copper_wire';
const EXTRACTOR='theurgy:logistics_item_extractor';
const INSERTER='theurgy:logistics_item_inserter';
const SOURCE=[42,80,40], EX=[41,80,40], IN=[47,80,40], DEST=[46,80,40];
const s=p=>p.map(String);
function slots(reply){return Array.isArray(reply?.slots)?reply.slots:[];}
function countAt(reply,i){const v=slots(reply)[i];if(!v)return 0;const inner=v.item&&typeof v.item==='object'?v.item:v;return Number(inner.count??v.count??0);}
function total(reply){return slots(reply).reduce((n,_,i)=>n+countAt(reply,i),0);}
function itemAt(reply,i){const v=slots(reply)[i];if(!v)return null;const inner=v.item&&typeof v.item==='object'?v.item:v;return inner.id??inner.item??null;}
function wireCount(reply){return Number(reply?.counts?.[WIRE]??0);}
function held(reply){return String(reply?.held?.components??'');}
function packed(pos){return ((BigInt(pos[0])&0x3ffffffn)<<38n)|((BigInt(pos[2])&0x3ffffffn)<<12n)|(BigInt(pos[1])&0xfffn);}

export async function runCustom(call,assert){
  const results=[];
  const step=async(name,args)=>{const reply=await call(args);results.push({name,args,reply});if(['give','setblock','tp','container','command'].includes(args[0]))await call(['wait','2']);return reply;};
  const place=async(name,p,id,state='')=>{
    await step(name+' clear old fixture',['setblock',...s(p),'minecraft:air']);
    return step(name,['setblock',...s(p),id+state]);
  };
  const nbt=async p=>JSON.stringify(await call(['command',`data get block ${p.join(' ')}`]));
  const click=async(name,p)=>{
    await step(name+' approach',['tp',String(p[0]),String(p[1]),String(p[2]-2)]);
    await step(name+' look',['look-at','block',...s(p)]);
    return step(name,['interact',...s(p)]);
  };
  const container=(name,p)=>step(name,['container',...s(p)]);
  const put=(name,item,count=1)=>step(name,['container',...s(SOURCE),item,String(count)]);

  await step('wire graph lab',['tp','44','80','36']);
  await step('wire graph safe floor',['fill','38','79','36','50','79','44','minecraft:stone']);
  await step('wire graph chunks',['wait','40']);
  await step('clear wire graph inventory',['command','clear','@s']);
  await step('wire conservation in survival',['command','gamemode','survival','@s']);
  await place('source chest',SOURCE,'minecraft:chest');
  await place('wired extractor',EX,EXTRACTOR,'[facing=west]');
  await place('wired inserter',IN,INSERTER,'[facing=east]');
  await place('destination chest',DEST,'minecraft:chest');
  await step('give two wires',['give',WIRE,'2']);
  assert.equal(wireCount(await step('initial wires',['inventory'])),2);

  await click('store extractor',EX);
  assert.ok(held(await step('stored endpoint',['inventory'])).includes('wire_end_point'));
  await click('connect inserter',IN);
  const connected=await step('connected wires',['inventory']);
  assert.equal(wireCount(connected),1,'connect consumes exactly one wire');
  assert.ok(!held(connected).includes('wire_end_point'),'connect clears stored endpoint');
  const serverConnected=await step('authoritative connected stack',['command','data','get','entity','@s','SelectedItem']);
  assert.ok(!JSON.stringify(serverConnected).includes('wire_end_point'),'server stack must clear wire_end_point: '+JSON.stringify(serverConnected));
  const connectedNbt=await nbt(IN);
  assert.ok(connectedNbt.includes('wireConnections'),'clicked endpoint must persist Java wireConnections: '+connectedNbt);
  assert.ok(connectedNbt.includes(String(packed(EX))),'wire from must be packed BlockPos.asLong(EX)');
  assert.ok(connectedNbt.includes(String(packed(IN))),'wire to must be packed BlockPos.asLong(IN)');
  assert.ok(!(await nbt(EX)).includes('nexusId'),'wire graph must not use a nexusId shortcut');
  assert.ok(!(await nbt(IN)).includes('nexusId'),'inserter must join through the wire, not nexusId');

  await put('seed bread','minecraft:bread',3);
  await step('bread transfer wait',['wait','60']);
  assert.equal(total(await container('source after bread',SOURCE)),0,'wired extractor drains source');
  const breadDest=await container('bread destination',DEST);
  assert.equal(itemAt(breadDest,0),'minecraft:bread');
  assert.equal(total(breadDest),3,'three bread cross the wire exactly once');

  await step('save and reload wire world',['reload']);
  await step('loaded wire chunks',['wait','40']);
  await put('seed apple after reload','minecraft:apple',1);
  await step('persisted wire transfer wait',['wait','60']);
  assert.equal(total(await container('source after persisted apple',SOURCE)),0,'persisted wire must still extract');
  assert.equal(total(await container('destination after persisted apple',DEST)),4,'reload must not duplicate or lose items');
  assert.equal(wireCount(await step('wire count after reload',['inventory'])),1);
  const serverReloaded=await step('authoritative reloaded stack',['command','data','get','entity','@s','SelectedItem']);
  assert.ok(!JSON.stringify(serverReloaded).includes('wire_end_point'),'reload must not resurrect wire_end_point: '+JSON.stringify(serverReloaded));

  await click('select wire for disconnect',EX);
  await click('disconnect inserter',IN);
  const disconnected=await step('disconnected wires',['inventory']);
  assert.equal(wireCount(disconnected),2,'disconnect returns exactly one wire');
  assert.ok(!held(disconnected).includes('wire_end_point'),'disconnect clears endpoint');
  assert.ok(!(await nbt(IN)).includes('wireConnections'),'disconnect removes persisted wire record');
  await put('seed cookie while disconnected','minecraft:cookie',1);
  await step('disconnected wait',['wait','60']);
  assert.equal(total(await container('cookie blocked at source',SOURCE)),1,'disconnected graph must not transfer');
  assert.equal(total(await container('destination unchanged by disconnect',DEST)),4);

  await click('select wire for reconnect',EX);
  await click('reconnect inserter',IN);
  assert.equal(wireCount(await step('reconnected wire count',['inventory'])),1);
  await step('reconnected cookie wait',['wait','60']);
  assert.equal(total(await container('source after reconnect',SOURCE)),0);
  assert.equal(total(await container('destination after reconnect',DEST)),5,'reconnect transfers the blocked cookie once');

  await step('isolate inserter frequency',['command','data','merge','block',...s(IN),'{frequency:7}']);
  await put('seed berries on wrong frequency','minecraft:sweet_berries',1);
  await step('wrong frequency wait',['wait','60']);
  assert.equal(total(await container('berries blocked by frequency',SOURCE)),1,'different frequencies must not share leaf targets');
  await step('restore inserter frequency',['command','data','merge','block',...s(IN),'{frequency:0}']);
  await step('matching frequency wait',['wait','60']);
  assert.equal(total(await container('source after frequency restore',SOURCE)),0);
  assert.equal(total(await container('destination after frequency restore',DEST)),6,'matching frequency transfers berries once');
  return results;
}
export default {name:'wire-graph',runCustom};
