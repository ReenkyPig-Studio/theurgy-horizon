// SPDX-License-Identifier: MIT
// Port of the theurgy apparatus block entity renderers onto Horizon's native
// block entity render hook (native_block_entity_render). The GeckoLib models
// (DistillerModel, CalcinationOvenModel) and the cauldron's baked model are
// carried by the block template, so the draw hook only adds what the carried
// model can't express: the liquefaction cauldron's solvent fluid inside the
// tank. Everything below is pure geometry/state: it computes the quads that
// the host then submits through the draw session. No GL, no I/O.

use horizon_client::containers::ModBlockEntity;
use horizon_world::nbt::{Compound, JavaString, Tag};

use crate::game::theurgy_host as host;

/// Resolution (in mB) a cauldron tank's fill fraction is measured against when
/// the entity does not carry its own capacity. Matches the registry's chosen
/// capacity for `theurgy:liquefaction_cauldron` (host.rs::fluids).
pub const CAULDRON_CAPACITY: i32 = 10_000;

/// Vertical extent of the solvent model, from the Java renderer:
/// the fluid sits between y=0.25 and y=0.25 + fillRatio * 0.875.
const FLUID_BOTTOM_Y: f32 = 0.25;
const FLUID_HEIGHT: f32 = 0.875;
/// Horizontal inset of the fluid from the block's edges.
const FLUID_INSET: f32 = 0.25;

/// A block entity that carries a GeckoLib/JSON model only needs the host to
/// submit the carried model; this reports whether `block` is one of them.
pub fn carries_model(block: &str) -> bool {
    matches!(
        block,
        "theurgy:calcination_oven" | "theurgy:distiller"
    )
}

/// The fluid render state of a liquefaction cauldron, mirroring the Java
/// `LiquefactionCauldronRenderState`. `empty` short-circuits the submit path.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct CauldronRenderState {
    pub empty: bool,
    pub color: u32,
    pub fluid_light: i32,
    pub bottom_y: f32,
    pub surface_y: f32,
    pub u0: f32,
    pub u1: f32,
    pub v0: f32,
    pub v1: f32,
    pub side_v0: f32,
}

impl CauldronRenderState {
    fn neutral() -> Self {
        CauldronRenderState {
            empty: true,
            color: 0xFFFF_FFFF,
            fluid_light: 0,
            bottom_y: 0.0,
            surface_y: 0.0,
            u0: 0.0,
            u1: 1.0,
            v0: 0.0,
            v1: 1.0,
            side_v0: 0.0,
        }
    }
}

/// The state a cauldron renderer needs, namespaced to the render hook payload.
/// `Sprite` bounds are kept as plain floats; sampling the atlas is host work.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteBounds {
    pub u0: f32,
    pub u1: f32,
    pub v0: f32,
    pub v1: f32,
}

impl SpriteBounds {
    pub const FULL: SpriteBounds = SpriteBounds {
        u0: 0.0,
        u1: 1.0,
        v0: 0.0,
        v1: 1.0,
    };
}

impl Default for SpriteBounds {
    fn default() -> Self {
        SpriteBounds::FULL
    }
}

/// What `host::tank` reads: the entity's first tank and its capacity.
fn solvent_tank(entity: &ModBlockEntity) -> theurgy_engine::recipes::Tank {
    host::tank(entity, 0, CAULDRON_CAPACITY)
}

/// Extracts the cauldron render state from a client block entity, the way the
/// Java renderer's `extractRenderState` does. `light` is the packed block+sky
/// light of the entity (`BlockEntityRenderState.lightCoords`); `luminosity`
/// is the fluid type's light level for the carried stack (host-provided, since
/// fluid definitions live in the resources). `color` is the fluid's tint.
pub fn extract_cauldron(
    entity: &ModBlockEntity,
    light: i32,
    luminosity: i32,
    color: u32,
    sprite: SpriteBounds,
) -> CauldronRenderState {
    let tank = solvent_tank(entity);
    let Some(_fluid) = tank.fluid.as_deref() else {
        return CauldronRenderState::neutral();
    };
    if tank.amount <= 0 {
        return CauldronRenderState::neutral();
    }

    let capacity = tank.capacity.max(1);
    let fill_ratio = (tank.amount as f32 / capacity as f32).clamp(0.0, 1.0);

    let block_light_in = (light >> 4) & 0xF;
    let lit = block_light_in.max(luminosity.clamp(0, 0xF));

    let bottom_y = FLUID_BOTTOM_Y;
    let surface_y = bottom_y + fill_ratio * FLUID_HEIGHT;
    let side_v0 = lerp(1.0 - fill_ratio, sprite.v0, sprite.v1);

    CauldronRenderState {
        empty: false,
        color,
        fluid_light: (light & 0xF0_0000) | (lit << 4),
        bottom_y,
        surface_y,
        u0: sprite.u0,
        u1: sprite.u1,
        v0: sprite.v0,
        v1: sprite.v1,
        side_v0,
    }
}

