import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {root} from './common.mjs';
import {verifyUpstream} from './upstream.mjs';

const javaRoot=path.join(root,'upstream/theurgy/src/main/java/com/klikli_dev/theurgy');
const generated=path.join(root,'upstream/theurgy/src/generated/resources');
const content=path.join(root,'content/theurgy');
const out=path.join(root,'artifacts/coverage.json');
const slash=p=>p.split(path.sep).join('/');
async function files(dir){const out=[];for(const e of await fsp.readdir(dir,{withFileTypes:true})){const p=path.join(dir,e.name);if(e.isDirectory())out.push(...await files(p));else if(e.isFile())out.push(p);}return out;}
const read=async p=>fsp.readFile(p,'utf8');
const exists=async p=>{try{await fsp.access(p);return true;}catch{return false;}};
const sha=p=>createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const source=await verifyUpstream(root);
const javaFiles=(await files(javaRoot)).filter(f=>f.endsWith('.java'));
const gameTestDir=path.join(javaRoot,'gametest');
const registryPath=path.join(gameTestDir,'GameTestRegistry.java');
const registry=await read(registryPath);
const registrations=[...registry.matchAll(/TEST_FUNCTIONS\.register\("([^"]+)",\s*\(\)\s*->\s*(\w+GameTests)::(\w+)\)/g)]
  .map(m=>({id:m[1],className:m[2],method:m[3]}));
const instanceNames=new Set([...registry.matchAll(/registerTest\(event,\s*(\w+)/g)].map(m=>m[1]));
const constantNames=new Map([...registry.matchAll(/public static final DeferredHolder[^\n]+ (\w+)\s*=\s*\r?\n?\s*TEST_FUNCTIONS\.register\("([^"]+)"/g)].map(m=>[m[2],m[1]]));
const bodies=[];
for(const file of (await files(gameTestDir)).filter(f=>f.endsWith('.java')&&!f.endsWith('GameTestRegistry.java'))){
 const text=await read(file);for(const m of text.matchAll(/public static void\s+(\w+)\s*\(\s*GameTestHelper\s+\w+\s*\)/g))bodies.push({className:path.basename(file,'.java'),method:m[1]});
}
const bodyKeys=new Set(bodies.map(b=>b.className+'::'+b.method));
const tests=registrations.map(r=>({...r,hasInstance:instanceNames.has(constantNames.get(r.id)),hasBody:bodyKeys.has(r.className+'::'+r.method)}));
const registeredBodies=new Set(registrations.map(r=>r.className+'::'+r.method));
const unregistered=bodies.filter(b=>!registeredBodies.has(b.className+'::'+b.method));
for(const b of unregistered)tests.push({id:'unregistered:'+b.method.toLowerCase(),...b,hasInstance:false,hasBody:true,unregistered:true});

const recipeFiles=(await files(path.join(generated,'data/theurgy/recipe'))).filter(f=>f.endsWith('.json'));
const conditionPath=path.join(root,'artifacts/recipe-conditions.json');
const conditionReport=await exists(conditionPath)?JSON.parse(await read(conditionPath)):null;
const conditionallyDisabled=new Set((conditionReport?.rows??[]).filter(r=>!r.accepted).map(r=>r.id));
const recipeTypes={};
const recipeRows=[];
for(const file of recipeFiles){
 const j=JSON.parse(await read(file));const type=j.type??'<missing>';recipeTypes[type]=(recipeTypes[type]??0)+1;
 const relative=slash(path.relative(generated,file));const target=path.join(content,relative);
 recipeRows.push({id:slash(path.relative(path.join(generated,'data/theurgy/recipe'),file)).replace(/\.json$/,''),type,source:relative,
  present:await exists(target),sourceSha256:sha(file),portedSha256:await exists(target)?sha(target):null});
}
const assetFiles=(await files(path.join(generated,'assets'))).filter(f=>fs.statSync(f).isFile());
const assetRows=[];
for(const file of assetFiles){const relative=slash(path.relative(generated,file));const target=path.join(content,relative);assetRows.push({source:relative,present:await exists(target),sameHash:await exists(target)&&sha(file)===sha(target)});}
const featuresPath=path.join(root,'docs/FEATURES.md');
const features=await read(featuresPath);
const featureRows=[];
for(const [index,line] of features.split(/\r?\n/).entries()){
 if(!line.startsWith('|'))continue;
 const cells=line.split(/(?<!\\)\|/).slice(1,-1).map(x=>x.trim());
 if(cells.length===6&&/^[A-Z]+-[0-9]+$/.test(cells[0]))featureRows.push({line:index+1,id:cells[0],behaviour:cells[1],status:cells[2],java:cells[3],rust:cells[4],evidence:cells[5]});
}
const javaTraceResults=(await files(path.join(root,'runs'))).filter(f=>f.endsWith('result.json')&&f.includes('java-oracle-'));
const javaOracle=[];
for(const file of javaTraceResults){try{const j=JSON.parse(await read(file));if(j.traces)javaOracle.push({file:slash(path.relative(root,file)),status:j.status,passed:j.traces.passed,total:j.instrumented?.totalBehaviourTests??151,sha256:j.traces.sha256});}catch{}}
javaOracle.sort((a,b)=>a.file.localeCompare(b.file));
const nativeFiles=(await files(path.join(root,'native/theurgy'))).filter(f=>f.endsWith('.rs'));
const placeholders=[];
for(const file of nativeFiles){const text=await read(file);for(const [i,line] of text.split(/\r?\n/).entries())if(/todo!\s*\(|unimplemented!\s*\(|PLACEHOLDER/i.test(line))placeholders.push({file:slash(path.relative(root,file)),line:i+1,text:line.trim()});}
const missingInstances=tests.filter(t=>!t.unregistered&&!t.hasInstance).map(t=>t.id);
const missingBodies=tests.filter(t=>!t.hasBody).map(t=>t.id);
const report={
 schema:1,generatedAt:new Date().toISOString(),source:{revision:source.revision,tag:source.tag,minecraft:source.minecraft},
 counts:{java:javaFiles.length,registeredGameTests:registrations.length,instantiatedGameTests:registrations.length-missingInstances.length,
  gameTestBodies:bodies.length,totalBehaviourTests:tests.length,recipes:recipeFiles.length,assets:assetFiles.length,featureRows:featureRows.length},
 gameTests:{rows:tests,missingInstances,missingBodies},
 recipes:{types:recipeTypes,rows:recipeRows,available:recipeRows.filter(r=>r.present).length,conditionallyDisabled:[...conditionallyDisabled],missing:recipeRows.filter(r=>!r.present&&!conditionallyDisabled.has(r.id+'.json')).map(r=>r.id),changed:recipeRows.filter(r=>r.present&&r.sourceSha256!==r.portedSha256).map(r=>r.id)},
 assets:{rows:assetRows,missing:assetRows.filter(r=>!r.present).map(r=>r.source),changed:assetRows.filter(r=>r.present&&!r.sameHash).map(r=>r.source)},
 features:{rows:featureRows,byStatus:Object.groupBy(featureRows,r=>r.status)},
 javaOracle,placeholders,
 notes:['MAPPED files and proposals are not parity evidence. GameTest rows require a real Java trace and a linked real Horizon differential result.',
  'Recipe hash changes are audited transformations, not automatic failures: conditions and Horizon custom-type conversions require semantic comparison.',
  'The four declared-but-uninstantiated functions and the unregistered public body are mandatory supplemental coverage.'],
};
await fsp.mkdir(path.dirname(out),{recursive:true});await fsp.writeFile(out,JSON.stringify(report,null,2)+'\n');
const summary={out:slash(path.relative(root,out)),counts:report.counts,missingInstances,missingBodies,missingRecipes:report.recipes.missing.length,
 available:report.recipes.available,conditionallyDisabled:report.recipes.conditionallyDisabled.length,changedRecipes:report.recipes.changed.length,missingAssets:report.assets.missing.length,changedAssets:report.assets.changed.length,
 featureStatuses:Object.fromEntries(Object.entries(report.features.byStatus).map(([k,v])=>[k,v.length])),placeholders:placeholders.length,javaOracle:latest(javaOracle)};
function latest(rows){return rows.at(-1)??null;}
console.log(JSON.stringify(summary,null,2));
if(missingBodies.length||report.recipes.missing.length||report.assets.missing.length||placeholders.length)process.exitCode=1;
