import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {root,rustEnv} from './common.mjs';
import {verifyUpstream} from './upstream.mjs';

// Real Java/NeoForge GameTestServer baseline. This deliberately does NOT call
// itself a Rust differential pass: traces from the Horizon port are separate.
const source=await verifyUpstream(root);
const stamp=new Date().toISOString().replaceAll(':','-');
const out=path.join(root,'runs/java-oracle-'+stamp);
await fsp.mkdir(out,{recursive:true});
const logPath=path.join(out,'game-test-server.log');
const log=fs.createWriteStream(logPath,{flags:'w'});
const cwd=path.join(root,'upstream/theurgy');
const env={...rustEnv,GRADLE_USER_HOME:path.join(root,'.local/gradle'),CI:'1'};
const follow=process.argv.includes('--follow');
const child=process.platform==='win32'
  ?spawn('cmd.exe',['/d','/s','/c','gradlew.bat runGameTestServer --no-daemon --console=plain'],{cwd,env,windowsHide:true})
  :spawn('./gradlew',['runGameTestServer','--no-daemon','--console=plain'],{cwd,env,windowsHide:true});
let text='';
const data=stream=>stream.on('data',chunk=>{
  const s=String(chunk);text=(text+s).slice(-200000);log.write(s);if(follow)process.stdout.write(s);
});
data(child.stdout);data(child.stderr);
const timer=setTimeout(()=>{
  if(process.platform==='win32')spawn('taskkill.exe',['/pid',String(child.pid),'/T','/F'],{windowsHide:true});
  else child.kill('SIGTERM');
},3600000);
const [code,signal]=await new Promise(resolve=>{
  child.on('error',error=>{log.write(String(error));resolve([1,null]);});
  child.on('exit',(code,signal)=>resolve([code,signal]));
});
clearTimeout(timer);log.end();
const markers=[...text.matchAll(/.*(?:GameTestServer|tests? (?:passed|failed)|required tests).*$/gim)].map(m=>m[0]).slice(-100);
const passed=code===0&&/(all (?:required )?tests (?:have )?passed|all tests passed|game tests? (?:have )?passed)/i.test(text);
const result={status:passed?'JAVA_GAMETESTS_PASS':'JAVA_GAMETESTS_FAIL',kind:'upstream Java baseline, not Rust differential evidence',
  source:{tag:source.tag,revision:source.revision,minecraft:source.minecraft},command:'gradlew runGameTestServer --no-daemon --console=plain',
  exitCode:code,signal,log:logPath,markers};
await fsp.writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2));
console.log(JSON.stringify(result,null,2));
if(!passed)process.exitCode=1;
