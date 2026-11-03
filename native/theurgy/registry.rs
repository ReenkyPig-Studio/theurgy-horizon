// SPDX-License-Identifier: MIT
use horizon_client::containers::{ModContainer,ModFluidTanks,ModBlockEntity};
use horizon_world::nbt::Compound;
pub fn is_be(block:&str)->bool { matches!(block,
 "theurgy:calcination_oven"|"theurgy:pyromantic_brazier"|"theurgy:liquefaction_cauldron"|"theurgy:distiller"|
 "theurgy:incubator"|"theurgy:incubator_mercury_vessel"|"theurgy:incubator_salt_vessel"|
 "theurgy:incubator_sulfur_vessel"|"theurgy:sal_ammoniac_tank"|"theurgy:sal_ammoniac_accumulator"|
 "theurgy:logistics_nexus"|"theurgy:mercury_catalyst"|"theurgy:mercury_capacitor"|"theurgy:caloric_flux_emitter"|
 "theurgy:reformation_source_pedestal"|"theurgy:reformation_target_pedestal"|"theurgy:reformation_result_pedestal"|
 "theurgy:sulfuric_flux_emitter"|"theurgy:mercury_flux_emitter"|"theurgy:fermentation_vat"|"theurgy:digestion_vat"|
 "theurgy:logistics_item_inserter"|"theurgy:logistics_item_extractor"|"theurgy:logistics_fluid_inserter"|
 "theurgy:logistics_fluid_extractor"|"theurgy:logistics_capability_proxy"|"theurgy:logistics_mercury_flux_connector") }
pub fn create(block:&str,entity:&mut Option<(String,Compound)>){if is_be(block)&&entity.is_none(){
 *entity=Some((block.to_owned(),Compound::new()));}}
pub fn ticks(block:&str,ticks:&mut bool){if is_be(block){*ticks=true;}}
pub fn container(block:&str)->Option<ModContainer>{
 let(slots,input_count)=match block {
  "theurgy:calcination_oven"|"theurgy:distiller"|"theurgy:liquefaction_cauldron"=>(2,1),
  "theurgy:fermentation_vat"|"theurgy:digestion_vat"=>(4,3),
  "theurgy:pyromantic_brazier"|"theurgy:incubator"|"theurgy:incubator_mercury_vessel"|
  "theurgy:incubator_salt_vessel"|"theurgy:incubator_sulfur_vessel"|"theurgy:reformation_source_pedestal"|
  "theurgy:reformation_target_pedestal"|"theurgy:reformation_result_pedestal"|"theurgy:mercury_catalyst"=>
   (1,usize::from(block!="theurgy:incubator"&&block!="theurgy:reformation_result_pedestal")),
  _=>return None};
 Some(ModContainer{slots,faces:std::array::from_fn(|_| (0..slots).collect()),inputs:(0..input_count).collect(),
  outputs:if input_count<slots{vec![slots-1]}else{vec![]},
  menu:Some(format!("theurgy:{}",block.trim_start_matches("theurgy:")))})}
pub fn fluids(block:&str)->Option<ModFluidTanks>{let capacity=match block {
 "theurgy:liquefaction_cauldron"|"theurgy:fermentation_vat"|"theurgy:digestion_vat"|
 "theurgy:sal_ammoniac_accumulator"|"theurgy:sal_ammoniac_tank"=>10000,_=>return None};
 Some(ModFluidTanks{tanks:vec![capacity],faces:std::array::from_fn(|_| vec![0]),inputs:vec![0],outputs:vec![0]})}
pub fn tick(game:&mut crate::game::Game<'_>,pos:[i32;3],entity:&mut ModBlockEntity){
 match entity.id.as_str(){
  "theurgy:calcination_oven"|"theurgy:distiller"|"theurgy:liquefaction_cauldron"|
  "theurgy:fermentation_vat"|"theurgy:digestion_vat"=>crate::game::theurgy_machines::tick(game,pos,entity),
  "theurgy:pyromantic_brazier"=>crate::game::theurgy_heat::tick(game,pos,entity),
  "theurgy:mercury_catalyst"|"theurgy:mercury_capacitor"|"theurgy:caloric_flux_emitter"|
  "theurgy:mercury_flux_emitter"=>crate::game::theurgy_flux::tick(game,pos,entity),
  "theurgy:incubator"|"theurgy:incubator_mercury_vessel"|"theurgy:incubator_salt_vessel"|
  "theurgy:incubator_sulfur_vessel"=>crate::game::theurgy_incubation::tick(game,pos,entity),
  "theurgy:sal_ammoniac_accumulator"|"theurgy:sal_ammoniac_tank"=>crate::game::theurgy_fluids::tick(game,pos,entity),
  "theurgy:sulfuric_flux_emitter"|"theurgy:reformation_source_pedestal"|
  "theurgy:reformation_target_pedestal"|"theurgy:reformation_result_pedestal"=>crate::game::theurgy_reformation::tick(game,pos,entity),
  id if id.starts_with("theurgy:logistics_")=>crate::game::theurgy_logistics::tick(game,pos,entity),
  _=>{}
 }}
pub fn component_types(types:&mut Vec<String>){for name in ["mercury_flux_storage","mercury_flux_to_convert",
 "current_mercury_flux_per_tick","divination_settings_allowed_blocks_tag","divination_settings_disallowed_blocks_tag",
 "divination_settings_duration","divination_settings_max_damage","divination_settings_range","divination_settings_tier",
 "divination_settings_allow_attuning","divination_distance","divination_pos","divination_linked_block",
 "divination_linked_tag","wire_end_point","mercurial_wand_item_mode","source_item","source_tag",
 "mercury_catalyst_inventory","filter_items","filter_is_deny_list","filter_respects_data_components","filter_mode",
 "filter_attributes","selected_frequency","logistics_nexus_id"]{types.push(format!("theurgy:{name}"));}}
