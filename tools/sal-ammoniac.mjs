// SPDX-License-Identifier: MIT
import fs from 'node:fs/promises';
import path from 'node:path';
import {decodePng,encodePng} from './png.mjs';

// Sal ammoniac fluid visuals for Horizon, which renders only water and lava
// as fluids.
// - Block: Theurgy's generated `theurgy:block/sal_ammoniac` model carries only
//   a particle (vanilla draws LiquidBlocks with FluidRenderer, Theurgy.java
//   onRegisterFluidModels: still/flow/overlay + constant tint 0xFFFF00FF).
//   The native mod runs Horizon's FluidRenderer for it
//   (native/theurgy/sal_fluid.rs), which takes the sprites from the block
//   atlas; Horizon stitches only sprites a block model's face uses, so the
//   model keeps them there with faces too small to see.
// - Bucket: `neoforge:fluid_container` (DynamicFluidContainerModel): base
//   `item/bucket`, then the fluid's still sprite through NeoForge's
//   `item/mask/bucket_fluid`, forced opaque (force_opaque_fluid) and tinted by
//   FluidContentsTint (the same constant tint). Baked here as a static layer.

export const SAL_AMMONIAC_TINT=0xFFFF00FF;
/** NeoForge 26.3.0.76 assets/neoforge/textures/item/mask/bucket_fluid.png (LGPL-2.1): opaque pixels by row. */
export const BUCKET_FLUID_MASK={3:[4,11],4:[3,12],5:[5,10]};

/** The block model: particle, plus a 0.02-texel cube whose faces name the fluid's sprites. */
export function fluidBlockModel(){
  const face=(texture,cullface)=>({texture,cullface});
  return {render_type:'minecraft:translucent',ambientocclusion:false,
    textures:{particle:'theurgy:block/sal_ammoniac/still',still:'theurgy:block/sal_ammoniac/still',
      flow:'theurgy:block/sal_ammoniac/flow',overlay:'theurgy:block/sal_ammoniac/overlay'},
    elements:[{from:[7.99,7.99,7.99],to:[8.01,8.01,8.01],
      faces:{up:face('#still','up'),down:face('#flow','down'),north:face('#overlay','north')}}]};
}

/** First still frame through the bucket mask, tinted and opaque. */
export function bucketFluidLayer(still){
  const out=new Uint8Array(16*16*4);
  const scale=still.width/16;
  const tint=[(SAL_AMMONIAC_TINT>>>16)&255,(SAL_AMMONIAC_TINT>>>8)&255,SAL_AMMONIAC_TINT&255];
  for(const [row,[from,to]] of Object.entries(BUCKET_FLUID_MASK)){
    const y=Number(row);
    for(let x=from;x<=to;x++){
      const s=((y*scale|0)*still.width+(x*scale|0))*4,d=(y*16+x)*4;
      for(let c=0;c<3;c++)out[d+c]=Math.round(still.pixels[s+c]*tint[c]/255);
      out[d+3]=255;
    }
  }
  return {width:16,height:16,pixels:out};
}

export async function bakeSalAmmoniac(dest){
  const assets=path.join(dest,'assets/theurgy');
  // Replaces the per-level face models of the older block-model fluid.
  await fs.rm(path.join(assets,'models/block/sal_ammoniac'),{recursive:true,force:true});
  await fs.writeFile(path.join(assets,'models/block/sal_ammoniac.json'),JSON.stringify(fluidBlockModel(),null,2)+'\n');
  await fs.writeFile(path.join(assets,'blockstates/sal_ammoniac.json'),
    JSON.stringify({variants:{'':{model:'theurgy:block/sal_ammoniac'}}},null,2)+'\n');

  const definitionFile=path.join(assets,'items/sal_ammoniac_bucket.json');
  const definition=JSON.parse(await fs.readFile(definitionFile,'utf8'));
  if(definition.model?.type!=='neoforge:fluid_container'||definition.model.fluid!=='theurgy:sal_ammoniac')
    throw new Error('sal_ammoniac_bucket: expected the neoforge:fluid_container item model');
  const still=decodePng(await fs.readFile(path.join(assets,'textures/block/sal_ammoniac/still.png')));
  await fs.mkdir(path.join(assets,'textures/item'),{recursive:true});
  await fs.writeFile(path.join(assets,'textures/item/sal_ammoniac_bucket_fluid.png'),encodePng(bucketFluidLayer(still)));
  await fs.mkdir(path.join(assets,'models/item'),{recursive:true});
  await fs.writeFile(path.join(assets,'models/item/sal_ammoniac_bucket.json'),JSON.stringify({parent:'minecraft:item/generated',
    textures:{layer0:'minecraft:item/bucket',layer1:'theurgy:item/sal_ammoniac_bucket_fluid'}},null,2)+'\n');
  await fs.writeFile(definitionFile,JSON.stringify({model:{type:'minecraft:model',model:'theurgy:item/sal_ammoniac_bucket'}},null,2)+'\n');
  return {bucket:'theurgy:item/sal_ammoniac_bucket'};
}