fn lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + (end - start) * delta
}

/// A fully resolved quad to hand to `native_draw_quad`: corner positions in
/// block-local space, UVs, ARGB colour, packed light and a face normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub positions: [[f32; 3]; 4],
    pub uvs: [[f32; 2]; 4],
    pub color: u32,
    pub light: i32,
    pub normal: [f32; 3],
}

const UP: [f32; 3] = [0.0, 1.0, 0.0];
const NORTH: [f32; 3] = [0.0, 0.0, -1.0];
const SOUTH: [f32; 3] = [0.0, 0.0, 1.0];
const WEST: [f32; 3] = [-1.0, 0.0, 0.0];
const EAST: [f32; 3] = [1.0, 0.0, 0.0];

/// Builds the five quads of the cauldron's solvent surface and walls. Returns
/// nothing when the tank is empty; the geometry otherwise matches
/// `LiquefactionCauldronRenderer.submit` exactly (winding included).
pub fn cauldron_quads(state: &CauldronRenderState) -> Vec<Quad> {
    if state.empty {
        return Vec::new();
    }
    let min = FLUID_INSET;
    let max = 1.0 - min;
    let y_min = state.bottom_y;
    let y_max = state.surface_y;

    let mut quads = Vec::with_capacity(5);

    // Top face, carved in the Java order.
    quads.push(Quad {
        positions: [
            [min, y_max, min],
            [min, y_max, max],
            [max, y_max, max],
            [max, y_max, min],
        ],
        uvs: [
            [state.u0, state.v0],
            [state.u0, state.v1],
            [state.u1, state.v1],
            [state.u1, state.v0],
        ],
        color: state.color,
        light: state.fluid_light,
        normal: UP,
    });

    // Side faces (north, south, west, east), bottom at y_min, top at y_max.
    let side = |normal: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]| Quad {
        positions: [a, b, c, d],
        uvs: [
            [state.u0, state.v1],
            [state.u0, state.side_v0],
            [state.u1, state.side_v0],
            [state.u1, state.v1],
        ],
        color: state.color,
        light: state.fluid_light,
        normal,
    };

    quads.push(side(
        NORTH,
        [min, y_min, min],
        [min, y_max, min],
        [max, y_max, min],
        [max, y_min, min],
    ));
    quads.push(side(
        SOUTH,
        [max, y_min, max],
        [max, y_max, max],
        [min, y_max, max],
        [min, y_min, max],
    ));
    quads.push(side(
        WEST,
        [min, y_min, max],
        [min, y_max, max],
        [min, y_max, min],
        [min, y_min, min],
    ));
    quads.push(side(
        EAST,
        [max, y_min, min],
        [max, y_max, min],
        [max, y_max, max],
        [max, y_min, max],
    ));

    quads
}

/// Reads the fluid colour from the entity's stored tank stack when it carries
/// one (`FluidStack.getFluidType` tint), otherwise returns `fallback`.
pub fn cauldron_color(entity: &ModBlockEntity, fallback: u32) -> u32 {
    let tank = solvent_tank(entity);
    match tank.fluid {
        Some(_) => {
            // The fluid's tint lives in resources; the entity itself carries
            // only the fluid id and amount. Reuse the stored `color` component
            // when the host wrote one, else keep the host-provided fallback.
            host::int(&entity.nbt, "fluidColor", fallback as i32) as u32
        }
        None => fallback,
    }
}

/// Reads the entity tag written by a compatible host, preferring the full
/// sprite when none was provided.
pub fn cauldron_sprite(entity: &ModBlockEntity) -> SpriteBounds {
    let Some(Tag::Compound(map)) = entity.nbt.get(&JavaString::from("FluidSprite")) else {
        return SpriteBounds::FULL;
    };
    let read = |key: &str| -> f32 {
        match map.get(&JavaString::from(key)) {
            Some(Tag::Float(v)) => f32::from_bits(*v),
            Some(Tag::Double(v)) => *v as f32,
            Some(Tag::Int(v)) => *v as f32,
            _ => 0.0,
        }
    };
    SpriteBounds {
        u0: read("u0"),
        u1: read("u1"),
        v0: read("v0"),
        v1: read("v1"),
    }
}

