// Placement/default-state and simple direct-storage behaviours from the Java
// GameTests. More involved processing/network scenarios live in their own files.
import {at, record, setblock, container, merge, wait} from '../differential/harness.mjs';

const tests = [
  ['theurgy:caloric_flux_emitter_placement', 'theurgy:caloric_flux_emitter[facing=up]'],
  ['theurgy:caloric_flux_emitter_enabled', 'theurgy:caloric_flux_emitter'],
  ['theurgy:caloric_flux_emitter_energy', 'theurgy:caloric_flux_emitter'],
  ['theurgy:digestion_vat_placement', 'theurgy:digestion_vat'],
  ['theurgy:digestion_vat_facing', 'theurgy:digestion_vat[facing=east]'],
  ['theurgy:fermentation_vat_placement', 'theurgy:fermentation_vat'],
  ['theurgy:fermentation_vat_facing', 'theurgy:fermentation_vat[facing=south]'],
  ['theurgy:logistics_extractor_placement', 'theurgy:logistics_item_extractor[facing=north]', [4, 2, 2]],
  ['theurgy:logistics_inserter_placement', 'theurgy:logistics_item_inserter[facing=north]', [3, 2, 2]],
  ['theurgy:logistics_node_placement', 'theurgy:logistics_connector_node[facing=north]'],
  ['theurgy:mercury_capacitor_placement', 'theurgy:mercury_capacitor', [2, 1, 2]],
  ['theurgy:mercury_capacitor_starts_empty', 'theurgy:mercury_capacitor', [2, 1, 2]],
  ['theurgy:mercury_capacitor_has_correct_capacity', 'theurgy:mercury_capacitor', [2, 1, 2]],
  ['theurgy:mercury_capacitor_side_modes', 'theurgy:mercury_capacitor', [2, 1, 2]],
  ['theurgy:mercury_catalyst_placement', 'theurgy:mercury_catalyst', [2, 1, 2]],
  ['theurgy:mercury_flux_connector_placement', 'theurgy:logistics_mercury_flux_connector[facing=west]'],
  ['theurgy:mercury_flux_connector_buffer_empty', 'theurgy:logistics_mercury_flux_connector[facing=west]'],
  ['theurgy:mercury_flux_connector_exposes_capability', 'theurgy:logistics_mercury_flux_connector[facing=west]', [2, 2, 2], '{buffer:{energy:500}}'],
  ['theurgy:mercury_flux_emitter_placement', 'theurgy:mercury_flux_emitter[facing=up]'],
  ['theurgy:mercury_flux_emitter_energy', 'theurgy:mercury_flux_emitter'],
  ['theurgy:pyromantic_brazier_placement', 'theurgy:pyromantic_brazier', [2, 1, 2]],
  ['theurgy:reformation_source_placement', 'theurgy:reformation_source_pedestal'],
  ['theurgy:reformation_target_placement', 'theurgy:reformation_target_pedestal', [3, 2, 2]],
  ['theurgy:reformation_result_placement', 'theurgy:reformation_result_pedestal', [4, 2, 2]],
  ['theurgy:sal_ammoniac_accumulator_placement', 'theurgy:sal_ammoniac_accumulator'],
  ['theurgy:sal_ammoniac_accumulator_insert_item', 'theurgy:sal_ammoniac_accumulator', [2, 2, 2], null, 'theurgy:sal_ammoniac_crystal'],
  ['theurgy:sal_ammoniac_accumulator_insert_water', 'theurgy:sal_ammoniac_accumulator', [2, 2, 2], '{fluids:[{Fluid:"minecraft:water",Amount:1000}]}'],
  ['theurgy:sal_ammoniac_tank_placement', 'theurgy:sal_ammoniac_tank'],
  ['theurgy:sal_ammoniac_tank_insert_fluid', 'theurgy:sal_ammoniac_tank', [2, 2, 2], '{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:1000}]}'],
  ['theurgy:sal_ammoniac_tank_extract_fluid', 'theurgy:sal_ammoniac_tank', [2, 2, 2], '{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:500}]}'],
  ['theurgy:sal_ammoniac_tank_persistence', 'theurgy:sal_ammoniac_tank', [2, 2, 2], '{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:1500}]}'],
  ['theurgy:sulfuric_flux_emitter_placement', 'theurgy:sulfuric_flux_emitter[facing=up]', [2, 1, 2]],
  ['theurgy:sulfuric_flux_emitter_energy', 'theurgy:sulfuric_flux_emitter', [2, 1, 2]],
];

export async function runCustom(call) {
  const out = [];
  let index = 0;
  for (const [id, block, local = [2, 2, 2], nbt = null, item = null] of tests) {
    const origin = [(index++) * 16, 80, 0];
    await call(['fill', String(origin[0] - 4), '78', '-4', String(origin[0] + 4), '86', '4', 'minecraft:air']);
    await setblock(call, at(origin, local), block);
    if (nbt) await merge(call, at(origin, local), nbt);
    if (item) await container(call, at(origin, local), item, 1);
    await record(out, id, call, origin, [local]);
  }
  // Capability proxy placement also places the east-facing probe.
  {
    const origin = [(index++) * 16, 80, 0];
    await setblock(call, at(origin), 'theurgy:logistics_capability_proxy[facing=north]');
    await setblock(call, at(origin, [3, 2, 2]), 'theurgy:logistics_capability_probe[facing=east]');
    await record(out, 'theurgy:logistics_capability_proxy_placement', call, origin, [[2, 2, 2], [3, 2, 2]]);
  }
  // Incubator's double-high placement and six false connector properties.
  {
    const origin = [(index++) * 16, 80, 0];
    await setblock(call, at(origin), 'theurgy:incubator');
    await setblock(call, at(origin, [2, 3, 2]), 'theurgy:incubator[half=upper]');
    await record(out, 'theurgy:incubator_placement', call, origin, [[2, 2, 2], [2, 3, 2]]);
  }
  {
    const origin = [(index++) * 16, 80, 0];
    await setblock(call, at(origin), 'theurgy:incubator');
    await setblock(call, at(origin, [2, 3, 2]), 'theurgy:incubator[half=upper]');
    await record(out, 'theurgy:incubator_connection_states_default_false', call, origin, [[2, 2, 2], [2, 3, 2]]);
  }
  return out;
}

export default {name: 'differential-placements', runCustom};
