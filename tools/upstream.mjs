import fs from 'node:fs/promises';
import path from 'node:path';
import {execFile} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const exec=(command,args)=>new Promise((resolve,reject)=>{
  execFile(command,args,{encoding:'utf8'},(error,stdout,stderr)=>{
    if(error)reject(error);else resolve({stdout,stderr});
  });
});
export const upstreamLock=JSON.parse(await fs.readFile(new URL('../upstream.lock.json',import.meta.url),'utf8'));
export function validateProperties(text,lock=upstreamLock) {
  const properties=Object.fromEntries(text.split(/\r?\n/).filter(line=>/^\w+=/.test(line))
    .map(line=>{const i=line.indexOf('=');return [line.slice(0,i),line.slice(i+1).trim()];}));
  for(const [key,value] of [['minecraft_version',lock.minecraft],['neo_version',lock.neoforge],
    ['mod_version',lock.gradleModVersion]]) {
    if(properties[key]!==value)throw new Error('Upstream '+key+' mismatch: '+properties[key]+' != '+value);
  }
}
export async function verifyUpstream(root=fileURLToPath(new URL('../',import.meta.url))) {
  const dir=path.join(root,'upstream/theurgy');
  const {stdout}=await exec('git',['-C',dir,'rev-parse','HEAD']);
  if(stdout.trim()!==upstreamLock.revision)throw new Error('Wrong Theurgy commit. Expected '+upstreamLock.tag+' @ '+upstreamLock.revision);
  await exec('git',['-C',dir,'diff','--quiet','HEAD','--','src','gradle.properties','REUSE.toml']);
  validateProperties(await fs.readFile(path.join(dir,'gradle.properties'),'utf8'));
  return upstreamLock;
}
