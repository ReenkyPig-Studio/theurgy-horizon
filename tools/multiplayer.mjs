import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {root,config,rustEnv} from './common.mjs';

const stamp=Date.now();
const out=path.join(root,'runs/multiplayer-'+stamp);
const hostHome=path.join(root,'.local/multiplayer-host');
const guestHome=path.join(root,'.local/multiplayer-guest');
const exe=process.env.THEURGY_GAME_BIN??path.join(root,'target/native-agent-cache/fast/horizon-client.exe');
const version=config.horizonNativeVersion;
const ctl=path.join(version,'horizon-ctl.exe');
await fs.mkdir(out,{recursive:true});
await fs.access(exe);
await fs.access(ctl);
const sockets={host:path.join(hostHome,'theurgy-control.json'),guest:path.join(guestHome,'theurgy-control.json')};
const evidence=[];
async function call(side,args,allowError=false) {
  const home=side==='host'?hostHome:guestHome;
  const result=await new Promise((resolve,reject)=>{
    let stdout='',stderr='';
    const child=spawn(ctl,['--socket',sockets[side],'--timeout','120',...args],{cwd:home,env:{...rustEnv,HORIZON_HOME:home,HORIZON_CONTROL:sockets[side],HORIZON_NO_AUDIO:'1'},windowsHide:true});
    const timer=setTimeout(()=>{child.kill();reject(new Error(side+' ctl timeout '+args[0]));},180000);
    child.stdout.on('data',data=>{stdout+=data;});
    child.stderr.on('data',data=>{stderr+=data;});
    child.on('error',reject);
    child.on('exit',code=>{
      clearTimeout(timer);
      if(code!==0)return reject(new Error(side+' '+args[0]+': '+stdout+stderr));
      try{resolve(JSON.parse(stdout));}catch{reject(new Error(side+' non-JSON reply '+stdout));}
    });
  });
  if(result.error&&!allowError)throw new Error(side+' '+args[0]+': '+JSON.stringify(result));
  evidence.push({side,args,reply:result});
  return result;
}
async function launchHost() {
  await call('host',['launch','--game',exe,'--world','TheurgyHost','--seed','42','--creative','--wait',
    '--resources',path.join(version,'.local/horizon-client/26.3'),'--game-dir',hostHome,'--home',hostHome,
    '--test-terrain','--render-distance','2','--headless','--dev-mod',path.join(root,'content/theurgy')]);
  await call('host',['wait-ready','180']);
}
async function launchGuest() {
  await call('guest',['launch','--game',exe,'--wait','--resources',path.join(version,'.local/horizon-client/26.3'),
    '--game-dir',guestHome,'--home',guestHome,'--render-distance','2','--headless','--dev-mod',path.join(root,'content/theurgy')]);
}
let hostUp=false,guestUp=false;
try {
  await fs.mkdir(hostHome,{recursive:true});
  await fs.mkdir(guestHome,{recursive:true});
  for(const socket of Object.values(sockets)) {
    try{await fs.access(socket);throw new Error('stale test socket: '+socket);}catch(error){if(error.code!=='ENOENT')throw error;}
  }
  await launchHost();hostUp=true;
  const lan=await call('host',['lan','open','0','false']);
  const port=lan.port??lan.address?.split(':').at(-1);
  if(!port)throw new Error('LAN did not expose a port: '+JSON.stringify(lan));
  await launchGuest();guestUp=true;
  await call('guest',['connect','localhost:'+port]);
  await call('guest',['wait-ready','180']);
  const guests=await call('host',['lan','status']);
  const guest=guests.guests?.[0]??{};
  const name=guest.name??guest.player?.name??guest.username;
  if(!name)throw new Error('guest name not found: '+JSON.stringify(guests));

  // Guests have no cheat permission, but the level-0 native filter action is
  // still allowed and validated by the server, not by the client.
  await call('host',['command','give',name,'theurgy:list_filter','1']);
  await call('host',['command','give',name,'minecraft:apple','1']);
  await call('guest',['command','theurgy_filter','main','mode','deny']);
  await call('guest',['command','theurgy_filter','main','add','auto']);
  await call('guest',['command','theurgy_filter','main','components','true']);
  const edited=await call('guest',['command','data','get','entity','@s','SelectedItem']);
  const editedText=JSON.stringify(edited);
  if(!editedText.includes('deny')||!editedText.includes('minecraft:apple')||!editedText.includes('respect_data_components'))
    throw new Error('guest filter edit did not reach the authoritative stack: '+editedText);
  const hostView=await call('host',['command','data','get','entity',name,'SelectedItem']);
  if(!JSON.stringify(hostView).includes('minecraft:apple'))throw new Error('host does not see guest filter contents');
  await call('guest',['command','theurgy_filter','main','rule_add','codedefinedgui:standard','{"value":"definitely_not_a_real_attribute"}']);
  const rejected=await call('guest',['command','data','get','entity','@s','SelectedItem']);
  if(JSON.stringify(rejected).includes('definitely_not_a_real_attribute'))throw new Error('invalid attribute rule was accepted');
  // Operator-only probes stay unavailable to a guest with commands disabled.
  const probe=await call('guest',['command','theurgy_probe','0','80','0'],true);
  if(JSON.stringify(probe).includes('"block"'))throw new Error('guest elevated to operator probe without permission');
  await call('host',['lan','close']);
  const result={schema:1,status:'PASS',scope:'isolated LAN host/guest, server-authoritative filter edits, invalid-rule rejection and operator-permission checks',
    hostHome,guestHome,guest:name,evidence};
  await fs.writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
  console.log('THEURGY_MULTIPLAYER_PASS observations='+evidence.length+' evidence='+out);
} catch(error) {
  await fs.writeFile(path.join(out,'result.json'),JSON.stringify({schema:1,status:'FAIL',error:String(error?.message??error),evidence},null,2)+'\n');
  throw error;
} finally {
  if(guestUp)try{await call('guest',['quit']);}catch{}
  if(hostUp)try{await call('host',['quit']);}catch{}
}
