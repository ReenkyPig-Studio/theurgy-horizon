import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {geoCubeFaces,itemElements,itemPose,vanillaCorners,GECKOLIB_ITEMS} from '../tools/geckolib-items.mjs';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const geoDir=path.join(root,'upstream/theurgy/src/main/resources/assets/theurgy/geckolib/models/block');

// Horizon bake.rs: EulerXyz rows = Rz*Ry*Rx, vertex' = M(v - origin) + origin.
function eulerMatrix({x=0,y=0,z=0}) {
  const r=v=>v*Math.PI/180;
  const [sx,cx,sy,cy,sz,cz]=[Math.sin(r(x)),Math.cos(r(x)),Math.sin(r(y)),Math.cos(r(y)),Math.sin(r(z)),Math.cos(r(z))];
  return [[cz*cy,-sz*cx+cz*sy*sx,sz*sx+cz*sy*cx],[sz*cy,cz*cx+sz*sy*sx,-cz*sx+sz*sy*cx],[-sy,cy*sx,cy*cx]];
}
function bake(element) {
  const quads=[];
  const m=element.rotation?eulerMatrix(element.rotation):null;
  for(const [direction,face] of Object.entries(element.faces)) {
    const positions=vanillaCorners(element.from,element.to,direction).map(p=>{
      if(!m)return p.map(v=>v/16);
      return [0,1,2].map(i=>(m[i][0]*p[0]+m[i][1]*p[1]+m[i][2]*p[2])/16);
    });
    const [u0,v0,u1,v1]=face.uv;
    const template=[[u0,v0],[u0,v1],[u1,v1],[u1,v0]];
    const r=(face.rotation??0)/90;
    quads.push({direction,points:positions.map((p,i)=>({p,uv:template[(i+r)%4]}))});
  }
  return quads;
}
const close=(a,b,eps=1e-4)=>Math.abs(a-b)<eps;

test('GeckoLib box UV and X mirror match the Rust baker fixtures', () => {
  const faces=geoCubeFaces({origin:[-3,15.1,-3],size:[6,2,6],uv:[73,0]},{},128,128);
  const texel=v=>v/128*16;
  const north=faces.find(f=>f.direction==='north');
  assert.deepEqual(north.uv,[texel(79),texel(6),texel(85),texel(8)]);
  assert.ok(close(north.corners[0][0],3/16)&&close(north.corners[0][1],17.1/16));
  const shifted=geoCubeFaces({origin:[1,2,-3],size:[2,4,6],uv:[0,0]},{},64,64);
  const xs=shifted.flatMap(f=>f.corners.map(c=>c[0]));
  assert.ok(close(Math.min(...xs),-3/16)&&close(Math.max(...xs),-1/16));
});

// End to end: Horizon's own element baking of every generated element must
// reproduce GeckoLib's transformed vertices and per-vertex UVs exactly.
for(const [item,spec] of Object.entries(GECKOLIB_ITEMS)) {
  test(`item model elements reproduce GeckoLib geometry: ${item}`, async () => {
    const geo=JSON.parse(await fs.readFile(path.join(geoDir,spec.geo+'.geo.json'),'utf8'));
    const elements=itemElements(geo,itemPose(item),{hidden:spec.hidden??[]});
    assert.ok(elements.length>0);
    for(const element of elements) {
      for(const value of [...element.from,...element.to])assert.ok(value>=-16&&value<=32,`${item}: ${value} outside model bounds`);
    }
    // Reference: GeckoLib quads under the full bone/cube/item transform.
    const reference=[];
    const geometry=geo['minecraft:geometry'][0];
    const {texture_width:tw,texture_height:th}=geometry.description;
    const mul=(a,b)=>a.map(row=>b[0].map((_,j)=>row.reduce((s,v,k)=>s+v*b[k][j],0)));
    const T=([x,y,z])=>[[1,0,0,x],[0,1,0,y],[0,0,1,z],[0,0,0,1]];
    const R=(axis,d)=>{const r=d*Math.PI/180,c=Math.cos(r),s=Math.sin(r);
      return axis==='x'?[[1,0,0,0],[0,c,-s,0],[0,s,c,0],[0,0,0,1]]:axis==='y'?[[c,0,s,0],[0,1,0,0],[-s,0,c,0],[0,0,0,1]]:[[c,-s,0,0],[s,c,0,0],[0,0,1,0],[0,0,0,1]];};
    const zyx=([x,y,z])=>mul(mul(R('z',z),R('y',y)),R('x',x));
    const visit=(bone,pose)=>{
      if((spec.hidden??[]).includes(bone.name))return;
      const p=[-(bone.pivot?.[0]??0)/16,(bone.pivot?.[1]??0)/16,(bone.pivot?.[2]??0)/16];
      const r=bone.rotation??[0,0,0];
      const local=mul(mul(mul(pose,T(p)),zyx([-r[0],-r[1],r[2]])),T(p.map(v=>-v)));
      for(const cube of bone.cubes??[]) {
        const cp=[-(cube.pivot?.[0]??0)/16,(cube.pivot?.[1]??0)/16,(cube.pivot?.[2]??0)/16];
        const cr=cube.rotation??[0,0,0];
        const m=mul(mul(mul(local,T(cp)),zyx([-cr[0],-cr[1],cr[2]])),T(cp.map(v=>-v)));
        for(const face of geoCubeFaces(cube,bone,tw,th)) {
          const [u0,v0,u1,v1]=face.uv;const uvs=[[u0,v0],[u0,v1],[u1,v1],[u1,v0]];
          reference.push(face.corners.map((c,i)=>({p:[0,1,2].map(k=>m[k][0]*c[0]+m[k][1]*c[1]+m[k][2]*c[2]+m[k][3]),uv:uvs[i]})));
        }
      }
      for(const child of geometry.bones.filter(b=>b.parent===bone.name))visit(child,local);
    };
    for(const bone of geometry.bones.filter(b=>!b.parent))visit(bone,itemPose(item));
    const baked=elements.flatMap(bake);
    assert.equal(baked.length,reference.length);
    for(const quad of reference) {
      const match=baked.some(candidate=>quad.every(vertex=>candidate.points.some(point=>
        point.p.every((v,i)=>close(v,vertex.p[i]))&&point.uv.every((v,i)=>close(v,vertex.uv[i])))));
      assert.ok(match,`${item}: no baked quad reproduces ${JSON.stringify(quad)}`);
    }
  });
}
