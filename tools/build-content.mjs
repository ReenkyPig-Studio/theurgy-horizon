import fs from 'node:fs/promises';
import path from 'node:path';
import {root} from './common.mjs';
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
const be=new Set(['calcination_oven','pyromantic_brazier','liquefaction_cauldron','distiller','incubator',
 'incubator_mercury_vessel','incubator_salt_vessel','incubator_sulfur_vessel','sal_ammoniac_tank',
 'sal_ammoniac_accumulator','logistics_nexus','mercury_catalyst','mercury_capacitor','caloric_flux_emitter',
 'reformation_source_pedestal','reformation_target_pedestal','reformation_result_pedestal','sulfuric_flux_emitter',
 'mercury_flux_emitter','fermentation_vat','digestion_vat','logistics_item_inserter','logistics_item_extractor',
 'logistics_fluid_inserter','logistics_fluid_extractor','logistics_capability_proxy','logistics_mercury_flux_connector']);
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
 digestion_vat:[bool('open')],
 logistics_item_inserter:[bool('has_filter')],logistics_item_extractor:[bool('has_filter')],
 logistics_fluid_inserter:[bool('has_filter'),bool('waterlogged')],logistics_fluid_extractor:[bool('has_filter'),bool('waterlogged')]
};
const defaults={lit:'false',half:'lower',open:'false',has_output:'false',enabled:'true',has_filter:'false',
 waterlogged:'false',north:'false',south:'false',east:'false',west:'false',up:'false',down:'false',facing:'north'};
const templateFor=id=>{
 if(id==='sal_ammoniac_ore')return 'minecraft:iron_ore';
 if(id==='deepslate_sal_ammoniac_ore')return 'minecraft:deepslate_iron_ore';
 if(id==='sal_ammoniac')return 'minecraft:water';
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
 // Model inventory contents and progress without borrowing vanilla machine state.
 if(['calcination_oven','liquefaction_cauldron','distiller','fermentation_vat','digestion_vat'].includes(id)) {
   const slots=id==='fermentation_vat'||id==='digestion_vat'?4:2;
   await fs.mkdir(path.join(dest,'data/theurgy/horizon/menu'),{recursive:true});
   await fs.writeFile(path.join(dest,'data/theurgy/horizon/menu',id+'.json'),JSON.stringify({slots,data_slots:4,results:[slots-1]},null,2));
 }
 if(['pyromantic_brazier','incubator','incubator_mercury_vessel','incubator_salt_vessel','incubator_sulfur_vessel',
   'reformation_source_pedestal','reformation_target_pedestal','reformation_result_pedestal','mercury_catalyst'].includes(id)) {
   await fs.mkdir(path.join(dest,'data/theurgy/horizon/menu'),{recursive:true});
   await fs.writeFile(path.join(dest,'data/theurgy/horizon/menu',id+'.json'),JSON.stringify({slots:1,data_slots:4,
     results:id==='incubator'||id==='reformation_result_pedestal'?[0]:[]},null,2));
 }
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
for(const id of items) {
 const block=blockSet.has(id);
 if(block)continue;
 const rod=id.includes('divination_rod');
 const bucket=id==='sal_ammoniac_bucket';
 const d={template:bucket?'minecraft:water_bucket':rod?'minecraft:stick':'minecraft:diamond',
   components:{}};
 if(rod){d.max_stack_size=1;d.components['minecraft:max_damage']=1;}
 if(bucket){d.max_stack_size=1;d.crafting_remainder='minecraft:bucket';}
 const p=path.join(dest,'data/theurgy/horizon/item',id+'.json');
 await fs.mkdir(path.dirname(p),{recursive:true});await fs.writeFile(p,JSON.stringify(d,null,2)+'\n');
}
for(const file of await files(path.join(dest,'data/theurgy/recipe'))) {
 if(!file.endsWith('.json'))continue;
 const j=JSON.parse(await fs.readFile(file,'utf8'));
 if(['theurgy:logistics_nexus','theurgy:divination_rod'].includes(j.type)) {
   j.type='minecraft:crafting_shaped';
   await fs.writeFile(file,JSON.stringify(j,null,2)+'\n');
 }
}
await fs.mkdir(path.join(dest,'data/theurgy/horizon/creative_tab'),{recursive:true});
await fs.writeFile(path.join(dest,'data/theurgy/horizon/creative_tab/theurgy.json'),JSON.stringify({
 title:'itemGroup.theurgy',icon:'theurgy:empty_jar_icon',items:items.filter(id=>!id.endsWith('_icon')).map(id=>'theurgy:'+id)
},null,2)+'\n');
console.log(JSON.stringify({blocks:blocks.length,items:items.length,blockEntities:be.size,resources:'main + generated, generated wins'},null,2));
