fn hook(models: &mut Vec<String>) {
    for id in [
        "calcination_oven", "distiller", "incubator_vessel_iron",
        "incubator_vessel_gold", "incubator_vessel_bronze", "logistics_nexus",
        "sal_ammoniac_accumulator", "sal_ammoniac_tank",
    ] {
        models.push(format!("theurgy:block/geckolib_atlas/{id}"));
    }
    // DigestionVatRenderer textures (tools/digestion-vat.mjs).
    for id in [
        "digestion_vat_base", "digestion_vat_base_open", "digestion_vat_side",
        "digestion_vat_side_front", "digestion_vat_side_front_active",
    ] {
        models.push(format!("theurgy:block/digestion_vat_atlas/{id}"));
    }
}
