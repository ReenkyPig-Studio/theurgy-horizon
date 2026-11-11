import fs from 'node:fs/promises';
import path from 'node:path';

// Native replacement data for Modonomicon page renderers
// (BookCraftingRecipePageRenderer, BookMultiblockPageRenderer,
// BookImagePageRenderer, BookSpotlightPageRenderer and Theurgy's
// integration/modonomicon/page/* recipe pages). The book keeps every page;
// this only resolves what those Java renderers resolve at runtime: recipe
// JSON -> ingredient alternatives (tags expanded with the same merged tag
// table as recipe conditions) and multiblock patterns.

const MAX_ALTERNATIVES=12;

function ingredientItems(value,tagItems) {
  if(value===null||value===undefined)return [];
  if(typeof value==='string')return value.startsWith('#')?tagItems(value.slice(1)).slice(0,MAX_ALTERNATIVES):[value];
  if(Array.isArray(value))return [...new Set(value.flatMap(entry=>ingredientItems(entry,tagItems)))].slice(0,MAX_ALTERNATIVES);
  if(typeof value==='object') {
    if(value.ingredient!==undefined)return ingredientItems(value.ingredient,tagItems);
    if(typeof value.item==='string')return [value.item];
    if(typeof value.tag==='string')return tagItems(value.tag).slice(0,MAX_ALTERNATIVES);
    if(Array.isArray(value.items))return ingredientItems(value.items,tagItems);
  }
  return [];
}
const count=value=>Number(value&&typeof value==='object'&&!Array.isArray(value)?value.count??1:1);
function result(value) {
  if(!value||typeof value!=='object')return null;
  if(value.amount!==undefined)return {fluid:value.id,amount:Number(value.amount)};
  return {item:value.id,count:Number(value.count??1)};
}
function fluid(value,tagFluids=id=>[id]) {
  if(!value||typeof value!=='object')return null;
  const ingredient=value.ingredient??value.fluid??value.id;
  const id=typeof ingredient==='string'?(ingredient.startsWith('#')?tagFluids(ingredient.slice(1))[0]??ingredient:ingredient):null;
  return id?{fluid:id,amount:Number(value.amount??1000)}:null;
}

/** Display model of one recipe, as the Java book page renders it. */
export function displayRecipe(id,json,tagItems) {
  const type=json.type;
  if(type==='minecraft:crafting_shaped') {
    const pattern=json.pattern??[];
    const grid=[];
    for(let row=0;row<3;row++)for(let col=0;col<3;col++) {
      const symbol=pattern[row]?.[col];
      grid.push(symbol&&symbol!==' '?ingredientItems(json.key?.[symbol],tagItems):[]);
    }
    return {id,type,kind:'crafting',grid,result:result(json.result)};
  }
  if(type==='minecraft:crafting_shapeless') {
    const grid=(json.ingredients??[]).slice(0,9).map(entry=>ingredientItems(entry,tagItems));
    while(grid.length<9)grid.push([]);
    return {id,type,kind:'crafting',shapeless:true,grid,result:result(json.result)};
  }
  const inputs=[];
  const add=(label,value)=>{if(value!==undefined)inputs.push({label,items:ingredientItems(value,tagItems),count:count(value)});};
  add('ingredient',json.ingredient);
  for(const entry of json.ingredients??[])add('ingredient',entry);
  add('solute',json.solute);
  add('mercury',json.mercury);add('salt',json.salt);add('sulfur',json.sulfur);
  add('target',json.target);
  for(const entry of json.sources??[])add('source',entry);
  const fluidIn=fluid(json.solvent)??fluid(json.fluid)??fluid(json.evaporant);
  const extra={};
  for(const key of ['mercuryFlux','mercuryFluxPerTick','totalMercuryFlux'])if(json[key]!==undefined)extra[key]=json[key];
  return {id,type,kind:'machine',inputs,fluid:fluidIn,result:result(json.result),time:json.time??null,extra};
}

/** BookMultiblockPageRenderer data: layers top-down, char -> block id. */
export function displayMultiblock(id,json) {
  const mapping={};
  for(const [symbol,target] of Object.entries(json.mapping??{}))
    mapping[symbol]=target.block??target.display??target.blocks?.[0]??null;
  return {id,layers:json.pattern??[],mapping};
}

async function exists(file){try{await fs.access(file);return true;}catch{return false;}}

/**
 * Enriches the flattened Hermetica entries in place.
 */
export async function enrichHermetica({dest,entries,tagItems,textureRoots}) {
  const report={recipes:0,missingRecipes:[],multiblocks:0,missingMultiblocks:[],images:0,missingImages:[]};
  for(const entry of entries) {
    for(const page of entry.pages??[]) {
      const recipeIds=['recipe_id_1','recipe_id_2'].map(key=>page[key]).filter(value=>typeof value==='string'&&value);
      if(recipeIds.length) {
        page.recipes=[];
        for(const recipeId of recipeIds) {
          const [namespace,recipePath]=recipeId.split(':');
          const file=path.join(dest,'data',namespace,'recipe',recipePath+'.json');
          if(!await exists(file)) {
            // sulfur_attuned_divination_rod_<tier> ships as one shaped recipe
            // per sulfur (`<id>__<sulfur>`, see tools/recipe-oracle.mjs); the
            // Java page shows the one DivinationRodRecipe with every sulfur of
            // the tier as alternatives in the same cell.
            const dir=path.dirname(file),prefix=path.basename(file,'.json')+'__';
            const expanded=(await fs.readdir(dir).catch(()=>[])).filter(name=>name.startsWith(prefix)&&name.endsWith('.json')).sort();
            if(!expanded.length){report.missingRecipes.push(recipeId);page.recipes.push({id:recipeId,missing:true});continue;}
            const views=[];
            for(const name of expanded)views.push(displayRecipe(recipeId,JSON.parse(await fs.readFile(path.join(dir,name),'utf8')),tagItems));
            const merged=views[0];
            merged.grid=merged.grid.map((cell,index)=>[...new Set(views.flatMap(view=>view.grid[index]))].slice(0,MAX_ALTERNATIVES*4));
            merged.expandedFrom=expanded.length;
            page.recipes.push(merged);
            report.recipes++;
            continue;
          }
          page.recipes.push(displayRecipe(recipeId,JSON.parse(await fs.readFile(file,'utf8')),tagItems));
          report.recipes++;
        }
      }
      if(typeof page.multiblock_id==='string') {
        const [namespace,multiblockPath]=page.multiblock_id.split(':');
        const file=path.join(dest,'data',namespace,'modonomicon/multiblocks',multiblockPath+'.json');
        if(await exists(file)){page.multiblock=displayMultiblock(page.multiblock_id,JSON.parse(await fs.readFile(file,'utf8')));report.multiblocks++;}
        else report.missingMultiblocks.push(page.multiblock_id);
      }
      // Page images are drawn straight from the GUI atlas
      // (`assets/<ns>/textures/gui/...` -> sprite `<ns>:gui/...`); only check they exist.
      for(const image of Array.isArray(page.images)?page.images:[]) {
        const [namespace,texture]=image.split(':');
        let found=false;
        for(const rootDir of textureRoots)if(await exists(path.join(rootDir,'assets',namespace,texture))){found=true;break;}
        if(found)report.images++;else report.missingImages.push(image);
      }
    }
  }
  return report;
}
