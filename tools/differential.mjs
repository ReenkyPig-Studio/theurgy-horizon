import fs from 'node:fs/promises';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';
import zlib from 'node:zlib';
import {root} from './common.mjs';

const args=process.argv.slice(2);
const value=(flag,fallback='')=>args.includes(flag)?args[args.indexOf(flag)+1]:fallback;
const evidenceArgs=args.flatMap((arg,index)=>arg==='--evidence'?[args[index+1]]:[]);
if(args.includes('--all')) {
  const runs=(await fs.readdir(path.join(root,'runs'),{withFileTypes:true}))
    .filter(entry=>entry.isDirectory()&&entry.name.startsWith('game-'))
    .map(entry=>entry.name).sort().reverse();
  for(const run of runs)evidenceArgs.push('runs/'+run);
}
const only=value('--only');
const allowPartial=args.includes('--allow-partial');
const strict=args.includes('--strict');

function snbt(text) {
  let index=0;
  const skip=()=>{while(/\s/.test(text[index]??''))index++;};
  const quoted=quote=>{
    let out='';index++;
    while(index<text.length) {
      const ch=text[index++];
      if(ch===quote)return out;
      if(ch==='\\') {
        const next=text[index++];
        out+=next==='n'?'\n':next==='r'?'\r':next==='t'?'\t':next;
      } else out+=ch;
    }
    throw new Error('unterminated SNBT string');
  };
  const token=stop=>{
    skip();const start=index;
    while(index<text.length&&!stop.includes(text[index])&&!/\s/.test(text[index]))index++;
    return text.slice(start,index);
  };
  const scalar=raw=>{
    if(/^[+-]?\d+$/.test(raw))return Number(raw);
    if(/^[+-]?\d+[bs]$/.test(raw))return Number(raw.slice(0,-1));
    if(/^[+-]?\d+[lL]$/.test(raw)) {
      const value=BigInt(raw.slice(0,-1));
      return value<=BigInt(Number.MAX_SAFE_INTEGER)&&value>=BigInt(Number.MIN_SAFE_INTEGER)?Number(value):value;
    }
    if(/^[+-]?(?:\d+\.\d*|\d*\.\d+|\d+)(?:[eE][+-]?\d+)?[fFdD]$/.test(raw))return Number(raw.slice(0,-1));
    if(/^[+-]?(?:\d+\.\d*|\d*\.\d+|\d+)(?:[eE][+-]?\d+)?$/.test(raw))return Number(raw);
    return raw;
  };
  const parse=()=>{
    skip();const ch=text[index];
    if(ch==='"'||ch==="'")return quoted(ch);
    if(ch==='{') {
      index++;const out={};skip();
      while(index<text.length&&text[index]!=='}') {
        skip();const key=text[index]==='"'||text[index]==="'"?quoted(text[index]):token(':');
        skip();if(text[index]!==':')throw new Error('SNBT compound colon expected at '+index);
        index++;out[key]=parse();skip();
        if(text[index]===','){index++;continue;}
        if(text[index]!=='}')throw new Error('SNBT compound comma expected at '+index);
      }
      index++;return out;
    }
    if(ch==='[') {
      index++;skip();
      const arrayStart=index;
      const first=token([',',']',';']);
      skip();
      if(/^[BIL];$/.test(first)) {
        index=arrayStart+2;const values=[];skip();
        while(index<text.length&&text[index]!==']') {
          values.push(scalar(token([',',']'])));skip();
          if(text[index]===','){index++;continue;}
          if(text[index]!==']')throw new Error('SNBT typed array comma expected at '+index);
        }
        index++;return {type:first[0]==='I'?'IntArray':first[0]==='L'?'LongArray':'ByteArray',values};
      }
      index=arrayStart;const out=[];skip();
      while(index<text.length&&text[index]!==']') {
        out.push(parse());skip();
        if(text[index]===','){index++;continue;}
        if(text[index]!==']')throw new Error('SNBT list comma expected at '+index+' near '+text.slice(index,index+30));
      }
      index++;return out;
    }
    return scalar(token([',',']','}']));
  };
  const result=parse();skip();
  if(index!==text.length)throw new Error('trailing SNBT at '+index+' near '+text.slice(index,index+40));
  return result;
}

