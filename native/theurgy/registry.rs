// SPDX-License-Identifier: MIT
use horizon_client::containers::{ModBlockEntity, ModContainer, ModFluidTanks};
use horizon_world::nbt::Compound;

/// All Theurgy blocks that carry a block entity, mirroring BlockEntityRegistry.
const BLOCK_ENTITIES: &[&str] = &[
    "theurgy:calcination_oven",
    "theurgy:pyromantic_brazier",
    "theurgy:liquefaction_cauldron",
    "theurgy:distiller",
    "theurgy:incubator",
    "theurgy:incubator_mercury_vessel",
    "theurgy:incubator_salt_vessel",
    "theurgy:incubator_sulfur_vessel",
    "theurgy:sal_ammoniac_tank",
    "theurgy:sal_ammoniac_accumulator",
    "theurgy:logistics_nexus",
    "theurgy:mercury_catalyst",
    "theurgy:mercury_capacitor",
    "theurgy:caloric_flux_emitter",
    "theurgy:reformation_source_pedestal",
    "theurgy:reformation_target_pedestal",
    "theurgy:reformation_result_pedestal",
    "theurgy:sulfuric_flux_emitter",
    "theurgy:mercury_flux_emitter",
    "theurgy:fermentation_vat",
    "theurgy:digestion_vat",
    "theurgy:logistics_item_inserter",
    "theurgy:logistics_item_extractor",
    "theurgy:logistics_fluid_inserter",
    "theurgy:logistics_fluid_extractor",
    // These two Java blocks have no BE; Horizon uses a data-only native BE as
    // their per-endpoint Wire SavedData record (no slots, tanks or tick logic).
    "theurgy:logistics_connector_node",
    "theurgy:logistics_capability_probe",
    "theurgy:logistics_capability_proxy",
    "theurgy:logistics_mercury_flux_connector",
];

/// All data component types registered by DataComponentRegistry.
const COMPONENT_TYPES: &[&str] = &[
    "mercury_flux_storage",
    "mercury_flux_to_convert",
    "current_mercury_flux_per_tick",
    "divination_settings_allowed_blocks_tag",
    "divination_settings_disallowed_blocks_tag",
    "divination_settings_duration",
    "divination_settings_max_damage",
    "divination_settings_range",
    "divination_settings_tier",
    "divination_settings_allow_attuning",
    "divination_distance",
    "divination_pos",
    "divination_linked_block",
    "divination_linked_tag",
    "wire_end_point",
    "mercurial_wand_item_mode",
    "source_item",
    "source_tag",
    "mercury_catalyst_inventory",
    "filter_items",
    "filter_is_deny_list",
    "filter_respects_data_components",
    "filter_mode",
    "filter_attributes",
    "selected_frequency",
    "logistics_nexus_id",
];

/// Item container layout: total slots, input slots and output slots. Mirrors
/// the item-storage behaviour of each machine block entity (Distillation /
/// Fermentation / output storage behaviours).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Layout {
    /// `inputs` leading slots are inputs, last slot is the output.
    Stack { slots: usize, inputs: usize },
    /// Discrete input/output slot lists.
    Split {
        slots: usize,
        inputs: Vec<usize>,
        outputs: Vec<usize>,
    },
}

