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
use horizon_assets::item_model::DisplayContext;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, OnceLock};

use crate::game::theurgy_host as host;
use crate::game::theurgy_model_data::{ANIMATIONS, MODELS};

/// Resolution (in mB) a cauldron tank's fill fraction is measured against when
/// the entity does not carry its own capacity. Matches the registry's chosen
/// capacity for `theurgy:liquefaction_cauldron` (registry.rs::fluids).
pub const CAULDRON_CAPACITY: i32 = 2_000;

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
            | "theurgy:incubator_mercury_vessel" | "theurgy:incubator_salt_vessel" | "theurgy:incubator_sulfur_vessel"
            | "theurgy:logistics_nexus" | "theurgy:sal_ammoniac_accumulator" | "theurgy:sal_ammoniac_tank"
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
    fluid_quads(FLUID_INSET, state)
}

fn fluid_quads(inset: f32, state: &CauldronRenderState) -> Vec<Quad> {
    if state.empty {
        return Vec::new();
    }
    let min = inset;
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
    nbt.insert(JavaString::from("isProcessing"), Tag::Byte((entity.data.get(4).copied().unwrap_or(0) != 0) as i8));
    *tag = Some(nbt);
}

#[derive(Deserialize, Debug, Clone, Default)]
struct GeoModelFile {
    #[serde(rename = "minecraft:geometry", default)]
    geometry: Vec<GeoGeometry>,
}
#[derive(Deserialize, Debug, Clone, Default)]
struct GeoGeometry {
    #[serde(default)]
    description: Option<GeoDescription>,
    #[serde(default)]
    bones: Vec<GeoBoneJson>,
}
#[derive(Deserialize, Debug, Clone, Default)]
struct GeoDescription {
    #[serde(default)]
    texture_width: Option<f32>,
    #[serde(default)]
    texture_height: Option<f32>,
    #[serde(default)]
    visible_bounds_width: Option<f32>,
    #[serde(default)]
    visible_bounds_height: Option<f32>,
    #[serde(default)]
    visible_bounds_offset: Option<[f32; 3]>,
}
#[derive(Deserialize, Debug, Clone, Default)]
struct GeoBoneJson {
    name: String,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    pivot: [f32; 3],
    #[serde(default)]
    rotation: [f32; 3],
    #[serde(default)]
    cubes: Vec<GeoCubeJson>,
    #[serde(default)]
    bones: Vec<GeoBoneJson>,
    #[serde(default)]
    inflate: f32,
    #[serde(default)]
    neverrender: bool,
}
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum GeoUv {
    Array([f32; 2]),
    Faces(HashMap<String, GeoFaceUv>),
}
impl Default for GeoUv {
    fn default() -> Self { GeoUv::Array([0.0, 0.0]) }
}
#[derive(Deserialize, Debug, Clone, Default)]
struct GeoFaceUv {
    uv: [f32; 2],
    uv_size: [f32; 2],
}
#[derive(Deserialize, Debug, Clone, Default)]
struct GeoCubeJson {
    #[serde(default)]
    origin: [f32; 3],
    #[serde(default)]
    size: [f32; 3],
    #[serde(default)]
    uv: GeoUv,
    #[serde(default)]
    inflate: f32,
    #[serde(default)]
    rotation: [f32; 3],
    #[serde(default)]
    pivot: [f32; 3],
}

#[derive(Deserialize, Debug, Clone)]
struct AnimationFile {
    animations: HashMap<String, AnimationDef>,
}
#[derive(Deserialize, Debug, Clone)]
struct AnimationDef {
    #[serde(default)]
    animation_length: Option<f32>,
    #[serde(rename = "loop", default)]
    looping: Option<LoopSetting>,
    bones: HashMap<String, HashMap<String, serde_json::Value>>,
}
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum LoopSetting { Bool(bool), Text(String) }

fn models() -> &'static HashMap<&'static str, GeoModelFile> {
    static MODELS_ONCE: OnceLock<HashMap<&'static str, GeoModelFile>> = OnceLock::new();
    MODELS_ONCE.get_or_init(|| MODELS.iter().filter_map(|(name, text)| {
        match serde_json::from_str::<GeoModelFile>(text) {
            Ok(model) => Some((*name, model)),
            Err(error) => {
                eprintln!("THEURGY_RENDER model parse failed {name}: {error}");
                None
            }
        }
    }).collect())
}
fn animations() -> &'static HashMap<&'static str, AnimationFile> {
    static ANIMATIONS_ONCE: OnceLock<HashMap<&'static str, AnimationFile>> = OnceLock::new();
    ANIMATIONS_ONCE.get_or_init(|| ANIMATIONS.iter().filter_map(|(name, text)| {
        serde_json::from_str::<AnimationFile>(text).ok().map(|file| (*name, file))
    }).collect())
}

