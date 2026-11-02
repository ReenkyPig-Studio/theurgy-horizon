import path from 'node:path';
import fs from 'node:fs/promises';
import {root,config,run} from './common.mjs';
const cmd=process.argv[2]??'plan';
if(!['plan','stage','check','build'].includes(cmd))throw new Error('plan | stage | check | build');
const stage=path.join(root,'target/horizon-stage');
await fs.mkdir(path.join(root,'target'),{recursive:true});
if(cmd==='plan') {
  await run(config.mixins,['plan','--workspace',config.horizonSource,'--mods',path.join(root,'native'),
    '--out',path.join(root,'target/native-plan.json')]);
} else {
  // Mixins writes its own stage, never the launcher's source/build or user instance.
  await run(config.mixins,['apply','--workspace',config.horizonSource,'--mods',path.join(root,'native'),'--out',stage]);
  if(cmd!=='stage')await run(path.join(config.rust,'bin/cargo.exe'),[
    cmd==='check'?'check':'build','--locked','--offline','--manifest-path',path.join(stage,'Cargo.toml'),
    '--target-dir',path.join(root,'target/horizon-cargo'),
    '-p','horizon-client','--bin','horizon-client','--profile','fast']);
  console.log('Isolated stage:',stage);
}
