import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {root,config} from './common.mjs';

// Compile and execute the pure NBT/algorithm tests embedded in selected glue
// files. These tests use no Game instance. Game-only functions are omitted by
// exact declaration name in the generated TEST WRAPPER; the production file
// is read, never modified, and every retained helper/test is its actual text.
// Full-game evidence remains mandatory: this does not verify native hooks.
const specs={
 worldgen:['chunk_load'],
 selection:['sneaking','right_click','inventory_tick','left_click','placed'],
 interactions:['accepts_input','entity_pos','right_click'],
 items:[
 'upper_state','placed','component_string','component_bool','component_i32',
 'set_held_component','load_wire_endpoint','store_wire_endpoint',
 'clear_wire_endpoint','consume_one_wire','wire_use_on','attune_rod',
 'is_sneaking','wand_mode','block_facing','wand_use_on','mouse_scroll',
 'right_click_block','use_item','use_on','destroyed',
 'acting_position','play_tuning_fork','overlay','held_distance','scan_rod',
 'uuid_from_random','uuid_text','apply_block_item_components','inventory_tick',
 'replace_held','fluid_use_on','state_for_placement','use_duration',
 'release_using','finish_using','tooltip',
 ],
 registry:['tick'],
 logistics:[
 'sync_filter_state','filter_use_on','facing_of','tick','wire_sneaking',
 'load_wire_endpoint','change_wire_component','store_wire_endpoint',
 'change_wire_count','find_wire','add_wire','remove_wire','wire_use_on',
 'wire_network','collect_network','tick_connector',
]};
const name=process.argv[2]??'';
if(!specs[name])throw new Error('module-tests: '+Object.keys(specs).join(' | '));
const source=await fs.readFile(path.join(root,'native/theurgy',name+'.rs'),'utf8');
function stripFunction(source,name) {
 const match=new RegExp(`^(?:pub\\s+)?fn\\s+${name}\\s*\\(`,'m').exec(source);
 if(!match)throw new Error('missing production helper: '+name);
 const open=source.indexOf('{',match.index);if(open<0)throw new Error('malformed '+name);
 let depth=0,quote=false,escape=false,lineComment=false,blockComment=0;
 for(let i=open;i<source.length;i++) {
  const c=source[i],n=source[i+1];
  if(lineComment){if(c==='\n')lineComment=false;continue;}
  if(blockComment){if(c==='/'&&n==='*'){blockComment++;i++;}else if(c==='*'&&n==='/'){blockComment--;i++;}continue;}
  if(quote){if(escape)escape=false;else if(c==='\\')escape=true;else if(c==='"')quote=false;continue;}
  if(c==='/'&&n==='/'){lineComment=true;i++;continue;}
  if(c==='/'&&n==='*'){blockComment=1;i++;continue;}
  if(c==='"'){quote=true;continue;}
  if(c==='{')depth++;else if(c==='}'&&--depth===0)return source.slice(0,match.index)+source.slice(i+1);
 }
 throw new Error('unclosed '+name);
}
let pure=source;for(const fn of specs[name])pure=stripFunction(pure,fn);
const out=path.join(root,'target/module-tests');await fs.mkdir(out,{recursive:true});
const deps=path.join(root,'target/horizon-headless-cargo/fast/deps');
const wrapper=path.join(out,name+'-wrapper.rs');
await fs.writeFile(wrapper,`extern crate horizon_client_lib as horizon_client;\npub mod game {\n #[path = ${JSON.stringify(path.join(root,'native/theurgy/host.rs'))}]\n pub mod theurgy_host;\n pub mod under_test {\n${pure}\n }\n}\n`);
const names=await fs.readdir(deps);
function extern(crate) {
 const hits=names.filter(n=>n.startsWith('lib'+crate+'-')&&n.endsWith('.rlib')).sort();
 if(hits.length!==1)throw new Error('expected one pinned rlib for '+crate+': '+hits.join(','));
 return path.join(deps,hits[0]);
}
const args=['--edition','2024','--test',wrapper,'-L','dependency='+deps];
// Cargo contributes these link-search paths to normal builds. rustc needs
// them too when reusing the already-built Horizon rlibs for this test binary.
for(const dir of await fs.readdir(path.join(root,'target/horizon-headless-cargo/fast/build'))) {
 if(!dir.startsWith('windows_x86_64_gnu-'))continue;
 const output=path.join(root,'target/horizon-headless-cargo/fast/build',dir,'output');
 const text=await fs.readFile(output,'utf8').catch(()=>'');
 for(const m of text.matchAll(/^cargo:rustc-link-search=native=(.+)$/gm))args.push('-L','native='+m[1].trim());
}
for(const crate of ['horizon_client_lib','horizon_core','horizon_world','winit','theurgy_engine'])args.push('--extern',crate+'='+extern(crate));
args.push('-A','dead_code','-o',path.join(out,name+'.exe'));
async function execute(file,args){await new Promise((resolve,reject)=>{const p=spawn(file,args,{cwd:root,stdio:'inherit'});p.on('error',reject);p.on('exit',code=>code===0?resolve():reject(new Error(path.basename(file)+' exited '+code)));});}
await execute(path.join(config.rust,'bin/rustc.exe'),args);
await execute(path.join(out,name+'.exe'),[]);
