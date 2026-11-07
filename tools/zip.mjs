import fs from 'node:fs';
import zlib from 'node:zlib';

// Minimal read-only ZIP central-directory reader for pinned dependency data.
// No process execution, temporary extraction or third-party dependency.
export function readZipEntries(file,prefix='') {
 const bytes=fs.readFileSync(file);let eocd=-1;
 for(let i=bytes.length-22;i>=Math.max(0,bytes.length-22-65535);i--)if(bytes.readUInt32LE(i)===0x06054b50){eocd=i;break;}
 if(eocd<0)throw new Error('ZIP EOCD not found: '+file);
 const count=bytes.readUInt16LE(eocd+10);let offset=bytes.readUInt32LE(eocd+16);const out=new Map();
 for(let index=0;index<count;index++) {
  if(bytes.readUInt32LE(offset)!==0x02014b50)throw new Error('ZIP central directory mismatch: '+file);
  const method=bytes.readUInt16LE(offset+10),compressed=bytes.readUInt32LE(offset+20);
  const nameLength=bytes.readUInt16LE(offset+28),extraLength=bytes.readUInt16LE(offset+30),commentLength=bytes.readUInt16LE(offset+32);
  const local=bytes.readUInt32LE(offset+42),name=bytes.subarray(offset+46,offset+46+nameLength).toString('utf8');
  if(!prefix||name.startsWith(prefix)) {
   if(bytes.readUInt32LE(local)!==0x04034b50)throw new Error('ZIP local header mismatch: '+name);
   const localName=bytes.readUInt16LE(local+26),localExtra=bytes.readUInt16LE(local+28),start=local+30+localName+localExtra;
   const packed=bytes.subarray(start,start+compressed);
   out.set(name,method===0?packed:method===8?zlib.inflateRawSync(packed):null);
   if(![0,8].includes(method))throw new Error('Unsupported ZIP compression '+method+' for '+name);
  }
  offset+=46+nameLength+extraLength+commentLength;
 }
 return out;
}
