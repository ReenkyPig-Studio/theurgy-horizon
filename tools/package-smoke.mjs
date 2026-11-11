import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {root,rustEnv} from './common.mjs';
import {readZipEntries} from './zip.mjs';

// Build and run the actual packaged native source, not the checkout's native/
// directory. The only game home is an owned, isolated .local directory.
// Two mods: content .hmod (resources only) and native .hnmod (code only).
const manifest=JSON.parse(await fs.readFile(path.join(root,'artifacts/package.json'),'utf8'));
const work=path.join(root,'.local/package-smoke');
await fs.rm(work,{recursive:true,force:true});
await fs.mkdir(work,{recursive:true});
const sha256={};
const entries=[];
for(const kind of ['content','native']) {
  const file=path.join(root,manifest.packages[kind].file);
  sha256[kind]=createHash('sha256').update(await fs.readFile(file)).digest('hex');
  if(sha256[kind]!==manifest.packages[kind].sha256)throw new Error(kind+' package manifest hash mismatch');
  for(const [name,bytes] of readZipEntries(file)) {
    const native=name==='horizon-native.json'||name.startsWith('mixins/');
    if(native!==(kind==='native'))throw new Error(`${name} does not belong in the ${kind} package`);
    entries.push([name,bytes]);
  }
}
const packageFile=manifest.packages.content.file+' + '+manifest.packages.native.file;
let contentFiles=0,nativeFiles=0;
for(const [name,bytes] of entries) {
  if(!bytes||name.includes('..')||name.startsWith('/')||name.includes('\\'))throw new Error('unsafe package entry '+name);
  let destination;
  if(name.startsWith('mixins/theurgy/')) {
    destination=path.join(work,'native/theurgy',name.slice('mixins/theurgy/'.length));
    nativeFiles++;
  } else if(name==='horizon-native.json'||name.startsWith('mixins/')) {
    continue;
  } else {
    destination=path.join(work,'content',name);
    contentFiles++;
  }
  const resolved=path.resolve(destination);
  if(!resolved.startsWith(path.resolve(work)+path.sep))throw new Error('package path escaped owned root');
  await fs.mkdir(path.dirname(resolved),{recursive:true});
  await fs.writeFile(resolved,bytes);
}
if(!contentFiles||!nativeFiles)throw new Error('package is missing content or native source');
const out=path.join(root,'runs/package-smoke-'+new Date().toISOString().replaceAll(':','-'));
await fs.mkdir(out,{recursive:true});
const env={...rustEnv};
async function command(label,args,extra={}) {
  const result=await new Promise((resolve,reject)=>{
    const child=spawn(process.execPath,args,{cwd:root,env:{...env,...extra},stdio:['ignore','pipe','pipe']});
    let output='';
    child.stdout.on('data',data=>{output+=data;process.stdout.write(data);});
    child.stderr.on('data',data=>{output+=data;process.stderr.write(data);});
    child.on('error',reject);
    child.on('exit',(code,signal)=>resolve({code,signal,output}));
  });
  await fs.writeFile(path.join(out,label+'.log'),result.output);
  if(result.code!==0)throw new Error(label+' failed: '+result.code+' '+result.signal);
  return result.output;
}
const nativeRoot=path.join(work,'native');
const cargoTarget=path.join(work,'cargo');
const nativeEnv={THEURGY_NATIVE_ROOT:nativeRoot,THEURGY_CARGO_TARGET:cargoTarget};
async function cleanStages() {
  if(process.env.THEURGY_ACCEPTANCE_CLEAN_CACHES!=='1')return;
  const dir=path.join(root,'target/horizon-stages');
  for(const entry of await fs.readdir(dir,{withFileTypes:true}).catch(()=>[]))
    if(entry.isDirectory())await fs.rm(path.join(dir,entry.name),{recursive:true,force:true});
}
await cleanStages();
await command('native-check',['tools/native.mjs','check-headless'],nativeEnv);
await cleanStages();
await command('native-build',['tools/native.mjs','build-headless'],nativeEnv);
// Several independent behaviours, each in its own isolated home/world, all
// on the binary built from the unpacked package: every apparatus, logistics
// filters, flux, fluids and the sal ammoniac chain, and save/reload.
const scenarios=['all','custom:filters','flux','fluids','persistence'];
const runsByScenario=[];
for(const scenario of scenarios) {
  const label=scenario.replace(/[^a-z0-9]+/gi,'-');
  const output=await command('game-'+label,['tools/game.mjs',scenario],{
    THEURGY_GAME_BIN:path.join(cargoTarget,'fast/horizon-client.exe'),
    THEURGY_GAME_HOME:'.local/package-smoke/game-'+label,
    HORIZON_DEV_MOD:path.join(work,'content'),
    THEURGY_GUI:'0',
  });
  const evidenceAbs=[...output.matchAll(/Evidence:\s*(.+)/g)].at(-1)?.[1]?.trim();
  const evidence=evidenceAbs&&path.relative(root,path.resolve(root,evidenceAbs)).replaceAll('\\','/');
  if(!evidence)throw new Error('package game '+scenario+' did not emit evidence');
  runsByScenario.push({scenario,evidence,home:'.local/package-smoke/game-'+label});
}
const evidence=runsByScenario[0].evidence;
const result={status:'PACKAGE_SMOKE_PASS',package:packageFile,sha256,contentFiles,nativeFiles,
  nativeRoot:path.relative(root,nativeRoot),scenarios:runsByScenario,evidence,log:path.relative(root,out)};
await fs.writeFile(path.join(root,'artifacts/package-smoke.json'),JSON.stringify(result,null,2)+'\n');
console.log('THEURGY_PACKAGE_SMOKE_PASS',JSON.stringify(result));
