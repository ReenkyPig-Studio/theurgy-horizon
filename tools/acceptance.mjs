import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn,execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {root} from './common.mjs';

// Trusted, sequential acceptance driver. It deliberately does not execute
// candidate proposals: every command below operates on the integrated tree and
// writes a fresh, immutable run/evidence directory.
const args=process.argv.slice(2);
const skipJava=args.includes('--skip-java');
const skipGui=args.includes('--skip-gui');
const started=new Date().toISOString();
const outDir=path.join(root,'runs','acceptance-'+started.replaceAll(':','-'));
await fs.mkdir(outDir,{recursive:true});
const steps=[];
// Evidence hygiene: an accepted run is bound to one committed tree. A dirty
// tree (tracked changes or untracked non-ignored files) fails before any
// step; --allow-dirty exists only for diagnostics and yields PARTIAL.
const allowDirty=args.includes('--allow-dirty');
const git=gitArgs=>execFileSync('git',gitArgs,{cwd:root,encoding:'utf8'}).trim();
// trimEnd only: porcelain's leading status column (" M path") is significant.
const dirtyFiles=()=>execFileSync('git',['status','--porcelain','--untracked-files=all'],{cwd:root,encoding:'utf8'}).trimEnd().split(/\r?\n/).filter(Boolean);
async function trackedTreeHash() {
  // SHA-256 over every tracked path and its current bytes (the working tree,
  // not only the index), so the result names exactly what was tested.
  const hash=createHash('sha256');
  for(const file of git(['ls-files','-z']).split('\0').filter(Boolean).sort()) {
    hash.update(file+'\0');
    hash.update(await fs.readFile(path.join(root,file)).catch(()=>Buffer.from('<deleted>')));
    hash.update('\0');
  }
  return hash.digest('hex');
}
const source={head:git(['rev-parse','HEAD']),tree:git(['rev-parse','HEAD^{tree}']),
  dirty:dirtyFiles(),workingTreeSha256:await trackedTreeHash()};
if(source.dirty.length&&!allowDirty) {
  const final={status:'ACCEPTANCE_FAIL',started,error:'dirty working tree; commit the tested tree first',source};
  await fs.writeFile(path.join(outDir,'result.json'),JSON.stringify(final,null,2));
  console.error('Acceptance requires a clean committed tree:\n'+source.dirty.slice(0,40).join('\n'));
  process.exit(1);
}
const differentialEvidence=[];
const scenarioEvidence={};
let packageSmoke=null;
const differentialModes=[
  'calcination','liquefaction','distiller','vats','brazier','flux','incubator',
  'logistics','pedestals','placements',
];
const guiDifferentialModes=['divination-recipes'];
const customModes=['worldgen','filter-ui','book','recipes','client','render','wire-render','wand','client-features','tooltips'];
const headlessBinary=path.join(root,'target/horizon-headless-cargo/fast/horizon-client.exe');
const guiBinary=path.join(root,'target/horizon-cargo/fast/horizon-client.exe');
const headlessModes=['all','machines','chaos-machines','interactions','items','flux','logistics','tools','divination','config-attunement','filters','content-rules','incubation-chaos','persistence','reformation','fluids','wire-graph','logistics-advanced','filter-matching','attribute-filters','standard-attributes','special-attributes','review-fixes','vat-interactions'];
const customHeadlessModes=new Set(['config-attunement','wire-graph','logistics-advanced','filter-matching','attribute-filters','standard-attributes','special-attributes','review-fixes','vat-interactions']);