/// Renders the cauldron solvent by submitting each quad to `draw`, or does
/// nothing for any block that isn't the liquefaction cauldron.
pub fn render(
    block: &str,
    entity: &ModBlockEntity,
    light: i32,
    luminosity: i32,
    fallback_color: u32,
    mut draw: impl FnMut(&Quad),
) {
    if block != "theurgy:liquefaction_cauldron" {
        return;
    }
    let color = cauldron_color(entity, fallback_color);
    let sprite = cauldron_sprite(entity);
    let state = extract_cauldron(entity, light, luminosity, color, sprite);
    for quad in cauldron_quads(&state) {
        draw(&quad);
    }
}

/// The cauldron model's bounding box in block-local space. One block tall,
/// matching the carried template, expanded slightly to enclose the fluid.
pub fn cauldron_bounding_box() -> ([f32; 3], [f32; 3]) {
    ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])
}

/// Client update tag with the same item-template shape used by Java's
/// `ItemStack.OPTIONAL_CODEC`; client item slots otherwise stay empty.
pub fn update_tag(
    _pos: [i32; 3],
    entity: &ModBlockEntity,
    tag: &mut Option<Compound>,
) {
    let mut nbt = entity.nbt.clone();
    let items = entity.items.iter().enumerate().filter_map(|(slot, stack)| {
        let stack = stack.as_ref()?;
        let Tag::Compound(mut record) = crate::game::theurgy_logistics::stack_tag(stack) else {
            return None;
        };
        record.insert(JavaString::from("Slot"), Tag::Byte(slot as i8));
        Some(Tag::Compound(record))
    }).collect();
    nbt.insert(JavaString::from("Items"), Tag::List(items));
    *tag = Some(nbt);
}

/// Submit carried fluid geometry. GeckoLib model submission is handled by the
/// same entry point below; only the cauldron adds quads here.
pub fn submit(
    game: &crate::game::Game<'_>,
    _pos: [i32; 3],
    block: &str,
    _state: horizon_world::registry::BlockStateId,
    entity: &ModBlockEntity,
    draw: &mut crate::game::native_block_entity_draw::BlockEntityDraw,
) {
    if block != "theurgy:liquefaction_cauldron" {
        return;
    }
    let color = cauldron_color(entity, 0xFFFF_FFFF);
    let sprite = cauldron_sprite(entity);
    let state = extract_cauldron(entity, draw.light as i32, 0, color, sprite);
    draw.light = state.fluid_light as u32;
    for quad in cauldron_quads(&state) {
        game.native_draw_quad(
            draw,
            "theurgy:block/sal_ammoniac/still",
            quad.positions,
            [state.u0 * 16.0, state.v0 * 16.0, state.u1 * 16.0, state.v1 * 16.0],
            0xFFFF_FFFF,
        );
    }
}

