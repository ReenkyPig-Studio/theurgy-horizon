// Source-derived mapping from FEATURES rows to executable evidence. This file
// contains no verdicts: refresh-features validates every reference against the
// saved differential, isolated-game and acceptance artifacts.
const D=(...ids)=>ids.map(id=>'theurgy:'+id);
const coverage={};
function add(rows,{diff=[],modes=[],offline=[]}={}) {
  for(const id of rows.split(/\s+/).filter(Boolean)) {
    if(coverage[id])throw new Error('duplicate feature evidence mapping: '+id);
    coverage[id]={diff:D(...diff),modes,offline};
  }
}

// Registries and data/component identity.
add('REG-01',{modes:['all'],offline:['content-check','audit']});
add('REG-02',{offline:['modules-registry','audit']});
add('REG-03',{diff:[
  'calcination_oven_be_only_lower','distiller_be_only_lower','lc_block_entity_only_on_lower_half',
  'incubator_placement','incubator_break_lower_destroys_upper','incubator_break_upper_destroys_lower',
]});
add('REG-04',{diff:[
  'mercury_catalyst_rejected_placeable_dont_duplicate','logistics_nexus_be_accepts_item_uuid',
  'sal_ammoniac_tank_persistence','incubator_mercury_vessel_connection',
],modes:['interactions','persistence','logistics-advanced'],offline:['modules-items']});
add('REG-05',{modes:['fluids'],offline:['content-check']});
add('REG-10',{modes:['recipes'],offline:['content-check','audit']});
add('REG-11',{modes:['recipes'],offline:['content-check','audit']});
add('REG-12',{modes:['filters','wand','divination','logistics-advanced'],offline:['modules-registry']});
add('REG-13',{modes:['items'],offline:['content-check','audit']});
add('REG-14',{modes:['client'],offline:['content-check','audit']});
add('REG-15',{modes:['client-features','wand','divination'],offline:['content-check']});
add('REG-16',{modes:['content-rules','filter-matching'],offline:['content-check','audit']});
add('REG-17',{diff:['caloric_flux_emitter_energy','caloric_flux_emitter_enabled'],modes:['review-fixes','flux']});

// Shared crafting implementation: exact unit tests plus the Java state oracle.
add('CB-01',{diff:['calcination_oven_processing_starts','calcination_oven_stops_without_input','calcination_oven_stops_without_heat'],offline:['rust-fast']});
add('CB-02',{diff:['calcination_oven_input_consumed_output_produced','incubator_inputs_consumed_output_produced'],offline:['rust-fast']});
add('CB-03',{diff:['calcination_oven_processing_starts'],offline:['rust-fast']});
add('CB-04',{diff:['calcination_oven_stops_without_input'],offline:['rust-fast']});
add('CB-05',{diff:['calcination_oven_blocked_output_does_not_consume_input'],offline:['rust-fast']});
add('CB-06',{diff:['calcination_oven_input_consumed_output_produced','calcination_oven_blocked_output_does_not_consume_input'],offline:['rust-fast']});
add('CB-07',{diff:['calcination_oven_processing_starts'],offline:['rust-fast']});
add('CB-08',{diff:['calcination_oven_processing_starts'],offline:['rust-fast']});
add('CB-09',{diff:['calcination_oven_stops_without_input'],offline:['rust-fast']});
add('CB-10',{diff:['calcination_oven_input_consumed_output_produced'],offline:['rust-fast']});
add('CB-11',{modes:['persistence','machines','incubation-chaos']});
add('CB-12',{diff:['calcination_oven_processing_starts','incubator_inputs_consumed_output_produced'],modes:['client-features']});
add('CB-13',{diff:['calcination_oven_processing_starts','calcination_oven_input_consumed_output_produced'],offline:['rust-fast']});
add('CB-14',{diff:['calcination_oven_insert_input','calcination_oven_extract_output'],modes:['review-fixes'],offline:['modules-registry']});

