// Isolated Hermetica preview, no Horizon/Minecraft: builds tools/book-preview
// (native/theurgy/book.rs + serde_json only), records every category screen
// and double page, and rasterizes them to PNG with the real GUI sprites and
// the vanilla and Modonomicon bitmap fonts. Items are their model's flat
// texture or an isometric cube of a block's top/side textures (in game the
// real models), so this checks layout, sprites and text, not item models.
// Usage: node tools/book-preview.mjs [entry-id filter]   -> target/book-preview/png/*.png
import fs from 'node:fs/promises';
import path from 'node:path';
import {root,config,run,rustEnv} from './common.mjs';
import {decodePng,encodePng} from './png.mjs';

const filter=process.argv[2]??'';
const out=path.join(root,'target/book-preview');
const content=path.join(root,'content/theurgy');
const vanilla=path.join(config.horizonVersion,'.local/horizon-client/26.3');
const roots=[content,vanilla];
const K=3;
await fs.mkdir(path.join(out,'scenes'),{recursive:true});
await fs.mkdir(path.join(out,'png'),{recursive:true});

const images=new Map();
async function image(file) {
  if(!images.has(file))images.set(file,await fs.readFile(file).then(decodePng).catch(()=>null));
  return images.get(file);
}
async function asset(ns,rel) {
  for(const r of roots){const f=path.join(r,'assets',ns,rel);try{await fs.access(f);return f;}catch{}}
  return null;
}

// ascii.png advances (BitmapProvider: last opaque column, scaled to 8 px
// high, rounded, plus 1; space 4): the vanilla font and `modonomicon:default`.
const fonts=[];
for(const file of [path.join(vanilla,'assets/minecraft/textures/font/ascii.png'),path.join(content,'assets/theurgy/textures/font/modonomicon/ascii.png')]) {
  const png=await image(file),cell=png.width/16,advance={};
  for(let code=32;code<256;code++){
    const cx=(code%16)*cell,cy=Math.floor(code/16)*cell;let w=0;
    for(let x=0;x<cell;x++)for(let y=0;y<cell;y++)if(png.pixels[((cy+y)*png.width+cx+x)*4+3]>0)w=Math.max(w,x+1);
    advance[String.fromCharCode(code)]=code===32?4:Math.floor(0.5+w*8/cell)+1;
  }
  fonts.push({png,cell,advance});
}
await fs.writeFile(path.join(out,'font.json'),JSON.stringify(fonts.map(f=>f.advance)));

// GUI sprite sizes: assets/<ns>/textures/gui/** -> <ns>:gui/**.
const sprites={};
async function walk(dir,ns,base) {
  for(const e of await fs.readdir(dir,{withFileTypes:true}).catch(()=>[])) {
    const p=path.join(dir,e.name);
    if(e.isDirectory())await walk(p,ns,base);
    else if(e.name.endsWith('.png')){const png=await image(p);if(png)sprites[`${ns}:gui/${path.relative(base,p).replaceAll('\\','/').slice(0,-4)}`]=[png.width,png.height];}
  }
}
for(const ns of await fs.readdir(path.join(content,'assets')))await walk(path.join(content,'assets',ns,'textures/gui'),ns,path.join(content,'assets',ns,'textures/gui'));
await fs.writeFile(path.join(out,'sprites.json'),JSON.stringify(sprites));

await run('cargo',['build','--offline','--release','--quiet','--target-dir',path.join(root,'target/book-preview/cargo')],{cwd:path.join(root,'tools/book-preview'),env:rustEnv});
const exe=path.join(root,'target/book-preview/cargo/release/book-preview'+(process.platform==='win32'?'.exe':''));
await run(exe,[path.join(content,'data/theurgy/hermetica.json'),path.join(out,'font.json'),path.join(out,'sprites.json'),path.join(out,'scenes'),filter]);

