import fs from 'node:fs/promises';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {root} from './common.mjs';

// Compares the recipes RecipeManager actually loaded in the pinned Java 26.3
// oracle (after neoforge:conditions; TraceGameTests "recipes" row) with the
// recipe JSON tools/build-content.mjs ships to Horizon. Only the theurgy
// namespace is Theurgy's; vanilla/NeoForge recipes are not part of the port.
// One documented, deliberate replacement: each Java
// `theurgy:crafting/shaped/sulfur_attuned_divination_rod_<tier>` (one
// theurgy:divination_rod recipe that copies the sulfur's source id from any
// alchemical sulfur of that tier at assemble time) ships as one shaped recipe
// per concrete sulfur, `..._<tier>__<sulfur>`, because Horizon's crafting has
// no custom recipe serializer; artifacts/attunement-recipes.json lists them.
const args=process.argv.slice(2);
const explicit=args.includes('--java')?args[args.indexOf('--java')+1]:null;

async function latestJava() {
  if(explicit)return path.resolve(root,explicit);
  const dirs=(await fs.readdir(path.join(root,'runs'),{withFileTypes:true}))
    .filter(entry=>entry.isDirectory()&&entry.name.startsWith('java-oracle-')).map(entry=>entry.name).sort().reverse();
  for(const name of dirs) {
    const result=JSON.parse(await fs.readFile(path.join(root,'runs',name,'result.json'),'utf8').catch(()=>'{}'));
    if(result.status!=='JAVA_STATE_ORACLE_PASS')continue;
    const raw=await fs.readFile(result.traces.path,'utf8').catch(()=>'');
    if(raw.includes('"phase":"recipes"'))return path.join(root,'runs',name);
  }
  throw new Error('No passing Java oracle with a RecipeManager dump; run node tools/java-oracle.mjs --trace');
}
async function files(dir){const out=[];for(const e of await fs.readdir(dir,{withFileTypes:true}).catch(()=>[])){const p=path.join(dir,e.name);if(e.isDirectory())out.push(...await files(p));else if(e.name.endsWith('.json'))out.push(p);}return out;}

const javaDir=await latestJava();
const javaResult=JSON.parse(await fs.readFile(path.join(javaDir,'result.json'),'utf8'));
const raw=await fs.readFile(javaResult.traces.path,'utf8');
const row=raw.split(/\r?\n/).filter(line=>line.includes('"phase":"recipes"')).map(line=>JSON.parse(line))[0];
const java=new Map();
for(const entry of row.ids) {
  const [id,type]=entry.split('|');
  if(id.startsWith('theurgy:'))java.set(id,type);
}
const recipeRoot=path.join(root,'content/theurgy/data/theurgy/recipe');
const rust=new Map();
const replaced=new Map();
for(const file of await files(recipeRoot)) {
  const id='theurgy:'+path.relative(recipeRoot,file).split(path.sep).join('/').replace(/\.json$/,'');
  const json=JSON.parse(await fs.readFile(file,'utf8'));
  const attuned=/^(theurgy:crafting\/shaped\/sulfur_attuned_divination_rod_(?:abundant|common|rare|precious))__.+$/.exec(id);
  if(attuned) {
    if(!replaced.has(attuned[1]))replaced.set(attuned[1],[]);
    replaced.get(attuned[1]).push(id);
    continue;
  }
  rust.set(id,json.type);
}
const missing=[],extra=[],typeMismatch=[],replacements=[];
for(const [id,type] of java) {
  if(replaced.has(id)) {
    if(type!=='theurgy:divination_rod'&&type!=='minecraft:crafting')typeMismatch.push({id,java:type,rust:'expanded shaped'});
    replacements.push({java:id,javaType:type,rust:replaced.get(id).sort()});
    continue;
  }
  if(!rust.has(id)){missing.push(id);continue;}
}
for(const id of rust.keys())if(!java.has(id))extra.push(id);
for(const id of replaced.keys())if(!java.has(id))extra.push(...replaced.get(id));
const report={schema:1,kind:'pinned Java 26.3 RecipeManager vs shipped Horizon recipe JSON (theurgy namespace)',
  java:{run:path.relative(root,javaDir),traceSha256:createHash('sha256').update(raw).digest('hex'),theurgyRecipes:java.size,allRecipes:row.ids.length},
  rust:{recipes:rust.size,expandedAttunedRods:[...replaced.values()].reduce((sum,list)=>sum+list.length,0)},
  missing,extra,typeMismatch,replacements,
  status:missing.length||extra.length||typeMismatch.length?'RECIPE_ORACLE_MISMATCH':'RECIPE_ORACLE_MATCH'};
await fs.writeFile(path.join(root,'artifacts/recipe-oracle.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({status:report.status,java:report.java.theurgyRecipes,rust:report.rust,missing:missing.length,extra:extra.length,typeMismatch:typeMismatch.length},null,2));
for(const id of [...missing.slice(0,20)])console.error('MISSING',id);
for(const id of [...extra.slice(0,20)])console.error('EXTRA',id);
if(report.status!=='RECIPE_ORACLE_MATCH')process.exitCode=1;
