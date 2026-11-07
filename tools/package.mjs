import fs from 'node:fs/promises';
import path from 'node:path';
import zlib from 'node:zlib';
import {createHash} from 'node:crypto';
import {root,run} from './common.mjs';

const HORIZON='294448014fba585f8d5d09dd8886f482493b6417';
const MIXINS='ddd9700b21997c95d95ac6ee5ecafe3830cf622a';
const skipBuild=process.argv.includes('--no-build');
if(!skipBuild) {
  await run(process.execPath,['tools/build-content.mjs'],{cwd:root});
  await run(process.execPath,['tools/content-check.mjs'],{cwd:root});
}
const entries=[];
async function add(name,data) {
  const bytes=Buffer.isBuffer(data)?data:Buffer.from(data);
  entries.push({name:name.replaceAll('\\','/'),bytes});
}
async function addFile(name,file) {add(name,await fs.readFile(file));}
async function walk(dir,visitor) {
  for(const entry of await fs.readdir(dir,{withFileTypes:true})) {
    const file=path.join(dir,entry.name);
    if(entry.isDirectory())await walk(file,visitor);
    else if(entry.isFile())await visitor(file);
  }
}
await addFile('horizon-mod.toml',path.join(root,'content/theurgy/horizon-mod.toml'));
for(const kind of ['assets','data'])await walk(path.join(root,'content/theurgy',kind),async file=>{
  const relative=path.relative(path.join(root,'content/theurgy'),file);
  await addFile(relative,file);
});
await add('horizon-native.json',JSON.stringify({schema:1,id:'theurgy',version:'1.120.1',game:'26.3',horizon:HORIZON,mixins:MIXINS,hooks:1},null,2)+'\n');
await walk(path.join(root,'native/theurgy'),async file=>{
  const relative=path.relative(path.join(root,'native/theurgy'),file).replaceAll('\\','/');
  if(relative==='bootstrap.rs')return;
  if(!/^(?:horizon-mixin\.toml|hooks\/.+\.rs|[^/]+\.rs|engine\/.+|data\/.+)$/.test(relative))return;
  await addFile('mixins/theurgy/'+relative,file);
});
{
  const licenses=path.join(root,'upstream/theurgy/LICENSES');
  for(const name of await fs.readdir(licenses))await addFile('mixins/theurgy/LICENSE-'+name,path.join(licenses,name));
}
await addFile('mixins/theurgy/REUSE.toml',path.join(root,'upstream/theurgy/REUSE.toml'));
await addFile('mixins/theurgy/NOTICE.md',path.join(root,'THIRD_PARTY_NOTICES.md'));
entries.sort((a,b)=>a.name.localeCompare(b.name));
const nativeFiles=entries.filter(entry=>entry.name==='horizon-native.json'||entry.name.startsWith('mixins/'));
if(nativeFiles.length>512)throw new Error('native file limit exceeded');
const nativeBytes=nativeFiles.reduce((sum,entry)=>sum+entry.bytes.length,0);
if(nativeBytes>64*1024*1024)throw new Error('native unpacked size limit exceeded');
if(entries.some(entry=>!entry.name||entry.name.includes('..')||entry.name.startsWith('/')))throw new Error('unsafe zip path');

const table=new Uint32Array(256);
for(let n=0;n<256;n++) {
  let c=n;
  for(let k=0;k<8;k++)c=(c&1)?(c>>>1)^0xEDB88320:(c>>>1);
  table[n]=c>>>0;
}
function crc32(bytes) {
  let crc=0xFFFFFFFF;
  for(const byte of bytes)crc=(crc>>>8)^table[(crc^byte)&0xFF];
  return (crc^0xFFFFFFFF)>>>0;
}
const local=[],central=[];
let offset=0;
for(const entry of entries) {
  const name=Buffer.from(entry.name,'utf8');
  const packed=zlib.deflateRawSync(entry.bytes,{level:9});
  const crc=crc32(entry.bytes);
  const header=Buffer.alloc(30);
  header.writeUInt32LE(0x04034B50,0);
  header.writeUInt16LE(20,4);
  header.writeUInt16LE(0x0800,6);
  header.writeUInt16LE(8,8);
  header.writeUInt16LE(0,10);
  header.writeUInt16LE(0x0021,12);
  header.writeUInt32LE(crc,14);
  header.writeUInt32LE(packed.length,18);
  header.writeUInt32LE(entry.bytes.length,22);
  header.writeUInt16LE(name.length,26);
  header.writeUInt16LE(0,28);
  local.push(header,name,packed);
  const record=Buffer.alloc(46);
  record.writeUInt32LE(0x02014B50,0);
  record.writeUInt16LE(20,4);
  record.writeUInt16LE(20,6);
  record.writeUInt16LE(0x0800,8);
  record.writeUInt16LE(8,10);
  record.writeUInt16LE(0,12);
  record.writeUInt16LE(0x0021,14);
  record.writeUInt32LE(crc,16);
  record.writeUInt32LE(packed.length,20);
  record.writeUInt32LE(entry.bytes.length,24);
  record.writeUInt16LE(name.length,28);
  record.writeUInt16LE(0,30);
  record.writeUInt16LE(0,32);
  record.writeUInt16LE(0,34);
  record.writeUInt16LE(0,36);
  record.writeUInt16LE(0,38);
  record.writeUInt32LE(0,40);
  record.writeUInt32LE(offset,42);
  central.push(record,name);
  offset+=header.length+name.length+packed.length;
}
const centralBytes=Buffer.concat(central);
const end=Buffer.alloc(22);
end.writeUInt32LE(0x06054B50,0);
end.writeUInt16LE(0,4);
end.writeUInt16LE(0,6);
end.writeUInt16LE(entries.length,8);
end.writeUInt16LE(entries.length,10);
end.writeUInt32LE(centralBytes.length,12);
end.writeUInt32LE(offset,16);
end.writeUInt16LE(0,20);
const archive=Buffer.concat([...local,centralBytes,end]);
if(archive.length>64*1024*1024)throw new Error('64 MiB package limit exceeded');
const out=path.join(root,'artifacts/theurgy-26.3-1.120.1-rust-horizon.hmod');
await fs.mkdir(path.dirname(out),{recursive:true});
await fs.writeFile(out,archive);
const sha256=createHash('sha256').update(archive).digest('hex');
await fs.writeFile(path.join(root,'artifacts/package.json'),JSON.stringify({schema:1,file:out,sha256,
  bytes:archive.length,entries:entries.length,nativeFiles:nativeFiles.length,nativeBytes,
  manifests:{horizon:HORIZON,mixins:MIXINS,minecraft:'26.3',theurgy:'1.120.1'}},null,2)+'\n');
console.log(JSON.stringify({file:out,sha256,bytes:archive.length,entries:entries.length,nativeFiles:nativeFiles.length},null,2));
