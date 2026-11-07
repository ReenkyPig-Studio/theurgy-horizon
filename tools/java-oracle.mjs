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
  await fsp.copyFile(path.join(root,'oracle/WorldgenTraceGameTests.java'),path.join(cwd,'src/main/java/com/klikli_dev/theurgy/gametest/WorldgenTraceGameTests.java'));
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
  supplemented=supplemented.replace(
    /(registerTest\(event, LOG_ITEM_EXTRACTOR_TRANSFERS_ITEMS[^\n]*\n)/,
    '        registerTest(event, LOG_ITEM_EXTRACTOR_TRANSFERS_ITEMS, environment, Theurgy.loc("logistics_expanded_test"), 100, 0);\n');
  const additions=(supplemented.match(/registerTest\(event,/g)??[]).length-(transformed.match(/registerTest\(event,/g)??[]).length;
  if(additions!==5)throw new Error('Expected five supplemental Java GameTest instances, got '+additions);
  supplemented=supplemented.replace(
    'public static void onRegisterGameTests(RegisterGameTestsEvent event) {',
    `public static final DeferredHolder<Consumer<GameTestHelper>, Consumer<GameTestHelper>> WORLDGEN_FEATURE_INDEX =
            TEST_FUNCTIONS.register("worldgen_feature_index", () -> TraceGameTests.instrument(WorldgenTraceGameTests::featureIndex));

    public static void onRegisterGameTests(RegisterGameTestsEvent event) {
        registerWorldgenTraceTests(event);`);
  supplemented=supplemented.replace(/\n}\s*$/,
    '\n    private static void registerWorldgenTraceTests(RegisterGameTestsEvent event) {\n        var environment = event.registerEnvironment(Theurgy.loc("worldgen_trace"));\n        registerTest(event, WORLDGEN_FEATURE_INDEX, environment, Theurgy.loc("logistics_test"), 40, 0);\n    }\n}\n');
  await fsp.writeFile(registryPath,supplemented);
  // The unregistered transfer body also names the two chests backwards: an
  // extractor pulls from ITEM_EXTRACTOR_TARGET_POS and pushes into the
  // inserter's ITEM_INSERTER_TARGET_POS. Correct only that fixture wiring in
  // the isolated oracle copy; all transfer assertions and timing stay intact.
  const logisticsPath=path.join(cwd,'src/main/java/com/klikli_dev/theurgy/gametest/LogisticsGameTests.java');
  let logistics=await fsp.readFile(logisticsPath,'utf8');
  logistics=logistics.replaceAll('UUID.randomUUID()','UUID.fromString("12345678-1234-1234-1234-123456789abc")');
  logistics=logistics.replace('helper.assertTrue(uuid.equals(nexus.nexusId()), "Nexus should keep the exact item-provided uuid");',
    'TraceGameTests.recordText(nexus.nexusId().toString());\n            helper.assertTrue(uuid.equals(nexus.nexusId()), "Nexus should keep the exact item-provided uuid");');
  logistics=logistics.replaceAll('var sourceChest = helper.getBlockEntity(ITEM_INSERTER_TARGET_POS, ChestBlockEntity.class);','var sourceChest = helper.getBlockEntity(ITEM_EXTRACTOR_TARGET_POS, ChestBlockEntity.class);')
    .replaceAll('var targetChest = helper.getBlockEntity(ITEM_EXTRACTOR_TARGET_POS, ChestBlockEntity.class);','var targetChest = helper.getBlockEntity(ITEM_INSERTER_TARGET_POS, ChestBlockEntity.class);');
  await fsp.writeFile(logisticsPath,logistics);
  // The three recipe-only tests otherwise end with an empty world snapshot.
  // Record the actual assembled stack as semantic Java oracle state too.
  const divinationPath=path.join(cwd,'src/main/java/com/klikli_dev/theurgy/gametest/DivinationRodGameTests.java');
  let divination=await fsp.readFile(divinationPath,'utf8');
  divination=divination.replaceAll('var output = recipe.value().assemble(input);','var output = recipe.value().assemble(input);\n        TraceGameTests.recordStack(output);');
  if((divination.match(/TraceGameTests\.recordStack/g)??[]).length!==3)throw new Error('Expected three semantic divination stack traces');
  await fsp.writeFile(divinationPath,divination);
  for(const file of ['CaloricFluxEmitterGameTests.java','SulfuricFluxEmitterGameTests.java','MercuryFluxEmitterGameTests.java']) {
    const sourcePath=path.join(cwd,'src/main/java/com/klikli_dev/theurgy/gametest',file);
    let source=await fsp.readFile(sourcePath,'utf8');
    source=source.replace('helper.assertTrue(blockEntity.mercuryFluxHandler.getCapacityAsInt()',
      'TraceGameTests.recordValue(blockEntity.mercuryFluxHandler.getCapacityAsInt());\n            helper.assertTrue(blockEntity.mercuryFluxHandler.getCapacityAsInt()');
    if(!source.includes('TraceGameTests.recordValue'))throw new Error('Missing capacity trace in '+file);
    await fsp.writeFile(sourcePath,source);
  }
  const capacitorPath=path.join(cwd,'src/main/java/com/klikli_dev/theurgy/gametest/MercuryCapacitorGameTests.java');
  let capacitor=await fsp.readFile(capacitorPath,'utf8');
  capacitor=capacitor.replace(/helper\.assertTrue\(\r?\n\s*blockEntity\.mercuryFluxHandler\.getCapacityAsInt\(\) == MercuryCapacitorBlockEntity\.CAPACITY,/,
    'TraceGameTests.recordValue(blockEntity.mercuryFluxHandler.getCapacityAsInt());\n            helper.assertTrue(\n                    blockEntity.mercuryFluxHandler.getCapacityAsInt() == MercuryCapacitorBlockEntity.CAPACITY,');
  if(!capacitor.includes('TraceGameTests.recordValue'))throw new Error('Missing capacitor capacity trace');
  capacitor=capacitor.replace(/        helper\.succeed\(\);\r?\n    \}\r?\n\r?\n    \/\*\*\r?\n     \* Tests that a disabled Mercury Capacitor/,
    '        helper.runAfterDelay(2, helper::succeed);\n    }\n\n    /**\n     * Tests that a disabled Mercury Capacitor');
  await fsp.writeFile(capacitorPath,capacitor);
  // The upstream unregistered test uses x=5,6 in a 5x5x5 structure, so its
  const zlib=await import('node:zlib');
  const structure= zlib.gunzipSync(await fsp.readFile(path.join(cwd,'src/main/resources/data/theurgy/structure/logistics_test.nbt')));
  const sizeTag=Buffer.from([0x09,0x00,0x04,0x73,0x69,0x7a,0x65]);
  const at=structure.indexOf(sizeTag);
  if(at<0)throw new Error('Structure size tag not found');
  structure.writeInt32BE(7,at+sizeTag.length+5);
  structure.writeInt32BE(5,at+sizeTag.length+9);
  structure.writeInt32BE(7,at+sizeTag.length+13);
  await fsp.writeFile(path.join(cwd,'src/main/resources/data/theurgy/structure/logistics_expanded_test.nbt'),zlib.gzipSync(structure));
  instrumented={worktree:cwd,registeredTests:count,supplementalInstances:additions,totalBehaviourTests:152,auxiliaryTests:1,traceSource:path.join(root,'oracle/TraceGameTests.java'),
    fixtureCorrections:['expanded unregistered logistics fixture from 5x5x5 to 7x5x7 without changing assertions','swapped reversed source/target chest variables in the unregistered item-transfer fixture without changing assertions']};
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
  const complete=passed.length===153&&failed.length===0&&code===0&&(Boolean(passMatch)||genericPass);
  traces={path:tracePath,sha256:createHash('sha256').update(raw).digest('hex'),snapshots:rows.length,
    passed:passed.length,failed,ids:passed,complete};
}
const passed=code===0&&(Boolean(passMatch)||genericPass)&&(!trace||traces.complete);
const worldgenMarker=text.match(/THEURGY_WORLDGEN_TRACE=([^;]+);SEED=([^;]+);ORIGIN=([^;]+);COUNT=(\d+)/);
const worldgen=worldgenMarker?{path:worldgenMarker[1],seed:worldgenMarker[2],origin:worldgenMarker[3],oreCount:Number(worldgenMarker[4])}:null;
const result={status:passed?(trace?'JAVA_STATE_ORACLE_PASS':'JAVA_GAMETESTS_PASS'):'JAVA_GAMETESTS_FAIL',
  kind:trace?'real pinned Java per-tick block/BE/item state oracle; not Rust differential evidence':'upstream Java baseline, not Rust differential evidence',
  source:{tag:source.tag,revision:source.revision,minecraft:source.minecraft},command:'gradlew runGameTestServer --no-daemon --console=plain',
  exitCode:code,signal,tests:passMatch?Number(passMatch[1]):null,log:logPath,markers,instrumented,worldgen,traces};
await fsp.writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2));
console.log(JSON.stringify(result,null,2));
if(!passed)process.exitCode=1;
