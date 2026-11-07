import fs from 'node:fs/promises';
import path from 'node:path';
import {root,config} from './common.mjs';
import {applyRecipeConditions} from './recipe-conditions.mjs';
const source=path.join(root,'upstream/theurgy/src');
const dest=path.join(root,'content/theurgy');
await fs.rm(dest,{recursive:true,force:true});
await fs.mkdir(dest,{recursive:true});
async function copy(from,to) {
  for(const e of await fs.readdir(from,{withFileTypes:true})) {
    const p=path.join(from,e.name),q=path.join(to,e.name);
    if(e.isDirectory())await copy(p,q);
    else if(e.isFile()){await fs.mkdir(path.dirname(q),{recursive:true});await fs.copyFile(p,q);}
  }
}
// Generated resources intentionally replace handwritten duplicates, as the Java build does.
for(const base of ['main/resources','generated/resources'])for(const dir of ['assets','data'])
  await copy(path.join(source,base,dir),path.join(dest,dir));
await fs.writeFile(path.join(dest,'horizon-mod.toml'),`id = "theurgy"
name = "Theurgy"
version = "1.120.1"
description = "Classical alchemy: replicate and transmute matter. Rust/Horizon port."
authors = ["Kli Kli (original)", "Theurgy Horizon contributors"]
api = "0.2"
side = "both"
capabilities = ["registries"]
`);
await copy(path.join(root,'upstream/theurgy/LICENSES'),path.join(dest,'LICENSES'));
await fs.copyFile(path.join(root,'upstream/theurgy/REUSE.toml'),path.join(dest,'REUSE.toml'));
// Horizon's generator catalog is built before dev-mod configured features are
// visible. Do not leave a dangling placed-feature/biome-modifier reference:
// native/theurgy/worldgen.rs applies this exact registered ore contribution on
// first chunk load instead.
await fs.rm(path.join(dest,'data/theurgy/worldgen'),{recursive:true,force:true});
await fs.rm(path.join(dest,'data/theurgy/neoforge/biome_modifier/add_sal_ammoniac_ore.json'),{force:true});
const be=new Set(['calcination_oven','pyromantic_brazier','liquefaction_cauldron','distiller','incubator',
 'incubator_mercury_vessel','incubator_salt_vessel','incubator_sulfur_vessel','sal_ammoniac_tank',
 'sal_ammoniac_accumulator','logistics_nexus','mercury_catalyst','mercury_capacitor','caloric_flux_emitter',
 'reformation_source_pedestal','reformation_target_pedestal','reformation_result_pedestal','sulfuric_flux_emitter',
 'mercury_flux_emitter','fermentation_vat','digestion_vat','logistics_item_inserter','logistics_item_extractor',
 'logistics_fluid_inserter','logistics_fluid_extractor','logistics_connector_node','logistics_capability_probe',
 'logistics_capability_proxy','logistics_mercury_flux_connector']);
const bool=n=>[n,{type:'boolean'}];
const props={
 calcination_oven:[bool('lit'),['half',{type:'enum',values:['lower','upper']}]],
 pyromantic_brazier:[bool('lit')],
 liquefaction_cauldron:[bool('lit'),['half',{type:'enum',values:['lower','upper']}]],
 distiller:[bool('lit'),['half',{type:'enum',values:['lower','upper']}]],
 incubator:['north','east','south','west'].map(bool).concat([['half',{type:'enum',values:['lower','upper']}]]),
 logistics_nexus:['up','down','north','south','east','west'].map(bool),
 logistics_connector_node:['up','down','north','south','east','west'].map(bool),
 mercury_catalyst:[bool('enabled')],mercury_capacitor:[bool('enabled')],
 caloric_flux_emitter:[['facing',{type:'enum',values:['down','up','north','south','west','east']}],bool('enabled')],
 sulfuric_flux_emitter:[['facing',{type:'enum',values:['down','up','north','south','west','east']}]],
 mercury_flux_emitter:[['facing',{type:'enum',values:['down','up','north','south','west','east']}]],
 fermentation_vat:[['facing',{type:'enum',values:['north','south','east','west']}],bool('open'),bool('has_output')],
 digestion_vat:[['facing',{type:'enum',values:['north','south','east','west']}],bool('open')],
 logistics_item_inserter:[bool('has_filter')],logistics_item_extractor:[bool('has_filter')],
 logistics_fluid_inserter:[bool('has_filter'),bool('waterlogged')],logistics_fluid_extractor:[bool('has_filter'),bool('waterlogged')],
 logistics_mercury_flux_connector:[bool('has_filter')]
};
const defaults={lit:'false',half:'lower',open:'true',has_output:'false',enabled:'true',has_filter:'false',
 waterlogged:'false',north:'false',south:'false',east:'false',west:'false',up:'false',down:'false',facing:'north'};
