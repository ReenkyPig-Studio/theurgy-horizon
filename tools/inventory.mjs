import fs from 'node:fs/promises';
import path from 'node:path';
import {root} from './common.mjs';
async function walk(dir) {
  const out=[];
  for(const e of await fs.readdir(dir,{withFileTypes:true})) {
    if(e.name === '.git') continue;
    const p=path.join(dir,e.name);
    if(e.isDirectory()) out.push(...await walk(p));
    else if(e.isFile()) out.push(p);
  }
  return out;
}
const files=await walk(path.join(root,'upstream/theurgy/src'));
const counts={java:0,recipes:0,assets:0,gameTests:0};
const recipeTypes={};
for(const p of files) {
  const s=p.replaceAll('\\','/');
  if(s.endsWith('.java')) counts.java++;
  if(s.includes('/gametest/') && s.endsWith('.java')) counts.gameTests++;
  if(s.includes('/assets/')) counts.assets++;
  if(/\/recipe\/.*\.json$/.test(s)) {
    const j=JSON.parse(await fs.readFile(p,'utf8'));
    counts.recipes++;
    recipeTypes[j.type??'unknown']=(recipeTypes[j.type??'unknown']??0)+1;
  }
}
const result={counts,recipeTypes,warning:'Inventory only, not port coverage or Java oracle evidence'};
await fs.mkdir(path.join(root,'runs'),{recursive:true});
await fs.writeFile(path.join(root,'runs/inventory.json'),JSON.stringify(result,null,2));
console.log(JSON.stringify(result,null,2));