fn layout(block: &str) -> Option<Layout> {
    Some(match block {
        // Single input + single output (DistillationStorageBehaviour and kin).
        "theurgy:calcination_oven"
        | "theurgy:distiller"
        | "theurgy:liquefaction_cauldron" => Layout::Stack { slots: 2, inputs: 1 },
        // Three inputs + output (FermentationStorageBehaviour).
        "theurgy:fermentation_vat" | "theurgy:digestion_vat" => Layout::Stack { slots: 4, inputs: 3 },
        // Single-slot machines where the slot is output-like; result pedestal
        // and incubator are outputs-only.
        "theurgy:incubator" | "theurgy:reformation_result_pedestal" => Layout::Split {
            slots: 1,
            inputs: vec![],
            outputs: vec![0],
        },
        "theurgy:pyromantic_brazier"
        | "theurgy:incubator_mercury_vessel"
        | "theurgy:incubator_salt_vessel"
        | "theurgy:incubator_sulfur_vessel"
        | "theurgy:reformation_source_pedestal"
        | "theurgy:reformation_target_pedestal"
        | "theurgy:mercury_catalyst"
        | "theurgy:sal_ammoniac_accumulator" => Layout::Split {
            slots: 1,
            inputs: vec![0],
            outputs: vec![],
        },
        _ => return None,
    })
}

pub fn is_be(block: &str) -> bool {
    BLOCK_ENTITIES.contains(&block)
}

pub fn create(block: &str, entity: &mut Option<(String, Compound)>) {
    if is_be(block) && entity.is_none() {
        *entity = Some((block.to_owned(), Compound::new()));
    }
}

pub fn ticks(block: &str, ticks: &mut bool) {
    if is_be(block) {
        *ticks = true;
    }
}

pub fn container(block: &str) -> Option<ModContainer> {
    let layout = layout(block)?;
    let (slots, inputs, outputs) = match layout {
        Layout::Stack { slots, inputs } => {
            let outputs = if inputs < slots { vec![slots - 1] } else { vec![] };
            (slots, (0..inputs).collect::<Vec<_>>(), outputs)
        }
        Layout::Split {
            slots,
            inputs,
            outputs,
        } => (slots, inputs, outputs),
    };
    Some(ModContainer {
        slots,
        faces: std::array::from_fn(|_| (0..slots).collect()),
        inputs,
        outputs,
        menu: Some(menu_id(block)),
    })
}

fn menu_id(block: &str) -> String {
    format!("theurgy:{}", block.trim_start_matches("theurgy:"))
}

/// Fluid tanks for blocks that carry one (liquefaction cauldron, vats,
/// ammonia tank/accumulator). Capacities in millibuckets.
const TANK_CAPACITY: i32 = 10_000;
/// `SalAmmoniacTankBlockEntity`: `new Tank(FluidType.BUCKET_VOLUME * 2, ...)`.
const SAL_AMMONIAC_TANK_CAPACITY: i32 = 2_000;

pub fn fluids(block: &str) -> Option<ModFluidTanks> {
    let capacity = match block {
        "theurgy:sal_ammoniac_tank" => SAL_AMMONIAC_TANK_CAPACITY,
        "theurgy:liquefaction_cauldron"
        | "theurgy:fermentation_vat"
        | "theurgy:digestion_vat"
        | "theurgy:sal_ammoniac_accumulator" => TANK_CAPACITY,
        _ => return None,
    };
    Some(ModFluidTanks {
        tanks: vec![capacity],
        faces: std::array::from_fn(|_| vec![0]),
        inputs: vec![0],
        outputs: vec![0],
    })
}

pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    match entity.id.as_str() {
        "theurgy:calcination_oven"
        | "theurgy:distiller"
        | "theurgy:liquefaction_cauldron"
        | "theurgy:fermentation_vat"
        | "theurgy:digestion_vat" => crate::game::theurgy_machines::tick(game, pos, entity),
        "theurgy:pyromantic_brazier" => crate::game::theurgy_heat::tick(game, pos, entity),
        "theurgy:mercury_catalyst"
        | "theurgy:mercury_capacitor"
        | "theurgy:caloric_flux_emitter"
        | "theurgy:mercury_flux_emitter" => crate::game::theurgy_flux::tick(game, pos, entity),
        "theurgy:incubator"
        | "theurgy:incubator_mercury_vessel"
        | "theurgy:incubator_salt_vessel"
        | "theurgy:incubator_sulfur_vessel" => crate::game::theurgy_incubation::tick(game, pos, entity),
        "theurgy:sal_ammoniac_accumulator" | "theurgy:sal_ammoniac_tank" => {
            crate::game::theurgy_fluids::tick(game, pos, entity)
        }
        "theurgy:sulfuric_flux_emitter"
        | "theurgy:reformation_source_pedestal"
        | "theurgy:reformation_target_pedestal"
        | "theurgy:reformation_result_pedestal" => {
            crate::game::theurgy_reformation::tick(game, pos, entity)
        }
        id if id.starts_with("theurgy:logistics_") => {
            crate::game::theurgy_logistics::tick(game, pos, entity)
        }
        _ => {}
    }
}

