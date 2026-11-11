// SPDX-License-Identifier: MIT
// Sal ammoniac in the section mesher. Java: Theurgy.java onRegisterFluidModels
// registers a FluidModel (still/flowing/overlay, constant tint), so vanilla's
// FluidRenderer.tesselate draws the LiquidBlock with averaged corner heights.
// Horizon's port of it (`mesher::fluid::tesselate`) knows only water and lava,
// so it runs here over a copy of the mesh tables in which sal ammoniac is the
// only fluid and the water sprites are sal ammoniac's.
use super::super::{MeshTables, Mesher, Padded, SectionMesh, pi};
use horizon_world::{
    fluid::{FluidKind, FluidState},
    registry::BlockStateId,
};
use std::{cell::Cell, ops::RangeInclusive, sync::{Arc, Mutex}};

const SAL_AMMONIAC: &str = "theurgy:sal_ammoniac";
/// FluidTypeRegistry.SAL_AMMONIAC tint 0xFFFF00FF, without the alpha.
const TINT: u32 = 0xFF_00FF;

/// What the tables were made from: the real tables' address, size and water sprite.
type Key = (usize, usize, u16);
/// The tables and the span of state ids sal ammoniac lies in.
type Sal = (Arc<MeshTables>, RangeInclusive<u32>);
static TABLES: Mutex<Option<(Key, Option<Sal>)>> = Mutex::new(None);

thread_local! {
    /// `Mesher::mesh_reusing` is running under `Wrapped` on this thread.
    static WRAPPED: Cell<bool> = const { Cell::new(false) };
}

/// Held around the original `mesh_reusing` so its HEAD hook lets it run.
pub(crate) struct Wrapped;
impl Wrapped {
    /// None when already inside: the hook then continues into the original.
    pub(crate) fn enter() -> Option<Self> {
        (!WRAPPED.replace(true)).then_some(Self)
    }
}
impl Drop for Wrapped {
    fn drop(&mut self) {
        WRAPPED.set(false);
    }
}

/// LiquidBlock's `level` as a fluid state (FlowingFluid.getLegacyLevel):
/// 0 the source, 1..=7 flowing with amount 8 - level, 8.. falling.
pub(crate) fn level_fluid(level: u8) -> FluidState {
    match level {
        0 => FluidState::WATER,
        1..=7 => FluidState::new(FluidKind::FlowingWater, 8 - level, false),
        _ => FluidState::new(FluidKind::FlowingWater, 8, true),
    }
}

fn tables(mesher: &Mesher<'_>) -> Option<Sal> {
    let real = mesher.tables;
    let key = (std::ptr::from_ref(real) as usize, real.info.len(), real.water_still);
    let mut cache = TABLES.lock().ok()?;
    if let Some((_, sal)) = cache.as_ref().filter(|(k, _)| *k == key) {
        return sal.clone();
    }
    let sal = build(mesher);
    *cache = Some((key, sal.clone()));
    sal
}

fn build(mesher: &Mesher<'_>) -> Option<Sal> {
    let real = mesher.tables;
    let layer = |name: &str| mesher.render.sprite_layers.get(name).map(|&l| l as u16);
    let still = layer("theurgy:block/sal_ammoniac/still")?;
    let flow = layer("theurgy:block/sal_ammoniac/flow").unwrap_or(still);
    let mut info = real.info.clone();
    let mut ids: Option<(u32, u32)> = None;
    for (id, state) in info.iter_mut().enumerate() {
        let id = id as u32;
        state.fluid = match mesher.assets.registry.state(BlockStateId(id)).filter(|s| s.name == SAL_AMMONIAC) {
            Some(s) => {
                ids = Some(ids.map_or((id, id), |(lo, _)| (lo, id)));
                level_fluid(s.properties.get("level").and_then(|v| v.parse().ok()).unwrap_or(0))
            }
            None => FluidState::EMPTY,
        };
    }
    let (lo, hi) = ids?;
    let tables = MeshTables {
        info,
        // `tesselate` reads only `info`, the water sprites and `animated_layers`.
        solid: Vec::new(),
        full_faces: Vec::new(),
        visibility: real.visibility.clone(),
        water_still: still,
        water_flow: flow,
        water_overlay: layer("theurgy:block/sal_ammoniac/overlay").unwrap_or(flow),
        lava_still: real.lava_still,
        lava_flow: real.lava_flow,
        animated_layers: real.animated_layers.clone(),
    };
    Some((Arc::new(tables), lo..=hi))
}

/// Adds the section's sal ammoniac to `out` as FluidRenderer draws a fluid.
/// `pad` is the section `mesh_reusing` just meshed.
pub(crate) fn add(mesher: &Mesher<'_>, pad: &Padded, out: &mut SectionMesh) {
    let Some((tables, ids)) = tables(mesher) else { return };
    let shade = (mesher.nether_shading, mesher.shader_pack_lighting, mesher.separate_ao);
    let mut any = false;
    for y in 0..16 {
        for z in 0..16 {
            for x in 0..16 {
                let id = pad.blocks[pi(x, y, z)];
                if ids.contains(&id) && tables.info[id as usize].fluid.kind() != FluidKind::Empty {
                    super::tesselate(&tables, shade, pad, TINT, out, [x, y, z]);
                    any = true;
                }
            }
        }
    }
    if any {
        out.finish_sprite_layers();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_is_the_liquid_block_legacy_level() {
        assert!(level_fluid(0).is_source());
        assert_eq!((level_fluid(1).amount(), level_fluid(1).falling()), (7, false));
        assert_eq!((level_fluid(7).amount(), level_fluid(7).falling()), (1, false));
        assert_eq!((level_fluid(8).amount(), level_fluid(8).falling()), (8, true));
        assert!(!level_fluid(8).is_source());
    }
}
