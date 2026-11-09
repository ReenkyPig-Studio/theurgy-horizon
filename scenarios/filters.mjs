// FilterBehaviour.useItemOn / ItemStack.OPTIONAL_CODEC: insert a count-one
// copy, persist its custom component, synchronize has_filter, and return the
// exact component-bearing stack on Shift + empty-hand click. No filter GUI or
// matching semantics are claimed by this bounded scenario.
const FILTER='theurgy:list_filter', CONNECTOR='theurgy:logistics_item_inserter';
const POS=[70,80,70];
function held(inv){return inv?.held??null;}
function count(inv,id){return Number(inv?.counts?.[id]??0);}
export async function runCustom(call,assert){
  const results=[];
  const step=async(name,args)=>{const reply=await call(args);results.push({name,args,reply});if(['give','setblock','tp','command'].includes(args[0]))await call(['wait','2']);return reply;};
  await step('filter lab',['tp','70','82','68']);
  await step('filter chunks',['wait','40']);
  await step('clear test inventory',['command','clear','@s']);
  await step('connector',['setblock','70','80','70',CONNECTOR+'[facing=up]']);
  await step('give filter',['command','give @s '+FILTER+'[minecraft:custom_name="Marked Filter"] 1']);
  let inv=await step('filter initial',['inventory']);
  assert.equal(held(inv).item,FILTER);assert.equal(held(inv).count,1);
  inv=await step('marked filter',['inventory']);
  assert.ok(String(held(inv).components).includes('Marked Filter'),'test fixture must carry a real custom component: '+JSON.stringify(held(inv)));
  await step('approach filter connector',['tp','70','82','68']);
  await step('look filter connector',['look-at','block','70','80','70']);
  await step('insert filter',['interact','70','80','70']);
  inv=await step('filter consumed',['inventory']);
  assert.equal(held(inv)?.item??null,null,'insert consumes the held filter');
  assert.equal(count(inv,FILTER),0,'only one filter existed and was consumed');
  const nbt=await step('saved filter',['command','data get block 70 80 70']);
  assert.ok(JSON.stringify(nbt).includes('filter'),'Java filter field must persist');
  assert.ok(JSON.stringify(nbt).includes('Marked Filter'),'saved filter must preserve custom components: '+JSON.stringify(nbt));
  const state=await step('filter state',['block','70','80','70']);
  assert.ok(String(state.state).includes('has_filter=true'),'filter presence must reach block state: '+JSON.stringify(state));
  await step('save filter world',['reload']);
  await step('filter loaded',['wait','40']);
  const loaded=await step('saved filter after reload',['command','data get block 70 80 70']);
  assert.ok(JSON.stringify(loaded).includes('Marked Filter'),'filter components survive reload');
  await step('sneak down',['key','shift','down']);
  await step('re-approach filter',['tp','70','82','68']);
  await step('look removal',['look-at','block','70','80','70']);
  await step('remove filter',['interact','70','80','70']);
  await step('sneak up',['key','shift','up']);
  inv=await step('returned filter',['inventory']);
  assert.equal(count(inv,FILTER),1,'remove returns one filter');
  assert.ok(String(held(inv)?.components??'').includes('Marked Filter'),'returned stack retains its components: '+JSON.stringify(held(inv)));
  const empty=await step('filter removed NBT',['command','data get block 70 80 70']);
  assert.ok(!/filter\s*:\s*\{/.test(JSON.stringify(empty)),'successful removal clears filter NBT: '+JSON.stringify(empty));
  const removedState=await step('filter removed state',['block','70','80','70']);
  assert.ok(String(removedState.state).includes('has_filter=false'));
  return results;
}
export default {name:'filters',runCustom};