fn model_name(block: &str) -> Option<&'static str> {
    match block {
        "theurgy:calcination_oven" => Some("calcination_oven.geo.json"),
        "theurgy:distiller" => Some("distiller.geo.json"),
        "theurgy:incubator_mercury_vessel" | "theurgy:incubator_salt_vessel" | "theurgy:incubator_sulfur_vessel" => Some("incubator_vessel.geo.json"),
        "theurgy:logistics_nexus" => Some("logistics_nexus.geo.json"),
        "theurgy:sal_ammoniac_accumulator" => Some("sal_ammoniac_accumulator.geo.json"),
        "theurgy:sal_ammoniac_tank" => Some("sal_ammoniac_tank.geo.json"),
        _ => None,
    }
}
fn animation_name(block: &str) -> Option<&'static str> {
    match block {
        "theurgy:calcination_oven" => Some("calcination_oven.animation.json"),
        "theurgy:distiller" => Some("distiller.animation.json"),
        "theurgy:logistics_nexus" => Some("logistics_nexus.animation.json"),
        _ => None,
    }
}
fn model_texture(block: &str) -> &'static str {
    match block {
        "theurgy:calcination_oven" => "theurgy:block/calcination_oven",
        "theurgy:distiller" => "theurgy:block/distiller",
        "theurgy:incubator_mercury_vessel" => "theurgy:block/incubator_vessel_iron",
        "theurgy:incubator_salt_vessel" => "theurgy:block/incubator_vessel_gold",
        "theurgy:incubator_sulfur_vessel" => "theurgy:block/incubator_vessel_bronze",
        "theurgy:logistics_nexus" => "theurgy:block/logistics_nexus",
        "theurgy:sal_ammoniac_accumulator" => "theurgy:block/sal_ammoniac_accumulator",
        "theurgy:sal_ammoniac_tank" => "theurgy:block/sal_ammoniac_tank",
        _ => "theurgy:block/distiller",
    }
}
#[derive(Clone, Copy)]
struct AnimationTrack {
    processing: bool,
    changed_at: f32,
    last_seen: f32,
}
static ANIMATION_TRACKS: LazyLock<Mutex<HashMap<(String, [i32; 3]), AnimationTrack>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// GeoAnimationBehaviour START_AND_ON/STOP_AND_OFF transition state. GeckoLib
/// keeps this per block entity, not globally by model.
fn animation_id_at(block: &str, pos: [i32; 3], processing: bool, now: f32) -> Option<(&'static str, f32)> {
    if block == "theurgy:logistics_nexus" {
        return Some(("center", now));
    }
    if !matches!(block, "theurgy:calcination_oven" | "theurgy:distiller") {
        return None;
    }
    let mut tracks = ANIMATION_TRACKS.lock().ok()?;
    if tracks.len() > 256 {
        tracks.retain(|_, track| now - track.last_seen < 120.0);
    }
    let track = tracks.entry((block.to_owned(), pos)).or_insert(AnimationTrack {
        processing,
        changed_at: f32::NEG_INFINITY,
        last_seen: now,
    });
    if track.processing != processing {
        track.processing = processing;
        track.changed_at = now;
    }
    track.last_seen = now;
    let transition = if processing {
        if block == "theurgy:distiller" { "animation.distiller.start" } else { "animation.calcination_oven.start" }
    } else if block == "theurgy:distiller" { "animation.distiller.stop" } else { "animation.calcination_oven.stop" };
    let looping = if processing {
        if block == "theurgy:distiller" { "animation.distiller.on" } else { "animation.calcination_oven.on" }
    } else if block == "theurgy:distiller" { "animation.distiller.off" } else { "animation.calcination_oven.off" };
    let elapsed = now - track.changed_at;
    let length = animations().get(animation_name(block)?)
        .and_then(|file| file.animations.get(transition))
        .and_then(|animation| animation.animation_length)
        .unwrap_or(0.0);
    if elapsed >= 0.0 && elapsed < length {
        Some((transition, elapsed))
    } else {
        Some((looping, now))
    }
}
fn vector(value: &serde_json::Value) -> Option<[f32; 3]> {
    let values = value.as_array()?;
    if values.len() != 3 { return None; }
    Some([
        values[0].as_f64()? as f32,
        values[1].as_f64()? as f32,
        values[2].as_f64()? as f32,
    ])
}
fn ease(name: &str, time: f32, args: &[f32]) -> f32 {
    let t = time.clamp(0.0, 1.0);
    match name {
        "easeInOutCubic" => if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 },
        "easeInOutQuart" => if t < 0.5 { 8.0 * t.powi(4) } else { 1.0 - (-2.0 * t + 2.0).powi(4) / 2.0 },
        "easeInOutQuint" => if t < 0.5 { 16.0 * t.powi(5) } else { 1.0 - (-2.0 * t + 2.0).powi(5) / 2.0 },
        "easeInOutBack" => {
            let c1 = args.first().copied().unwrap_or(1.70158);
            let c2 = c1 * 1.525;
            if t < 0.5 {
                (2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2) / 2.0
            } else {
                ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (2.0 * t - 2.0) + c2) + 2.0) / 2.0
            }
        }
        _ => t,
    }
}

