import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {root,config,rustEnv} from './common.mjs';
import {chaosPlan,machineChaosPlan} from './chaos-plan.mjs';
import {machineScenarios,selfTest,selfTestChaos,blockName,slots,totalItems} from '../scenarios/machines.mjs';
import {runAll} from '../scenarios/all.mjs';

// Every mode gets its own world folder, game home and control socket. The socket always
// lives inside the mode's home, never in the player's runtime directory.
const MODES={
  smoke: {home:'.local/game', world:'TheurgyLab'},
  theurgy: {home:'.local/game', world:'TheurgyLab', native:true},
  content: {home:'.local/content', world:'TheurgyLab', content:true},
  chaos: {home:'.local/chaos', world:'TheurgyLab', chaos:true},
  machines: {home:'.local/machines', world:'TheurgyMachines', native:true, content:true},
  'chaos-machines': {home:'.local/chaos-machines', world:'TheurgyMachinesChaos', native:true, content:true, machinesChaos:true},
  all: {home:'.local/all-machines', world:'TheurgyAll', native:true, content:true, all:true},
};
const mode=process.argv[2]??'smoke';
const spec=MODES[mode];
if(!spec)throw new Error(Object.keys(MODES).join(' | '));
const native=!!spec.native;
const withContent=!!spec.content;
// Real integrated-server gameplay without a window, GPU or renderer. Set
// THEURGY_GUI=1 only when a scenario explicitly needs pixels or input.
const headless=process.env.THEURGY_GUI!=='1';
const gameHome=path.join(root,spec.home);
const out=path.join(root,'runs/game-'+Date.now());
await fs.mkdir(gameHome,{recursive:true});
await fs.mkdir(out,{recursive:true});
// One socket per mode, inside that mode's own home: never the player's global socket.
const socket=path.join(gameHome,'theurgy-control.json');
const version=native?config.horizonNativeVersion:config.horizonVersion;
const exe=process.env.THEURGY_GAME_BIN?path.resolve(process.env.THEURGY_GAME_BIN)
  :native?path.join(root,'target/horizon-cargo/fast/horizon-client.exe'):path.join(version,'horizon-client.exe');
await fs.access(exe); // fail before launching if native binary not built
const ctl=path.join(version,'horizon-ctl.exe');
const env={...rustEnv,HORIZON_HOME:gameHome,HORIZON_CONTROL:socket,HORIZON_NO_AUDIO:'1'};
const evidence=[];
let launched=false;

// Validate the offline plan before touching Minecraft: shape, determinism, lab containment.
if (mode === 'machines') selfTest();
if (mode === 'chaos-machines') {
  selfTestChaos(machineChaosPlan());
}

async function call(args) {
  const result=await new Promise((resolve,reject)=>{
    let stdout='',stderr='';
    const p=spawn(ctl,['--socket',socket,'--timeout','120',...args],{cwd:gameHome,env,windowsHide:true});
    const timer=setTimeout(()=>{p.kill();reject(new Error('ctl timeout: '+args[0]));},360000);
    p.stdout.on('data',s=>{stdout+=s;});
    p.stderr.on('data',s=>{stderr+=s;});
    p.on('error',e=>{clearTimeout(timer);reject(e);});
    p.on('exit',code=>{
      clearTimeout(timer);
      if(code!==0)reject(new Error(args[0]+': '+stdout+stderr));
      else {
        try{resolve(JSON.parse(stdout));}catch{reject(new Error('Non-JSON reply '+stdout));}
      }
    });
  });
  if(result.error)throw new Error(String(result.error));
  evidence.push({command:args[0],args,reply:result});
  return result;
}

async function launch() {
  const args=['launch','--game',exe,'--world',spec.world,'--seed','42','--creative','--wait',
    '--resources',path.join(version,'.local/horizon-client/26.3'),
    '--game-dir',gameHome,'--home',gameHome,'--test-terrain','--render-distance','2'];
  if(headless)args.push('--headless');
  if(withContent)args.push('--dev-mod',path.join(root,'content/theurgy'));
  await call(args);
  launched=true;
  await call(['wait-ready','180']);
}

// Save-and-relaunch on the SAME home/world/socket: proves persistence, not a fresh world.
async function reload() {
  await call(['quit']);
  launched=false;
  await fs.rm(socket,{force:true});
  await launch();
}

function assertStep(step,reply) {
  const check=step.check;
  const verdict=typeof check==='function'?check(reply):null;
  if(verdict&&!verdict.ok)
    throw new Error('Machine scenario step FAILED: '+verdict.name+' ('+(verdict.detail??'')+')');
  if(step.expectsBlock!==undefined){
    assert.equal(blockName(reply),step.expectsBlock,
      'expected block '+step.expectsBlock+' at '+JSON.stringify(step.pos??null));
  }
  if(step.kind==='save')assert.ok(reply&&(reply.ok===true||reply.saved!==undefined),'save must succeed: '+JSON.stringify(reply));
  if((step.kind==='entities-count'||step.kind==='break')&&step.dropOf){
    // A player break reports actual drops synchronously. Prefer that direct
    // evidence; a later entity scan can miss a picked-up or fallen item.
    const dropped=step.kind==='break'
      ?Number(reply?.drops?.[step.dropOf]??0)
      :(()=>{
        const idOf=e=>{const v=e&&(e.item??e.stack);return v&&typeof v==='object'?(v.id??v.item??null):(v??null);};
        const countOf=e=>{const v=e&&(e.item??e.stack);return Number((v&&typeof v==='object'?v.count:undefined)??e?.count??1);};
        const list=Array.isArray(reply.entities)?reply.entities:[];
        return list.filter(e=>e&&String(idOf(e))===step.dropOf).reduce((n,e)=>n+countOf(e),0);
      })();
    assert.ok(dropped<=1,'break duplicated items: expected <=1 '+step.dropOf+', saw '+dropped+' ('+JSON.stringify(reply).slice(0,200)+')');
    assert.ok(dropped>=1,'break lost the held item: expected 1 '+step.dropOf+' drop, saw '+dropped);
  }
}

