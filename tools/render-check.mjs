import fs from 'node:fs/promises';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {decodePng} from './png.mjs';
import {root} from './common.mjs';

// Pixel evidence for the native GeckoLib renderer. Each apparatus is shot
// twice from the same fixed camera with the HUD hidden: once with the block
// and once with only the block removed. Pixels that differ form the block's
// silhouette mask; the mask is compared (IoU on a coarse grid) and its mean
// colour checked against a reference that a human accepted after viewing the
// screenshot. `shot.ok` alone is never evidence.
export const referenceDir=path.join(root,'scenarios/render-reference');
const COLS=96,ROWS=54;

export async function silhouette(sceneFile,backgroundFile,threshold=40) {
  const scene=decodePng(await fs.readFile(sceneFile));
  const background=decodePng(await fs.readFile(backgroundFile));
  if(scene.width!==background.width||scene.height!==background.height)throw new Error('screenshot sizes differ');
  const {width,height}=scene;
  const mask=new Uint8Array(width*height);
  let area=0,minX=width,minY=height,maxX=-1,maxY=-1;
  const sum=[0,0,0];
  for(let y=0;y<height;y++)for(let x=0;x<width;x++) {
    const i=(y*width+x)*4;
    const delta=Math.abs(scene.pixels[i]-background.pixels[i])+Math.abs(scene.pixels[i+1]-background.pixels[i+1])+Math.abs(scene.pixels[i+2]-background.pixels[i+2]);
    if(delta<=threshold)continue;
    mask[y*width+x]=1;area++;
    for(let c=0;c<3;c++)sum[c]+=scene.pixels[i+c];
    minX=Math.min(minX,x);minY=Math.min(minY,y);maxX=Math.max(maxX,x);maxY=Math.max(maxY,y);
  }
  const cells=new Array(COLS*ROWS).fill(0);
  for(let y=0;y<height;y++)for(let x=0;x<width;x++)if(mask[y*width+x])
    cells[Math.min(ROWS-1,Math.floor(y*ROWS/height))*COLS+Math.min(COLS-1,Math.floor(x*COLS/width))]++;
  const cellArea=(width/COLS)*(height/ROWS);
  const grid=cells.map(count=>count>=cellArea*0.35?'1':'0').join('');
  return {width,height,area,bbox:area?[minX,minY,maxX,maxY]:null,
    mean:area?sum.map(value=>Math.round(value/area)):null,grid,cols:COLS,rows:ROWS,
    sceneSha256:createHash('sha256').update(await fs.readFile(sceneFile)).digest('hex')};
}

export function compare(actual,reference) {
  let both=0,either=0;
  for(let i=0;i<actual.grid.length;i++) {
    const a=actual.grid[i]==='1',b=reference.grid[i]==='1';
    if(a&&b)both++;if(a||b)either++;
  }
  const iou=either?both/either:0;
  const colorDelta=actual.mean&&reference.mean?Math.max(...actual.mean.map((value,index)=>Math.abs(value-reference.mean[index]))):255;
  const areaRatio=reference.area?actual.area/reference.area:0;
  return {iou:Number(iou.toFixed(4)),colorDelta,areaRatio:Number(areaRatio.toFixed(4))};
}

export async function check(name,sceneFile,backgroundFile,{minIou=0.85,maxColorDelta=24}={}) {
  const actual=await silhouette(sceneFile,backgroundFile);
  const referenceFile=path.join(referenceDir,name+'.json');
  if(process.env.THEURGY_RENDER_ACCEPT==='1') {
    // Trusted manual acceptance: only after a human looked at sceneFile.
    await fs.mkdir(referenceDir,{recursive:true});
    const reference={schema:1,name,accepted:{by:process.env.THEURGY_RENDER_ACCEPTED_BY??'integrator',
      at:new Date().toISOString(),note:process.env.THEURGY_RENDER_ACCEPT_NOTE??'visually compared with the GeckoLib model'},
      ...actual};
    await fs.writeFile(referenceFile,JSON.stringify(reference,null,1)+'\n');
    return {name,accepted:true,...compare(actual,reference),area:actual.area,bbox:actual.bbox};
  }
  const reference=JSON.parse(await fs.readFile(referenceFile,'utf8').catch(()=>{
    throw new Error('no accepted render reference for '+name+' ('+referenceFile+')');
  }));
  const result=compare(actual,reference);
  const ok=actual.area>0&&result.iou>=minIou&&result.colorDelta<=maxColorDelta&&result.areaRatio>=0.8&&result.areaRatio<=1.25;
  return {name,ok,minIou,maxColorDelta,...result,area:actual.area,bbox:actual.bbox,mean:actual.mean,
    reference:{area:reference.area,bbox:reference.bbox,mean:reference.mean,sceneSha256:reference.sceneSha256}};
}
