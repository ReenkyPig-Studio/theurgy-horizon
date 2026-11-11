// SPDX-License-Identifier: MIT
import fs from 'node:fs/promises';
import path from 'node:path';
import {fitFace,euler,round} from './geckolib-items.mjs';

// DigestionVatRenderer / DigestionVatItemRenderer replacement. Theurgy draws
// the vat only through a vanilla ModelPart block entity renderer (its block
// model carries just a particle) and a `theurgy:digestion_vat` special item
// model, which Horizon does not know, so both block and item were invisible.
// native/theurgy/render.rs draws the block (draw_digestion_vat); this bakes
// the same layers (createBaseLayer/createSidesLayer) into item-model elements
// for the item's default state (north, open, no output), and moves the
// renderer's entity textures into the block atlas.

const PI=Math.PI;
const NECK=[{tex:[0,0],origin:[4,17,4],size:[8,3,8],grow:-0.1},{tex:[0,5],origin:[5,20,5],size:[6,1,6],grow:0.2}];
const PLANE=[{tex:[-14,13],origin:[0,0,0],size:[14,0,14],grow:0}];
const SIDE=[{tex:[1,0],origin:[0,0,0],size:[14,16,0],grow:0,northOnly:true}];
export const DIGESTION_PARTS=[
  {name:'neck',offset:[0,37,16],rotation:[PI,0,0],cubes:NECK,texSize:[32,32],texture:'#base'},
  {name:'top',offset:[1,16,1],rotation:[0,0,0],cubes:PLANE,texSize:[32,32],texture:'#base',hidden:'down'},
  {name:'bottom',offset:[1,0,1],rotation:[0,0,0],cubes:PLANE,texSize:[32,32],texture:'#base',hidden:'up'},
  {name:'front',offset:[1,16,15],rotation:[PI,0,0],cubes:SIDE,texSize:[16,16],texture:'#front'},
  {name:'back',offset:[15,16,1],rotation:[0,0,PI],cubes:SIDE,texSize:[16,16],texture:'#side'},
  {name:'left',offset:[1,16,1],rotation:[0,-PI/2,PI],cubes:SIDE,texSize:[16,16],texture:'#side'},
  {name:'right',offset:[15,16,15],rotation:[0,PI/2,PI],cubes:SIDE,texSize:[16,16],texture:'#side'},
];
export const DIGESTION_TEXTURES=['digestion_vat_base','digestion_vat_base_open','digestion_vat_side',
  'digestion_vat_side_front','digestion_vat_side_front_active'];

/** ModelPart.Cube + Polygon, same as render.rs vanilla_cube_faces. */
export function vanillaCubeFaces(cube,[tw,th]){
  const [dx,dy,dz]=cube.size;
  const min=cube.origin.map(v=>(v-cube.grow)/16),max=cube.origin.map((v,i)=>(v+cube.size[i]+cube.grow)/16);
  const v=[[min[0],min[1],min[2]],[max[0],min[1],min[2]],[max[0],max[1],min[2]],[min[0],max[1],min[2]],
    [min[0],min[1],max[2]],[max[0],min[1],max[2]],[max[0],max[1],max[2]],[min[0],max[1],max[2]]];
  const [u,tv]=cube.tex;
  const U=[u,u+dz,u+dz+dx,u+dz+dx+dx,u+dz+dx+dz,u+dz+dx+dz+dx],V=[tv,tv+dz,tv+dz+dy];
  const polygons=[['down',[5,4,0,1],[U[1],V[0],U[2],V[1]],!dx||!dz],['up',[2,3,7,6],[U[2],V[1],U[3],V[0]],!dx||!dz],
    ['west',[0,4,7,3],[U[0],V[1],U[1],V[2]],!dz||!dy],['north',[1,0,3,2],[U[1],V[1],U[2],V[2]],!dx||!dy],
    ['east',[5,1,2,6],[U[2],V[1],U[4],V[2]],!dz||!dy],['south',[4,5,6,7],[U[4],V[1],U[5],V[2]],!dx||!dy]];
  const faces=[];
  for(const [direction,idx,[a,b,c,d],empty] of polygons){
    if(empty||(cube.northOnly&&direction!=='north'))continue;
    faces.push({direction,corners:[v[idx[1]],v[idx[2]],v[idx[3]],v[idx[0]]],
      uv:[a/tw*16,b/th*16,c/tw*16,d/th*16],box:[min,max]});
  }
  return faces;
}

