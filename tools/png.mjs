import zlib from 'node:zlib';

// Minimal, dependency-free PNG decoder for trusted build/check tools:
// greyscale, RGB, palette (with tRNS), grey+alpha and RGBA; 8-bit samples,
// plus 1/2/4-bit palette/greyscale; non-interlaced; all five filters.
export function decodePng(buffer) {
  const signature=[137,80,78,71,13,10,26,10];
  if(!signature.every((byte,index)=>buffer[index]===byte))throw new Error('not a PNG');
  let offset=8,width=0,height=0,bitDepth=0,colorType=0,interlace=0;
  let palette=null,transparency=null;
  const idat=[];
  while(offset<buffer.length) {
    const length=buffer.readUInt32BE(offset);
    const type=buffer.toString('ascii',offset+4,offset+8);
    const data=buffer.subarray(offset+8,offset+8+length);
    if(type==='IHDR') {
      width=data.readUInt32BE(0);height=data.readUInt32BE(4);
      bitDepth=data[8];colorType=data[9];interlace=data[12];
    } else if(type==='PLTE')palette=data;
    else if(type==='tRNS')transparency=data;
    else if(type==='IDAT')idat.push(data);
    else if(type==='IEND')break;
    offset+=12+length;
  }
  if(interlace!==0)throw new Error('interlaced PNG unsupported');
  const channels={0:1,2:3,3:1,4:2,6:4}[colorType];
  if(!channels)throw new Error('unsupported PNG colour type '+colorType);
  if(bitDepth!==8&&!((colorType===0||colorType===3)&&[1,2,4].includes(bitDepth)))
    throw new Error(`unsupported PNG depth ${bitDepth} for type ${colorType}`);
  const raw=zlib.inflateSync(Buffer.concat(idat));
  const stride=Math.ceil(width*channels*bitDepth/8);
  const bpp=Math.max(1,channels*bitDepth/8);
  const out=new Uint8Array(width*height*4);
  let previous=new Uint8Array(stride);
  for(let y=0;y<height;y++) {
    const filter=raw[y*(stride+1)];
    const line=raw.subarray(y*(stride+1)+1,(y+1)*(stride+1));
    const current=new Uint8Array(stride);
    for(let x=0;x<stride;x++) {
      const a=x>=bpp?current[x-bpp]:0,b=previous[x],c=x>=bpp?previous[x-bpp]:0;
      let value=line[x];
      if(filter===1)value+=a;
      else if(filter===2)value+=b;
      else if(filter===3)value+=(a+b)>>1;
      else if(filter===4){const p=a+b-c,pa=Math.abs(p-a),pb=Math.abs(p-b),pc=Math.abs(p-c);value+=pa<=pb&&pa<=pc?a:pb<=pc?b:c;}
      else if(filter!==0)throw new Error('bad PNG filter '+filter);
      current[x]=value&255;
    }
    const sample=x=>{
      if(bitDepth===8)return current[x];
      const perByte=8/bitDepth,byte=current[Math.floor(x/perByte)];
      const shift=8-bitDepth*(x%perByte+1);
      return (byte>>shift)&((1<<bitDepth)-1);
    };
    for(let x=0;x<width;x++) {
      const i=(y*width+x)*4,s=x*channels;
      if(colorType===3) {
        const index=sample(x);
        out[i]=palette[index*3];out[i+1]=palette[index*3+1];out[i+2]=palette[index*3+2];
        out[i+3]=transparency&&index<transparency.length?transparency[index]:255;
      } else if(channels>=3){out[i]=current[s];out[i+1]=current[s+1];out[i+2]=current[s+2];out[i+3]=channels===4?current[s+3]:255;}
      else {
        const grey=bitDepth===8?current[s]:Math.round(sample(x)*255/((1<<bitDepth)-1));
        out[i]=out[i+1]=out[i+2]=grey;out[i+3]=channels===2?current[s+1]:255;
      }
    }
    previous=current;
  }
  return {width,height,pixels:out};
}

const CRC_TABLE=(()=>{const t=new Uint32Array(256);for(let n=0;n<256;n++){let c=n;for(let k=0;k<8;k++)c=c&1?0xEDB88320^(c>>>1):c>>>1;t[n]=c>>>0;}return t;})();
function crc32(buffer){let c=0xFFFFFFFF;for(const byte of buffer)c=CRC_TABLE[(c^byte)&255]^(c>>>8);return (c^0xFFFFFFFF)>>>0;}
function chunk(type,data){
  const out=Buffer.alloc(12+data.length);
  out.writeUInt32BE(data.length,0);out.write(type,4,'ascii');data.copy(out,8);
  out.writeUInt32BE(crc32(out.subarray(4,8+data.length)),8+data.length);
  return out;
}
/** Minimal RGBA 8-bit PNG encoder (filter 0), the inverse of decodePng. */
export function encodePng({width,height,pixels}) {
  const header=Buffer.alloc(13);
  header.writeUInt32BE(width,0);header.writeUInt32BE(height,4);header[8]=8;header[9]=6;
  const raw=Buffer.alloc(height*(width*4+1));
  for(let y=0;y<height;y++){raw[y*(width*4+1)]=0;Buffer.from(pixels.buffer,pixels.byteOffset+y*width*4,width*4).copy(raw,y*(width*4+1)+1);}
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',header),chunk('IDAT',zlib.deflateSync(raw)),chunk('IEND',Buffer.alloc(0))]);
}
