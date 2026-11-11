import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';
export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const config = JSON.parse(await fs.readFile(path.join(root, 'config.json'), 'utf8').catch(() => {
  throw new Error('config.json is missing: copy config.example.json to config.json and set your local paths');
}));
export const rustEnv = { ...process.env, CARGO_HOME: config.cargoHome, PATH: [
  path.join(config.rust, 'bin'), path.join(config.rust, 'mingw64/bin'),
  process.env.PATH].join(path.delimiter) };
export function run(exe, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(exe, args, { cwd: root, env: rustEnv, stdio: 'inherit', ...options });
    child.on('error', reject);
    child.on('exit', code => code === 0 ? resolve() : reject(new Error(exe + ' exited ' + code)));
  });
}
