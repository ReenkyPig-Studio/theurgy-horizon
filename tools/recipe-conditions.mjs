import fs from 'node:fs/promises';
import path from 'node:path';
import {readZipEntries} from './zip.mjs';

const slash=p=>p.split(path.sep).join('/');
async function files(dir){const out=[];for(const e of await fs.readdir(dir,{withFileTypes:true})){const p=path.join(dir,e.name);if(e.isDirectory())out.push(...await files(p));else if(e.isFile())out.push(p);}return out;}
async function exists(file){try{await fs.access(file);return true;}catch{return false;}}

/** Evaluate NeoForge's recipe conditions against the exact merged tag table. */
export async function applyRecipeConditions({dest,sources,neoforgeJar,reportFile,itemAssets=[]}) {
 const knownItems=new Set();
 for(const root of itemAssets)if(await exists(root))for(const file of await files(root)) {
  const match=/^assets\/([^/]+)\/items\/(.+)\.json$/.exec(slash(path.relative(root,file)));
  if(match)knownItems.add(match[1]+':'+match[2]);
 }
 const layers=[];
 for(const source of sources)if(await exists(source))layers.push({kind:'dir',root:source,files:await files(source)});
 if(neoforgeJar)layers.push({kind:'zip',entries:readZipEntries(neoforgeJar,'data/')});
 const definitions=new Map();
 for(const layer of layers) {
  const entries=layer.kind==='dir'
   ?await Promise.all(layer.files.filter(f=>f.includes(`${path.sep}tags${path.sep}item${path.sep}`)&&f.endsWith('.json')).map(async file=>['data/'+slash(path.relative(layer.root,file)),await fs.readFile(file,'utf8')]))
   :[...layer.entries].filter(([name])=>name.includes('/tags/item/')&&name.endsWith('.json'));
  for(const [name,text] of entries) {
   const match=/^data\/([^/]+)\/tags\/item\/(.+)\.json$/.exec(name);if(!match)continue;
   const id=match[1]+':'+match[2];const json=JSON.parse(text);
   if(json.replace===true||!definitions.has(id))definitions.set(id,[]);
   definitions.get(id).push(...(json.values??[]));
  }
 }
 const cache=new Map(),visiting=new Set();
 function tag(id) {
  if(cache.has(id))return cache.get(id);
  if(visiting.has(id))return new Set(); // malformed cycles are empty/fail closed
  visiting.add(id);const values=new Set();
  for(const entry of definitions.get(id)??[]) {
   const value=typeof entry==='string'?entry:entry?.id;
   const required=typeof entry==='string'||entry?.required!==false;
   if(typeof value!=='string')continue;
   if(value.startsWith('#'))for(const item of tag(value.slice(1)))values.add(item);
   else if(knownItems.has(value))values.add(value);
   void required; // absent optional values simply do not expand the set
  }
  visiting.delete(id);cache.set(id,values);return values;
 }
 function condition(value) {
  if(Array.isArray(value))return value.every(condition);
  if(!value||typeof value!=='object')return false;
  switch(value.type) {
   case 'neoforge:true':return true;
   case 'neoforge:false':return false;
   case 'neoforge:not':return !condition(value.value);
   case 'neoforge:and':return (value.values??[]).every(condition);
   case 'neoforge:or':return (value.values??[]).some(condition);
   case 'neoforge:tag_empty':return tag(String(value.tag)).size===0;
   case 'neoforge:tag_not_empty':return tag(String(value.tag)).size>0;
   case 'neoforge:mod_loaded':return value.modid==='minecraft'||value.modid==='theurgy';
   default:return false; // unknown conditions fail closed exactly as NeoForge
  }
 }
 const recipes=(await files(path.join(dest,'data/theurgy/recipe'))).filter(f=>f.endsWith('.json'));
 const rows=[];
 for(const file of recipes) {
  const json=JSON.parse(await fs.readFile(file,'utf8'));const conditions=json['neoforge:conditions'];
  if(!conditions)continue;
  const accepted=condition(conditions);const id=slash(path.relative(path.join(dest,'data/theurgy/recipe'),file));
  rows.push({id,accepted,conditions});
  if(accepted){delete json['neoforge:conditions'];await fs.writeFile(file,JSON.stringify(json,null,2)+'\n');}
  else await fs.rm(file);
 }
 if(reportFile){await fs.mkdir(path.dirname(reportFile),{recursive:true});await fs.writeFile(reportFile,JSON.stringify({schema:1,tagDefinitions:definitions.size,debugDefinition:definitions.get('theurgy:alchemical_sulfurs/animals/abundant'),knownItemCount:knownItems.size,hasSulfurBeef:knownItems.has('theurgy:alchemical_sulfur_beef'),resolvedTags:cache.size,tagSizes:Object.fromEntries([...cache].map(([id,values])=>[id,values.size])),accepted:rows.filter(r=>r.accepted).length,skipped:rows.filter(r=>!r.accepted).length,rows},null,2)+'\n');}
 return {accepted:rows.filter(r=>r.accepted).length,skipped:rows.filter(r=>!r.accepted).length,tagDefinitions:definitions.size};
}