pub fn component_types(types: &mut Vec<String>) {
    for name in COMPONENT_TYPES {
        types.push(format!("theurgy:{name}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_block_has_a_block_entity() {
        for block in BLOCK_ENTITIES {
            assert!(is_be(block), "{block} should be a block entity");
        }
        assert!(!is_be("theurgy:not_a_block"));
        assert!(!is_be("minecraft:chest"));
    }

    #[test]
    fn create_only_when_empty_and_known() {
        let mut entity = None;
        create("minecraft:stone", &mut entity);
        assert!(entity.is_none());
        create("theurgy:distiller", &mut entity);
        assert_eq!(
            entity.as_ref().map(|(id, _)| id.as_str()),
            Some("theurgy:distiller")
        );
        // Does not overwrite an existing entity.
        create("theurgy:calcination_oven", &mut entity);
        assert_eq!(
            entity.as_ref().map(|(id, _)| id.as_str()),
            Some("theurgy:distiller")
        );
    }

    #[test]
    fn ticks_set_only_for_block_entities() {
        let mut t = false;
        ticks("theurgy:fermentation_vat", &mut t);
        assert!(t);
        let mut t2 = false;
        ticks("minecraft:dirt", &mut t2);
        assert!(!t2);
    }

    #[test]
    fn machine_containers_expose_input_and_output() {
        let c = container("theurgy:distiller").expect("distiller container");
        assert_eq!(c.slots, 2);
        assert_eq!(c.inputs, vec![0]);
        assert_eq!(c.outputs, vec![1]);
        assert_eq!(c.menu.as_deref(), Some("theurgy:distiller"));

        let vat = container("theurgy:fermentation_vat").expect("vat container");
        assert_eq!(vat.slots, 4);
        assert_eq!(vat.inputs, vec![0, 1, 2]);
        assert_eq!(vat.outputs, vec![3]);
    }

    #[test]
    fn single_slot_blocks_route_the_right_slot() {
        let catalyst = container("theurgy:mercury_catalyst").expect("catalyst");
        assert_eq!(catalyst.inputs, vec![0]);
        assert!(catalyst.outputs.is_empty());

        let result = container("theurgy:reformation_result_pedestal").expect("result pedestal");
        assert!(result.inputs.is_empty());
        assert_eq!(result.outputs, vec![0]);

        assert!(container("theurgy:logistics_nexus").is_none());
    }

    #[test]
    fn fluid_tanks_only_where_expected() {
        let tank = fluids("theurgy:sal_ammoniac_tank").expect("tank");
        assert_eq!(tank.tanks, vec![SAL_AMMONIAC_TANK_CAPACITY]);
        assert_eq!(tank.inputs, vec![0]);
        assert_eq!(tank.outputs, vec![0]);
        assert!(fluids("theurgy:distiller").is_none());
    }

    #[test]
    fn all_component_types_are_theurgy_namespaced() {
        let mut types = Vec::new();
        component_types(&mut types);
        assert_eq!(types.len(), COMPONENT_TYPES.len());
        assert!(types.iter().all(|t| t.starts_with("theurgy:")));
        assert!(types.contains(&"theurgy:mercury_flux_storage".to_owned()));
        assert!(types.contains(&"theurgy:logistics_nexus_id".to_owned()));
    }
}
