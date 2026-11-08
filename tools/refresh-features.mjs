import fs from 'node:fs/promises';
import path from 'node:path';
import {root} from './common.mjs';
import {featureCoverage,hasFeatureCoverage,featureCoverageSize} from './feature-evidence.mjs';

const documentPath=path.join(root,'docs/FEATURES.md');
const differential=JSON.parse(await fs.readFile(path.join(root,'artifacts/differential.json'),'utf8'));
const gui=JSON.parse(await fs.readFile(path.join(root,'artifacts/gui-evidence.json'),'utf8'));
const offline=JSON.parse(await fs.readFile(path.join(root,'artifacts/offline-evidence.json'),'utf8').catch(()=>'{"steps":{}}'));
const allDifferential=new Map(differential.rows.map(row=>[row.id,row]));
const matched=new Set(differential.rows.filter(row=>row.status==='MATCH').map(row=>row.id));

const owners={
  'CB-10':'native/theurgy/engine/src/crafting.rs','CB-12':'native/theurgy/host.rs',
  'CB-13':'native/theurgy/engine/src/recipes.rs','CB-14':'native/theurgy/interactions.rs',
  'HEA-02':'native/theurgy/heat.rs','HEA-03':'native/theurgy/heat.rs','HEA-06':'native/theurgy/heat.rs',
  'MA-03':'native/theurgy/engine/src/recipes.rs + native/theurgy/machines.rs',
  'SAL-02':'native/theurgy/machines.rs','SAL-03':'native/theurgy/machines.rs',
  'FLU-03':'native/theurgy/flux.rs','FLU-06':'native/theurgy/flux.rs',
  'FLU-07':'native/theurgy/flux.rs','FLU-10':'native/theurgy/flux.rs',
  'REF-03':'native/theurgy/reformation.rs','REF-05':'native/theurgy/reformation.rs',
  'REF-08':'native/theurgy/reformation.rs','REF-09':'native/theurgy/reformation.rs',
  'LOG-10':'native/theurgy/logistics.rs','ITM-08':'native/theurgy/client.rs',
  'REC-01':'native/theurgy/engine/src/recipes.rs','REG-11':'native/theurgy/engine/src/recipes.rs + native/theurgy/recipe_client.rs + generated data',
  'CFG-01':'native/theurgy/items.rs','CFG-02':'native/theurgy/config.rs',
  'CFG-03':'native/theurgy/client.rs + render.rs','INT-01':'native/theurgy/recipe_client.rs',
  'INT-02':'native/theurgy/client.rs','INT-03':'native/theurgy/recipe_client.rs + filtered_transfer.rs + generated data',
  'CLI-03':'native/theurgy/items.rs','GEN-05':'content/theurgy/data/theurgy/modonomicon + native/theurgy/client.rs',
  'INC-02':'native/theurgy/incubation.rs',
};
const fallbackOwners=[
  [/^REG-/, 'native/theurgy/registry.rs + generated content'],
  [/^CB-/, 'native/theurgy/engine/src + native/theurgy/host.rs'],
  [/^(HEA|MA)-/, 'native/theurgy/machines.rs + native/theurgy/interactions.rs'],
  [/^SAL-/, 'native/theurgy/fluids.rs + native/theurgy/machines.rs'],
  [/^FLU-/, 'native/theurgy/flux.rs'], [/^INC-/, 'native/theurgy/incubation.rs'],
  [/^REF-/, 'native/theurgy/reformation.rs'], [/^LOG-/, 'native/theurgy/logistics.rs'],
  [/^ITM-/, 'native/theurgy/items.rs'], [/^(REC|GEN)-/, 'tools/build-content.mjs + content/theurgy'],
  [/^(CFG|NET|INT|CLI)-/, 'native/theurgy/client.rs + native hooks'],
];
function rustOwner(id,current) {
  if(owners[id])return owners[id];
  const cleaned=current.replaceAll('`','').replace(/\s*\(placeholder\)/gi,'').trim();
  if(cleaned&&cleaned!=='-')return cleaned;
  return fallbackOwners.find(([pattern])=>pattern.test(id))?.[1]??'native/theurgy';
}
async function fileExists(file) {
  if(!file)return false;
  try{await fs.access(path.join(root,file));return true;}catch{return false;}
}

