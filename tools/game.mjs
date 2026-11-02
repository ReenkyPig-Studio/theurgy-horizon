import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {root,config,rustEnv} from './common.mjs';
import {chaosPlan} from './chaos-plan.mjs';
const mode=process.argv[2]??'smoke';
if(!['smoke','theurgy','chaos'].includes(mode))throw new Error('smoke | theurgy | chaos');
const chaos=mode==='chaos'?chaosPlan():null;
const gameHome=path.join(root,chaos?'.local/chaos':'.local/game');
const out=path.join(root,'runs/game-'+Date.now());
await fs.mkdir(gameHome,{recursive:true});
await fs.mkdir(out,{recursive:true});
const socket=path.join(gameHome,'theurgy-control.json');
const native=mode==='theurgy';
const version=native?config.horizonNativeVersion:config.horizonVersion;
const exe=native?path.join(root,'target/horizon-cargo/fast/horizon-client.exe'):path.join(version,'horizon-client.exe');
await fs.access(exe); // fail before launching if native binary not built
const ctl=path.join(version,'horizon-ctl.exe');
const env={...rustEnv,HORIZON_HOME:gameHome,HORIZON_CONTROL:socket};
const evidence=[];
async function call(args) {
  const result=await new Promise((resolve,reject)=>{
    let stdout='',stderr='';
    const p=spawn(ctl,['--socket',socket,'--timeout','120',...args],{cwd:gameHome,env,windowsHide:true});
    const timer=setTimeout(()=>{p.kill();reject(new Error('ctl timeout: '+args[0]));},360000);
    p.stdout.on('data',s=>{stdout+=s;});
    p.stderr.on('data',s=>{stderr+=s;});
    p.on('error',e=>{clearTimeout(timer);reject(e);});
    p.on('exit',code=>{
      clearTimeout(timer);
      if(code!==0)reject(new Error(args[0]+': '+stdout+stderr));
      else {
        try{resolve(JSON.parse(stdout));}catch{reject(new Error('Non-JSON reply '+stdout));}
      }
    });
  });
  if(result.error)throw new Error(String(result.error));
  evidence.push({command:args[0],args,reply:result});
  return result;
}
let started=false;
try {
  // Never connect to the player's global socket.
  try {await fs.access(socket);throw new Error('Test socket already exists; stop its game or remove stale socket explicitly.');}
  catch(e){if(e.code!=='ENOENT')throw e;}
  await call(['launch','--game',exe,'--world','TheurgyLab','--seed','42','--creative','--wait',
    '--resources',path.join(version,'.local/horizon-client/26.3'),
    '--game-dir',gameHome,'--home',gameHome,'--test-terrain','--render-distance','2']);
  started=true;
  await call(['state']);
  await call(['setblock','0','80','0',native?'theurgy:distiller':'minecraft:stone']);
  const block=await call(['block','0','80','0']);
  const id=native?'theurgy:distiller':'minecraft:stone';
  assert.ok(JSON.stringify(block).includes(id),'expected block identity '+id);
  if(chaos) {
    for(const action of chaos.actions) {
      const reply=await call(action.args);
      if(action.expected)assert.ok(JSON.stringify(reply).includes(action.expected),
        'Chaos seed '+chaos.seed+': '+JSON.stringify(action.args)+' expected '+action.expected);
    }
    console.log('VANILLA_CHAOS_PASS seed='+chaos.seed+' steps='+chaos.steps+' (not Theurgy parity)');
  }
  await call(['save']);
  console.log(native?'THEURGY_REGISTRATION_SMOKE_PASS (not machine parity)':'VANILLA_HORIZON_SMOKE_PASS');
} finally {
  if(started)try{await call(['quit']);}catch(e){evidence.push({cleanupError:e.message});}
  await fs.writeFile(path.join(out,'evidence.json'),JSON.stringify({mode,chaos:chaos?{seed:chaos.seed,steps:chaos.steps}:null,
    scope:native?'distiller block registration only':chaos?'vanilla bounded adversarial block/control stability, NOT machine parity':'vanilla game/control harness only',evidence},null,2));
  console.log('Evidence:',out);
}
