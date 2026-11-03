// SPDX-License-Identifier: MIT
use horizon_client::containers::{ModContainer, ModFluidTanks, ModBlockEntity};
use horizon_world::nbt::Compound;

fn theurgy_is_be(block: &str) -> bool {
    matches!(block,
        "theurgy:calcination_oven" | "theurgy:pyromantic_brazier" |
        "theurgy:liquefaction_cauldron" | "theurgy:distiller" | "theurgy:incubator" |
        "theurgy:incubator_mercury_vessel" | "theurgy:incubator_salt_vessel" |
        "theurgy:incubator_sulfur_vessel" | "theurgy:sal_ammoniac_tank" |
        "theurgy:sal_ammoniac_accumulator" | "theurgy:logistics_nexus" |
        "theurgy:mercury_catalyst" | "theurgy:mercury_capacitor" |
        "theurgy:caloric_flux_emitter" | "theurgy:reformation_source_pedestal" |
        "theurgy:reformation_target_pedestal" | "theurgy:reformation_result_pedestal" |
        "theurgy:sulfuric_flux_emitter" | "theurgy:mercury_flux_emitter" |
        "theurgy:fermentation_vat" | "theurgy:digestion_vat" |
        "theurgy:logistics_item_inserter" | "theurgy:logistics_item_extractor" |
        "theurgy:logistics_fluid_inserter" | "theurgy:logistics_fluid_extractor" |
        "theurgy:logistics_capability_proxy" | "theurgy:logistics_mercury_flux_connector")
}

fn theurgy_block_entity_create(game: &mut crate::game::Game<'_>, block: &str,
    entity: &mut Option<(String, Compound)>) {
    let _ = game;
    if theurgy_is_be(block) && entity.is_none() {
        *entity = Some((block.to_owned(), Compound::new()));
    }
}
fn theurgy_block_entity_ticks(game: &crate::game::Game<'_>, block: &str, ticks: &mut bool) {
    let _ = game;
    if theurgy_is_be(block) { *ticks = true; }
}
fn theurgy_container_spec(block: &str) -> Option<ModContainer> {
    let (slots, input_count, has_menu) = match block {
        "theurgy:calcination_oven" | "theurgy:distiller" | "theurgy:liquefaction_cauldron" => (2, 1, true),
        "theurgy:fermentation_vat" | "theurgy:digestion_vat" => (4, 3, true),
        "theurgy:pyromantic_brazier" | "theurgy:incubator" |
        "theurgy:incubator_mercury_vessel" | "theurgy:incubator_salt_vessel" |
        "theurgy:incubator_sulfur_vessel" | "theurgy:reformation_source_pedestal" |
        "theurgy:reformation_target_pedestal" | "theurgy:reformation_result_pedestal" |
        "theurgy:mercury_catalyst" => (1, usize::from(block != "theurgy:incubator" && block != "theurgy:reformation_result_pedestal"), true),
        _ => return None,
    };
    let inputs: Vec<usize> = (0..input_count).collect();
    let outputs = if input_count < slots { vec![slots - 1] } else { Vec::new() };
    Some(ModContainer { slots, faces: [(0..slots).collect(); 6], inputs, outputs,
        menu: has_menu.then(|| format!("theurgy:{}", block.trim_start_matches("theurgy:"))) })
}
fn theurgy_block_entity_container(game: &crate::game::Game<'_>, block: &str,
    container: &mut Option<ModContainer>) {
    let _ = game;
    if container.is_none() { *container = theurgy_container_spec(block); }
}
fn theurgy_block_entity_fluids(game: &crate::game::Game<'_>, block: &str,
    fluids: &mut Option<ModFluidTanks>) {
    let _ = game;
    let capacity = match block {
        "theurgy:liquefaction_cauldron" | "theurgy:fermentation_vat" |
        "theurgy:digestion_vat" | "theurgy:sal_ammoniac_accumulator" |
        "theurgy:sal_ammoniac_tank" => 10_000,
        _ => return,
    };
    if fluids.is_none() {
        *fluids = Some(ModFluidTanks { tanks: vec![capacity], faces: [vec![0]; 6],
            inputs: vec![0], outputs: vec![0] });
    }
}
fn theurgy_tick_adapter(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    theurgy_tick(game, pos, entity);
}
fn theurgy_component_types(types: &mut Vec<String>) {
    for name in ["mercury_flux_storage", "mercury_flux_to_convert", "current_mercury_flux_per_tick",
        "divination_settings_allowed_blocks_tag", "divination_settings_disallowed_blocks_tag",
        "divination_settings_duration", "divination_settings_max_damage", "divination_settings_range",
        "divination_settings_tier", "divination_settings_allow_attuning", "divination_distance",
        "divination_pos", "divination_linked_block", "divination_linked_tag", "wire_end_point",
        "mercurial_wand_item_mode", "source_item", "source_tag", "mercury_catalyst_inventory",
        "filter_items", "filter_is_deny_list", "filter_respects_data_components", "filter_mode",
        "filter_attributes", "selected_frequency", "logistics_nexus_id"] {
        types.push(format!("theurgy:{name}"));
    }
}
