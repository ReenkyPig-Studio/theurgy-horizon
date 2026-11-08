import fs from 'node:fs/promises';
import path from 'node:path';

// GeckoLib item rendering replacement. Theurgy's GeckoLib block items use
// `minecraft:special` + `geckolib:geckolib`; Horizon renders foreign specials
// with their `base` model, which carries only display transforms, so the
// items were invisible in GUIs, hands, frames and the Hermetica. Item models
// have no animation controllers in Java (*BlockItem.registerControllers is
// empty), so the exact static GeckoLib geometry can be baked into ordinary
// model elements: every cube becomes one element whose Euler XYZ rotation and
// box reproduce GeckoLib's bone/cube transform, and whose per-face UV + face
// rotation reproduce GeckoLib's per-vertex UVs (GeometryCube.bake,
// VertexSet, GeometryQuadUvs, UvFaceRotation, RenderUtil.prepMatrixForBone,
// GeoItemRenderer.adjustRenderPose and each Theurgy *ItemRenderer).

const M=(rows)=>rows; // row-major 4x4
const I=()=>[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]];
function mul(a,b){const o=[[0,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0]];for(let i=0;i<4;i++)for(let j=0;j<4;j++){let s=0;for(let k=0;k<4;k++)s+=a[i][k]*b[k][j];o[i][j]=s;}return o;}
const T=([x,y,z])=>M([[1,0,0,x],[0,1,0,y],[0,0,1,z],[0,0,0,1]]);
const S=([x,y,z])=>M([[x,0,0,0],[0,y,0,0],[0,0,z,0],[0,0,0,1]]);
function R(axis,degrees){
  const r=degrees*Math.PI/180,c=Math.cos(r),s=Math.sin(r);
  if(axis==='x')return M([[1,0,0,0],[0,c,-s,0],[0,s,c,0],[0,0,0,1]]);
  if(axis==='y')return M([[c,0,s,0],[0,1,0,0],[-s,0,c,0],[0,0,0,1]]);
  return M([[c,-s,0,0],[s,c,0,0],[0,0,1,0],[0,0,0,1]]);
}
// RenderUtil.optionalRotateZYX: pose * Rz * Ry * Rx.
const rotZYX=([x,y,z])=>mul(mul(R('z',z),R('y',y)),R('x',x));
const apply=(m,[x,y,z])=>[0,1,2].map(i=>m[i][0]*x+m[i][1]*y+m[i][2]*z+m[i][3]);
const geoRotation=([x,y,z]=[0,0,0])=>[-x,-y,z];
const geoPivot=([x,y,z]=[0,0,0])=>[-x/16,y/16,z/16];

function zeroSizeFace(size,direction){
  const ax=direction==='west'||direction==='east',ay=direction==='up'||direction==='down',az=direction==='north'||direction==='south';
  if(size[0]===0)return !ax;if(size[1]===0)return !ay;if(size[2]===0)return !az;return false;
}
function boxUv(direction,u,v,size){
  const [x,y,z]=size.map(Math.floor);
  return {west:[[u+z+x,v+z],[z,y]],east:[[u,v+z],[z,y]],north:[[u+z,v+z],[x,y]],
    south:[[u+z+x+z,v+z],[x,y]],up:[[u+z,v],[x,z]],down:[[u+z+x,v+z],[x,-z]]}[direction];
}

/** Same baker as native/theurgy/render.rs geo_cube_faces, in block units. */
export function geoCubeFaces(cube,bone,tw,th){
  const mirror=cube.mirror??(bone.mirror===true);
  const inflate=(cube.inflate??bone.inflate??0)/16;
  const size=cube.size??[0,0,0],o=cube.origin??[0,0,0];
  const origin=[-(o[0]+size[0])/16,o[1]/16,o[2]/16];
  const x0=origin[0]-inflate,y0=origin[1]-inflate,z0=origin[2]-inflate;
  const x1=origin[0]+size[0]/16+inflate,y1=origin[1]+size[1]/16+inflate,z1=origin[2]+size[2]/16+inflate;
  const blb=[x0,y0,z0],brb=[x0,y0,z1],tlb=[x0,y1,z0],trb=[x0,y1,z1],tlf=[x1,y1,z0],trf=[x1,y1,z1],blf=[x1,y0,z0],brf=[x1,y0,z1];
  const sets={west:[trb,tlb,blb,brb],east:[tlf,trf,brf,blf],north:[tlb,tlf,blf,blb],south:[trf,trb,brb,brf],up:[trb,trf,tlf,tlb],down:[blb,blf,brf,brb]};
  const boxUvs=Array.isArray(cube.uv);
  const faces=[];
  for(const direction of ['west','east','north','south','up','down']){
    if(zeroSizeFace(size,direction))continue;
    let uv,uvSize,rotation=0;
    if(boxUvs)[uv,uvSize]=boxUv(direction,cube.uv[0],cube.uv[1],size);
    else{const f=cube.uv?.[direction];if(!f)continue;uv=f.uv;uvSize=f.uv_size;rotation=f.uv_rotation??0;}
    const vertices=direction==='west'?(mirror?sets.east:sets.west):direction==='east'?(mirror?sets.west:sets.east)
      :direction==='up'?(mirror&&!boxUvs?sets.down:sets.up):direction==='down'?(mirror&&!boxUvs?sets.up:sets.down):sets[direction];
    const left=uv[0]/tw*16,right=(uv[0]+uvSize[0])/tw*16,top=uv[1]/th*16,bottom=(uv[1]+uvSize[1])/th*16;
    const [u,uw]=mirror?[left,right]:[right,left];
    const start={1:0,2:3,3:2}[(((rotation%360)+360)%360)/90|0]??1;
    const corners=[0,1,2,3].map(i=>vertices[(start+i)%4]);
    faces.push({direction,corners,uv:[uw,top,u,bottom],box:[[x0,y0,z0],[x1,y1,z1]]});
  }
  return faces;
}