// Run the ordered distiller scenarios; every failure is fatal and recorded.
async function runMachineScenarios(scenarios) {
  const failures=[];
  for(const step of scenarios.steps) {
    try{
      if(step.kind==='reload'){await reload();continue;}
      const reply=await call(step.args);
      assertStep(step,reply);
    }catch(e){
      failures.push({step:step.name,error:e.message});
      throw new Error('machine scenario halted at "'+step.name+'": '+e.message);
    }
  }
  return failures;
}

// Execute the adversarial machine plan: only "what must hold" invariants, never a scripted pass.
async function runMachineChaos(plan) {
  let probes=0;
  for(const action of plan.actions) {
    if(action.kind==='reload'){await reload();continue;}
    const reply=await call(action.args);
    if(action.expect!==undefined)assert.equal(blockName(reply),action.expect,
      'chaos '+action.name+' expected block '+action.expect+', got '+JSON.stringify(reply).slice(0,160));
    if(action.expectEmpty===true){
      const s=slots(reply);
      assert.ok(s!==null,'chaos '+action.name+': machine has no readable inventory: '+JSON.stringify(reply).slice(0,160));
      assert.equal(totalItems(reply),0,'chaos '+action.name+': a fresh machine must be empty, got '+JSON.stringify(s));
    }
    if(action.expectNonNegative===true){
      const s=slots(reply);
      assert.ok(s!==null,'chaos '+action.name+': machine has no readable inventory');
      for(const x of s)if(x)assert.ok(Number.isFinite(x.count)&&x.count>=0,'chaos '+action.name+': negative/invalid count: '+JSON.stringify(s));
    }
    if(action.invalidInput===true){
      const s=slots(reply);
      assert.ok(s!==null,'chaos '+action.name+': machine has no readable inventory: '+JSON.stringify(reply).slice(0,160));
      // Vanilla stone has no distillation recipe. It may sit in the input slot (Harness writes
      // bypass isValid), but the machine must never distil it: the output slot stays empty and
      // no item is ever created (total <= the single inserted stone).
      const out=s[1];
      assert.ok(!out,'chaos '+action.name+': invalid input was distilled into the output slot: '+JSON.stringify(s));
      assert.ok(totalItems(reply)<=1,'chaos '+action.name+': invalid input created items: '+JSON.stringify(s));
      probes++;
    }
  }
  return {probes};
}

try {
  // Never connect to the player's global socket.
  try {await fs.access(socket);throw new Error('Test socket already exists; stop its game or remove stale socket explicitly.');}
  catch(e){if(e.code!=='ENOENT')throw e;}
  await launch();
  await call(['state']);

  if(spec.machinesChaos) {
    const plan=machineChaosPlan({seed:0x26_04,steps:24});
    await runMachineChaos(plan);
    console.log('THEURGY_MACHINE_CHAOS_PASS seed='+plan.seed+' steps='+plan.steps);
  } else if(spec.all) {
    const results=await runAll(call,assert);
    console.log('THEURGY_ALL_MACHINES_PASS '+results.length+' observations');
  } else if(mode==='machines') {
    // Input/output ids come from the placed recipe/content pack so the harness never guesses.
    const scenarios=machineScenarios({
      input:process.env.THEURGY_TEST_INPUT,
      output:process.env.THEURGY_TEST_OUTPUT,
    });
    await runMachineScenarios(scenarios);
    console.log('THEURGY_MACHINE_SCENARIOS_PASS steps='+scenarios.steps.length+' ('+scenarios.input+' -> '+scenarios.output+')');
  } else {
    await call(['setblock','0','80','0',withContent?'theurgy:distiller':'minecraft:stone']);
    const block=await call(['block','0','80','0']);
    const id=withContent?'theurgy:distiller':'minecraft:stone';
    assert.ok(JSON.stringify(block).includes(id),'expected block identity '+id);
  }

  const chaos=mode==='chaos'?chaosPlan():null;
  if(chaos) {
    for(const action of chaos.actions) {
      const reply=await call(action.args);
      if(action.expected)assert.ok(JSON.stringify(reply).includes(action.expected),
        'Chaos seed '+chaos.seed+': '+JSON.stringify(action.args)+' expected '+action.expected);
    }
    console.log('VANILLA_CHAOS_PASS seed='+chaos.seed+' steps='+chaos.steps+' (not Theurgy parity)');
  }
  await call(['save']);
  const passes={
    smoke:'VANILLA_HORIZON_SMOKE_PASS',
    theurgy:'THEURGY_REGISTRATION_SMOKE_PASS (not machine parity)',
    content:'THEURGY_CONTENT_REGISTRATION_SMOKE_PASS (not machine parity)',
  };
  if(passes[mode])console.log(passes[mode]);
} finally {
  if(launched)try{await call(['quit']);}catch(e){evidence.push({cleanupError:e.message});}
  await fs.writeFile(path.join(out,'evidence.json'),JSON.stringify({
    mode, world:spec.world, home:gameHome, socket, headless, binary:exe,
    scope:mode==='machines'?'distiller place/insert/heat/process/save/reload/break inventory parity on the isolated machine world'
      :mode==='chaos-machines'?'bounded adversarial distiller probes: duplication, invalid input, repeated placement, reload, destruction'
      :native?'distiller block registration only'
      :'vanilla game/control harness only (not Theurgy parity)',
    evidence},null,2));
  console.log('Evidence:',out);
}
