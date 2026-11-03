import path from 'node:path';
import fs from 'node:fs/promises';
import {root,config,run} from './common.mjs';
const cmd=process.argv[2]??'plan';
if(!['plan','stage','check','build','check-headless','build-headless'].includes(cmd))
  throw new Error('plan | stage | check | build | check-headless | build-headless');
const base=cmd.replace('-headless','');
const headless=cmd.endsWith('-headless');
const stage=path.join(root,'target/horizon-stage');
await fs.mkdir(path.join(root,'target'),{recursive:true});
if(cmd==='plan') {
  await run(config.mixins,['plan','--workspace',config.horizonSource,'--mods',path.join(root,'native'),
    '--out',path.join(root,'target/native-plan.json')]);
} else {
  // Mixins writes its own stage, never the launcher's source/build or user instance.
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