fn sample(def: &AnimationDef, bone: &str, channel: &str, seconds: f32) -> Option<[f32; 3]> {
    let value = def.bones.get(bone)?.get(channel)?;
    if let Some(static_vector) = value.get("vector").and_then(vector) {
        return Some(static_vector);
    }
    let object = value.as_object()?;
    let mut frames = Vec::new();
    for (key, frame) in object {
        let Ok(time) = key.parse::<f32>() else { continue; };
        let Some(vector) = frame.get("vector").and_then(vector) else { continue; };
        let easing = frame.get("easing").and_then(serde_json::Value::as_str).unwrap_or("linear").to_owned();
        let args = frame.get("easingArgs").and_then(serde_json::Value::as_array)
            .map(|args| args.iter().filter_map(serde_json::Value::as_f64).map(|v| v as f32).collect())
            .unwrap_or_else(Vec::new);
        frames.push((time, vector, easing, args));
    }
    frames.sort_by(|a, b| a.0.total_cmp(&b.0));
    if frames.is_empty() { return None; }
    let length = def.animation_length.unwrap_or(frames.last()?.0.max(f32::EPSILON));
    let looping = match &def.looping {
        Some(LoopSetting::Bool(value)) => *value,
        Some(LoopSetting::Text(text)) => text == "true",
        None => false,
    };
    let time = if looping && length > 0.0 { seconds.rem_euclid(length) } else { seconds.clamp(0.0, length) };
    if time <= frames[0].0 { return Some(frames[0].1); }
    for pair in frames.windows(2) {
        let (ta, a) = (pair[0].0, pair[0].1);
        let (tb, b) = (pair[1].0, pair[1].1);
        if time <= tb {
            let factor = if tb > ta { ((time - ta) / (tb - ta)).clamp(0.0, 1.0) } else { 0.0 };
            let factor = ease(&pair[1].2, factor, &pair[1].3);
            return Some([
                a[0] + (b[0] - a[0]) * factor,
                a[1] + (b[1] - a[1]) * factor,
                a[2] + (b[2] - a[2]) * factor,
            ]);
        }
    }
    frames.last().map(|frame| frame.1)
}
fn pivot(values: [f32; 3]) -> [f32; 3] {
    [(values[0] + 8.0) / 16.0, values[1] / 16.0, (values[2] + 8.0) / 16.0]
}
fn rotate(draw: &mut crate::game::native_block_entity_draw::BlockEntityDraw, pivot: [f32; 3], rotation: [f32; 3]) {
    // GeckoLib's RenderUtil.optionalRotateZYX.
    if rotation[2] != 0.0 { draw.rotate_about([0.0, 0.0, 1.0], rotation[2], pivot); }
    if rotation[1] != 0.0 { draw.rotate_about([0.0, 1.0, 0.0], rotation[1], pivot); }
    if rotation[0] != 0.0 { draw.rotate_about([1.0, 0.0, 0.0], rotation[0], pivot); }
}
fn uv_rect(origin: [f32; 2], size: [f32; 2], width: f32, height: f32) -> [f32; 4] {
    [
        origin[0] / width * 16.0,
        origin[1] / height * 16.0,
        (origin[0] + size[0]) / width * 16.0,
        (origin[1] + size[1]) / height * 16.0,
    ]
}
fn render_bone(
    game: &crate::game::Game<'_>,
    draw: &mut crate::game::native_block_entity_draw::BlockEntityDraw,
    bone: &GeoBoneJson,
    bones: &[FlatBone<'_>],
    animation: Option<&AnimationDef>,
    seconds: f32,
    texture: &str,
    texture_width: f32,
    texture_height: f32,
) {
    if bone.neverrender { return; }
    draw.push();
    if let Some(animation) = animation {
        if let Some(offset) = sample(animation, &bone.name, "position", seconds) {
            draw.translate([offset[0] / 16.0, offset[1] / 16.0, offset[2] / 16.0]);
        }
    }
    let bone_pivot = pivot(bone.pivot);
    let animation_rotation = animation.and_then(|def| sample(def, &bone.name, "rotation", seconds)).unwrap_or([0.0; 3]);
    rotate(draw, bone_pivot, [
        bone.rotation[0] + animation_rotation[0],
        bone.rotation[1] + animation_rotation[1],
        bone.rotation[2] + animation_rotation[2],
    ]);
    if let Some(scale) = animation.and_then(|def| sample(def, &bone.name, "scale", seconds)) {
        draw.translate(bone_pivot);
        draw.scale(scale);
        draw.translate([-bone_pivot[0], -bone_pivot[1], -bone_pivot[2]]);
    }
    for cube in &bone.cubes {
        draw.push();
        rotate(draw, pivot(cube.pivot), cube.rotation);
        let inflate = (bone.inflate + cube.inflate) / 16.0;
        let from = [
            pivot(cube.origin)[0] - inflate,
            pivot(cube.origin)[1] - inflate,
            pivot(cube.origin)[2] - inflate,
        ];
        let to = [
            from[0] + cube.size[0] / 16.0 + inflate * 2.0,
            from[1] + cube.size[1] / 16.0 + inflate * 2.0,
            from[2] + cube.size[2] / 16.0 + inflate * 2.0,
        ];
        let [sx, sy, sz] = cube.size;
        let (u, v) = match &cube.uv {
            GeoUv::Array(base) => (base[0], base[1]),
            GeoUv::Faces(_) => (0.0, 0.0),
        };
        let face = |direction: &str, origin: [f32; 2], size: [f32; 2]| {
            let (uv, uv_size) = match &cube.uv {
                GeoUv::Array(base) => ([base[0] + origin[0], base[1] + origin[1]], size),
                GeoUv::Faces(faces) => faces.get(direction)
                    .map(|face| ([face.uv[0], face.uv[1]], face.uv_size))
                    .unwrap_or((origin, size)),
            };
            Some(crate::game::native_block_entity_draw::CubeFace {
                sprite: texture,
                uv: Some(uv_rect(uv, uv_size, texture_width, texture_height)),
            })
        };
        let faces = [
            face("down", [u + sz + sx, v], [sx, sz]),
            face("up", [u + sz, v], [sx, sz]),
            face("north", [u + sz, v + sz], [sx, sy]),
            face("south", [u + sz + sx + sz, v + sz], [sx, sy]),
            face("west", [u, v + sz], [sz, sy]),
            face("east", [u + sz + sx, v + sz], [sz, sy]),
        ];
        game.native_draw_cube(draw, from, to, faces, 0xFF_FFFF);
        draw.pop();
    }
    for child in bones.iter().filter(|child| child.parent == Some(bone.name.as_str())) {
        render_bone(game, draw, child.bone, bones, animation, seconds, texture, texture_width, texture_height);
    }
    draw.pop();
}

#[derive(Clone, Copy)]
struct FlatBone<'a> {
    bone: &'a GeoBoneJson,
    parent: Option<&'a str>,
}

