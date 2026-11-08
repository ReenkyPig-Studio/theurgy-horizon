import fs from 'node:fs/promises';
import path from 'node:path';
import {root,run,rustEnv} from './common.mjs';

// Agents write code concurrently, but a shared Cargo target and its final
// horizon-client.exe cannot be replaced while another game is running. This
// lock serialises only the real native build + game section, never research or
// editing. A crashed agent leaves a PID-tagged lock which is safe to reclaim.
const mode=process.argv[2]??'all';
// Wheel input is a real window event in Horizon (`scroll-at` has no headless
// fallback). Wand verification therefore uses the renderer explicitly; every
// other scenario remains headless by default.
const gui=mode==='wand'||mode==='custom:wand'||mode==='custom:render'||mode==='custom:client'||mode==='custom:book'||mode==='custom:filter-ui'||mode==='custom:recipes'||mode==='custom:client-features'||mode==='custom:tooltips'||mode==='custom:differential-divination-recipes'||process.env.THEURGY_GUI==='1';
const matrixRoot=process.env.THEURGY_MATRIX_ROOT??root;
const target=process.env.THEURGY_CARGO_TARGET??path.join(matrixRoot,'target/native-agent-cache');
const binary=path.join(target,'fast/horizon-client.exe');
const cached=process.env.THEURGY_CACHE_BINARY?path.resolve(process.env.THEURGY_CACHE_BINARY):null;
const lock=path.join(matrixRoot,'.local/native-harness.lock');
const token=process.pid+':'+crypto.randomUUID();
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function acquire() {
  await fs.mkdir(path.dirname(lock),{recursive:true});
  const started=Date.now();
  for(;;) {
    try {
      const handle=await fs.open(lock,'wx');
      await handle.writeFile(token);
      await handle.close();
      return;
    } catch(error) {
      if(error.code!=='EEXIST')throw error;
      try {
        const old=await fs.readFile(lock,'utf8');
        const pid=Number(old.split(':')[0]);
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
  try {
    if(await fs.readFile(lock,'utf8')===token)await fs.rm(lock,{force:true});
  } catch(error) {if(error.code!=='ENOENT')throw error;}
}
await acquire();
async function exists(file){if(!file)return false;try{await fs.access(file);return true;}catch{return false;}}
try {
  if(mode==='items'||mode==='wand'||mode==='custom:wire-graph'||mode==='filters'||mode==='custom:book'||mode==='custom:client'||mode==='custom:filter-ui'||mode==='custom:recipes') {
    await run(process.execPath,['tools/build-content.mjs'],{env:{...rustEnv}});
    await run(process.execPath,['tools/content-check.mjs'],{env:{...rustEnv}});
  }
  if(await exists(cached)) {
    console.log('[agent-harness] approved binary cache hit:',cached);
  } else {
  console.log('[agent-harness] locked; checking real native proposal for',mode);
  await run(process.execPath,['tools/native.mjs',gui?'check':'check-headless'],{
    env:{...rustEnv,THEURGY_CARGO_TARGET:target,THEURGY_NATIVE_LOCK_TOKEN:token,THEURGY_NATIVE_LOCK_ROOT:matrixRoot},
  });
  console.log(`[agent-harness] check passed; building ${gui?'GUI':'headless'} native binary`);
  await run(process.execPath,['tools/native.mjs',gui?'build':'build-headless'],{
    env:{...rustEnv,THEURGY_CARGO_TARGET:target,THEURGY_NATIVE_LOCK_TOKEN:token,THEURGY_NATIVE_LOCK_ROOT:matrixRoot},
  });
  if(cached){await fs.mkdir(path.dirname(cached),{recursive:true});await fs.copyFile(binary,cached);console.log('[agent-harness] cached approved binary');}
  }
  console.log('[agent-harness] running isolated real game:',mode);
  await run(process.execPath,['tools/game.mjs',mode],{
    env:{...rustEnv,THEURGY_GAME_BIN:cached??binary,THEURGY_GUI:gui?'1':'0'},
  });
} finally {
  await release();
}