const rx=a=>[[1,0,0],[0,Math.cos(a),-Math.sin(a)],[0,Math.sin(a),Math.cos(a)]];
const ry=a=>[[Math.cos(a),0,Math.sin(a)],[0,1,0],[-Math.sin(a),0,Math.cos(a)]];
const rz=a=>[[Math.cos(a),-Math.sin(a),0],[Math.sin(a),Math.cos(a),0],[0,0,1]];
const mul3=(a,b)=>a.map((row,i)=>[0,1,2].map(j=>row[0]*b[0][j]+row[1]*b[1][j]+row[2]*b[2][j]));

/** Item-model elements: one per cube, rotated about the origin (bake.rs EulerXyz = Rz*Ry*Rx). */
export function digestionVatElements(){
  const elements=[];
  for(const part of DIGESTION_PARTS){
    // ModelPart.translateAndRotate: T(offset/16) * rotationZYX.
    const q=mul3(mul3(rz(part.rotation[2]),ry(part.rotation[1])),rx(part.rotation[0]));
    const t=part.offset.map(v=>v/16);
    const [x,y,z]=euler(q).map(round);
    for(const cube of part.cubes){
      // hidden: inward face of a 0-thick plane, coplanar with the outer one (z-fighting).
      const faces=vanillaCubeFaces(cube,part.texSize).filter(face=>face.direction!==part.hidden);
      const [min,max]=faces[0].box;
      // Rotate about the cube's posed centre so from/to stay inside the
      // [-16,32] element bounds (the neck about the origin sat at y=-19.9 and
      // the whole item model was rejected: no inventory icon).
      const c=min.map((v,i)=>(v+max[i])/2);
      const o=[0,1,2].map(i=>t[i]+q[i][0]*c[0]+q[i][1]*c[1]+q[i][2]*c[2]);
      const element={from:min.map((v,i)=>round((v-c[i]+o[i])*16)),to:max.map((v,i)=>round((v-c[i]+o[i])*16)),faces:{}};
      if(x||y||z)element.rotation={origin:o.map(v=>round(v*16)),x,y,z};
      for(const face of faces){
        const fitted=fitFace(face);
        element.faces[face.direction]={texture:part.texture,uv:fitted.uv.map(round),...(fitted.rotation?{rotation:fitted.rotation}:{})};
      }
      elements.push(element);
    }
  }
  return elements;
}

/** Copies the renderer textures into the block atlas and bakes the item. */
export async function bakeDigestionVat(dest){
  const assets=path.join(dest,'assets/theurgy');
  const atlasDir=path.join(assets,'textures/block/digestion_vat');
  await fs.mkdir(atlasDir,{recursive:true});
  const modelDir=path.join(assets,'models/block/digestion_vat_atlas');
  await fs.mkdir(modelDir,{recursive:true});
  for(const id of DIGESTION_TEXTURES){
    await fs.copyFile(path.join(assets,'textures/entity/digestion_vat',id+'.png'),path.join(atlasDir,id+'.png'));
    // Atlas source for native_standalone_models (closed lid / active front
    // are not referenced by any baked model).
    await fs.writeFile(path.join(modelDir,id+'.json'),JSON.stringify({parent:'minecraft:block/cube_all',
      textures:{all:'theurgy:block/digestion_vat/'+id}},null,2)+'\n');
  }
  const definitionFile=path.join(assets,'items/digestion_vat.json');
  const definition=JSON.parse(await fs.readFile(definitionFile,'utf8'));
  if(definition.model?.type!=='minecraft:special'||definition.model.model?.type!=='theurgy:digestion_vat')
    throw new Error('digestion_vat: expected the theurgy:digestion_vat special item model');
  const modelFile=path.join(assets,'models/item/digestion_vat.json');
  const base=JSON.parse(await fs.readFile(modelFile,'utf8'));
  const elements=digestionVatElements();
  await fs.writeFile(modelFile,JSON.stringify({...base,textures:{
    base:'theurgy:block/digestion_vat/digestion_vat_base_open',side:'theurgy:block/digestion_vat/digestion_vat_side',
    front:'theurgy:block/digestion_vat/digestion_vat_side_front',particle:'minecraft:block/blue_terracotta'},elements},null,2)+'\n');
  await fs.writeFile(definitionFile,JSON.stringify({model:{type:'minecraft:model',model:definition.model.base}},null,2)+'\n');
  return {elements:elements.length,textures:DIGESTION_TEXTURES.length};
}