// Item icon: the model's layer0, else a block face texture.
const itemCache=new Map();
async function modelTextures(id,depth=0) {
  if(depth>8)return {};
  const [ns,p]=id.includes(':')?id.split(':'):['minecraft',id];
  const file=await asset(ns,`models/${p}.json`);if(!file)return {};
  const json=JSON.parse(await fs.readFile(file,'utf8'));
  const parent=json.parent?await modelTextures(json.parent,depth+1):{};
  return {...parent,...(json.textures??{})};
}
async function itemImage(id) {
  if(itemCache.has(id))return itemCache.get(id);
  const [ns,p]=id.split(':');
  let model=`${ns}:item/${p}`;
  const def=await asset(ns,`items/${p}.json`);
  if(def){const m=JSON.parse(await fs.readFile(def,"utf8")).model;const pick=v=>typeof v?.model==="string"?v.model:v?.model?pick(v.model):v?.fallback?pick(v.fallback):v?.cases?.[0]?pick(v.cases[0]):v?.entries?.[0]?pick(v.entries[0]):v?.on_false?pick(v.on_false):null;model=pick({model:m})??model;}
  const t=await modelTextures(model);
  const resolve=key=>{let v=t[key]?.sprite??t[key];for(let i=0;i<5&&typeof v==='string'&&v.startsWith('#');i++)v=t[v.slice(1)]?.sprite??t[v.slice(1)];return typeof v==='string'&&!v.startsWith('#')?v:null;};
  const load=async tex=>{
    if(!tex)return null;
    const [tn,tp]=tex.includes(':')?tex.split(':'):['minecraft',tex];
    const f=await asset(tn,`textures/${tp}.png`);let png=f?await image(f):null;
    if(png&&png.height>png.width)png={width:png.width,height:png.width,pixels:png.pixels.subarray(0,png.width*png.width*4)};
    return png;
  };
  let png=null;
  if(resolve('layer0'))png=await load(resolve('layer0'));
  else {
    // A block: the GUI's isometric cube from its top and side textures.
    const side=await load(resolve('side')??resolve('all')??resolve('front')??resolve('texture')??resolve('particle'));
    const top=await load(resolve('top')??resolve('end')??resolve('all'))??side;
    const front=await load(resolve('front'))??side;
    if(side)png=isoCube(top,front,side);
  }
  itemCache.set(id,png);
  return png;
}

/** 32 px GUI block icon: top, left (front) and right faces, shaded 1/0.8/0.6. */
function isoCube(top,left,right) {
  const S=32,pixels=new Uint8Array(S*S*4);
  const faces=[[top,[2,8],[14,-7],[14,7],1],[left,[2,8],[14,7],[0,16],0.8],[right,[16,15],[14,-7],[0,16],0.6]];
  for(let y=0;y<S;y++)for(let x=0;x<S;x++)for(const [tex,o,e1,e2,shade] of faces){
    const det=e1[0]*e2[1]-e1[1]*e2[0],px=x+0.5-o[0],py=y+0.5-o[1];
    const u=(px*e2[1]-py*e2[0])/det,v=(e1[0]*py-e1[1]*px)/det;
    if(u<0||v<0||u>=1||v>=1||!tex)continue;
    const i=(Math.floor(v*tex.height)*tex.width+Math.floor(u*tex.width))*4;
    if(tex.pixels[i+3]===0)continue;
    const o4=(y*S+x)*4;
    pixels[o4]=tex.pixels[i]*shade;pixels[o4+1]=tex.pixels[i+1]*shade;pixels[o4+2]=tex.pixels[i+2]*shade;pixels[o4+3]=tex.pixels[i+3];
  }
  return {width:S,height:S,pixels};
}

