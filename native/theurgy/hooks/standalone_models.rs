fn hook(models: &mut Vec<String>) {
    for id in [
        "calcination_oven", "distiller", "incubator_vessel_iron",
        "incubator_vessel_gold", "incubator_vessel_bronze", "logistics_nexus",
        "sal_ammoniac_accumulator", "sal_ammoniac_tank",
    ] {
        models.push(format!("theurgy:block/geckolib_atlas/{id}"));
    }
}