let lines=(await fs.readFile(documentPath,'utf8')).split(/\r?\n/);
const statuses=new Map();
const evidenceFailures=[];
for(let index=0;index<lines.length;index++) {
  const cells=lines[index].split(/(?<!\\)\|/);
  if(cells.length<7||!/^[A-Z]+-[A-Z0-9]+$/.test(cells[1]?.trim()??''))continue;
  const id=cells[1].trim();
  if(!hasFeatureCoverage(id))throw new Error('No feature evidence mapping for '+id);
  const coverage=featureCoverage(id);
  for(const testId of coverage.diff) {
    if(!allDifferential.has(testId))throw new Error(`${id}: unknown differential test ${testId}`);
  }
  const tests=[],runs=[],missing=[];
  if(coverage.diff.length) {
    const absent=coverage.diff.filter(testId=>!matched.has(testId));
    if(absent.length||!differential.summary?.complete)missing.push('differential:'+absent.join(','));
    else {
      tests.push(`Java GameTest differential (${coverage.diff.length} tests)`);
      runs.push('artifacts/differential.json');
    }
  }
  for(const mode of coverage.modes) {
    const run=gui[mode];
    const evidenceFile=mode==='multiplayer'?'result.json':'evidence.json';
    if(!run||!(await fileExists(path.join(run,evidenceFile))))missing.push('scenario:'+mode);
    else {tests.push('isolated game '+mode);runs.push(run);}
  }
  for(const step of coverage.offline) {
    const log=offline.steps?.[step];
    if(!log||!(await fileExists(log)))missing.push('offline:'+step);
    else {tests.push('offline '+step);runs.push(log);}
  }
  const complete=missing.length===0&&tests.length>0;
  const status=complete?'VERIFIED':'PORTED-UNVERIFIED';
  if(!complete)evidenceFailures.push({id,missing});
  statuses.set(id,status);
  cells[3]=' '+status+' ';
  cells[5]=' `'+rustOwner(id,cells[5].trim())+'` ';
  cells[6]=complete
    ? ` test=${tests.join(' + ')} | run=${runs.join(' + ')} | result=PASS `
    : ` test=coverage evidence incomplete | run=${missing.join(' + ')||'-'} | result=NOT EXECUTED `;
  cells.length=7;
  cells.push('');
  lines[index]=cells.join('|');
}
if(statuses.size!==featureCoverageSize)
  throw new Error(`Feature rows (${statuses.size}) and evidence mapping (${featureCoverageSize}) differ`);

const groups=[
  ['1 registries (REG)',id=>id.startsWith('REG-')],
  ['2 common crafting (CB)',id=>id.startsWith('CB-')],
  ['3 machines (HEA/MA)',id=>/^(HEA|MA)-/.test(id)],
  ['4 sal ammoniac (SAL)',id=>id.startsWith('SAL-')],
  ['5 flux (FLU)',id=>id.startsWith('FLU-')],
  ['6 incubator (INC)',id=>id.startsWith('INC-')],
  ['7 reformation (REF)',id=>id.startsWith('REF-')],
  ['8 logistics (LOG)',id=>id.startsWith('LOG-')],
  ['9 items (ITM)',id=>id.startsWith('ITM-')],
  ['10 recipes/gen (REC/GEN)',id=>/^(REC|GEN)-/.test(id)],
  ['11 config/network/integration/client',id=>/^(CFG|NET|INT|CLI)-/.test(id)],
];
const summary=[];
for(const [name,test] of groups) {
  const ids=[...statuses.keys()].filter(test);
  summary.push({subsystem:name,rows:ids.length,
    verified:ids.filter(id=>statuses.get(id)==='VERIFIED').length,
    portedUnverified:ids.filter(id=>statuses.get(id)==='PORTED-UNVERIFIED').length});
}
const total={rows:statuses.size,verified:[...statuses.values()].filter(s=>s==='VERIFIED').length,
  portedUnverified:[...statuses.values()].filter(s=>s==='PORTED-UNVERIFIED').length};
const table=[
  '## 13. Coverage summary (current, evidence-derived)',
  '',
  'Rows are `VERIFIED` only when every reference in `tools/feature-evidence.mjs` resolves to a passing Java differential, a saved isolated-game run, or a saved offline acceptance step. Missing artifacts leave the row `PORTED-UNVERIFIED`.',
  '',
  '| subsystem | rows | VERIFIED | PORTED-UNVERIFIED | PARTIAL | UNPORTED | N/A-CLIENT |',
  '|---|---|---|---|---|---|---|',
  ...summary.map(row=>`| ${row.subsystem} | ${row.rows} | ${row.verified} | ${row.portedUnverified} | 0 | 0 | 0 |`),
  '',
  `Total: **${total.rows} checklist rows**. VERIFIED = ${total.verified}, PORTED-UNVERIFIED = ${total.portedUnverified}, PARTIAL = 0, UNPORTED = 0, N/A-CLIENT = 0.`,
  'The 152-behaviour matrix is `artifacts/differential.json`; isolated-game runs are indexed by `artifacts/gui-evidence.json`; offline steps by `artifacts/offline-evidence.json`.',
  '',
  '## 14. Definition of done for one row',
];
const start=lines.findIndex(line=>line.startsWith('## 13. Coverage summary'));
const end=lines.findIndex(line=>line.startsWith('## 14. Definition of done'));
if(start<0||end<0)throw new Error('FEATURES summary anchors missing');
lines.splice(start,end-start,...table);
let seenDefinition=false;
lines=lines.filter(line=>{
  if(!line.startsWith('## 14. Definition of done'))return true;
  if(seenDefinition)return false;
  seenDefinition=true;
  return true;
});
await fs.writeFile(documentPath,lines.join('\n'));
await fs.writeFile(path.join(root,'artifacts/feature-status.json'),JSON.stringify({summary,total,
  unverified:[...statuses].filter(([,status])=>status!=='VERIFIED').map(([id])=>id),evidenceFailures},null,2)+'\n');
console.log(JSON.stringify({total,evidenceFailures},null,2));
