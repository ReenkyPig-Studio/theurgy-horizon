// SPDX-License-Identifier: MIT
use horizon_client::containers::ModBlockEntity;
use horizon_world::nbt::{Compound, JavaString, Tag};
pub fn int(nbt:&Compound,key:&str,default:i32)->i32 { match nbt.get(&JavaString::from(key)) {
 Some(Tag::Int(v))=>*v,Some(Tag::Short(v))=>i32::from(*v),Some(Tag::Byte(v))=>i32::from(*v),_=>default } }
pub fn set_int(nbt:&mut Compound,key:&str,value:i32){nbt.insert(JavaString::from(key),Tag::Int(value));}
pub fn bool(nbt:&Compound,key:&str)->bool { int(nbt,key,0)!=0 }
pub fn crafting(entity:&ModBlockEntity)->theurgy_engine::crafting::Crafting {
 theurgy_engine::crafting::Crafting { progress:int(&entity.nbt,"progress",0),
 total_time:int(&entity.nbt,"totalTime",100),is_processing:bool(&entity.nbt,"isProcessing"),
 could_craft_last_tick:bool(&entity.nbt,"couldCraftLastTick") } }
pub fn save_crafting(entity:&mut ModBlockEntity,state:&theurgy_engine::crafting::Crafting){
 set_int(&mut entity.nbt,"progress",state.progress);set_int(&mut entity.nbt,"totalTime",state.total_time);
 set_int(&mut entity.nbt,"isProcessing",i32::from(state.is_processing));
 set_int(&mut entity.nbt,"couldCraftLastTick",i32::from(state.could_craft_last_tick)); }
pub fn tank(entity:&ModBlockEntity,index:usize,capacity:i32)->theurgy_engine::recipes::Tank {
 let Some(Tag::List(list))=entity.nbt.get(&JavaString::from("fluids")) else {
  return theurgy_engine::recipes::Tank{fluid:None,amount:0,capacity}; };
 let Some(Tag::Compound(map))=list.get(index) else {
  return theurgy_engine::recipes::Tank{fluid:None,amount:0,capacity}; };
 let fluid=match map.get(&JavaString::from("Fluid")) {Some(Tag::String(v))=>{
  let v=v.to_string_lossy();(v!="minecraft:empty").then_some(v)},_=>None};
 theurgy_engine::recipes::Tank{fluid,amount:int(map,"Amount",0).clamp(0,capacity),capacity} }
pub fn save_tanks(entity:&mut ModBlockEntity,tanks:&[theurgy_engine::recipes::Tank]) {
 let list=tanks.iter().map(|t|{let mut map=Compound::new();
  map.insert(JavaString::from("Fluid"),Tag::String(JavaString::from(t.fluid.as_deref().unwrap_or("minecraft:empty"))));
  map.insert(JavaString::from("Amount"),Tag::Int(t.amount));Tag::Compound(map)}).collect();
 entity.nbt.insert(JavaString::from("fluids"),Tag::List(list)); }