function rasterize(scene,lookup) {
  const W=Math.round(scene.w*K),H=Math.round(scene.h*K);
  const px=new Uint8Array(W*H*4);
  for(let i=0;i<W*H;i++){px[i*4]=58;px[i*4+1]=74;px[i*4+2]=42;px[i*4+3]=255;}
  const blend=(x,y,r,g,b,a,clip)=>{
    if(x<0||y<0||x>=W||y>=H||a<=0)return;
    if(clip&&(x<clip[0]||y<clip[1]||x>=clip[2]||y>=clip[3]))return;
    const i=(y*W+x)*4,k=a/255;
    px[i]=px[i]*(1-k)+r*k;px[i+1]=px[i+1]*(1-k)+g*k;px[i+2]=px[i+2]*(1-k)+b*k;
  };
  for(const c of scene.cmds) {
    const [ox,oy,s]=c.scale??[0,0,1];
    const T=(x,y)=>[(ox+(x-ox)*s)*K,(oy+(y-oy)*s)*K];
    const clip=c.clip?[c.clip[0]*K,c.clip[1]*K,(c.clip[0]+c.clip[2]+1)*K,(c.clip[1]+c.clip[3]+1)*K]:null;
    const blit=(png,src,dst,tint)=>{
      if(!png)return;
      const [x0,y0]=T(dst[0],dst[1]),[x1,y1]=T(dst[0]+dst[2],dst[1]+dst[3]);
      const ta=(tint>>>24&255)/255,tr=(tint>>>16&255)/255,tg=(tint>>>8&255)/255,tb=(tint&255)/255;
      for(let y=Math.round(y0);y<Math.round(y1);y++)for(let x=Math.round(x0);x<Math.round(x1);x++){
        const u=Math.floor(src[0]+(x+0.5-x0)/(x1-x0)*src[2]),v=Math.floor(src[1]+(y+0.5-y0)/(y1-y0)*src[3]);
        if(u<0||v<0||u>=png.width||v>=png.height)continue;
        const i=(v*png.width+u)*4;
        blend(x,y,png.pixels[i]*tr,png.pixels[i+1]*tg,png.pixels[i+2]*tb,png.pixels[i+3]*ta,clip);
      }
    };
    const text=(runs,at,shadow)=>{
      for(const pass of shadow?[1,0]:[0]){
        let x=at[0];
        for(const r of runs)for(const ch of r.text){
          const {png:font,cell,advance}=fonts[r.book?1:0];
          const code=ch.charCodeAt(0);
          if(code<256&&code!==32){
            const cx=(code%16)*cell,cy=Math.floor(code/16)*cell;
            const k=pass?0.25:1,rgb=[(r.rgb>>16&255)*k,(r.rgb>>8&255)*k,(r.rgb&255)*k];
            for(const bold of r.bold?[0,1]:[0])for(let gy=0;gy<cell;gy++)for(let gx=0;gx<cell;gx++){
              if(font.pixels[((cy+gy)*font.width+cx+gx)*4+3]===0)continue;
              const skew=r.italic?(cell/2-gy)*0.125:0;
              const gxp=x+(gx*8/cell)+pass+bold+skew,gyp=at[1]+(gy*8/cell)+pass;
              const [a,b]=T(gxp,gyp),[e,f]=T(gxp+8/cell,gyp+8/cell);
              for(let yy=Math.round(b);yy<Math.round(f);yy++)for(let xx=Math.round(a);xx<Math.round(e);xx++)blend(xx,yy,rgb[0],rgb[1],rgb[2],255,clip);
            }
          }
          x+=(advance[ch]??6)+(r.bold?1:0);
        }
      }
    };
    if(c.k==='sprite')blit(lookup.sprite[c.name],c.src,c.dst,c.tint);
    else if(c.k==='item')blit(lookup.item[c.id],[0,0,lookup.item[c.id]?.width??16,lookup.item[c.id]?.height??16],c.dst,0xFFFFFFFF);
    else if(c.k==='fill'){const [x0,y0]=T(c.dst[0],c.dst[1]),[x1,y1]=T(c.dst[0]+c.dst[2],c.dst[1]+c.dst[3]);for(let y=Math.round(y0);y<Math.round(y1);y++)for(let x=Math.round(x0);x<Math.round(x1);x++)blend(x,y,c.argb>>>16&255,c.argb>>>8&255,c.argb&255,c.argb>>>24&255,clip);}
    else if(c.k==='text')text(c.runs,c.at,c.shadow);
  }
  return {width:W,height:H,pixels:px};
}

const scenes=JSON.parse(await fs.readFile(path.join(out,'scenes/scenes.json'),'utf8'));
const missing=new Set();
for(const name of scenes) {
  const scene=JSON.parse(await fs.readFile(path.join(out,'scenes',name+'.json'),'utf8'));
  const lookup={sprite:{},item:{}};
  for(const c of scene.cmds) {
    if(c.k==='sprite'&&!(c.name in lookup.sprite)){const [ns,p]=c.name.split(':');const f=await asset(ns,`textures/${p}.png`);lookup.sprite[c.name]=f?await image(f):null;if(!f)missing.add(c.name);}
    if(c.k==='item'&&!(c.id in lookup.item)){lookup.item[c.id]=await itemImage(c.id);if(!lookup.item[c.id])missing.add(c.id);}
  }
  await fs.writeFile(path.join(out,'png',name+'.png'),encodePng(rasterize(scene,lookup)));
}
console.log(`${scenes.length} pages -> ${path.join(out,'png')}`);
if(missing.size)console.log('not drawn (missing sprite/item texture):',[...missing].join(', '));
