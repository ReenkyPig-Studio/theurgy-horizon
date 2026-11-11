// Real GUI check of the copper wire (Java logistics/WireRenderer.java): a
// diagonal wire must be a thin curve, not the block-sized panels the old
// per-segment bounding boxes drew. Silhouette = scene minus the same scene
// without the wire.
import path from 'node:path';
import fs from 'node:fs/promises';
import {root} from '../tools/common.mjs';
import {silhouette} from '../tools/render-check.mjs';

const shots=path.join(root,'.local/agent-wire-render/shots');
const A=[30,80,40],B=[34,82,45];
const packed=p=>((BigInt(p[0])&0x3ffffffn)<<38n)|((BigInt(p[2])&0x3ffffffn)<<12n)|(BigInt(p[1])&0xfffn);
const signed=v=>BigInt.asIntN(64,v);

export async function runCustom(call,assert){
 const out=[];
 const step=async(name,args)=>{const r=await call(args);out.push({name,args,reply:r});return r;};
 await fs.mkdir(shots,{recursive:true});
 await step('quiet',['command','gamerule','send_command_feedback','false']);
 await step('clear',['fill','24','79','34','40','90','50','minecraft:air']);
 await step('floor',['fill','24','78','34','40','78','50','minecraft:stone']);
 await step('tp',['tp','27','81','37']);
 await step('chunks',['wait','40']);
 await step('weather',['command','weather','clear']);
 await step('time',['command','time','set','6000']);
 await step('hide HUD',['key','f1','down']);await step('release',['key','f1','up']);
 for(const p of [A,B]){
  await step('support',['fill',String(p[0]),'79',String(p[2]),String(p[0]),String(p[1]-1),String(p[2]),'minecraft:stone']);
  await step('node',['setblock',...p.map(String),'theurgy:logistics_connector_node[facing=up]']);
 }
 await step('wire',['command','data','merge','block',...A.map(String),`{wireConnections:[{from:${signed(packed(A))}L,to:${signed(packed(B))}L}]}`]);
 await step('look',['look-at','32.5','81.5','43']);
 await step('frame',['wait','10']);
 const scene=path.join(shots,'wire.png'),background=path.join(shots,'wire-background.png');
 await step('screenshot',['screenshot',scene]);
 await step('unwire',['command','data','remove','block',...A.map(String),'wireConnections']);
 await step('frame',['wait','10']);
 await step('screenshot background',['screenshot',background]);
 const s=await silhouette(scene,background);
 const fraction=s.area/(s.width*s.height);
 out.push({name:'wire silhouette',area:s.area,fraction,bbox:s.bbox});
 assert.ok(s.area>0,'the wire must be drawn');
 const [x0,y0,x1,y1]=s.bbox;
 assert.ok(Math.hypot(x1-x0,y1-y0)>s.width*0.15,'the wire must span between its nodes: '+JSON.stringify(s.bbox));
 // A thin curve covers a sliver of its bounding box; the old boxes filled it.
 assert.ok(s.area<(x1-x0+1)*(y1-y0+1)*0.2&&fraction<0.01,'the wire must be thin: area '+s.area+' bbox '+JSON.stringify(s.bbox));
 return out;
}