function stack(record,slot=0) {
  if(!record||typeof record!=='object'||Array.isArray(record))return null;
  const id=record.id??record.item;
  if(typeof id!=='string'||!id||id==='minecraft:air')return null;
  const count=Number(record.count??1);
  if(!Number.isFinite(count)||count<=0)return null;
  return {Slot:Number(record.Slot??slot),id,count,components:record.components??{}};
}
function attachedTargetLong(probe) {
  const pos=probe.state?.pos??probe.local;
  if(!Array.isArray(pos))return null;
  const offset={north:[0,0,1],south:[0,0,-1],east:[-1,0,0],west:[1,0,0],up:[0,-1,0],down:[0,1,0]}[probe.state?.properties?.facing];
  if(!offset)return null;
  const [x,y,z]=pos.map((value,index)=>Number(value)+offset[index]);
  return BigInt.asIntN(64,(BigInt(x)&0x3ffffffn)<<38n|(BigInt(z)&0x3ffffffn)<<12n|(BigInt(y)&0xfffn));
}
const attachedPlacementTargets=new Set([
  'theurgy:logistics_extractor_placement','theurgy:logistics_inserter_placement',
  'theurgy:mercury_flux_connector_placement','theurgy:mercury_flux_connector_buffer_empty',
]);

function stacksFromNbt(nbt) {
  if(!nbt||typeof nbt!=='object')return [];
  if(Array.isArray(nbt.Items))return nbt.Items.map((entry,index)=>stack(entry,index)).filter(Boolean);
  const out=[];
  const inventory=(value,offset=0)=>{
    if(!value||typeof value!=='object')return 0;
    const stacks=Array.isArray(value.stacks)?value.stacks:[];
    stacks.forEach((entry,index)=>{const item=stack(entry,offset+index);if(item)out.push(item);});
    return stacks.length;
  };
  if(nbt.inputInventory||nbt.outputInventory) {
    const inputs=inventory(nbt.inputInventory,0);
    inventory(nbt.outputInventory,inputs);
  } else if(nbt.inventory)inventory(nbt.inventory,0);
  return out;
}
function canonicalStacks(stacks) {
  return stacks.map((entry,index)=>{
    const normalized=stack(entry,Number(entry?.Slot??index))??entry;
    return {...normalized,Slot:Number(normalized.Slot??index),components:normalized.components??{}};
  }).sort((a,b)=>a.Slot-b.Slot||a.id.localeCompare(b.id));
}
function fluidsFromNbt(nbt) {
  if(!nbt||typeof nbt!=='object')return [];
  const out=[];
  if(Array.isArray(nbt.fluids))for(const tank of nbt.fluids) {
    const id=tank?.Fluid??tank?.id;
    const amount=Number(tank?.Amount??tank?.amount??0);
    if(typeof id==='string'&&id!=='minecraft:empty'&&amount>0)out.push({id,amount});
  }
  for(const key of ['tank','waterTank','solventTank','fluidTank']) {
    const outer=nbt[key];
    const fluid=outer?.Fluid??outer?.fluid??outer;
    const id=fluid?.id??fluid?.Fluid;
    const amount=Number(fluid?.amount??fluid?.Amount??0);
    if(typeof id==='string'&&id!=='minecraft:empty'&&amount>0)out.push({id,amount});
  }
  return out.sort((a,b)=>a.id.localeCompare(b.id));
}
function subset(expected,actual,path='',differences=[]) {
  if(expected===undefined)return differences;
  if(expected===null||typeof expected!=='object') {
    if(String(expected)!==String(actual))differences.push(`${path}: expected ${expected}, got ${actual}`);
    return differences;
  }
  if(Array.isArray(expected)) {
    if(JSON.stringify(expected)!==JSON.stringify(actual))differences.push(`${path}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    return differences;
  }
  if(expected.type&&Array.isArray(expected.values)) {
    return subset(expected.values,actual?.values??actual,path,differences);
  }
  if(actual===null||typeof actual!=='object'||Array.isArray(actual)) {
    differences.push(`${path}: expected object, got ${JSON.stringify(actual)}`);
    return differences;
  }
  for(const key of Object.keys(expected)) {
    if(['x','y','z','id','components'].includes(key))continue;
    if(actual[key]===undefined){differences.push(`${path}.${key}: missing`);continue;}
    subset(expected[key],actual[key],`${path}.${key}`,differences);
  }
  return differences;
}

function componentSubset(expected, actual, path, differences) {
  if(expected===null||expected===undefined)return;
  if(typeof expected!=='object') {
    if(String(expected)!==String(actual))differences.push(`${path}: expected ${expected}, got ${actual}`);
    return;
  }
  if(Array.isArray(expected)) {
    if(JSON.stringify(expected)!==JSON.stringify(actual))differences.push(`${path}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    return;
  }
  if(actual===null||typeof actual!=='object'||Array.isArray(actual)) {
    differences.push(`${path}: expected object, got ${JSON.stringify(actual)}`);
    return;
  }
  for(const [key,value] of Object.entries(expected)) {
    if(actual[key]===undefined){differences.push(`${path}.${key}: missing`);continue;}
    componentSubset(value,actual[key],`${path}.${key}`,differences);
  }
}

const stable= id=>strict||/placement$|blocked_output|starts_empty|buffer_empty|persistence$|no_ping_pong|disabled_does_not_receive|exposes_capability|side_modes/.test(id);
const processingIndependent=id=>!strict&&/placement|_be_only|_break_|_drops_|_insert_|_extract_/.test(id);
function compareBlock(expected,probe,id) {
  const differences=[];
  if(expected.id!==probe.state.block)differences.push(`block: expected ${expected.id}, got ${probe.state.block}`);
  for(const [key,value] of Object.entries(expected.properties??{})) {
    if(!strict&&key==='lit'&&!/lit|placement|stops_without_heat|stops_heat/.test(id))continue;
    const actual=probe.state.properties?.[key];
    if(String(actual).toLowerCase()!==String(value).toLowerCase())differences.push(`property ${key}: expected ${value}, got ${actual}`);
  }
  const nbt=expected.nbt?snbt(expected.nbt):{};
  const expectedStacks=canonicalStacks(stacksFromNbt(nbt));
  const actualStacks=canonicalStacks(probe.state.items??[]);
  if(JSON.stringify(expectedStacks)!==JSON.stringify(actualStacks))
    differences.push(`items: expected ${JSON.stringify(expectedStacks)}, got ${JSON.stringify(actualStacks)}`);
  const expectedFluids=fluidsFromNbt(nbt),actualFluids=fluidsFromNbt(probe.state.nbt);
  if(JSON.stringify(expectedFluids)!==JSON.stringify(actualFluids))
    differences.push(`fluids: expected ${JSON.stringify(expectedFluids)}, got ${JSON.stringify(actualFluids)}`);
  if(stable(id)) {
    for(const key of ['totalTime','mercuryFlux','currentMercuryFluxPerTick','totalMercuryFluxToConvert'])
      if(nbt[key]!==undefined)subset(nbt[key],probe.state.nbt?.[key],key,differences);
    if((strict||id.includes('persistence'))&&nbt.progress!==undefined)subset(nbt.progress,probe.state.nbt?.progress,'progress',differences);
    if(nbt.mercuryFluxHandler)subset(nbt.mercuryFluxHandler,probe.state.nbt?.mercuryFluxHandler??{energy:0},'mercuryFluxHandler',differences);
    if(nbt.buffer)subset(nbt.buffer,probe.state.nbt?.buffer??{energy:0},'buffer',differences);
    if(nbt.energyBuffer)subset(nbt.energyBuffer,probe.state.nbt?.energyBuffer??{energy:0},'energyBuffer',differences);
  } else {
    for(const key of ['mercuryFluxHandler','buffer','energyBuffer']) {
      const expectedEnergy=Number(nbt[key]?.energy??0);
      const actualEnergy=Number(probe.state.nbt?.[key]?.energy??0);
      if(expectedEnergy>0&&actualEnergy<=0)differences.push(`${key}.energy: expected positive flux, got ${actualEnergy}`);
    }
  }
  if(nbt.sideModes)subset(nbt.sideModes,probe.state.nbt?.sideModes,'sideModes',differences);
  if(Array.isArray(nbt.targets)) {
    const actualTargets=probe.state.nbt?.targets??[];
    const attachedOnly=nbt.targets.length===0&&attachedPlacementTargets.has(id)
      &&Array.isArray(actualTargets)&&actualTargets.length===1
      &&attachedTargetLong(probe)!==null
      &&BigInt.asIntN(64,BigInt(actualTargets[0]))===attachedTargetLong(probe);
    // Java's tick-0 placement snapshot predates onLoad. onLoad then adds the
    // attached BlockPos unconditionally (see *ConnectorBlockEntity::onLoad).
    if(!attachedOnly&&(!Array.isArray(actualTargets)||actualTargets.length!==nbt.targets.length))
      differences.push(`targets: expected ${nbt.targets.length}, got ${actualTargets?.length??'missing'}`);
  }
  for(const key of ['selectedPoints','sourcePedestals']) {
    if(!Array.isArray(nbt[key]))continue;
    const expectedPoints=nbt[key],actualPoints=probe.state.nbt?.[key]??[];
    if(!Array.isArray(actualPoints)||actualPoints.length!==expectedPoints.length) {
      differences.push(`${key}: expected ${expectedPoints.length} points, got ${actualPoints?.length??'missing'}`);
      continue;
    }
    for(let index=0;index<expectedPoints.length;index++) {
      const expectedType=expectedPoints[index]?.type,actualType=actualPoints[index]?.type;
      if(expectedType!==undefined&&expectedType!==actualType)
        differences.push(`${key}[${index}].type: expected ${expectedType}, got ${actualType}`);
    }
  }
  for(const key of ['targetPedestal','resultPedestal']) {
    if(nbt[key]?.type!==undefined&&probe.state.nbt?.[key]?.type!==nbt[key].type)
      differences.push(`${key}.type: expected ${nbt[key].type}, got ${probe.state.nbt?.[key]?.type}`);
  }
  if(nbt.isProcessing!==undefined&&!processingIndependent(id)) {
    const expectedProcessing=Number(nbt.isProcessing)!==0;
    const actualProcessing=Number(probe.state.data?.[4]??0)!==0;
    if(expectedProcessing!==actualProcessing)differences.push(`isProcessing: expected ${expectedProcessing}, got ${actualProcessing}`);
  }
  return differences;
}

async function latestJava() {
  const explicit=value('--java');
  if(explicit)return path.resolve(root,explicit);
  const dirs=(await fs.readdir(path.join(root,'runs'),{withFileTypes:true}))
    .filter(entry=>entry.isDirectory()&&entry.name.startsWith('java-oracle-'))
    .map(entry=>entry.name).sort().reverse();
  for(const name of dirs) {
    const dir=path.join(root,'runs',name);
    const result=JSON.parse(await fs.readFile(path.join(dir,'result.json'),'utf8').catch(()=>'{}'));
    if(result.status==='JAVA_STATE_ORACLE_PASS')return dir;
  }
  throw new Error('No passing traced Java oracle found');
}

const javaDir=await latestJava();
const javaResult=JSON.parse(await fs.readFile(path.join(javaDir,'result.json'),'utf8'));
const traceRaw=await fs.readFile(javaResult.traces.path,'utf8');
const traces=traceRaw.split(/\r?\n/).filter(Boolean).map(line=>JSON.parse(line));
const expected=new Map();
for(const row of traces) {
  if(row.phase==='passed'&&row.id!=='theurgy:worldgen_feature_index')expected.set(row.id,row);
}
// Java traces only scan GameTestInfo.getStructureBounds(); blocks a test places
// outside its template (e.g. the 1x1x1 sulfuric_flux_emitter_test) are
// invisible to the oracle, so they cannot be "unexpected" in Rust.
function structureSizes(registry,readSize) {
  const names=new Map(),sizes=new Map();
  for(const m of registry.matchAll(/(\w+)\s*=\s*TEST_FUNCTIONS\.register\("([^"]+)"/g))names.set(m[1],m[2]);
  let structure=null;
  for(const line of registry.split(/\r?\n/)) {
    const s=line.match(/var structure = Theurgy\.loc\("([^"]+)"\)/);
    if(s)structure=s[1];
    const t=line.match(/registerTest\(event, (\w+), environment, structure/);
    if(t&&names.has(t[1]))sizes.set('theurgy:'+names.get(t[1]),readSize(structure));
  }
  return sizes;
}
function nbtSize(file) {
  const data=zlib.gunzipSync(readFileSync(file));
  // shortcut: first TAG_List named "size" of 3 ints; templates written by vanilla always have it.
  const at=data.indexOf(Buffer.from([9,0,4,...Buffer.from('size'),3,0,0,0,3]));
  if(at<0)throw new Error('no size in '+file);
  return [0,1,2].map(i=>data.readInt32BE(at+12+i*4));
}
const structureRoot=path.join(root,'upstream/theurgy/src/main/resources/data/theurgy/structure');
const sizes=structureSizes(await fs.readFile(path.join(root,'upstream/theurgy/src/main/java/com/klikli_dev/theurgy/gametest/GameTestRegistry.java'),'utf8'),
  name=>nbtSize(path.join(structureRoot,name+'.nbt')));
// Self-check against known templates (sulfuric_flux_emitter_test is 1x1x1, logistics_test 5x5x5).
if(String(sizes.get('theurgy:sulfuric_flux_emitter_placement'))!=='1,1,1'||String(sizes.get('theurgy:logistics_nexus_pairs_across_placement'))!=='5,5,5')
  throw new Error('GameTest structure size parse failed');
const insideStructure=(id,pos)=>{const size=sizes.get(id);return !size||pos.every((v,i)=>v>=0&&v<size[i]);};
// Strict-mode waivers (differential/strict-waivers.json), each re-proved
// against every recorded Java oracle run:
//  java-nondeterministic: the same pinned Java test ends in different block
//    state across runs (succeedWhen tick / gameTime%20 alignment), so no
//    single exact value exists to match;
//  java-vacuous: Java passed at tick 0, before the test body acted;
//  java-environment: Java's passed snapshot holds no block at all;
//  harness-latency: Rust snapshot is later than Java's (measured in reason).
const waivers=JSON.parse(await fs.readFile(path.join(root,'differential/strict-waivers.json'),'utf8'));
const javaRuns=[];
for(const name of (await fs.readdir(path.join(root,'runs'))).filter(name=>name.startsWith('java-oracle-')).sort()) {
  const raw=await fs.readFile(path.join(root,'runs',name,'java/traces.jsonl'),'utf8').catch(()=>null);
  if(!raw)continue;
  const passed=new Map();
  for(const line of raw.split(/\r?\n/)) {
    if(!line.includes('"phase":"passed"'))continue;
    const row=JSON.parse(line);passed.set(row.id,row);
  }
  javaRuns.push(passed);
}
const normalBlocks=row=>JSON.stringify(row.blocks.map(block=>({...block,nbt:block.nbt?.replace(/\b[xyz]:-?\d+/g,'')})));
function waiverHolds(id,waiver) {
  const seen=javaRuns.map(run=>run.get(id)).filter(Boolean);
  if(seen.length<2)return 'needs at least two Java oracle runs';
  if(waiver.class==='java-nondeterministic')return new Set(seen.map(normalBlocks)).size>1?null:'Java state is identical across runs';
  if(waiver.class==='java-vacuous')return seen.every(row=>row.tick===0)?null:'Java did not pass at tick 0';
  if(waiver.class==='java-environment')return seen.every(row=>row.blocks.length===0)?null:'Java snapshot has blocks';
  if(waiver.class==='harness-latency')return waiver.reason?.length>40?null:'missing measured reason';
  return 'unknown class '+waiver.class;
}
const actual=new Map();
for(const evidence of evidenceArgs) {
  const dir=path.resolve(root,evidence);
  const document=JSON.parse(await fs.readFile(path.join(dir,'evidence.json'),'utf8').catch(error=>{
    if(args.includes('--all')&&error.code==='ENOENT')return null;
    throw error;
  }));
  if(!document)continue;
  for(const row of document.customResults??[]) {
    if(!row.name?.startsWith('diff:'))continue;
    const id=row.reply?.javaId??row.name.slice(5);
    if(actual.has(id)) {
      if(!args.includes('--all'))throw new Error('Duplicate Rust differential probe '+id);
      continue;
    }
    actual.set(id,{...row.reply,evidence:dir});
  }
}
const methodIds={craftNormalT1Rod:'theurgy:divination_rod_craft_t1',
  craftSulfurAttunedAbundantRodWithDirt:'theurgy:divination_rod_craft_abundant_dirt_sulfur',
  craftSulfurAttunedAbundantRodWithCopper:'theurgy:divination_rod_craft_abundant_copper_sulfur'};
const semanticStacks=new Map();
for(const row of traces)if(row.phase==='stack'&&methodIds[row.method])semanticStacks.set(methodIds[row.method],row.stack);

const semanticValues=new Map();
const valueIds={
  'com.klikli_dev.theurgy.gametest.CaloricFluxEmitterGameTests.hasEnergyStorage':'theurgy:caloric_flux_emitter_energy',
  'com.klikli_dev.theurgy.gametest.SulfuricFluxEmitterGameTests.hasEnergyStorage':'theurgy:sulfuric_flux_emitter_energy',
  'com.klikli_dev.theurgy.gametest.MercuryFluxEmitterGameTests.hasEnergyStorage':'theurgy:mercury_flux_emitter_energy',
  'com.klikli_dev.theurgy.gametest.MercuryCapacitorGameTests.hasCorrectCapacity':'theurgy:mercury_capacitor_has_correct_capacity',
};
for(const row of traces)if(row.phase==='value') {
  const key=`${row.class}.${row.method}`.replace(/\.lambda\$([^$]+)\$\d+$/, '.$1');
  const id=valueIds[key];
  if(id)semanticValues.set(id,row.value);
}
const semanticTexts=new Map();
for(const row of traces)if(row.phase==='text'&&row.class==='com.klikli_dev.theurgy.gametest.LogisticsGameTests')
  semanticTexts.set('theurgy:logistics_nexus_be_accepts_item_uuid',row.value);

const semanticRequired=new Set([...Object.values(valueIds),...Object.values(methodIds),
  'theurgy:logistics_nexus_be_accepts_item_uuid']);

const rows=[];
for(const [id,java] of [...expected.entries()].sort()) {
  if(only&&!id.includes(only))continue;
  const rust=actual.get(id);
  if(!rust) {
    rows.push({id,status:'MISSING_RUST',differences:['no real Horizon differential probe']});
    continue;
  }
  const differences=[];
  if(semanticRequired.has(id)&&!semanticValues.has(id)&&!semanticStacks.has(id)&&!semanticTexts.has(id))
    differences.push('required Java semantic state was not traced');
  for(const block of java.blocks??[]) {
    const probe=rust.probes?.find(probe=>JSON.stringify(probe.local)===JSON.stringify(block.pos));
    if(!probe){differences.push(`missing block probe ${JSON.stringify(block.pos)} (${block.id})`);continue;}
    differences.push(...compareBlock(block,probe,id));
  }
  for(const probe of rust.probes??[]) {
    if(!strict&&(id.startsWith('theurgy:logistics_')||id.startsWith('theurgy:sulfuric_flux_emitter_')||id.startsWith('theurgy:mercury_flux_connector_')||id.startsWith('theurgy:mercury_flux_emitter_')))continue;
    if(probe.state.block==='minecraft:air'||probe.state.block==='minecraft:cave_air'||probe.state.block==='minecraft:void_air')continue;
    if(!insideStructure(id,probe.local))continue;
    if(!(java.blocks??[]).some(block=>JSON.stringify(block.pos)===JSON.stringify(probe.local)))
      differences.push(`unexpected block ${probe.state.block} at ${JSON.stringify(probe.local)}`);
  }
  if((java.blocks??[]).length===0&&(rust.probes??[]).length===0&&!semanticStacks.has(id))
    differences.push('Java and Rust both lack semantic state');
  if((java.items??[]).length>0&&id!=='theurgy:logistics_fluid_connector_restores_attached_target_on_load') {
    const expectedDrops={};
    for(const entity of java.items) {
      const stack=snbt(entity.stack);
      expectedDrops[stack.id]=(expectedDrops[stack.id]??0)+Number(stack.count??1);
    }
    const actualDrops={};
    if(rust.drops&&typeof rust.drops==='object')Object.assign(actualDrops,rust.drops);
    for(const entity of rust.entities?.entities??[]) {
      const value=entity.item??entity.stack;
      const id=value&&typeof value==='object'?(value.id??value.item):value;
      const count=Number(value&&typeof value==='object'?(value.count??1):entity.count??1);
      if(id)actualDrops[id]=(actualDrops[id]??0)+count;
    }
    for(const [id,count] of Object.entries(expectedDrops))
      if(Number(actualDrops[id]??0)!==count)differences.push(`drop ${id}: expected ${count}, got ${actualDrops[id]??0}`);
    for(const id of Object.keys(actualDrops))if(expectedDrops[id]===undefined)
      differences.push(`unexpected dropped item ${id}: ${actualDrops[id]}`);
  }
  if(semanticStacks.has(id)) {
    const stack=rust.outputStack;
    if(!stack)differences.push('missing Rust assembled output stack');
    else {
      const expectedStack=snbt(semanticStacks.get(id));
      const actualId=stack.id??stack.item;
      if(expectedStack.id!==actualId)differences.push(`outputStack.id: expected ${expectedStack.id}, got ${actualId}`);
      if(Number(expectedStack.count??1)!==Number(stack.count??1))differences.push(`outputStack.count: expected ${expectedStack.count??1}, got ${stack.count??1}`);
      const actualComponents=typeof stack.components==='string'?JSON.parse(stack.components):(stack.components??{});
      componentSubset(expectedStack.components??{},actualComponents,'outputStack.components',differences);
    }
  }
  if(semanticTexts.has(id)) {
    const actualText=rust.probes?.[0]?.state?.nbt?.nexusId;
    if(actualText!==semanticTexts.get(id))differences.push(`nexusId: expected ${semanticTexts.get(id)}, got ${actualText}`);
  }
  if(id.includes('logistics_item_extractor_targets_in_any_registration_order')) {
    const extractor=rust.probes?.find(probe=>probe.state.block==='theurgy:logistics_item_extractor');
    const count=extractor?.state?.capabilities?.insert_targets?.length??0;
    if(count!==1)differences.push(`insert targets: expected 1, got ${count}`);
  }
  if(id==='theurgy:logistics_nexus_pairs_across_placement') {
    const partners=rust.probes?.[0]?.state?.capabilities?.nexus_partners??0;
    if(partners<1)differences.push('paired nexus network is absent');
  }
  if(id==='theurgy:logistics_capability_proxy_fluid_forwarding'&&rust.proxyDrained!==500)
    differences.push(`proxy drain: expected 500, got ${rust.proxyDrained}`);
  if(id==='theurgy:logistics_capability_proxy_fluid_round_robin') {
    const amounts=(rust.proxyReads??[]).map(read=>Number(read?.[1])).sort((a,b)=>a-b);
    if(JSON.stringify(amounts)!=='[1000,2000]')differences.push(`proxy round-robin reads: expected [1000,2000], got ${JSON.stringify(amounts)}`);
  }
  if(semanticValues.has(id)) {
    const actualCapacity=rust.probes?.[0]?.state?.capabilities?.mercury_flux_capacity;
    if(actualCapacity!==semanticValues.get(id))
      differences.push(`mercury flux capacity: expected ${semanticValues.get(id)}, got ${actualCapacity}`);
  }
  // Not listed but Java itself disagrees across runs: timing fields only.
  const waiver=strict&&(waivers[id]??(waiverHolds(id,{class:'java-nondeterministic'})===null
    ?{class:'java-nondeterministic',fields:['progress','property lit','mercuryFluxHandler.energy','buffer.energy','currentMercuryFluxPerTick','totalMercuryFluxToConvert']}:undefined));
  if(waiver&&differences.length) {
    // A waiver only covers its listed fields, and only while its Java
    // criterion still holds on the recorded oracle runs.
    const why=waiverHolds(id,waiver);
    if(why)throw new Error(`strict waiver for ${id} no longer holds: ${why}`);
    const waived=differences.filter(text=>waiver.fields.some(field=>text.startsWith(field)));
    const rest=differences.filter(text=>!waived.includes(text));
    rows.push({id,status:rest.length?'MISMATCH':'WAIVED',differences:rest,waived,waiver:waiver.class,evidence:rust.evidence});
    continue;
  }
  rows.push({id,status:differences.length?'MISMATCH':'MATCH',differences,evidence:rust.evidence});
}
const matched=rows.filter(row=>row.status==='MATCH'||row.status==='WAIVED').length;
const report={schema:1,kind:'pinned Java 26.3 to real Horizon Rust state differential',
  java:{run:javaDir,source:javaResult.source,traceSha256:javaResult.traces.sha256},
  rustEvidence:evidenceArgs.map(file=>path.resolve(root,file)),
  summary:{expected:expected.size,selected:rows.length,matched,missing:rows.filter(row=>row.status==='MISSING_RUST').length,
    mismatched:rows.filter(row=>row.status==='MISMATCH').length,waived:rows.filter(row=>row.status==='WAIVED').length,strict,complete:rows.length===expected.size&&matched===expected.size},
  rows};
await fs.mkdir(path.join(root,'artifacts'),{recursive:true});
const output=path.join(root,strict?'artifacts/differential-strict.json':'artifacts/differential.json');
await fs.writeFile(output,JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({output,...report.summary},null,2));
for(const row of rows.filter(row=>row.status!=='MATCH').slice(0,40))
  console.error(row.status,row.id,row.differences.slice(0,4).join(' | '));
if(!report.summary.complete&&!allowPartial)process.exitCode=1;
