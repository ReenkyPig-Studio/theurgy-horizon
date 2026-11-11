// Trial-port the native mod onto another Horizon revision without touching
// native/, the pinned source or the launcher: copy the mod with a rewritten pin,
// run the real mixins plan/apply + cargo check there, and report what broke
// mapped back to native/theurgy files. --apply writes the new pin when clean.
import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {root,config,rustEnv} from './common.mjs';

const PIN=/(horizon_revision\s*=\s*")[0-9a-f]{40}(")/;
export const readPin=toml=>toml.match(PIN)?.[0].match(/[0-9a-f]{40}/)[0];
export const writePin=(toml,rev)=>toml.replace(PIN,`$1${rev}$2`);

// Every named hook the manifest uses that the target catalog lacks. Mixins
// stops at the first one; this lists all of them before any slow step.
export function missingHooks(toml,catalog) {
  const have=new Set(catalog.hooks.map(h=>h.name+'@'+h.version));
  const used=[...toml.matchAll(/^hook\s*=\s*"([^"]+)"\s*\r?\nhook_version\s*=\s*(\d+)/gm)].map(m=>m[1]+'@'+m[2]);
  return [...new Set(used)].filter(h=>!have.has(h));
}

// What bypasses the stable hook catalog and so can break on any revision:
// direct `target =` injections and modules placed outside game.rs.
export const internals=toml=>[...toml.matchAll(/^(?:target\s*=\s*"([^"]+)"|file\s*=\s*"((?!crates\/horizon-client\/src\/game\.rs")[^"]+)")/gm)].map(m=>m[1]??m[2]);

// Mixins inlines `source = "x.rs"` as `pub(crate) mod theurgy_x {` in the
// staged file, so a staged line maps to the nearest module header above it.
export function mapToMod(stagedText,line,toml='') {
  const sources=Object.fromEntries([...toml.matchAll(/^name\s*=\s*"(\w+)"\s*\r?\nsource\s*=\s*"([^"]+)"/gm)].map(m=>[m[1],m[2]]));
  const lines=stagedText.split('\n');
  for(let i=Math.min(line,lines.length)-1;i>=0;i--) {
    const m=lines[i].match(/^pub\(crate\) mod (theurgy_(\w+)) \{/);
    if(m)return {file:'native/theurgy/'+(sources[m[1]]??m[2]+'.rs'),line:line-i-1};
    if(/^\}/.test(lines[i])&&i<line-1)return null; // left the module: Horizon's own code or a hook call site
  }
  return null;
}

function capture(exe,args) {
  return new Promise((resolve,reject)=>{
    const child=spawn(exe,args,{cwd:root,env:rustEnv,stdio:['ignore','pipe','pipe']});
    let out='',err='';
    child.stdout.on('data',d=>out+=d);
    child.stderr.on('data',d=>{err+=d;process.stderr.write(d);});
    child.on('error',reject);
    child.on('exit',code=>resolve({code,out,err}));
  });
}

async function main() {
  const args=process.argv.slice(2);
  const flag=name=>{const i=args.indexOf(name);return i<0?undefined:args.splice(i,2)[1];};
  const mixins=flag('--mixins')??config.mixins;
  const apply=args.includes('--apply');
  const source=args.find(a=>!a.startsWith('--'));
  if(!source)throw new Error('usage: node tools/horizon-port.mjs <horizon source dir> [--mixins <horizon-mixins.exe>] [--apply]');
  const rev=(await fs.readFile(path.join(source,'.horizon-revision'),'utf8')).trim().slice(0,40);
  if(!/^[0-9a-f]{40}$/.test(rev))throw new Error('no 40-hex commit in '+source+'/.horizon-revision');
  const manifest=path.join(root,'native/theurgy/horizon-mixin.toml');
  const toml=await fs.readFile(manifest,'utf8');
  const from=readPin(toml);
  const work=path.join(root,'target/port',rev.slice(0,7));
  const report={from,to:rev,source,mixins,missingHooks:[],internals:[...new Set(internals(toml))],mixinsError:null,errors:[],ok:false};
  const finish=async()=>{
    const dir=path.join(root,'runs','port-'+rev.slice(0,7));
    await fs.mkdir(dir,{recursive:true});
    await fs.writeFile(path.join(dir,'report.json'),JSON.stringify(report,null,2)+'\n');
    const byFile={};
    for(const e of report.errors)(byFile[e.file]??=[]).push(e);
    for(const [file,list] of Object.entries(byFile))console.log(`${file}: ${list.length} error(s), first ~line ${list[0].line}: ${list[0].message}`);
    console.log(report.ok?`PORT_OK ${from.slice(0,7)} -> ${rev.slice(0,7)}`:`PORT_BROKEN ${from.slice(0,7)} -> ${rev.slice(0,7)}`,
      `(${report.missingHooks.length} missing hooks, ${report.errors.length} compile errors) report: ${path.join(dir,'report.json')}`);
    if(!report.ok)console.log('Look first at the code tied to Horizon internals (everything else rides versioned hooks):\n  '+report.internals.join('\n  '));
    if(!report.ok)process.exitCode=1;
  };

  report.missingHooks=missingHooks(toml,JSON.parse(await fs.readFile(path.join(source,'native-hooks.json'),'utf8')));
  if(report.missingHooks.length) {
    console.log('Hooks absent from the target catalog (target is older than the mod needs, or the hook got a new version):',report.missingHooks.join(', '));
    return finish();
  }

  await fs.rm(work,{recursive:true,force:true});
  await fs.cp(path.join(root,'native'),path.join(work,'native'),{recursive:true});
  await fs.writeFile(path.join(work,'native/theurgy/horizon-mixin.toml'),writePin(toml,rev));
  const stage=path.join(work,'stage');
  for(const step of [['plan','--out',path.join(work,'plan.json')],['apply','--out',stage]]) {
    const r=await capture(mixins,[step[0],'--workspace',source,'--mods',path.join(work,'native'),...step.slice(1)]);
    if(r.code!==0){report.mixinsError=(r.err||r.out).trim();return finish();}
  }

  // One shared check target for every trial revision: Cargo keys by package path.
  const cargo=await capture(path.join(config.rust,'bin/cargo.exe'),['check','--locked','--offline','--message-format=json',
    '--manifest-path',path.join(stage,'Cargo.toml'),'--target-dir',path.join(root,'target/port-cargo'),
    '-p','horizon-client','--bin','horizon-client','--profile','fast']);
  const staged=new Map();
  for(const line of cargo.out.split('\n')) {
    let msg;try{msg=JSON.parse(line).message;}catch{continue;}
    if(msg?.level!=='error')continue;
    const span=msg.spans?.find(s=>s.is_primary);
    if(!span)continue;
    const abs=path.resolve(stage,span.file_name);
    if(!staged.has(abs))staged.set(abs,await fs.readFile(abs,'utf8').catch(()=>''));
    const mod=mapToMod(staged.get(abs),span.line_start,toml);
    report.errors.push({file:mod?.file??path.relative(stage,abs).replaceAll('\\','/'),line:mod?.line??span.line_start,
      code:msg.code?.code??null,message:msg.message,rendered:msg.rendered});
  }
  if(cargo.code!==0&&!report.errors.length)report.mixinsError='cargo check failed without a parsed error:\n'+cargo.err.slice(-4000);
  report.ok=cargo.code===0;
  if(report.ok&&apply) {
    await fs.writeFile(manifest,writePin(toml,rev));
    console.log('Pin written to native/theurgy/horizon-mixin.toml. Now point config.json horizonSource/horizonVersion/horizonNativeVersion/mixins',
      'at this revision and rerun acceptance: old evidence does not cover the new revision.');
  }
  return finish();
}

if(process.argv[1]&&path.resolve(process.argv[1])===path.resolve(root,'tools/horizon-port.mjs'))await main();