async function exists(file){try{await fs.access(file);return true;}catch{return false;}}
async function command(label,commandArgs,env={}) {
  const log=path.join(outDir,label.replaceAll(/[^a-z0-9_-]/gi,'_')+'.log');
  const begin=Date.now();
  const result=await new Promise((resolve,reject)=>{
    const child=spawn(process.execPath,commandArgs,{cwd:root,env:{...process.env,...env},stdio:['ignore','pipe','pipe']});
    let output='';
    child.stdout.on('data',data=>{output+=data;process.stdout.write(data);});
    child.stderr.on('data',data=>{output+=data;process.stderr.write(data);});
    child.on('error',reject);
    child.on('exit',(code,signal)=>resolve({code,signal,output}));
  });
  await fs.writeFile(log,result.output);
  const step={label,command:commandArgs,exitCode:result.code,signal:result.signal,log:path.relative(root,log),durationMs:Date.now()-begin};
  steps.push(step);
  await fs.writeFile(path.join(outDir,'checkpoint.json'),JSON.stringify({started,steps},null,2));
  if(result.code!==0)throw new Error(`${label} failed (${result.code??result.signal}); see ${log}`);
  return result.output;
}
async function cleanNativeStages() {
  if(process.env.THEURGY_ACCEPTANCE_CLEAN_CACHES!=='1')return;
  const dir=path.join(root,'target/horizon-stages');
  for(const entry of await fs.readdir(dir,{withFileTypes:true}).catch(()=>[]))
    if(entry.isDirectory())await fs.rm(path.join(dir,entry.name),{recursive:true,force:true});
}
async function game(mode,env={}) {
  const output=await command('game-'+mode,['tools/agent-harness.mjs',`custom:${mode}`],env);
  const match=[...output.matchAll(/Evidence:\s*(.+)/g)].at(-1);
  if(!match)throw new Error(`No evidence path emitted for custom:${mode}`);
  const evidence=match[1].trim();
  if(mode.startsWith('differential-'))differentialEvidence.push(evidence);
  const featureMode=mode==='differential-divination-recipes'?'divination-recipes':mode;
  if(mode==='worldgen'||mode==='differential-divination-recipes'||customModes.includes(mode))
    scenarioEvidence[featureMode]=path.relative(root,evidence);
  return evidence;
}