// Heat and spagyric apparatus.
add('HEA-01',{diff:['pyromantic_brazier_heat_calcination_oven','pyromantic_brazier_heat_distiller','pyromantic_brazier_heat_liquefaction_cauldron','calcination_oven_stops_without_heat']});
add('HEA-02',{diff:['caloric_flux_emitter_energy','caloric_flux_emitter_enabled'],modes:['flux','persistence']});
add('HEA-03',{diff:['caloric_flux_emitter_energy','caloric_flux_emitter_enabled'],modes:['flux']});
add('HEA-04',{diff:['pyromantic_brazier_insert_fuel','pyromantic_brazier_rejects_non_burnable','pyromantic_brazier_stacked_fuel_shrinks']});
add('HEA-05',{diff:['pyromantic_brazier_fuel_consumed','pyromantic_brazier_lit_state_updates','pyromantic_brazier_stops_heat_no_fuel','pyromantic_brazier_fuel_leaves_remainder']});
add('HEA-06',{diff:['pyromantic_brazier_lit_state_updates'],modes:['persistence'],offline:['modules-heat']});
add('HEA-07',{diff:['caloric_flux_emitter_enabled','caloric_flux_emitter_energy','caloric_flux_emitter_placement','caloric_flux_emitter_redstone'],modes:['flux']});
add('MA-01 MA-02 MA-03',{diff:['calcination_oven_insert_input','calcination_oven_input_consumed_output_produced'],offline:['rust-fast']});
add('MA-04',{diff:['calcination_oven_be_only_lower','calcination_oven_placement','calcination_oven_break_lower_destroys_upper','calcination_oven_break_upper_destroys_lower'],modes:['interactions']});
add('MA-05',{diff:['calcination_oven_insert_input','calcination_oven_extract_output'],modes:['review-fixes'],offline:['modules-registry']});
add('MA-06',{diff:['calcination_oven_processing_starts','calcination_oven_lit_during_processing','calcination_oven_input_consumed_output_produced','calcination_oven_blocked_output_does_not_consume_input','calcination_oven_stops_without_heat','calcination_oven_stops_without_input']});
add('MA-07',{diff:['calcination_oven_drops_items_when_broken']});
add('MA-08',{modes:['render']});
add('MA-20 MA-21',{diff:['distiller_insert_input','distiller_input_consumed_output_produced','distiller_extract_output'],offline:['rust-fast']});
add('MA-22',{diff:['distiller_be_only_lower','distiller_placement','distiller_break_lower_destroys_upper','distiller_break_upper_destroys_lower','distiller_drops_items_when_broken','distiller_input_consumed_output_produced'],modes:['interactions']});
add('MA-23',{modes:['render']});
add('MA-30 MA-31',{diff:['lc_insert_input_item','lc_insert_solvent_fluid','lc_input_consumed_output_produced','lc_processing_requires_solvent'],offline:['rust-fast']});
add('MA-32',{diff:['lc_insert_solvent_fluid','lc_extract_solvent_fluid','lc_insert_input_item','lc_extract_output_item'],offline:['modules-registry']});
add('MA-33',{diff:['lc_block_entity_only_on_lower_half','lc_placement_creates_two_block_structure','lc_breaking_lower_destroys_upper','lc_breaking_upper_destroys_lower','lc_drops_items_when_broken','lc_input_consumed_output_produced']});
add('MA-34',{modes:['render']});
add('MA-40',{diff:['fermentation_vat_insert_input','fermentation_vat_insert_fluid','fermentation_vat_blocked_output_does_not_consume_inputs'],modes:['all','content-rules'],offline:['rust-fast']});
add('MA-41',{diff:['digestion_vat_insert_input','digestion_vat_insert_fluid','digestion_vat_blocked_output_does_not_consume_inputs'],modes:['all'],offline:['rust-fast']});
add('MA-42',{diff:['fermentation_vat_lid_closes','fermentation_vat_has_output_true','fermentation_vat_has_output_false','digestion_vat_lid_closes','fermentation_vat_insert_input','fermentation_vat_insert_fluid'],offline:['modules-registry']});
add('MA-43',{modes:['vat-interactions']});
add('MA-44',{diff:['fermentation_vat_insert_input','fermentation_vat_insert_fluid','fermentation_vat_extract_output','fermentation_vat_blocked_output_does_not_consume_inputs','fermentation_vat_drops_items','fermentation_vat_facing','digestion_vat_insert_input','digestion_vat_insert_fluid','digestion_vat_extract_output','digestion_vat_blocked_output_does_not_consume_inputs','digestion_vat_drops_items','digestion_vat_facing']});

