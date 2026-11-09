// Mercurial wand: rotate direction, toggle logistics, choose/apply frequency,
// and cycle capacitor side modes through the early block-use hook.
const WAND = 'theurgy:mercurial_wand';
const CONNECTOR = 'theurgy:logistics_item_inserter';
const CAPACITOR = 'theurgy:mercury_capacitor';

function held(inv) {
  const stack = inv?.held;
  return stack ? {item: stack.item, components: String(stack.components ?? '')} : null;
}
function components(inv) { return String(held(inv)?.components ?? ''); }
function frequency(inv) { const m=components(inv).match(/theurgy:selected_frequency"\s*:\s*(\d+)/); return m?Number(m[1]):null; }
const MODE_NAMES=['cycle_direction','switch_logistics_enabled','select_frequency','set_selected_frequency','cycle_capacitor_side'];
function mode(inv) {
  const text=components(inv);
  const named=text.match(/theurgy:mercurial_wand_item_mode"\s*:\s*"([^"]+)"/);
  if(named)return MODE_NAMES.indexOf(named[1]);
  const numeric=text.match(/theurgy:mercurial_wand_item_mode"\s*:\s*(\d+)/);
  return numeric?Number(numeric[1]):0;
}
async function blockNbt(call, pos) {
  const reply=await call(['command', `data get block ${pos.join(' ')}`]);
  return JSON.stringify(reply);
}

export async function runCustom(call, assert) {
  const results=[];
  const step=async(name,args)=>{const reply=await call(args);results.push({name,args,reply});if(['give','setblock','tp'].includes(args[0]))await call(['wait','2']);return reply;};
  await step('wand lab',['tp','60','80','40']);
  await step('chunks',['wait','40']);
  // Java targetDirection() falls back to FACING; UP cycles to NORTH(2).
  await step('clear connector fixture',['setblock','60','80','42','minecraft:air']);
  await step('clear capacitor fixture',['setblock','60','80','46','minecraft:air']);
  await step('connector',['setblock','60','80','42',CONNECTOR+'[facing=up]']);
  await step('capacitor',['setblock','60','80','46',CAPACITOR]);
  await step('clear wand inventory',['command','clear','@s']);
  await step('give wand',['give',WAND,'1']);
  const initial=await step('wand initial',['inventory']);
  // Item defaults are read through native_component but are not repeated in
  // the stack's patch, so an unchanged inventory need not print this component.
  assert.equal(mode(initial),0,'wand starts in cycle_direction');

  await step('approach connector',['tp','60','82','40']);
  await step('connector look',['look-at','block','60','80','42','up']);
  await step('connector look settle',['wait','4']);
  await step('rotate direction',['command','theurgy_interact','60','80','42','up']);
  let nbt=await blockNbt(call,[60,80,42]);
  assert.ok(/directionOverride"?\s*:\s*2/.test(nbt),'UP block facing must cycle to north (2): '+nbt);

  const enabled=await blockNbt(call,[60,80,42]);
  assert.ok(/enabled"?\s*:\s*(true|1b|1)/.test(enabled),'connector starts enabled: '+enabled);
  // Mode switch is a dedicated deterministic input: shift + mouse wheel at the
  // wand changes mode, not the target connector.
  await step('sneak down mode',['key','shift','down']);
  await step('scroll mode',['scroll-at','0','0','1']);
  await step('sneak up mode',['key','shift','up']);
  const mode1=await step('wand enabled mode',['inventory']);
  assert.equal(mode(mode1),1,'shift-wheel advances to switch_logistics_enabled');
  assert.ok(components(mode1).includes('switch_logistics_enabled'),'mode change must use the Java string codec');
  // GUI input advances real time; re-approach instead of letting gravity move
  // the test player out of reach while inventory/scroll commands complete.
  await step('re-approach enabled connector',['tp','60','80','40']);
  await step('enabled connector look',['look-at','block','60','80','42']);
  await step('toggle connector',['command','theurgy_interact','60','80','42','north']);
  nbt=await blockNbt(call,[60,80,42]);
  assert.ok(/enabled"?\s*:\s*(false|0b|0)/.test(nbt),'wand must disable connector: '+nbt);
  await step('re-approach disabled connector',['tp','60','80','40']);
  await step('disabled connector look',['look-at','block','60','80','42']);
  await step('re-enable connector',['command','theurgy_interact','60','80','42','north']);

  await step('sneak down frequency mode',['key','shift','down']);
  await step('scroll frequency mode',['scroll-at','0','0','1']);
  await step('sneak up frequency mode',['key','shift','up']);
  const mode2=await step('wand frequency mode',['inventory']);
  assert.equal(mode(mode2),2,'next mode is select_frequency');
  await step('frequency increment',['use-item']);
  await step('frequency increment 2',['use-item']);
  const freq=await step('frequency inventory',['inventory']);
  assert.equal(frequency(freq),2,'frequency must increment twice');
  await step('sneak down set frequency mode',['key','shift','down']);
  await step('scroll set frequency mode',['scroll-at','0','0','1']);
  await step('sneak up set frequency mode',['key','shift','up']);
  const mode3=await step('wand apply frequency mode',['inventory']);
  assert.equal(mode(mode3),3,'next mode is set_selected_frequency');
  await step('re-approach frequency connector',['tp','60','80','40']);
  await step('frequency connector look',['look-at','block','60','80','42']);
  await step('apply connector frequency',['command','theurgy_interact','60','80','42','north']);
  nbt=await blockNbt(call,[60,80,42]);
  assert.ok(/frequency"?\s*:\s*2/.test(nbt),'wand must set connector frequency: '+nbt);

  await step('sneak down side mode',['key','shift','down']);
  await step('scroll side mode',['scroll-at','0','0','1']);
  await step('sneak up side mode',['key','shift','up']);
  const mode4=await step('wand side mode',['inventory']);
  assert.equal(mode(mode4),4,'next mode is cycle_capacitor_side');
  await step('approach capacitor',['tp','60','82','44']);
  await step('capacitor look',['look-at','block','60','80','46']);
  await step('capacitor look settle',['wait','4']);
  await step('cycle capacitor side',['command','theurgy_interact','60','80','46','up']);
  nbt=await blockNbt(call,[60,80,46]);
  assert.ok(/UP"?\s*:\s*3|up"?\s*:\s*3/.test(nbt),'capacitor UP output must cycle to BOTH (3): '+nbt);
  return results;
}
export default {name:'wand',runCustom};