pub fn bounds(
    _pos: [i32; 3],
    block: &str,
    _state: horizon_world::registry::BlockStateId,
    _entity: &ModBlockEntity,
    bounds: &mut [f32; 6],
) {
    if matches!(block, "theurgy:distiller" | "theurgy:calcination_oven" | "theurgy:liquefaction_cauldron") {
        *bounds = [-1.0, -1.0, -1.0, 2.0, 3.0, 2.0];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity_with(fluid: Option<(&str, i32)>, color: Option<u32>) -> ModBlockEntity {
        let mut nbt = Compound::new();
        if let Some((id, amount)) = fluid {
            let mut tank = Compound::new();
            tank.insert(JavaString::from("Fluid"), Tag::String(JavaString::from(id)));
            tank.insert(JavaString::from("Amount"), Tag::Int(amount));
            nbt.insert(JavaString::from("fluids"), Tag::List(vec![Tag::Compound(tank)]));
        }
        if let Some(c) = color {
            nbt.insert(JavaString::from("fluidColor"), Tag::Int(c as i32));
        }
        ModBlockEntity {
            id: "theurgy:liquefaction_cauldron".to_owned(),
            nbt,
            items: Vec::new(),
            data: Vec::new(),
        }
    }

    #[test]
    fn empty_tank_yields_no_quads() {
        let e = entity_with(None, None);
        let s = extract_cauldron(&e, 0xF0, 0, 0xFFFF_FFFF, SpriteBounds::FULL);
        assert!(s.empty);
        assert!(cauldron_quads(&s).is_empty());
    }

    #[test]
    fn zero_amount_yields_no_quads() {
        let e = entity_with(Some(("theurgy:sal_ammoniac", 0)), None);
        let s = extract_cauldron(&e, 0xF0, 0, 0xFFFF_FFFF, SpriteBounds::FULL);
        assert!(s.empty);
        assert!(cauldron_quads(&s).is_empty());
    }

    #[test]
    fn half_full_sits_between_bottom_and_midpoint() {
        let e = entity_with(Some(("theurgy:sal_ammoniac", CAULDRON_CAPACITY / 2)), None);
        let s = extract_cauldron(&e, 0xF0, 0, 0xFFFF_FFFF, SpriteBounds::FULL);
        assert!(!s.empty);
        assert!((s.bottom_y - FLUID_BOTTOM_Y).abs() < 1e-6);
        assert!((s.bottom_y + FLUID_HEIGHT * 0.5 - s.surface_y).abs() < 1e-4);
        let quads = cauldron_quads(&s);
        assert_eq!(quads.len(), 5);
        // Top quad corners share the surface height.
        for p in quads[0].positions {
            assert!((p[1] - s.surface_y).abs() < 1e-6);
        }
    }

    #[test]
    fn overfull_is_clamped_and_light_combines() {
        let e = entity_with(Some(("theurgy:sal_ammoniac", CAULDRON_CAPACITY + 500)), None);
        let light = 0x0000_0500; // block light 5, sky 0
        let s = extract_cauldron(&e, light, 15, 0xFFFF_FFFF, SpriteBounds::FULL);
        assert!((s.surface_y - (FLUID_BOTTOM_Y + FLUID_HEIGHT)).abs() < 1e-4);
        assert_eq!((s.fluid_light >> 4) & 0xF, 15);
    }

    #[test]
    fn side_texture_moves_with_fill() {
        let sprite = SpriteBounds {
            u0: 0.1,
            u1: 0.2,
            v0: 0.0,
            v1: 1.0,
        };
        let e = entity_with(Some(("theurgy:sal_ammoniac", CAULDRON_CAPACITY / 4)), None);
        let s = extract_cauldron(&e, 0, 0, 0xFFFF_FFFF, sprite);
        assert!((s.side_v0 - 0.75).abs() < 1e-4);
        assert!((s.v1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn color_falls_back_to_host_argument() {
        let e = entity_with(Some(("theurgy:sal_ammoniac", 100)), None);
        assert_eq!(cauldron_color(&e, 0x11223344), 0x11223344);
        let e = entity_with(Some(("theurgy:sal_ammoniac", 100)), Some(0xAABBCCDD));
        assert_eq!(cauldron_color(&e, 0), 0xAABBCCDD);
    }

    #[test]
    fn sprite_reads_from_nbt() {
        let mut nbt = Compound::new();
        let mut sprite = Compound::new();
        sprite.insert(JavaString::from("u0"), Tag::Float(0.125f32.to_bits()));
        sprite.insert(JavaString::from("u1"), Tag::Float(0.25f32.to_bits()));
        sprite.insert(JavaString::from("v0"), Tag::Double(0.5));
        sprite.insert(JavaString::from("v1"), Tag::Float(0.75f32.to_bits()));
        nbt.insert(JavaString::from("FluidSprite"), Tag::Compound(sprite));
        let e = ModBlockEntity {
            id: "theurgy:liquefaction_cauldron".to_owned(),
            nbt,
            items: Vec::new(),
            data: Vec::new(),
        };
        let s = cauldron_sprite(&e);
        assert!((s.u0 - 0.125).abs() < 1e-6);
        assert!((s.u1 - 0.25).abs() < 1e-6);
        assert!((s.v0 - 0.5).abs() < 1e-6);
        assert!((s.v1 - 0.75).abs() < 1e-6);
    }

    #[test]
    fn models_are_recognized() {
        assert!(carries_model("theurgy:distiller"));
        assert!(carries_model("theurgy:calcination_oven"));
        assert!(!carries_model("theurgy:liquefaction_cauldron"));
    }

    #[test]
    fn render_only_draws_for_cauldron() {
        let e = entity_with(Some(("theurgy:sal_ammoniac", CAULDRON_CAPACITY)), None);
        let mut count = 0;
        render("theurgy:distiller", &e, 0, 0, 0xFFFF_FFFF, |_| count += 1);
        assert_eq!(count, 0);
        render(
            "theurgy:liquefaction_cauldron",
            &e,
            0,
            0,
            0xFFFF_FFFF,
            |_| count += 1,
        );
        assert_eq!(count, 5);
    }

    #[test]
    fn bounding_box_covers_block() {
        let (min, max) = cauldron_bounding_box();
        assert_eq!(min, [0.0, 0.0, 0.0]);
        assert_eq!(max, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn neutral_state_never_draws() {
        let s = CauldronRenderState::neutral();
        assert!(s.empty);
        assert!(cauldron_quads(&s).is_empty());
    }
}