// Sal ammoniac and fluids.
add('SAL-01 SAL-02 SAL-03',{diff:['sal_ammoniac_accumulator_insert_item','sal_ammoniac_accumulator_insert_water'],modes:['fluids'],offline:['rust-fast']});
add('SAL-04',{diff:['sal_ammoniac_accumulator_insert_item','sal_ammoniac_accumulator_insert_water','sal_ammoniac_accumulator_placement'],modes:['fluids'],offline:['modules-registry']});
add('SAL-05',{diff:['sal_ammoniac_tank_insert_fluid','sal_ammoniac_tank_extract_fluid','sal_ammoniac_tank_persistence','sal_ammoniac_tank_placement'],modes:['fluids']});
add('SAL-06',{modes:['render']});

// Mercury catalyst/capacitor/emitters/connectors.
add('FLU-01 FLU-02',{diff:['mercury_catalyst_shard_generates_flux','mercury_catalyst_shard_is_consumed','mercury_catalyst_disabled_still_generates_flux'],modes:['flux'],offline:['modules-flux']});
add('FLU-03',{diff:['mercury_catalyst_rejected_placeable_dont_duplicate','mercury_catalyst_placement','mercury_catalyst_shard_generates_flux']});
add('FLU-04',{diff:['mercury_capacitor_has_correct_capacity','mercury_capacitor_side_modes','mercury_capacitor_placement'],modes:['wand'],offline:['modules-flux']});
add('FLU-05 FLU-06',{diff:['mercury_capacitor_receives_flux','mercury_capacitor_disabled_does_not_receive_flux','mercury_capacitor_no_ping_pong','mercury_capacitor_side_modes'],offline:['modules-flux']});
add('FLU-07',{diff:['mercury_capacitor_starts_empty','mercury_capacitor_side_modes'],modes:['flux','persistence'],offline:['modules-flux']});
add('FLU-08',{diff:['mercury_flux_emitter_energy','mercury_flux_emitter_flux_transfer','mercury_flux_emitter_linking','mercury_flux_emitter_placement'],modes:['flux','review-fixes']});
add('FLU-09',{diff:['mercury_flux_connector_buffer_empty','mercury_flux_connector_exposes_capability','mercury_flux_connector_forwards_flux','mercury_flux_connector_forwards_to_multiple','mercury_flux_connector_placement'],modes:['logistics-advanced']});
add('FLU-10',{diff:['caloric_flux_emitter_energy','sulfuric_flux_emitter_energy','mercury_flux_emitter_energy','mercury_capacitor_has_correct_capacity'],offline:['modules-flux']});
add('FLU-11',{diff:['caloric_flux_emitter_enabled','mercury_flux_emitter_flux_transfer','sulfuric_flux_emitter_linking'],modes:['client-features','flux']});

// Incubator and reformation.
add('INC-01 INC-02 INC-03 INC-04',{diff:['incubator_valid_multiblock','incubator_insert_mercury','incubator_insert_salt','incubator_insert_sulfur','incubator_inputs_consumed_output_produced','incubator_blocked_output_does_not_consume_vessel_inputs','incubator_processing_starts'],modes:['incubation-chaos']});
add('INC-05',{diff:['incubator_removing_vessel_connection','incubator_break_lower_destroys_upper','incubator_break_upper_destroys_lower'],modes:['incubation-chaos']});
add('INC-06',{diff:['incubator_mercury_vessel_connection','incubator_salt_vessel_connection','incubator_sulfur_vessel_connection','incubator_extract_items']});
add('INC-07',{modes:['render']});
add('REF-01 REF-02',{diff:['sulfuric_flux_emitter_processing','sulfuric_flux_emitter_insufficient_mercury_flux'],modes:['reformation'],offline:['modules-reformation']});
add('REF-03',{modes:['reformation'],offline:['modules-reformation']});
add('REF-04 REF-05',{diff:['sulfuric_flux_emitter_placement','sulfuric_flux_emitter_linking','sulfuric_flux_emitter_processing','sulfuric_flux_emitter_energy'],modes:['reformation','review-fixes']});
add('REF-06',{diff:['sulfuric_flux_emitter_insufficient_mercury_flux'],modes:['reformation']});
add('REF-07',{diff:['sulfuric_flux_emitter_linking','sulfuric_flux_emitter_placement'],modes:['review-fixes','reformation']});
add('REF-08',{diff:['reformation_source_insert','reformation_source_extract','reformation_target_insert','reformation_result_extract','reformation_source_placement','reformation_target_placement','reformation_result_placement']});
add('REF-09',{modes:['render'],offline:['modules-reformation']});

