import fs from 'node:fs/promises';
import path from 'node:path';
import {root,run,rustEnv} from './common.mjs';

// Agents write code concurrently, but a shared Cargo target and its final
// horizon-client.exe cannot be replaced while another game is running. This
// lock serialises only the real native build + game section, never research or
// editing. A crashed agent leaves a PID-tagged lock which is safe to reclaim.
const mode=process.argv[2]??'all';
const matrixRoot=process.env.THEURGY_MATRIX_ROOT??root;
const target=process.env.THEURGY_CARGO_TARGET??path.join(matrixRoot,'target/native-agent-cache');
const binary=path.join(target,'fast/horizon-client.exe');
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
try {
  console.log('[agent-harness] locked; building real headless native binary for',mode);
  await run(process.execPath,['tools/native.mjs','build-headless']);
  console.log('[agent-harness] running isolated real game:',mode);
  await run(process.execPath,['tools/game.mjs',mode],{
    env:{...rustEnv,THEURGY_GAME_BIN:binary,THEURGY_GUI:'0'},
  });
} finally {
  await release();
}
