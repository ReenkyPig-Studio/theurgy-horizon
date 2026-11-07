import fs from 'node:fs/promises';
import path from 'node:path';
import {root,config} from './common.mjs';
import {applyRecipeConditions} from './recipe-conditions.mjs';
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
 logistics_connector_node:['up','down','north','south','east','west'].map(bool).concat([['facing',{type:'enum',values:['down','up','north','south','west','east']}]]),
 logistics_capability_proxy:[['facing',{type:'enum',values:['down','up','north','south','west','east']}]],
 logistics_capability_probe:[['facing',{type:'enum',values:['down','up','north','south','west','east']}]],
 mercury_catalyst:[bool('enabled')],mercury_capacitor:[bool('enabled')],
 caloric_flux_emitter:[['facing',{type:'enum',values:['down','up','north','south','west','east']}],bool('enabled')],
 sulfuric_flux_emitter:[['facing',{type:'enum',values:['down','up','north','south','west','east']}]],
 mercury_flux_emitter:[['facing',{type:'enum',values:['down','up','north','south','west','east']}]],
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
 if(id.startsWith('logistics_')&&id!=='logistics_nexus')return 'minecraft:stone';
 if(!be.has(id))return 'minecraft:stone';
 if(['fermentation_vat','digestion_vat','caloric_flux_emitter','sulfuric_flux_emitter','mercury_flux_emitter'].includes(id))return 'minecraft:stone';
 if(['calcination_oven','liquefaction_cauldron','distiller'].includes(id))return 'minecraft:furnace';
 return 'minecraft:stone';
};
const known={stone:[],iron_ore:[],deepslate_iron_ore:[],water:['level'],chest:['facing','waterlogged'],
 barrel:['facing','open'],furnace:['facing','lit']};
const blockDir=path.join(source,'generated/resources/assets/theurgy/blockstates');
const blocks=(await fs.readdir(blockDir)).filter(f=>f.endsWith('.json')).map(f=>f.slice(0,-5)).sort();
for(const id of blocks) {
 const template=templateFor(id),short=template.split(':')[1];
 const state_properties=Object.fromEntries((props[id]??[]).filter(([n])=>!known[short].includes(n)));
 const default_state=Object.fromEntries((props[id]??[]).map(([n])=>[n,defaults[n]??'false'])
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
 if(kind==='Tag') {
  const tag=conventionTag(argument);
  if(tag)sulfurComponents['alchemical_sulfur_'+name]={tag};
 } else if(argument.startsWith('Items.')) {
  sulfurComponents['alchemical_sulfur_'+name]={item:'minecraft:'+argument.slice('Items.'.length).toLowerCase()};
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
// Register every GeckoLib texture in the model atlas. The actual bone
// geometry is submitted by native/theurgy/render.rs; these small cube models
// are only atlas sources used by `native_standalone_models`.
for(const id of ['calcination_oven','distiller','incubator_vessel_iron','incubator_vessel_gold',
 'incubator_vessel_bronze','logistics_nexus','sal_ammoniac_accumulator','sal_ammoniac_tank']) {
 const modelDir=path.join(dest,'assets/theurgy/models/block/geckolib_atlas');
 await fs.mkdir(modelDir,{recursive:true});
 await fs.writeFile(path.join(modelDir,id+'.json',),JSON.stringify({parent:'minecraft:block/cube_all',textures:{all:'theurgy:block/'+id}},null,2)+'\n');
}
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
const hermetica={book:await readJson(path.join(hermeticaRoot,'book.json')),categories:hermeticaCategories,
 entries:hermeticaEntries,translations:await readJson(path.join(dest,'assets/theurgy/lang/en_us.json'))};
await fs.writeFile(path.join(dest,'data/theurgy/hermetica.json'),JSON.stringify(hermetica)+'\n');
console.log(JSON.stringify({blocks:blocks.length,items:items.length,blockEntities:be.size,
 hermeticaCategories:hermeticaCategories.length,hermeticaEntries:hermeticaEntries.length,
 hermeticaPages:hermeticaEntries.reduce((n,e)=>n+e.pages.length,0),neoforgeTagFiles,resources:'main + generated, generated wins'},null,2));