fn flatten_bones<'a>(
    source: &'a [GeoBoneJson],
    inherited_parent: Option<&'a str>,
    out: &mut Vec<FlatBone<'a>>,
) {
    for bone in source {
        let parent = bone.parent.as_deref().or(inherited_parent);
        out.push(FlatBone { bone, parent });
        flatten_bones(&bone.bones, Some(&bone.name), out);
    }
}

fn client_stack(entity: &ModBlockEntity) -> Option<horizon_client::items::ItemStack> {
    let Tag::List(items) = entity.nbt.get(&JavaString::from("Items"))? else { return None; };
    for item in items {
        if let Some(stack) = crate::game::theurgy_logistics::template_stack(item) {
            return Some(stack);
        }
    }
    None
}

fn fluid_tank(entity: &ModBlockEntity, capacity: i32, java_key: &str) -> theurgy_engine::recipes::Tank {
    let native = host::tank(entity, 0, capacity);
    if native.amount > 0 {
        return native;
    }
    let Some(Tag::Compound(outer)) = entity.nbt.get(&JavaString::from(java_key)) else { return native; };
    let Some(Tag::Compound(fluid)) = outer.get(&JavaString::from("Fluid")) else { return native; };
    let id = match fluid.get(&JavaString::from("id")) {
        Some(Tag::String(id)) => id.to_string_lossy(),
        _ => return native,
    };
    let amount = host::int(fluid, "amount", 0).clamp(0, capacity);
    if amount == 0 {
        return native;
    }
    theurgy_engine::recipes::Tank { fluid: Some(id), amount, capacity }
}