// Logistics, wires and filters.
add('LOG-01 LOG-02 LOG-03 LOG-04',{diff:['logistics_item_extractor_targets_in_any_registration_order','logistics_item_extractor_targets_in_any_registration_order_reversed','logistics_item_extractor_transfers_items','logistics_item_connector_restores_attached_target_on_load','logistics_node_only_connects_when_attached_to_nexus'],modes:['logistics','wire-graph']});
add('LOG-05',{diff:['logistics_extractor_placement','logistics_inserter_placement','logistics_item_extractor_transfers_items','logistics_item_connector_restores_attached_target_on_load']});
add('LOG-06',{diff:['logistics_fluid_extracts_world_source','logistics_fluid_connector_restores_attached_target_on_load'],modes:['logistics','logistics-advanced']});
add('LOG-07',{diff:['logistics_node_only_connects_when_attached_to_nexus','logistics_node_placement']});
add('LOG-08',{diff:['logistics_nexus_be_accepts_item_uuid','logistics_nexus_pairs_across_placement'],modes:['render','logistics-advanced']});
add('LOG-09',{diff:['logistics_capability_proxy_fluid_forwarding','logistics_capability_proxy_fluid_round_robin','logistics_capability_proxy_placement'],modes:['logistics-advanced']});
add('LOG-10',{diff:['logistics_capability_proxy_placement'],modes:['logistics-advanced']});
add('LOG-11',{diff:['mercury_flux_connector_forwards_flux','mercury_flux_connector_forwards_to_multiple'],modes:['logistics-advanced']});
add('LOG-12 LOG-13',{modes:['wire-graph','tools'],offline:['modules']});
add('LOG-14',{modes:['filter-ui','filter-matching','attribute-filters','standard-attributes','special-attributes','filters']});
add('LOG-15',{modes:['client-features','wire-graph']});
add('LOG-16',{modes:['wand','multiplayer']});

// Items and tools.
add('ITM-01 ITM-02',{diff:['divination_rod_craft_t1'],modes:['divination','config-attunement']});
add('ITM-03',{diff:['divination_rod_craft_t1','divination_rod_craft_abundant_dirt_sulfur','divination_rod_craft_abundant_copper_sulfur'],modes:['divination','divination-recipes']});
add('ITM-04',{modes:['wand']});
add('ITM-05',{diff:['sal_ammoniac_tank_insert_fluid','sal_ammoniac_tank_extract_fluid'],modes:['fluids']});
add('ITM-06',{modes:['tools','wire-graph'],offline:['modules']});
add('ITM-07',{modes:['filter-ui','filter-matching','attribute-filters','filters']});
add('ITM-08',{modes:['book']});
add('ITM-09',{modes:['client']});
add('ITM-10',{modes:['client-features']});

// Generated content and external-integration replacements.
add('REC-01',{modes:['content-rules','review-fixes']});
add('REC-02',{modes:['all','recipes'],offline:['content-check','audit']});
add('REC-03',{diff:['divination_rod_craft_t1','divination_rod_craft_abundant_dirt_sulfur','divination_rod_craft_abundant_copper_sulfur'],modes:['all'],offline:['content-check','audit']});
add('REC-04',{modes:['worldgen']});
add('GEN-01',{diff:['calcination_oven_drops_items_when_broken','distiller_drops_items_when_broken','fermentation_vat_drops_items'],modes:['interactions'],offline:['content-check','audit']});
add('GEN-02',{modes:['book'],offline:['content-check','audit']});
add('GEN-03',{modes:['tooltips'],offline:['content-check','audit']});
add('GEN-04',{modes:['render'],offline:['content-check','audit']});
add('GEN-05',{modes:['book'],offline:['content-check','audit']});
add('CFG-01',{modes:['config-attunement','tooltips']});
add('CFG-02',{modes:['config-attunement'],offline:['audit']});
add('CFG-03',{modes:['client-features']});
add('NET-01 NET-02',{modes:['multiplayer']});
add('INT-01',{modes:['recipes']});
add('INT-02',{modes:['client-features']});
add('INT-03',{modes:['content-rules','review-fixes','recipes'],offline:['audit']});
add('CLI-01',{modes:['client-features']});
add('CLI-02',{modes:['client-features']});
add('CLI-03',{modes:['tooltips']});

export function featureCoverage(id) {
  return coverage[id]??{diff:[],modes:[],offline:[]};
}
export function hasFeatureCoverage(id) {
  return Object.hasOwn(coverage,id);
}
export const featureCoverageSize=Object.keys(coverage).length;
