import test from 'node:test';
import assert from 'node:assert/strict';
import {fluidBlockModel,bucketFluidLayer,BUCKET_FLUID_MASK,SAL_AMMONIAC_TINT} from '../tools/sal-ammoniac.mjs';

// The fluid is drawn natively (native/theurgy/sal_fluid.rs); the block model
// only has to put Theurgy.java onRegisterFluidModels' three sprites in the atlas.
test('sal ammoniac block model names the fluid sprites and shows nothing', () => {
  const model=fluidBlockModel(),[element]=model.elements;
  assert.equal(model.elements.length,1);
  assert.deepEqual(Object.values(element.faces).map(face=>model.textures[face.texture.slice(1)]).sort(),
    ['flow','overlay','still'].map(name=>'theurgy:block/sal_ammoniac/'+name));
  assert.ok(element.to.every((v,i)=>v-element.from[i]<0.05),'too small to see');
});

// DynamicFluidContainerModel: still sprite through bucket_fluid, opaque, tinted.
test('sal ammoniac bucket fluid layer is the masked, tinted still frame', () => {
  const still={width:16,height:32,pixels:new Uint8Array(16*32*4).fill(200)};
  const layer=bucketFluidLayer(still);
  let opaque=0;
  for(let y=0;y<16;y++)for(let x=0;x<16;x++){
    const i=(y*16+x)*4,span=BUCKET_FLUID_MASK[y],inside=span&&x>=span[0]&&x<=span[1];
    assert.equal(layer.pixels[i+3],inside?255:0);
    if(inside){opaque++;assert.deepEqual([...layer.pixels.slice(i,i+3)],[200,Math.round(200*((SAL_AMMONIAC_TINT>>>8)&255)/255),200]);}
  }
  assert.equal(opaque,8+10+6);
});