try {
  await command('syntax-check',['--check','matrix/swarm.mjs']);
  await command('npm-test',['--test','test/*.test.mjs']);
  await command('rust-fast',['tools/dev.mjs','fast']);
  await command('native-plan',['tools/native.mjs','plan']);
  await cleanNativeStages();
  await command('native-check',['tools/native.mjs','check-headless']);
  await command('audit',['tools/audit-port.mjs']);
  await command('content',['tools/build-content.mjs']);
  await command('content-check',['tools/content-check.mjs']);
  await cleanNativeStages();
  await command('native-build',['tools/native.mjs','build-headless']);
  await command('modules',['tools/module-tests.mjs','logistics']);
  for(const module of ['worldgen','selection','interactions','items','registry','heat','flux','reformation'])
    await command('modules-'+module,['tools/module-tests.mjs',module]);
  for(const mode of headlessModes) {
    const output=await command('headless-'+mode,['tools/headless.mjs',customHeadlessModes.has(mode)?'custom:'+mode:mode,'--no-build']);
    const match=[...output.matchAll(/Evidence:\s*(.+)/g)].at(-1);
    if(!match)throw new Error(`No evidence path emitted for headless ${mode}`);
    scenarioEvidence[mode]=path.relative(root,match[1].trim());
  }
  if(!skipJava)await command('java-oracle',['tools/java-oracle.mjs','--trace']);
  if(!skipJava)await command('recipe-oracle',['tools/recipe-oracle.mjs']);
  for(const mode of differentialModes)await game('differential-'+mode,{THEURGY_CACHE_BINARY:headlessBinary});
  if(!skipGui)await cleanNativeStages();
  if(!skipGui)await command('native-gui-build',['tools/native.mjs','build']);
  if(!skipGui)for(const mode of guiDifferentialModes)await game('differential-'+mode,{THEURGY_CACHE_BINARY:guiBinary});
  const oracle=process.env.THEURGY_WORLDGEN_ORACLE;
  if(!oracle&&!(await exists(path.join(root,'runs/java-oracle-2026-11-07T21-43-52.670Z/java/worldgen.json'))))
    throw new Error('A nonzero Java worldgen oracle is required; set THEURGY_WORLDGEN_ORACLE');
  await game('worldgen',{THEURGY_WORLDGEN_ORACLE:oracle??path.join(root,'runs/java-oracle-2026-11-07T21-43-52.670Z/java/worldgen.json'),THEURGY_WORLD_SEED:'0',THEURGY_CACHE_BINARY:headlessBinary});
  if(!skipGui)for(const mode of customModes.filter(mode=>mode!=='worldgen'))await game(mode,{THEURGY_CACHE_BINARY:guiBinary});
  const multiplayerOutput=await command('multiplayer',['tools/multiplayer.mjs'],{THEURGY_GAME_BIN:headlessBinary});
  const multiplayerEvidence=/evidence=([^\s]+)/.exec(multiplayerOutput)?.[1];
  if(!multiplayerEvidence)throw new Error('No evidence path emitted for multiplayer');
  scenarioEvidence.multiplayer=path.relative(root,multiplayerEvidence);
  const differentialArgs=['tools/differential.mjs'];
  for(const evidence of differentialEvidence)differentialArgs.push('--evidence',evidence);
  await command('differential-report',differentialArgs);
  // Exact fields too; every waiver is re-proved against the Java oracle runs.
  await command('differential-strict',[...differentialArgs,'--strict']);
  await command('package',['tools/package.mjs']);
  if(process.env.THEURGY_ACCEPTANCE_CLEAN_CACHES==='1') {
    await fs.rm(path.join(root,'target/horizon-headless-cargo'),{recursive:true,force:true});
    await fs.rm(path.join(root,'target/horizon-cargo'),{recursive:true,force:true});
  }
  await cleanNativeStages();
  await command('package-smoke',['tools/package-smoke.mjs']);
  packageSmoke=JSON.parse(await fs.readFile(path.join(root,'artifacts/package-smoke.json'),'utf8'));
  if(packageSmoke.status!=='PACKAGE_SMOKE_PASS')throw new Error('Clean package smoke did not pass');
  const report=JSON.parse(await fs.readFile(path.join(root,'artifacts/differential.json'),'utf8'));
  if(!report.summary?.complete||report.summary.missing||report.summary.mismatched)throw new Error('Differential matrix is not complete');
  // Steps may only rewrite generated evidence under artifacts/; anything
  // else changing means the committed tree was not what got tested.
  const changed=dirtyFiles().filter(line=>!source.dirty.includes(line));
  const unexpected=changed.filter(line=>!/^.. artifacts\//.test(line));
  if(unexpected.length)throw new Error('acceptance changed tracked sources: '+unexpected.join(', '));
  source.generatedArtifacts=changed;
  const status=skipJava||skipGui||source.dirty.length?'ACCEPTANCE_PARTIAL':'ACCEPTANCE_PASS';
  if(status==='ACCEPTANCE_PASS') {
    const guiPath=path.join(root,'artifacts/gui-evidence.json');
    const previous=JSON.parse(await fs.readFile(guiPath,'utf8').catch(()=>'{}'));
    await fs.writeFile(guiPath,JSON.stringify({...previous,...scenarioEvidence},null,2)+'\n');
    const offline=Object.fromEntries(steps.map(step=>[step.label,step.log]));
    await fs.writeFile(path.join(root,'artifacts/offline-evidence.json'),JSON.stringify({acceptance:path.relative(root,outDir),steps:offline},null,2)+'\n');
  }
  const final={status,started,finished:new Date().toISOString(),source,steps,differentialEvidence,scenarioEvidence,packageSmoke,report:'artifacts/differential.json'};
  await fs.writeFile(path.join(outDir,'result.json'),JSON.stringify(final,null,2));
  console.log(status==='ACCEPTANCE_PASS'?'THEURGY_ACCEPTANCE_PASS':'THEURGY_ACCEPTANCE_PARTIAL',outDir);
} catch(error) {
  const final={status:'ACCEPTANCE_FAIL',started,finished:new Date().toISOString(),error:String(error),source,steps,differentialEvidence};
  await fs.writeFile(path.join(outDir,'result.json'),JSON.stringify(final,null,2));
  console.error(error.stack??error);
  process.exitCode=1;
}