fn box_fluid_state(
    tank: &theurgy_engine::recipes::Tank,
    light: i32,
    bottom: f32,
    surface: f32,
) -> CauldronRenderState {
    let fill = (tank.amount as f32 / tank.capacity.max(1) as f32).clamp(0.0, 1.0);
    let block_light = (light >> 4) & 0xF;
    CauldronRenderState {
        empty: tank.amount <= 0,
        color: 0xFFFF_FFFF,
        fluid_light: (light & 0xF0_0000) | (block_light << 4),
        bottom_y: bottom,
        surface_y: surface,
        u0: 0.0,
        u1: 1.0,
        v0: 0.0,
        v1: 1.0,
        side_v0: 1.0 - fill,
    }
}

fn submit_fluid_quads(
    game: &crate::game::Game<'_>,
    draw: &mut crate::game::native_block_entity_draw::BlockEntityDraw,
    sprite: &str,
    state: &CauldronRenderState,
    inset: f32,
) {
    if state.empty {
        return;
    }
    let old_light = draw.light;
    draw.light = state.fluid_light as u32;
    for quad in fluid_quads(inset, state) {
        let u0 = quad.uvs.iter().map(|uv| uv[0]).fold(f32::INFINITY, f32::min);
        let u1 = quad.uvs.iter().map(|uv| uv[0]).fold(f32::NEG_INFINITY, f32::max);
        let v0 = quad.uvs.iter().map(|uv| uv[1]).fold(f32::INFINITY, f32::min);
        let v1 = quad.uvs.iter().map(|uv| uv[1]).fold(f32::NEG_INFINITY, f32::max);
        game.native_draw_quad(draw, sprite, quad.positions,
            [u0 * 16.0, v0 * 16.0, u1 * 16.0, v1 * 16.0], state.color);
    }
    draw.light = old_light;
}