// horizon-assets bake.rs `corners` selectors and UV template.
const SELECTORS={down:[[0,0,1],[0,0,0],[1,0,0],[1,0,1]],up:[[0,1,0],[0,1,1],[1,1,1],[1,1,0]],
  north:[[1,1,0],[1,0,0],[0,0,0],[0,1,0]],south:[[0,1,1],[0,0,1],[1,0,1],[1,1,1]],
  west:[[0,1,0],[0,0,0],[0,0,1],[0,1,1]],east:[[1,1,1],[1,0,1],[1,0,0],[1,1,0]]};
export const vanillaCorners=(from,to,direction)=>SELECTORS[direction].map(s=>s.map((bit,i)=>bit?to[i]:from[i]));
const template=([u0,v0,u1,v1])=>[[u0,v0],[u0,v1],[u1,v1],[u1,v0]];
const near=(a,b)=>Math.abs(a-b)<1e-6;
const samePoint=(a,b)=>a.every((value,i)=>near(value,b[i]));

/** Face UV rectangle + rotation reproducing the GeckoLib per-vertex UVs. */
export function fitFace(face){
  const [min,max]=face.box;
  const geoUv=template(face.uv);
  const wanted=vanillaCorners(min,max,face.direction).map(point=>{
    const index=face.corners.findIndex(corner=>samePoint(corner,point));
    if(index<0)throw new Error('GeckoLib face corner not on its box');
    return geoUv[index];
  });
  for(let r=0;r<4;r++){
    const t=[];for(let i=0;i<4;i++)t[(i+r)%4]=wanted[i];
    if(near(t[0][0],t[1][0])&&near(t[2][0],t[3][0])&&near(t[0][1],t[3][1])&&near(t[1][1],t[2][1]))
      return {uv:[t[0][0],t[0][1],t[2][0],t[2][1]],rotation:r*90};
  }
  throw new Error('GeckoLib face UVs are not a rectangle');
}

/** Horizon's EulerXyz element matrix is Rz*Ry*Rx (bake.rs). */
function euler(q){
  const sy=-q[2][0];
  if(Math.abs(sy)<0.999999){
    return [Math.atan2(q[2][1],q[2][2]),Math.asin(sy),Math.atan2(q[1][0],q[0][0])].map(v=>v*180/Math.PI);
  }
  return [0,Math.asin(Math.max(-1,Math.min(1,sy)))*180/Math.PI,Math.atan2(-q[0][1],q[1][1])*180/Math.PI];
}
const round=value=>Math.abs(value)<1e-9?0:Number(value.toFixed(6));

/**
 * Converts one geo model to item-model elements under `itemPose` (the
 * GeoItemRenderer pose: scale, translate(0.5,0.51,0.5), renderer extras).
 */
