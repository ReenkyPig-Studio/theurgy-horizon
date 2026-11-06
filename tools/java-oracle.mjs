import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import {spawn,execFile} from 'node:child_process';
import {createHash} from 'node:crypto';
import {promisify} from 'node:util';
import {root,rustEnv} from './common.mjs';
import {verifyUpstream} from './upstream.mjs';

const exec=promisify(execFile);
const trace=process.argv.includes('--trace');
const follow=process.argv.includes('--follow');
const source=await verifyUpstream(root);
const stamp=new Date().toISOString().replaceAll(':','-');
const out=path.join(root,'runs/java-oracle-'+stamp);
await fsp.mkdir(out,{recursive:true});
let cwd=path.join(root,'upstream/theurgy');
let instrumented=null;
if(trace) {
  cwd=path.join(root,'target/java-oracle-worktree');
  await exec('git',['-C',path.join(root,'upstream/theurgy'),'worktree','remove','--force',cwd],{cwd:root}).catch(()=>{});
  await fsp.rm(cwd,{recursive:true,force:true});
  await exec('git',['-C',path.join(root,'upstream/theurgy'),'worktree','add','--detach',cwd,source.revision],{cwd:root});
  const target=path.join(cwd,'src/main/java/com/klikli_dev/theurgy/gametest/TraceGameTests.java');
  await fsp.copyFile(path.join(root,'oracle/TraceGameTests.java'),target);
  const registryPath=path.join(cwd,'src/main/java/com/klikli_dev/theurgy/gametest/GameTestRegistry.java');
  const registry=await fsp.readFile(registryPath,'utf8');
  let count=0;
  const transformed=registry.replace(/\(\) -> (\w+GameTests::\w+)/g,(_,method)=>{count++;return `() -> TraceGameTests.instrument(${method})`;});
  if(count!==151)throw new Error('Expected to instrument 151 Java GameTests, got '+count);
  await fsp.writeFile(registryPath,transformed);
  // Upstream declares 151 functions but instantiates only 147 of them. Add the
  // four omitted instances and the one unregistered public test body in the
  // isolated oracle copy, so the Java baseline covers all 152 behaviours.
  let supplemented=transformed;
  supplemented=supplemented.replace(
    /(registerTest\(event, CAPACITOR_DISABLED_DOES_NOT_RECEIVE_FLUX[^\n]*\n)/,
    '$1        registerTest(event, CAPACITOR_NO_PING_PONG, environment, structure, 100, 0);\n        registerTest(event, CAPACITOR_SIDE_MODES, environment, structure, 100, 0);\n');
  supplemented=supplemented.replace(
    /(registerTest\(event, BRAZIER_FUEL_CONSUMED[^\n]*\n)/,
    '$1        registerTest(event, BRAZIER_STACKED_FUEL_SHRINKS, environment, structure, 200, 0);\n        registerTest(event, BRAZIER_FUEL_LEAVES_REMAINDER, environment, structure, 200, 0);\n');
  supplemented=supplemented.replace(
    /(public static final DeferredHolder<Consumer<GameTestHelper>, Consumer<GameTestHelper>> LOG_ITEM_EXTRACTOR_TARGETS_IN_ANY_REGISTRATION_ORDER_REVERSED[\s\S]*?;)/,
    '$1\n    public static final DeferredHolder<Consumer<GameTestHelper>, Consumer<GameTestHelper>> LOG_ITEM_EXTRACTOR_TRANSFERS_ITEMS =\n            TEST_FUNCTIONS.register("logistics_item_extractor_transfers_items", () -> TraceGameTests.instrument(LogisticsGameTests::itemExtractorTransfersItemsFromSourceChestToTargetChest));');
  supplemented=supplemented.replace(
    /(registerTest\(event, LOG_ITEM_EXTRACTOR_TARGETS_IN_ANY_REGISTRATION_ORDER_REVERSED[^\n]*\n)/,
    '$1        registerTest(event, LOG_ITEM_EXTRACTOR_TRANSFERS_ITEMS, environment, structure, 100, 0);\n');
  const additions=(supplemented.match(/registerTest\(event,/g)??[]).length-(transformed.match(/registerTest\(event,/g)??[]).length;
  if(additions!==5)throw new Error('Expected five supplemental Java GameTest instances, got '+additions);
  await fsp.writeFile(registryPath,supplemented);
  instrumented={worktree:cwd,registeredTests:count,supplementalInstances:additions,totalBehaviourTests:152,traceSource:path.join(root,'oracle/TraceGameTests.java')};
}
const logPath=path.join(out,'game-test-server.log');
const log=fs.createWriteStream(logPath,{flags:'w'});
const env={...rustEnv,GRADLE_USER_HOME:path.join(root,'.local/gradle'),CI:'1'};
if(trace)env.JAVA_TOOL_OPTIONS=[process.env.JAVA_TOOL_OPTIONS,'-Dtheurgy.trace.dir='+path.join(out,'java')].filter(Boolean).join(' ');
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
const markers=text.split(/\r?\n/).filter(line=>
  /GAME TESTS COMPLETE|All \d+ required tests passed|tests? failed|BUILD (?:SUCCESSFUL|FAILED)/i.test(line)).slice(-100);
const passMatch=text.match(/All\s+(\d+)\s+required\s+tests\s+passed/i);
const genericPass=/All tests passed/i.test(text);
let traces=null;
if(trace) {
  const tracePath=path.join(out,'java/traces.jsonl');
  const raw=await fsp.readFile(tracePath,'utf8').catch(error=>{throw new Error('Java trace missing: '+error.message);});
  const rows=raw.split(/\r?\n/).filter(Boolean).map((line,index)=>{
    try{return JSON.parse(line);}catch(error){throw new Error('Malformed Java trace line '+(index+1)+': '+error.message);}
  });
  const passed=[...new Set(rows.filter(row=>row.phase==='passed').map(row=>row.id))].sort();
  const failed=[...new Set(rows.filter(row=>row.phase==='failed').map(row=>row.id))].sort();
  const complete=passed.length===152&&failed.length===0&&code===0&&(Boolean(passMatch)||genericPass);
  traces={path:tracePath,sha256:createHash('sha256').update(raw).digest('hex'),snapshots:rows.length,
    passed:passed.length,failed,ids:passed,complete};
}
const passed=code===0&&(Boolean(passMatch)||genericPass)&&(!trace||traces.complete);
const result={status:passed?(trace?'JAVA_STATE_ORACLE_PASS':'JAVA_GAMETESTS_PASS'):'JAVA_GAMETESTS_FAIL',
  kind:trace?'real pinned Java per-tick block/BE/item state oracle; not Rust differential evidence':'upstream Java baseline, not Rust differential evidence',
  source:{tag:source.tag,revision:source.revision,minecraft:source.minecraft},command:'gradlew runGameTestServer --no-daemon --console=plain',
  exitCode:code,signal,tests:passMatch?Number(passMatch[1]):null,log:logPath,markers,instrumented,traces};
await fsp.writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2));
console.log(JSON.stringify(result,null,2));
if(!passed)process.exitCode=1;
