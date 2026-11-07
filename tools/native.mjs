import path from 'node:path';
import fs from 'node:fs/promises';
import {root,config,run} from './common.mjs';
const cmd=process.argv[2]??'plan';
const lockToken=process.env.THEURGY_NATIVE_LOCK_TOKEN;
const lockRoot=process.env.THEURGY_NATIVE_LOCK_ROOT??root;
const lock=path.join(lockRoot,'.local/native-harness.lock');
const owner=lockToken??process.pid+':'+crypto.randomUUID();
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function acquire() {
  if(lockToken)return;
  await fs.mkdir(path.dirname(lock),{recursive:true});
  const started=Date.now();
  for(;;) {
    try {
      const handle=await fs.open(lock,'wx');
      await handle.writeFile(owner);
      await handle.close();
      return;
    } catch(error) {
      if(error.code!=='EEXIST')throw error;
      try {
        const old=await fs.readFile(lock,'utf8');
        const pid=Number(old.split(':')[0]);
        // agent-harness already owns the lock when it invokes native.mjs as
        // its direct child. That child is nested, not a competing builder.
        if(pid===process.ppid)return;
        let alive=Number.isInteger(pid)&&pid>0;
        if(alive)try{process.kill(pid,0);}catch{alive=false;}
        if(!alive)await fs.rm(lock,{force:true});
      } catch(error) {if(error.code!=='ENOENT')throw error;}
      if(Date.now()-started>3*60*60*1000)throw new Error('native harness lock timeout');
      await sleep(1000+(process.pid%500));
    }
  }
}
async function release() {
  if(lockToken)return;
  try {if(await fs.readFile(lock,'utf8')===owner)await fs.rm(lock,{force:true});}catch(error){if(error.code!=='ENOENT')throw error;}
}
await acquire();
async function writableTree(dir) {
  let entries;
  try {entries=await fs.readdir(dir,{withFileTypes:true});}catch(error){if(error.code==='ENOENT')return;throw error;}
  for(const entry of entries) {
    const file=path.join(dir,entry.name);
    if(entry.isDirectory()) {
      await writableTree(file);
      await fs.chmod(file,0o777).catch(()=>{});
    } else {
      await fs.chmod(file,0o666).catch(()=>{});
    }
  }
}
try {
if(!['plan','stage','check','build','check-headless','build-headless'].includes(cmd))
  throw new Error('plan | stage | check | build | check-headless | build-headless');
const base=cmd.replace('-headless','');
const headless=cmd.endsWith('-headless');
const stage=path.join(root,'target/horizon-stages',String(process.pid));
await fs.mkdir(path.join(root,'target'),{recursive:true});
if(cmd==='plan') {
  await run(config.mixins,['plan','--workspace',config.horizonSource,'--mods',path.join(root,'native'),
    '--out',path.join(root,'target/native-plan.json')]);
} else {
  // Mixins writes its own stage, never the launcher's source/build or user instance.
  await writableTree(stage);
  await fs.rm(stage,{recursive:true,force:true}); // Windows mixins requires a fresh output directory
  await run(config.mixins,['apply','--workspace',config.horizonSource,'--mods',path.join(root,'native'),'--out',stage]);
  if(base!=='stage') {
    const target=process.env.THEURGY_CARGO_TARGET??path.join(root,headless?'target/horizon-headless-cargo':'target/horizon-cargo');
    const args=[base==='check'?'check':'build','--locked','--offline','--manifest-path',path.join(stage,'Cargo.toml'),
      '--target-dir',target,'-p','horizon-client','--bin','horizon-client','--profile','fast'];
    // Horizon's server/engine feature set: no audio or controller devices.
    // Rendering code remains in the upstream crate; --headless selects the
    // real no-window/no-GPU loop at runtime.
    if(headless)args.push('--no-default-features');
    await run(path.join(config.rust,'bin/cargo.exe'),args);
  }
  console.log('Isolated stage:',stage);
}
} finally {
  await release();
}