/// Submit carried fluid geometry. GeckoLib model submission is handled by the
/// same entry point below; only the cauldron adds quads here.
pub fn submit(
    game: &crate::game::Game<'_>,
    pos: [i32; 3],
    block: &str,
    state: horizon_world::registry::BlockStateId,
    entity: &ModBlockEntity,
    draw: &mut crate::game::native_block_entity_draw::BlockEntityDraw,
) {
    if block == "theurgy:liquefaction_cauldron" {
        let color = cauldron_color(entity, 0xFFFF_FFFF);
        let sprite = cauldron_sprite(entity);
        let state = extract_cauldron(entity, draw.light as i32, 0, color, sprite);
        submit_fluid_quads(game, draw, "theurgy:block/sal_ammoniac/still", &state, FLUID_INSET);
    }

    if let Some(name) = model_name(block) {
        if let Some(model) = models().get(name) {
            let Some(geometry) = model.geometry.first() else { return; };
            let texture = model_texture(block);
            let width = geometry.description.as_ref().and_then(|d| d.texture_width).unwrap_or(128.0).max(1.0);
            let height = geometry.description.as_ref().and_then(|d| d.texture_height).unwrap_or(128.0).max(1.0);
            let processing = entity.data.get(4).copied().unwrap_or(0) != 0
                || match entity.nbt.get(&JavaString::from("isProcessing")) {
                    Some(Tag::Byte(value)) => *value != 0,
                    _ => false,
                };
            let now = draw.game_time as f32 / 20.0 + draw.partial / 20.0;
            let (animation_name_id, seconds) = animation_id_at(block, pos, processing, now)
                .map_or(("", now), |selection| selection);
            let animation = animation_name(block).and_then(|name| animations().get(name))
                .and_then(|file| file.animations.get(animation_name_id));
            let mut bones = Vec::new();
            flatten_bones(&geometry.bones, None, &mut bones);
            draw.push();
            if let Some(facing) = game.native_block_state_property(state, "facing") {
                let yaw = match facing { "east" => 90.0, "south" => 180.0, "west" => 270.0, _ => 0.0 };
                if yaw != 0.0 {
                    draw.rotate_about([0.0, 1.0, 0.0], yaw, [0.5, 0.0, 0.5]);
                }
            }
            for flat in bones.iter().filter(|bone| bone.parent.is_none()) {
                let bone = flat.bone;
                if block == "theurgy:logistics_nexus" {
                    let property = match bone.name.as_str() {
                        "Top" => Some("up"), "Bottom" => Some("down"), "North" => Some("north"),
                        "South" => Some("south"), "East" => Some("east"), "West" => Some("west"),
                        _ => None,
                    };
                    if property.is_some_and(|key| game.native_block_state_property(state, key) == Some("false")) {
                        continue;
                    }
                }
                render_bone(game, draw, bone, &bones, animation, seconds, texture, width, height);
            }
            draw.pop();
            if animation.is_none() && !matches!(block, "theurgy:sal_ammoniac_tank" | "theurgy:sal_ammoniac_accumulator") {
                draw.keep();
            }
        }
    }

    if matches!(block, "theurgy:sal_ammoniac_tank" | "theurgy:sal_ammoniac_accumulator") {
        let tank_block = block == "theurgy:sal_ammoniac_tank";
        let capacity = if tank_block { 2_000 } else { 10_000 };
        let tank = fluid_tank(entity, capacity, if tank_block { "tank" } else { "waterTank" });
        let fill = (tank.amount as f32 / tank.capacity.max(1) as f32).clamp(0.0, 1.0);
        let (block_height, cap_height, inset) = if tank_block {
            (15.0 / 16.0 - 2.0 / 128.0, 0.25, 2.0 / 16.0 + 2.0 / 128.0)
        } else {
            (12.0 / 16.0 - 2.0 / 128.0, 0.0, 3.0 / 16.0 + 2.0 / 128.0)
        };
        let total_height = block_height - 2.0 * cap_height - 1.0 / 16.0;
        let bottom = cap_height;
        let surface = bottom + 1.0 / 16.0 + fill * total_height;
        let state = box_fluid_state(&tank, draw.light as i32, bottom, surface);
        let sprite = if tank.fluid.as_deref() == Some("minecraft:water") {
            "minecraft:block/water_still"
        } else {
            "theurgy:block/sal_ammoniac/still"
        };
        submit_fluid_quads(game, draw, sprite, &state, inset);
    }

    if matches!(block, "theurgy:incubator_mercury_vessel" | "theurgy:incubator_salt_vessel" | "theurgy:incubator_sulfur_vessel"
        | "theurgy:reformation_source_pedestal" | "theurgy:reformation_target_pedestal" | "theurgy:reformation_result_pedestal")
    {
        if let Some(stack) = client_stack(entity) {
            draw.push();
            draw.translate([0.5, 1.25, 0.5]);
            game.native_draw_item(draw, &stack, DisplayContext::Fixed, None, 0);
            draw.pop();
        }
    }
}

pub fn bounds(
    _pos: [i32; 3],
    block: &str,
    _state: horizon_world::registry::BlockStateId,
    _entity: &ModBlockEntity,
    bounds: &mut [f32; 6],
) {
    if let Some(model) = model_name(block).and_then(|name| models().get(name)) {
        if let Some(description) = model.geometry.first().and_then(|geometry| geometry.description.as_ref()) {
            let width = description.visible_bounds_width.unwrap_or(2.0);
            let height = description.visible_bounds_height.unwrap_or(2.5);
            let offset = description.visible_bounds_offset.unwrap_or([0.0, 0.75, 0.0]);
            *bounds = [
                -width / 2.0,
                offset[1] - 1.0,
                -width / 2.0,
                width / 2.0,
                offset[1] + height + 1.0,
                width / 2.0,
            ];
            return;
        }
    }
    if block == "theurgy:liquefaction_cauldron" {
        *bounds = [-1.0, -1.0, -1.0, 2.0, 2.0, 2.0];
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