export function itemElements(geo,itemPose,{hidden=[],texture='#texture'}={}){
  const geometry=geo['minecraft:geometry'][0];
  const tw=geometry.description.texture_width,th=geometry.description.texture_height;
  const bones=geometry.bones;
  const children=name=>bones.filter(bone=>bone.parent===name);
  const elements=[];
  const visit=(bone,pose)=>{
    if(hidden.includes(bone.name))return;
    const pivot=geoPivot(bone.pivot);
    const local=mul(mul(mul(pose,T(pivot)),rotZYX(geoRotation(bone.rotation))),T(pivot.map(v=>-v)));
    for(const cube of bone.cubes??[]){
      const cp=geoPivot(cube.pivot);
      const m=mul(mul(mul(local,T(cp)),rotZYX(geoRotation(cube.rotation))),T(cp.map(v=>-v)));
      const faces=geoCubeFaces(cube,bone,tw,th);
      if(!faces.length)continue;
      // m = [s*Q | t]: absorb the uniform scale into the box, rotate about 0.
      const scale=Math.hypot(m[0][0],m[1][0],m[2][0]);
      const q=[0,1,2].map(i=>[0,1,2].map(j=>m[i][j]/scale));
      const t=[m[0][3],m[1][3],m[2][3]];
      const d=[0,1,2].map(j=>q[0][j]*t[0]+q[1][j]*t[1]+q[2][j]*t[2]);
      const [min,max]=faces[0].box;
      const from=min.map((v,i)=>round((v*scale+d[i])*16)),to=max.map((v,i)=>round((v*scale+d[i])*16));
      const [x,y,z]=euler(q).map(round);
      const element={from,to,faces:{}};
      if(x||y||z)element.rotation={origin:[0,0,0],x,y,z};
      for(const face of faces){
        const fitted=fitFace(face);
        element.faces[face.direction]={texture,uv:fitted.uv.map(round),...(fitted.rotation?{rotation:fitted.rotation}:{})};
      }
      elements.push(element);
    }
    for(const child of children(bone.name))visit(child,local);
  };
  for(const bone of bones.filter(bone=>!bone.parent))visit(bone,itemPose);
  return elements;
}

/** GeoItemRenderer pose for each Theurgy GeckoLib item (Java *ItemRenderer). */
export function itemPose(item){
  const scaled=(scale,extra)=>mul(mul(S([scale,scale,scale]),T([0.5,0.51,0.5])),T(extra));
  switch(item){
    case 'calcination_oven':case 'sal_ammoniac_accumulator':case 'sal_ammoniac_tank':
    case 'incubator_mercury_vessel':case 'incubator_salt_vessel':case 'incubator_sulfur_vessel':
      return scaled(0.5,[0.5/0.5-0.5,-0.1,0.5/0.5-0.5]);
    case 'distiller':
      return scaled(0.35,[0.35/0.5+0.15,-0.1,0.35/0.5+0.15]);
    case 'logistics_nexus':
      // LogisticsNexusItemRenderer: scale x2, own adjustRenderPose (no super).
      return mul(S([2,2,2]),T([0.25,-4/16,0.25]));
    default:return null;
  }
}

export const GECKOLIB_ITEMS={
  calcination_oven:{geo:'calcination_oven',texture:'theurgy:block/calcination_oven'},
  distiller:{geo:'distiller',texture:'theurgy:block/distiller'},
  incubator_mercury_vessel:{geo:'incubator_vessel',texture:'theurgy:block/incubator_vessel_gold'},
  incubator_salt_vessel:{geo:'incubator_vessel',texture:'theurgy:block/incubator_vessel_bronze'},
  incubator_sulfur_vessel:{geo:'incubator_vessel',texture:'theurgy:block/incubator_vessel_iron'},
  logistics_nexus:{geo:'logistics_nexus',texture:'theurgy:block/logistics_nexus',hidden:['Portal','Connectors']},
  sal_ammoniac_accumulator:{geo:'sal_ammoniac_accumulator',texture:'theurgy:block/sal_ammoniac_accumulator'},
  sal_ammoniac_tank:{geo:'sal_ammoniac_tank',texture:'theurgy:block/sal_ammoniac_tank'},
};

/** Rewrites the shipped item definitions/models in `dest` (content root). */
export async function bakeGeckolibItems(dest){
  const assets=path.join(dest,'assets/theurgy');
  const report=[];
  for(const [item,spec] of Object.entries(GECKOLIB_ITEMS)){
    const definitionFile=path.join(assets,'items',item+'.json');
    const definition=JSON.parse(await fs.readFile(definitionFile,'utf8'));
    const special=definition.model;
    if(special?.type!=='minecraft:special'||special.model?.type!=='geckolib:geckolib')
      throw new Error(item+': expected a geckolib special item model');
    const geo=JSON.parse(await fs.readFile(path.join(assets,'geckolib/models/block',spec.geo+'.geo.json'),'utf8'));
    const modelFile=path.join(assets,'models/item',item+'.json');
    const base=JSON.parse(await fs.readFile(modelFile,'utf8'));
    const elements=itemElements(geo,itemPose(item),{hidden:spec.hidden??[]});
    const model={...base,textures:{texture:spec.texture,particle:spec.texture},elements};
    delete model.parent;
    await fs.writeFile(modelFile,JSON.stringify(model,null,2)+'\n');
    await fs.writeFile(definitionFile,JSON.stringify({model:{type:'minecraft:model',model:special.base}},null,2)+'\n');
    report.push({item,elements:elements.length});
  }
  return report;
}
