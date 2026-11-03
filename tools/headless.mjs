import path from 'node:path';
import {root,run,rustEnv} from './common.mjs';

// One command for a real, isolated Minecraft integration run with no window,
// renderer or GPU. `--no-build` reuses the last headless binary and content.
const args=process.argv.slice(2);
const mode=args.find(arg=>!arg.startsWith('--'))??'all';
const build=!args.includes('--no-build');
const binary=path.join(root,'target/horizon-headless-cargo/fast/horizon-client.exe');

if(build) {
  await run(process.execPath,['tools/build-content.mjs']);
  await run(process.execPath,['tools/content-check.mjs']);
  await run(process.execPath,['tools/native.mjs','build-headless']);
}
await run(process.execPath,['tools/game.mjs',mode],{
  env:{...rustEnv,THEURGY_GAME_BIN:binary,THEURGY_GUI:'0'},
});
