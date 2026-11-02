import {watch} from 'node:fs';
import path from 'node:path';
import {root,run} from './common.mjs';
let running=false,pending=false,timer;
async function check() {
  if(running){pending=true;return;}
  running=true;
  do {
    pending=false;
    try{await run(process.execPath,['tools/dev.mjs','fast']);}catch(e){console.error(e.message);}
  }while(pending);
  running=false;
}
watch(path.join(root,'native/theurgy/engine/src'),{recursive:true},(_event,file)=>{
  // Never add candidate modules automatically. Cargo compiles only explicit lib.rs modules.
  if(!file?.endsWith('.rs'))return;
  clearTimeout(timer);timer=setTimeout(check,200);
});
console.log('Watching accepted Rust core; Ctrl+C to stop.');
await check();
