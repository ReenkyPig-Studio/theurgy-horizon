import fs from 'node:fs/promises';
import path from 'node:path';
import {root,config} from './common.mjs';
import {applyRecipeConditions} from './recipe-conditions.mjs';
import {bakeGeckolibItems} from './geckolib-items.mjs';
import {bakeDigestionVat} from './digestion-vat.mjs';
import {bakeSalAmmoniac} from './sal-ammoniac.mjs';
import {enrichHermetica} from './hermetica-book.mjs';
import {readZipEntries} from './zip.mjs';
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
name = "Theurgy (Assets)"
version = "1.120.1"
description = "Classical alchemy: replicate and transmute matter. Rust/Horizon port."
authors = ["Kli Kli (original)", "Theurgy Horizon contributors"]
api = "0.2"
side = "both"
capabilities = ["registries"]
`);
await copy(path.join(root,'upstream/theurgy/LICENSES'),path.join(dest,'LICENSES'));
// Upstream REUSE.toml names upstream paths; annotate the derived addon data too.
await fs.writeFile(path.join(dest,'REUSE.toml'),await fs.readFile(path.join(root,'upstream/theurgy/REUSE.toml'),'utf8')+
 ['','[[annotations]]','path = ["assets/theurgy/**", "data/theurgy/**"]','precedence = "aggregate"',
  'SPDX-FileCopyrightText = "2024 klikli-dev"','SPDX-License-Identifier = "CC-BY-4.0"',''].join('\n'));
await fs.copyFile(path.join(root,'THIRD_PARTY_NOTICES.md'),path.join(dest,'THIRD_PARTY_NOTICES.md'));
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
 logistics_connector_node:[['facing',{type:'enum',values:['down','up','north','south','west','east']}]],
 logistics_capability_proxy:[['facing',{type:'enum',values:['down','up','north','south','west','east']}]],
 logistics_capability_probe:[['facing',{type:'enum',values:['down','up','north','south','west','east']}]],
 mercury_catalyst:[bool('enabled')],mercury_capacitor:[bool('enabled')],
 caloric_flux_emitter:[['facing',{type:'enum',values:['down','up','north','south','west','east']}],bool('enabled')],
 sulfuric_flux_emitter:[['facing',{type:'enum',values:['down','up','north','south','west','east']}]],
 mercury_flux_emitter:[['facing',{type:'enum',values:['down','up','north','south','west','east']}],bool('enabled')],
 fermentation_vat:[['facing',{type:'enum',values:['north','south','east','west']}],bool('open'),bool('has_output')],
 digestion_vat:[['facing',{type:'enum',values:['north','south','east','west']}],bool('open')],
 logistics_item_inserter:[['facing',{type:'enum',values:['down','up','north','south','west','east']}],bool('has_filter')],logistics_item_extractor:[['facing',{type:'enum',values:['down','up','north','south','west','east']}],bool('has_filter')],
 logistics_fluid_inserter:[['facing',{type:'enum',values:['down','up','north','south','west','east']}],bool('has_filter'),bool('waterlogged')],logistics_fluid_extractor:[['facing',{type:'enum',values:['down','up','north','south','west','east']}],bool('has_filter'),bool('waterlogged')],
 logistics_mercury_flux_connector:[['facing',{type:'enum',values:['down','up','north','south','west','east']}],bool('has_filter')]
};
const defaults={lit:'false',half:'lower',open:'true',has_output:'false',enabled:'true',has_filter:'false',
 waterlogged:'false',north:'false',south:'false',east:'false',west:'false',up:'false',down:'false',facing:'north'};
const templateFor=id=>{
 if(id==='sal_ammoniac_ore')return 'minecraft:iron_ore';
 if(id==='deepslate_sal_ammoniac_ore')return 'minecraft:deepslate_iron_ore';
 if(id==='sal_ammoniac')return 'minecraft:water';
 if(['calcination_oven','liquefaction_cauldron','distiller'].includes(id))return 'minecraft:furnace';
 // BlockRegistry: every apparatus but the fermentation vat is .noOcclusion().
 // Occlusion comes from the template in Horizon, so a stone template hid the
 // neighbours' faces behind partial blocks (see-through holes into the world).
 if(be.has(id)&&id!=='fermentation_vat')return 'minecraft:glass';
 return 'minecraft:stone';
};
// BlockRegistry .sound(SoundType.X) as a vanilla block with that SoundType;
// unset is Properties.of()'s STONE.
const soundFor={calcination_oven:'copper_block',pyromantic_brazier:'copper_block',liquefaction_cauldron:'copper_block',
 distiller:'copper_block',incubator:'oak_planks',incubator_mercury_vessel:'oak_planks',incubator_salt_vessel:'oak_planks',
 incubator_sulfur_vessel:'oak_planks',sal_ammoniac_accumulator:'copper_block',sal_ammoniac_tank:'copper_block',
 logistics_nexus:'copper_block',mercury_catalyst:'iron_block',mercury_capacitor:'iron_block',caloric_flux_emitter:'iron_block',
 sulfuric_flux_emitter:'iron_block',mercury_flux_emitter:'iron_block',fermentation_vat:'iron_block',digestion_vat:'decorated_pot'};
// getShape of the Java blocks, in block units. Directional shapes follow
// apparatus/DirectionalBlockShape.java and the emitters' SHAPE_LENGTH maps.
const px=b=>b.map(v=>v/16);
const directional=(length,width,height,center=8)=>({property:'facing',values:{
 up:[px([center-width/2,0,center-length/2,center+width/2,height,center+length/2])],
 down:[px([center-width/2,16-height,center-length/2,center+width/2,16,center+length/2])],
 west:[px([16-height,center-width/2,center-length/2,16,center+width/2,center+length/2])],
 east:[px([0,center-width/2,center-length/2,height,center+width/2,center+length/2])],
 north:[px([center-width/2,center-length/2,16-height,center+width/2,center+length/2,16])],
 south:[px([center-width/2,center-length/2,0,center+width/2,center+length/2,height])]}});
const emitter=(l=4)=>({property:'facing',values:{
 east:[px([0,4,4,l,12,12])],west:[px([16-l,4,4,16,12,12])],north:[px([4,4,16-l,12,12,16])],
 south:[px([4,4,0,12,12,l])],up:[px([4,0,4,12,l,12])],down:[px([4,16-l,4,12,16,12])]}});
const halves=(lower,upper)=>({property:'half',values:{lower:lower.map(px),upper:upper.map(px)}});
const FULL=[0,0,0,16,16,16];
const shapesFor={
 calcination_oven:halves([FULL],[[0,0,0,16,5,16]]),
 distiller:halves([FULL],[FULL]),
 liquefaction_cauldron:halves([FULL],[[1,0,1,15,6,15]]),
 incubator:halves([[0,0,0,16,4,16],[2,4,2,14,10,14],[1,10,1,15,16,15]],[[0,12,0,16,14,16],[1,0,1,15,12,15]]),
 digestion_vat:{default:[px([1,0,1,15,16,15])]},
 sal_ammoniac_accumulator:{default:[px([1,0,1,15,13,15])]},
 caloric_flux_emitter:emitter(),mercury_flux_emitter:emitter(),sulfuric_flux_emitter:emitter(),
 logistics_connector_node:directional(6,6,8),
 logistics_capability_probe:directional(2,2,8),
 logistics_item_inserter:directional(2,2,8),logistics_item_extractor:directional(2,2,8),
 logistics_fluid_inserter:directional(2,2,8),logistics_fluid_extractor:directional(2,2,8),
 logistics_mercury_flux_connector:directional(2,2,8),
};
const known={stone:[],iron_ore:[],deepslate_iron_ore:[],water:['level'],chest:['facing','waterlogged'],
 barrel:['facing','open'],furnace:['facing','lit'],glass:[]};
const blockDir=path.join(source,'generated/resources/assets/theurgy/blockstates');
const blocks=(await fs.readdir(blockDir)).filter(f=>f.endsWith('.json')).map(f=>f.slice(0,-5)).sort();
for(const id of blocks) {
 const template=templateFor(id),short=template.split(':')[1];
 const state_properties=Object.fromEntries((props[id]??[]).filter(([n])=>!known[short].includes(n)));
const upFacing=new Set(['caloric_flux_emitter','sulfuric_flux_emitter','mercury_flux_emitter','logistics_connector_node','logistics_capability_probe','logistics_capability_proxy','logistics_item_inserter','logistics_item_extractor','logistics_fluid_inserter','logistics_fluid_extractor','logistics_mercury_flux_connector']);
 const default_state=Object.fromEntries((props[id]??[]).map(([n])=>[n,upFacing.has(id)&&n==='facing'?'up':defaults[n]??'false'])
   .filter(([n])=>state_properties[n]));
 const d={template,item:id!=='sal_ammoniac',properties:{hardness:id.includes('ore')?3:1,
   explosion_resistance:id.includes('ore')?3:1,requires_tool:id.includes('ore')}};
 // Water's LiquidBlock is INVISIBLE (FluidRenderer draws it); Horizon renders
 // only vanilla fluids, so draw sal ammoniac's own per-level block model.
 if(id==='sal_ammoniac')d.properties.render_shape='MODEL';
 if(soundFor[id])d.properties.sounds='minecraft:'+soundFor[id];
 else if(template==='minecraft:glass')d.properties.sounds='minecraft:stone';
 if(shapesFor[id])d.shapes=shapesFor[id];
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
// Default SOURCE_ITEM/SOURCE_TAG components from SulfurRegistry. Besides
// automatic derivative names/icons, DivinationRodRecipe consumes these exact
// components when it links a sulfur-attuned rod to a block or ore tag.
const sulfurSource=path.join(source,'main/java/com/klikli_dev/theurgy/registry/SulfurRegistry.java');
const sulfurJava=await fs.readFile(sulfurSource,'utf8');
const itemTagJava=await fs.readFile(path.join(source,'main/java/com/klikli_dev/theurgy/registry/ItemTagRegistry.java'),'utf8');
const customTags={};
for(const match of itemTagJava.matchAll(/(?:public|private)\s+static\s+final\s+TagKey<Item>\s+(\w+)\s*=\s*tag\("([^"]+)"\)/g))customTags[match[1]]='theurgy:'+match[2];
const conventionTag=token=>{
 if(token==='ItemTags.COALS')return 'minecraft:coals';
 if(token.startsWith('ItemTagRegistry.'))return customTags[token.slice('ItemTagRegistry.'.length)];
 if(!token.startsWith('Tags.Items.'))return null;
 const name=token.slice('Tags.Items.'.length).toLowerCase();
 const prefix=['storage_blocks','raw_materials','ingots','nuggets','gems','dusts','ores'].find(prefix=>name.startsWith(prefix+'_'));
 return prefix?'c:'+prefix+'/'+name.slice(prefix.length+1):'c:'+name;
};
const sulfurComponents={};
for(const match of sulfurJava.matchAll(/DeferredItem<AlchemicalSulfurItem>\s+(\w+)\s*=\s*registerForSource(Item|Tag)\(([^,]+),/g)) {
 const name=match[1].toLowerCase(),kind=match[2],argument=match[3].trim();
 const declaration=sulfurJava.slice(match.index,sulfurJava.indexOf(';',match.index));
 const tier=declaration.match(/AlchemicalDerivativeTier\.(\w+)/)?.[1].toLowerCase();
 if(kind==='Tag') {
  const tag=conventionTag(argument);
  if(tag)sulfurComponents['alchemical_sulfur_'+name]={tag,tier};
 } else if(argument.startsWith('Items.')) {
  sulfurComponents['alchemical_sulfur_'+name]={item:'minecraft:'+argument.slice('Items.'.length).toLowerCase(),tier};
 }
}
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
 const sulfur=sulfurComponents[id];
 if(sulfur?.item)d.components['theurgy:source_item']=sulfur.item;
 if(sulfur?.tag)d.components['theurgy:source_tag']=sulfur.tag;
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
const neoforgeJar=await locateNeoforgeJar();
// NeoForge's ingredient tags are runtime data, not merely recipe-condition
// evidence. Carry the small JSON tag definitions (with attribution) so #c:*
// ingredients such as cobblestones/ores actually resolve in Horizon.
let neoforgeTagFiles=0;
for(const [name,bytes] of readZipEntries(neoforgeJar,'data/')) {
 if(!/^data\/c\/tags\/(?:item|block)\/.+\.json$/.test(name))continue;
 const target=path.join(dest,name);
 try {await fs.access(target);continue;} catch{}
 await fs.mkdir(path.dirname(target),{recursive:true});
 await fs.writeFile(target,bytes);
 neoforgeTagFiles++;
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
 neoforgeJar,
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
// DivinationRodRecipe is dynamic in Java: its result copies SOURCE_ITEM or
// SOURCE_TAG from the sulfur in the grid. Horizon's local crafting menu does
// not dispatch prepare_item_craft, so provide the equivalent authoritative
// per-sulfur shaped recipes instead of relying on a client-side callback.
const oreTag=tag=>{
 if(tag==='minecraft:coals')return 'c:ores/coal';
 const [namespace,path]=tag.split(':');
 for(const prefix of ['ingots','nuggets','raw_materials','dusts','storage_blocks','gems'])
  if(path.startsWith(prefix+'/'))return namespace+':ores/'+path.slice(prefix.length+1);
 if(path.endsWith('/netherite'))return namespace+':ores/netherite_scrap';
 return tag;
};
let attunedRecipes=0;
const attunedFiles={abundant:[],common:[],rare:[],precious:[]};
for(const [id,sulfur] of Object.entries(sulfurComponents)) {
 const tier=sulfur.tier;
 if(!tier)continue;
 const generic=path.join(dest,`data/theurgy/recipe/crafting/shaped/sulfur_attuned_divination_rod_${tier}.json`);
 const template=JSON.parse(await fs.readFile(generic,'utf8'));
 template.key.S='theurgy:'+id;
 if(sulfur.item)template.result.components['theurgy:divination_linked_block']=sulfur.item;
 if(sulfur.tag)template.result.components['theurgy:divination_linked_tag']=oreTag(sulfur.tag);
 const out=path.join(dest,`data/theurgy/recipe/crafting/shaped/sulfur_attuned_divination_rod_${tier}__${id}.json`);
 await fs.writeFile(out,JSON.stringify(template,null,2)+'\n');
 attunedFiles[tier].push(path.relative(dest,out).replaceAll('\\','/'));
 attunedRecipes++;
}
for(const tier of ['abundant','common','rare','precious'])
 await fs.rm(path.join(dest,`data/theurgy/recipe/crafting/shaped/sulfur_attuned_divination_rod_${tier}.json`),{force:true});
await fs.writeFile(path.join(root,'artifacts/attunement-recipes.json'),JSON.stringify({schema:1,
  reason:'DivinationRodRecipe copies sulfur source components; authoritative per-sulfur shaped replacements are required because Horizon does not dispatch the local prepare_item_craft callback.',
  replacements:Object.entries(attunedFiles).map(([tier,files])=>({source:`crafting/shaped/sulfur_attuned_divination_rod_${tier}`,files}))
},null,2)+'\n');
console.log(JSON.stringify({attunedRecipes}));
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
// CreativeModeTabRegistry.displayItems order first; the salt/sulfur/niter
// registries append theirs afterwards via BuildCreativeModeTabContentsEvent.
const tabSource=await fs.readFile(path.join(source,'main/java/com/klikli_dev/theurgy/registry/CreativeModeTabRegistry.java'),'utf8');
const tabOrder=[...tabSource.slice(tabSource.indexOf('displayItems')).matchAll(/ItemRegistry\.([A-Z0-9_]+)\.get\(\)/g)].map(m=>m[1].toLowerCase())
 .map(id=>id==='logistics_connection_node'?'logistics_connector_node':id); // BlockRegistry path differs from the constant
if(tabOrder.length<40||tabOrder.some(id=>!items.includes(id)))throw new Error('creative tab order does not resolve: '+tabOrder.filter(id=>!items.includes(id)));
items.sort((a,b)=>(tabOrder.includes(a)?tabOrder.indexOf(a):1e9)-(tabOrder.includes(b)?tabOrder.indexOf(b):1e9));
await fs.mkdir(path.join(dest,'data/theurgy/horizon/creative_tab'),{recursive:true});
await fs.writeFile(path.join(dest,'data/theurgy/horizon/creative_tab/theurgy.json'),JSON.stringify({
 title:'itemGroup.theurgy',icon:'theurgy:empty_jar_icon',items:items.filter(id=>!id.endsWith('_icon')).map(id=>'theurgy:'+id)
},null,2)+'\n');
// Register every GeckoLib texture in the model atlas. The actual bone
// geometry is submitted by native/theurgy/render.rs; these small cube models
// are only atlas sources used by `native_standalone_models`.
for(const id of ['calcination_oven','distiller','incubator_vessel_iron','incubator_vessel_gold',
 'incubator_vessel_bronze','logistics_nexus','sal_ammoniac_accumulator','sal_ammoniac_tank']) {
 const modelDir=path.join(dest,'assets/theurgy/models/block/geckolib_atlas');
 await fs.mkdir(modelDir,{recursive:true});
 await fs.writeFile(path.join(modelDir,id+'.json',),JSON.stringify({parent:'minecraft:block/cube_all',textures:{all:'theurgy:block/'+id}},null,2)+'\n');
}
// GeckoLib block items: static geometry baked into item-model elements
// (tools/geckolib-items.mjs) so GUIs, hands, frames and the Hermetica draw them.
const geckolibItems=await bakeGeckolibItems(dest);
console.log(JSON.stringify({geckolibItems}));
// DigestionVatRenderer: ModelPart BER + special item (tools/digestion-vat.mjs).
console.log(JSON.stringify({digestionVat:await bakeDigestionVat(dest)}));
// Sal ammoniac LiquidBlock + fluid_container bucket (tools/sal-ammoniac.mjs).
console.log(JSON.stringify({salAmmoniac:await bakeSalAmmoniac(dest)}));
// GeckoLib special item renderers are implemented by the native item
// decoration hook. Give each derivative a valid visible base model instead
// of leaving Horizon's loader on the unknown `theurgy:*` special model type.
for(const id of items) {
 const base=id.startsWith('alchemical_niter_')?'empty_ceramic_jar_icon'
  :id.startsWith('alchemical_sulfur_')?'empty_jar_icon'
  :id.startsWith('alchemical_salt_')?'salt_icon':null;
 if(!base)continue;
 await fs.writeFile(path.join(dest,'assets/theurgy/items',id+'.json'),JSON.stringify({
  model:{type:'minecraft:model',model:'theurgy:item/'+base}
 },null,2)+'\n');
}
// Flatten the complete Modonomicon book into one runtime data file for the
// native Hermetica screen. This preserves every category, entry, page type,
// recipe/link id and image path; no upstream book content is discarded.
const hermeticaRoot=path.join(dest,'data/theurgy/modonomicon/books/the_hermetica');
const readJson=async file=>JSON.parse(await fs.readFile(file,'utf8'));
const hermeticaCategories=[];
for(const file of await files(path.join(hermeticaRoot,'categories'))) {
 if(!file.endsWith('.json'))continue;
 const id=path.basename(file,'.json');
 hermeticaCategories.push({id,...await readJson(file)});
}
hermeticaCategories.sort((a,b)=>(a.sort_number??0)-(b.sort_number??0)||a.id.localeCompare(b.id));
const hermeticaEntries=[];
const hermeticaEntryRoot=path.join(hermeticaRoot,'entries');
for(const file of await files(hermeticaEntryRoot)) {
 const relative=path.relative(hermeticaEntryRoot,file).replaceAll('\\','/');
 if(!file.endsWith('.json')||relative.split('/').length!==2)continue;
 const entry=await readJson(file);
 const pages=[];
 const pageRoot=file.slice(0,-5)+'/pages';
 try {
  for(const pageFile of await files(pageRoot)) {
   if(!pageFile.endsWith('.json'))continue;
   pages.push({id:path.basename(pageFile,'.json'),...await readJson(pageFile)});
  }
 } catch(error) {if(error.code!=='ENOENT')throw error;}
 pages.sort((a,b)=>(a.sort_number??0)-(b.sort_number??0)||a.id.localeCompare(b.id));
 hermeticaEntries.push({...entry,pages});
}
hermeticaEntries.sort((a,b)=>String(a.category).localeCompare(String(b.category))||(a.sort_number??0)-(b.sort_number??0)||String(a.id).localeCompare(String(b.id)));
// Modonomicon's default theme sprites (frame, page backgrounds, node
// connections, buttons) for the native book GUI, as `theurgy:gui/modonomicon/*`.
// Modonomicon 2.19.0 is MIT AND CC-BY-4.0 (klikli-dev), see THIRD_PARTY_NOTICES.md.
const modonomiconJar=path.join(root,'.local/gradle/caches/modules-2/files-2.1/com.klikli_dev/modonomicon-26.3-neoforge/2.19.0/6d1bf159662cc1eae605bbfd21c898639e20700/modonomicon-26.3-neoforge-2.19.0.jar');
const themePrefix='assets/modonomicon/textures/gui/sprites/modonomicon/themes/default/';
let modonomiconSprites=0;
for(const [name,bytes] of readZipEntries(modonomiconJar,themePrefix)) {
 if(!name.endsWith('.png'))continue;
 const out=path.join(dest,'assets/theurgy/textures/gui/modonomicon',name.slice(themePrefix.length));
 await fs.mkdir(path.dirname(out),{recursive:true});await fs.writeFile(out,bytes);modonomiconSprites++;
}
if(modonomiconSprites<50)throw new Error('Modonomicon theme sprites missing from '+modonomiconJar);
// The book font (book.json "font": `modonomicon:default`) as `theurgy:hermetica`:
// Horizon content mods add assets only in their own namespace, so its
// space + bitmap providers are inlined with the textures moved under theurgy.
{
 const fontEntries=readZipEntries(modonomiconJar,'assets/modonomicon/');
 const include=name=>JSON.parse(fontEntries.get(`assets/modonomicon/font/include/${name}.json`)).providers;
 const providers=[...include('space'),...include('default')].map(p=>p.file?{...p,file:p.file.replace('modonomicon:font/','theurgy:font/modonomicon/')}:p);
 for(const p of providers)if(p.file) {
  const name=p.file.slice('theurgy:font/modonomicon/'.length);
  const out=path.join(dest,'assets/theurgy/textures/font/modonomicon',name);
  await fs.mkdir(path.dirname(out),{recursive:true});await fs.writeFile(out,fontEntries.get('assets/modonomicon/textures/font/'+name));
 }
 await fs.mkdir(path.join(dest,'assets/theurgy/font'),{recursive:true});
 await fs.writeFile(path.join(dest,'assets/theurgy/font/hermetica.json'),JSON.stringify({providers},null,1));
}
const modonomiconLang=JSON.parse(readZipEntries(modonomiconJar,'assets/modonomicon/lang/en_us.json').get('assets/modonomicon/lang/en_us.json'));
const hermeticaReport=await enrichHermetica({dest,entries:hermeticaEntries,tagItems:conditionReport.tagItems,
 textureRoots:[dest,path.join(source,'main/resources'),path.join(source,'generated/resources')]});
console.log(JSON.stringify({hermetica:hermeticaReport}));
if(hermeticaReport.missingRecipes.length||hermeticaReport.missingMultiblocks.length||hermeticaReport.missingImages.length)
 throw new Error('Hermetica references unresolved content: '+JSON.stringify(hermeticaReport));
const hermeticaLang=await readJson(path.join(dest,'assets/theurgy/lang/en_us.json'));
// Modonomicon shows an empty-label link as the item's hover name / the
// entry's title; resolve them here (vanilla names from the pinned 26.3 lang).
const vanillaLang=await readJson(path.join(prepared,'assets/minecraft/lang/en_us.json'));
const linkName=(kind,target)=>{
 const [ns,id]=target.split(':');
 if(kind==='item')return hermeticaLang[`item.${ns}.${id}`]??hermeticaLang[`block.${ns}.${id}`]??vanillaLang[`item.${ns}.${id}`]??vanillaLang[`block.${ns}.${id}`];
 const entry=hermeticaEntries.find(e=>e.id===ns+':'+id.split('/').slice(1).join('/'));
 return entry&&hermeticaLang[entry.name];
};
for(const [key,value] of Object.entries(hermeticaLang))
 if(typeof value==='string')hermeticaLang[key]=value.replace(/\[\]\((item|entry):\/\/([^)]+)\)/g,(link,kind,target)=>{
  const name=linkName(kind,target);
  if(!name)throw new Error('Hermetica link without a name: '+link);
  return `[${name}](${kind}://${target})`;
 });
// Modonomicon's GUI strings (bookmarks, search, condition tooltips).
for(const [key,value] of Object.entries(modonomiconLang))if(/^(tooltip\.)?modonomicon\./.test(key))hermeticaLang[key]??=value;
// Every Hermetica research node is entry_viewed_once facts: node -> the
// entries whose first view grants them (ResearchHook trigger_target).
const research={};
{
 const researchRoot=path.join(dest,'data/theurgy/modonomicon/research');
 const nodes=[],factEntry={};
 for(const file of await files(researchRoot)) {
  if(file.endsWith('nodes.json'))nodes.push(...await readJson(file));
  if(file.endsWith('hooks.json'))for(const hook of await readJson(file)) {
   if(hook.trigger_type!=='modonomicon:entry_viewed_once')throw new Error('unsupported research hook '+hook.id);
   factEntry[hook.fact_id]=hook.trigger_target;
  }
 }
 for(const node of nodes)research[node.id]=node.required_facts.map(fact=>{
  if(!factEntry[fact])throw new Error('research fact without a hook: '+fact);
  return factEntry[fact];
 });
}
const hermetica={book:await readJson(path.join(hermeticaRoot,'book.json')),categories:hermeticaCategories,
 entries:hermeticaEntries,research,translations:hermeticaLang};
await fs.writeFile(path.join(dest,'data/theurgy/hermetica.json'),JSON.stringify(hermetica)+'\n');
console.log(JSON.stringify({blocks:blocks.length,items:items.length,blockEntities:be.size,
 hermeticaCategories:hermeticaCategories.length,hermeticaEntries:hermeticaEntries.length,
 hermeticaPages:hermeticaEntries.reduce((n,e)=>n+e.pages.length,0),neoforgeTagFiles,resources:'main + generated, generated wins'},null,2));
