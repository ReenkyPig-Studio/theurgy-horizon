import path from 'node:path';
import {root,config,run} from './common.mjs';
await run(path.join(config.rust,'bin/cargo.exe'),['run','--locked','--offline','--manifest-path',
 path.join(config.horizonSource,'Cargo.toml'),'--target-dir',path.join(root,'target/horizon-tools'),
 '-p','cargo-horizon-mod','--','build'],{cwd:path.join(root,'content/theurgy')});
