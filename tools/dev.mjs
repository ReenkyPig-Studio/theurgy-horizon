import fs from 'node:fs/promises';
import path from 'node:path';
import { root, config, run } from './common.mjs';
const cargo = path.join(config.rust, 'bin/cargo.exe');
const cmd = process.argv[2];
if (cmd === 'doctor') {
  const {verifyUpstream} = await import('./upstream.mjs');
  const source = await verifyUpstream(root);
  console.log('OK upstream:',source.tag,source.revision,'Minecraft',source.minecraft);
  for (const p of [cargo, path.join(config.rust,'mingw64/bin/gcc.exe'),
    path.join(config.horizonSource,'native-hooks.json'),
    path.join(config.horizonVersion,'horizon-ctl.exe'), path.join(root,'upstream/theurgy/gradle.properties')]) {
    try { await fs.access(p); console.log('OK', p); } catch { process.exitCode=1; console.log('MISSING',p); }
  }
  await run(cargo, ['--version']);
} else if (cmd === 'fast' || cmd === 'check') {
  await run(cargo, [cmd === 'fast' ? 'test' : 'check', '--workspace', '--offline']);
} else throw new Error('doctor | fast | check');