const templateFor=id=>{
 if(id==='sal_ammoniac_ore')return 'minecraft:iron_ore';
 if(id==='deepslate_sal_ammoniac_ore')return 'minecraft:deepslate_iron_ore';
 if(id==='sal_ammoniac')return 'minecraft:water';
 if(id.startsWith('logistics_')&&id!=='logistics_nexus')return 'minecraft:barrel';
 if(!be.has(id))return 'minecraft:stone';
 if(['fermentation_vat','digestion_vat','caloric_flux_emitter','sulfuric_flux_emitter','mercury_flux_emitter'].includes(id))return 'minecraft:barrel';
 if(['calcination_oven','liquefaction_cauldron','distiller'].includes(id))return 'minecraft:furnace';
 return 'minecraft:chest';
};
const known={stone:[],iron_ore:[],deepslate_iron_ore:[],water:['level'],chest:['facing','waterlogged'],
 barrel:['facing','open'],furnace:['facing','lit']};
const blockDir=path.join(source,'generated/resources/assets/theurgy/blockstates');
const blocks=(await fs.readdir(blockDir)).filter(f=>f.endsWith('.json')).map(f=>f.slice(0,-5)).sort();
for(const id of blocks) {
 const template=templateFor(id),short=template.split(':')[1];
 const state_properties=Object.fromEntries((props[id]??[]).filter(([n])=>!known[short].includes(n)));
 const default_state=Object.fromEntries(Object.entries(props[id]??[]).map(([n])=>[n,defaults[n]??'false'])
   .filter(([n])=>state_properties[n]));
 const d={template,item:id!=='sal_ammoniac',properties:{hardness:id.includes('ore')?3:1,
   explosion_resistance:id.includes('ore')?3:1,requires_tool:id.includes('ore')}};
 if(Object.keys(state_properties).length) {d.state_properties=state_properties;d.default_state=default_state;}
 // Theurgy 26.3 machines have no menu: their Java useItemOn behaviours move
 // items directly. Native interactions live in interactions.rs; do not emit
 // invented Horizon apparatus menus here.
 await fs.mkdir(path.join(dest,'data/theurgy/horizon/block'),{recursive:true});
 await fs.writeFile(path.join(dest,'data/theurgy/horizon/block',id+'.json'),JSON.stringify(d,null,2)+'\n');
}
async function files(dir) {
 const out=[];for(const e of await fs.readdir(dir,{withFileTypes:true})){const p=path.join(dir,e.name);
 if(e.isDirectory())out.push(...await files(p));else if(e.isFile())out.push(p);}return out;
}
const itemRoot=path.join(source,'generated/resources/assets/theurgy/items');
const items=(await files(itemRoot)).filter(f=>f.endsWith('.json')).map(f=>path.relative(itemRoot,f).replaceAll('\\','/').slice(0,-5)).sort();
const blockSet=new Set(blocks);
// DataComponentRegistry defaults for the tool items. The native component
// types are registered by native/theurgy/registry.rs before content loads.
const rodSpecs={
 divination_rod_t1:{tier:'stone',damage:8,attune:true,allowed:'divination_rod_t1_allowed_blocks',disallowed:'divination_rod_t1_disallowed_blocks'},
 divination_rod_t2:{tier:'iron',damage:16,attune:true,allowed:'divination_rod_t2_allowed_blocks',disallowed:'divination_rod_t2_disallowed_blocks'},
 divination_rod_t3:{tier:'diamond',damage:32,attune:true,allowed:'divination_rod_t3_allowed_blocks',disallowed:'divination_rod_t3_disallowed_blocks'},
 divination_rod_t4:{tier:'netherite',damage:128,attune:true,allowed:'divination_rod_t4_allowed_blocks',disallowed:'divination_rod_t4_disallowed_blocks'},
 sulfur_attuned_divination_rod_abundant:{tier:'stone',damage:16,attune:false,allowed:'divination_rod_t1_allowed_blocks',disallowed:'divination_rod_t1_disallowed_blocks'},
 sulfur_attuned_divination_rod_common:{tier:'stone',damage:16,attune:false,allowed:'divination_rod_t1_allowed_blocks',disallowed:'divination_rod_t1_disallowed_blocks'},
 sulfur_attuned_divination_rod_rare:{tier:'stone',damage:32,attune:false,allowed:'divination_rod_t2_allowed_blocks',disallowed:'divination_rod_t2_disallowed_blocks'},
 sulfur_attuned_divination_rod_precious:{tier:'stone',damage:32,attune:false,allowed:'divination_rod_t3_allowed_blocks',disallowed:'divination_rod_t3_disallowed_blocks'},
 amethyst_divination_rod:{tier:'stone',damage:16,attune:false,allowed:'divination_rod_t1_allowed_blocks',disallowed:'divination_rod_t1_disallowed_blocks'},
};
for(const id of items) {
 const block=blockSet.has(id);
 if(block)continue;
 const rod=rodSpecs[id];
 const bucket=id==='sal_ammoniac_bucket';
 const d={template:bucket?'minecraft:water_bucket':rod?'minecraft:stick':id==='mercurial_wand'?'minecraft:stick':'minecraft:diamond',
   components:{}};
 if(rod){
   d.max_stack_size=1;
   d.components['minecraft:max_damage']=rod.damage;
   d.components['theurgy:divination_settings_tier']=rod.tier;
   d.components['theurgy:divination_settings_allowed_blocks_tag']='theurgy:'+rod.allowed;
   d.components['theurgy:divination_settings_disallowed_blocks_tag']='theurgy:'+rod.disallowed;
   d.components['theurgy:divination_settings_range']=96;
   d.components['theurgy:divination_settings_duration']=40;
   d.components['theurgy:divination_settings_max_damage']=rod.damage;
   d.components['theurgy:divination_settings_allow_attuning']=rod.attune;
 }
 if(id==='mercurial_wand'){
   d.max_stack_size=1;
   d.components['theurgy:mercurial_wand_item_mode']='cycle_direction';
   d.components['theurgy:selected_frequency']=0;
 }
 if(bucket){d.max_stack_size=1;d.crafting_remainder='minecraft:bucket';}
 const p=path.join(dest,'data/theurgy/horizon/item',id+'.json');
 await fs.mkdir(path.dirname(p),{recursive:true});await fs.writeFile(p,JSON.stringify(d,null,2)+'\n');
}
async function locateNeoforgeJar() {
 const base=path.join(root,'.local/gradle/caches/modules-2/files-2.1/net.neoforged/neoforge/26.3.0.76');
 for(const file of await files(base))if(file.endsWith('neoforge-26.3.0.76-universal.jar'))return file;
 throw new Error('Pinned NeoForge universal jar not found offline');
}
// Resolve `not(tag_empty)` before Horizon loads recipes. Its loader does not
// know NeoForge tag conditions and would otherwise drop all conditioned files.
// Unknown external-mod tags/items fail closed, while vanilla/Theurgy tags keep
// exactly the recipes available in this installation.
const prepared=path.join(config.horizonVersion,'.local/horizon-client/26.3');
const conditionReport=await applyRecipeConditions({
 dest,
 sources:[path.join(dest,'data'),path.join(prepared,'data')],
 itemAssets:[path.join(source,'main/resources'),path.join(source,'generated/resources'),prepared],
 neoforgeJar:await locateNeoforgeJar(),
 reportFile:path.join(root,'artifacts/recipe-conditions.json'),
});
console.log(JSON.stringify({recipeConditions:conditionReport}));
for(const file of await files(path.join(dest,'data/theurgy/recipe'))) {
 if(!file.endsWith('.json'))continue;
 const j=JSON.parse(await fs.readFile(file,'utf8'));
 if(['theurgy:logistics_nexus','theurgy:divination_rod'].includes(j.type)) {
   j.type='minecraft:crafting_shaped';
   await fs.writeFile(file,JSON.stringify(j,null,2)+'\n');
 }
}
// NeoForge provides these convention tags outside Theurgy's own data. Horizon
// does not ship NeoForge's tag bundle, so carry the small set used by the
// divination-rod Java tags. Do not synthesize unrelated convention tags.
const conventionTags={
 'data/c/tags/block/stones.json':['minecraft:stone','minecraft:granite','minecraft:polished_granite','minecraft:diorite','minecraft:polished_diorite','minecraft:andesite','minecraft:polished_andesite','minecraft:deepslate','minecraft:cobbled_deepslate','minecraft:polished_deepslate','minecraft:tuff','minecraft:polished_tuff','minecraft:calcite','minecraft:dripstone_block','minecraft:basalt','minecraft:smooth_basalt','minecraft:blackstone','minecraft:polished_blackstone'],
 'data/c/tags/block/sandstone/blocks.json':['minecraft:sandstone','minecraft:cut_sandstone','minecraft:chiseled_sandstone','minecraft:smooth_sandstone','minecraft:red_sandstone','minecraft:cut_red_sandstone','minecraft:chiseled_red_sandstone','minecraft:smooth_red_sandstone'],
 'data/c/tags/block/obsidians.json':['minecraft:obsidian','minecraft:crying_obsidian'],
};
for(const [relative,values] of Object.entries(conventionTags)) {
 const target=path.join(dest,relative);
 await fs.mkdir(path.dirname(target),{recursive:true});
 await fs.writeFile(target,JSON.stringify({values},null,2)+'\n');
}
await fs.mkdir(path.join(dest,'data/theurgy/horizon/creative_tab'),{recursive:true});
await fs.writeFile(path.join(dest,'data/theurgy/horizon/creative_tab/theurgy.json'),JSON.stringify({
 title:'itemGroup.theurgy',icon:'theurgy:empty_jar_icon',items:items.filter(id=>!id.endsWith('_icon')).map(id=>'theurgy:'+id)
},null,2)+'\n');
console.log(JSON.stringify({blocks:blocks.length,items:items.length,blockEntities:be.size,resources:'main + generated, generated wins'},null,2));
