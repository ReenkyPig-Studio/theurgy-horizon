// SPDX-License-Identifier: MIT
// SelectionBehaviour and the three emitter selection messages (Java 26.3).
//
// Java keeps pending points on the client and sends them when the emitter
// block item is placed. Horizon's interaction hooks are server-authoritative,
// so the equivalent pending state is keyed by acting player and emitter kind;
// it is never discovered by scanning the world and never persisted in the item.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use horizon_client::items::ItemStack;
use horizon_world::nbt::{Compound, JavaString, Tag};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Emitter {
    Mercury,
    Caloric,
    Sulfuric,
}

#[derive(Clone, Debug, Default)]
struct Pending {
    points: Vec<[i32; 3]>,
    target: Option<[i32; 3]>,
    result: Option<[i32; 3]>,
}

static PENDING: OnceLock<Mutex<HashMap<(u128, Emitter), Pending>>> = OnceLock::new();

fn pending() -> &'static Mutex<HashMap<(u128, Emitter), Pending>> {
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_pending() -> std::sync::MutexGuard<'static, HashMap<(u128, Emitter), Pending>> {
    pending().lock().unwrap_or_else(|error| error.into_inner())
}

fn emitter(item: &ItemStack) -> Option<Emitter> {
    match item.item.as_ref() {
        "theurgy:mercury_flux_emitter" => Some(Emitter::Mercury),
        "theurgy:caloric_flux_emitter" => Some(Emitter::Caloric),
        "theurgy:sulfuric_flux_emitter" => Some(Emitter::Sulfuric),
        _ => None,
    }
}

fn sneaking(game: &crate::game::Game<'_>) -> bool {
    game.has_shift_down() || game.keys.contains(&winit::keyboard::KeyCode::ShiftLeft)
}

pub fn heat_receiver(block: &str) -> bool {
    matches!(
        block,
        "theurgy:calcination_oven"
            | "theurgy:distiller"
            | "theurgy:liquefaction_cauldron"
            | "theurgy:incubator"
    )
}

pub fn mercury_target(block: &str) -> bool {
    matches!(
        block,
        "theurgy:mercury_catalyst"
            | "theurgy:mercury_capacitor"
            | "theurgy:mercury_flux_emitter"
            | "theurgy:caloric_flux_emitter"
            | "theurgy:sulfuric_flux_emitter"
            | "theurgy:logistics_mercury_flux_connector"
    )
}

fn pedestal(block: &str) -> Option<&'static str> {
    match block {
        "theurgy:reformation_source_pedestal" => Some("SOURCE"),
        "theurgy:reformation_target_pedestal" => Some("TARGET"),
        "theurgy:reformation_result_pedestal" => Some("RESULT"),
        _ => None,
    }
}

/// `SelectionBehaviour.onRightClickBlock`. A second click toggles a point off;
/// target/result selections replace the previous point, sources are unlimited.
pub fn right_click(
    game: &mut crate::game::Game<'_>,
    stack: Option<&ItemStack>,
    pos: [i32; 3],
    canceled: &mut bool,
) {
    if *canceled {
        return;
    }
    let Some(kind) = stack.and_then(emitter) else {
        return;
    };
    // MercuryFluxEmitterSelectionBehaviour explicitly lets Shift+use place.
    if kind == Emitter::Mercury && sneaking(game) {
        return;
    }

    let block = game.native_block(pos);
    let sulfuric_type = (kind == Emitter::Sulfuric)
        .then(|| pedestal(&block))
        .flatten();
    let valid = match kind {
        Emitter::Mercury => mercury_target(&block),
        Emitter::Caloric => heat_receiver(&block),
        Emitter::Sulfuric => sulfuric_type.is_some(),
    };
    if !valid {
        return;
    }

    let player = game.native_acting_id();
    let mut pending = lock_pending();
    let selection = pending.entry((player, kind)).or_default();
    match kind {
        Emitter::Mercury | Emitter::Caloric => {
            if selection.points.first() == Some(&pos) {
                selection.points.clear();
            } else {
                selection.points.clear();
                selection.points.push(pos);
            }
        }
        Emitter::Sulfuric => match sulfuric_type.expect("validated pedestal") {
            "SOURCE" => {
                if let Some(index) = selection.points.iter().position(|point| *point == pos) {
                    selection.points.remove(index);
                } else {
                    selection.points.push(pos);
                }
            }
            "TARGET" => {
                selection.target = (selection.target != Some(pos)).then_some(pos);
            }
            "RESULT" => {
                selection.result = (selection.result != Some(pos)).then_some(pos);
            }
            _ => unreachable!(),
        },
    }
    drop(pending);
    *canceled = true;
}

/// `SelectionBehaviour.tick`: changing the held item abandons pending points.
pub fn inventory_tick(game: &mut crate::game::Game<'_>, slot: i32, selected: bool) {
    if !selected {
        return;
    }
    let current = game
        .native_carried_stack(slot)
        .cloned()
        .and_then(|stack| emitter(&stack));
    let player = game.native_acting_id();
    lock_pending().retain(|(owner, kind), _| *owner != player || Some(*kind) == current);
}

/// `SelectionBehaviour.onLeftClickBlock`: remove only a previously selected
/// point and claim precisely that click; other attacks proceed normally.
pub fn left_click(
    game: &mut crate::game::Game<'_>,
    stack: Option<&ItemStack>,
    pos: [i32; 3],
    canceled: &mut bool,
) {
    if *canceled {
        return;
    }
    let Some(kind) = stack.and_then(emitter) else {
        return;
    };
    let block = game.native_block(pos);
    let sulfuric_type = (kind == Emitter::Sulfuric)
        .then(|| pedestal(&block))
        .flatten();
    let player = game.native_acting_id();
    let mut pending = lock_pending();
    let Some(selection) = pending.get_mut(&(player, kind)) else {
        return;
    };
    let removed = match kind {
        Emitter::Mercury | Emitter::Caloric => {
            let found = selection.points.first() == Some(&pos);
            if found {
                selection.points.clear();
            }
            found
        }
        Emitter::Sulfuric => match sulfuric_type {
            Some("SOURCE") => {
                let old = selection.points.len();
                selection.points.retain(|point| *point != pos);
                old != selection.points.len()
            }
            Some("TARGET") => selection.target.take_if(|target| *target == pos).is_some(),
            Some("RESULT") => selection.result.take_if(|result| *result == pos).is_some(),
            _ => false,
        },
    };
    if removed {
        *canceled = true;
    }
}

fn closer_than(origin: [i32; 3], point: [i32; 3], range: i32) -> bool {
    let dx = i64::from(origin[0] - point[0]);
    let dy = i64::from(origin[1] - point[1]);
    let dz = i64::from(origin[2] - point[2]);
    // BlockPos.closerThan uses distSqr < range * range.
    dx * dx + dy * dy + dz * dz < i64::from(range * range)
}

pub fn point_tag(pos: [i32; 3], kind: Option<&str>) -> Tag {
    let mut point = Compound::new();
    point.insert(JavaString::from("blockPos"), Tag::IntArray(pos.to_vec()));
    if let Some(kind) = kind {
        point.insert(
            JavaString::from("type"),
            Tag::String(JavaString::from(kind)),
        );
    }
    Tag::Compound(point)
}

/// `BlockPos.CODEC`, with read compatibility for the old native `{x,y,z}` form.
pub fn read_point_tag(tag: &Tag) -> Option<[i32; 3]> {
    let Tag::Compound(point) = tag else {
        return None;
    };
    if let Some(tag) = point.get(&JavaString::from("blockPos")) {
        match tag {
            Tag::IntArray(values) if values.len() == 3 => {
                return Some([values[0], values[1], values[2]]);
            }
            Tag::List(values) if values.len() == 3 => {
                let mut out = [0; 3];
                for (index, value) in values.iter().enumerate() {
                    let Tag::Int(value) = value else {
                        return None;
                    };
                    out[index] = *value;
                }
                return Some(out);
            }
            _ => return None,
        }
    }
    // Saves made by the incomplete native port used flat x/y/z fields.
    let x = super::theurgy_host::int(point, "x", i32::MIN);
    let y = super::theurgy_host::int(point, "y", i32::MIN);
    let z = super::theurgy_host::int(point, "z", i32::MIN);
    (x != i32::MIN && y != i32::MIN && z != i32::MIN).then_some([x, y, z])
}

pub fn read_point(nbt: &Compound, key: &str) -> Option<[i32; 3]> {
    read_point_tag(nbt.get(&JavaString::from(key))?)
}

pub fn read_points(nbt: &Compound, key: &str) -> Vec<[i32; 3]> {
    match nbt.get(&JavaString::from(key)) {
        Some(Tag::List(points)) => points.iter().filter_map(read_point_tag).collect(),
        _ => Vec::new(),
    }
}

/// `SelectionBehaviour.onPlace` plus the corresponding placement message.
pub fn placed(game: &mut crate::game::Game<'_>, stack: &ItemStack, pos: [i32; 3]) {
    let Some(kind) = emitter(stack) else {
        return;
    };
    let player = game.native_acting_id();
    let selection = lock_pending().remove(&(player, kind));
    let Some(mut selection) = selection else {
        return;
    };
    let range = if kind == Emitter::Sulfuric { 16 } else { 8 };
    selection.points.retain(|point| closer_than(pos, *point, range));
    selection.target = selection.target.filter(|point| closer_than(pos, *point, range));
    selection.result = selection.result.filter(|point| closer_than(pos, *point, range));

    let _ = game.native_with_mod_block_entity(pos, |entity| match kind {
        Emitter::Mercury | Emitter::Caloric => {
            let points = selection
                .points
                .iter()
                .map(|point| point_tag(*point, None))
                .collect();
            entity
                .nbt
                .insert(JavaString::from("selectedPoints"), Tag::List(points));
        }
        Emitter::Sulfuric => {
            let sources = selection
                .points
                .iter()
                .map(|point| point_tag(*point, Some("SOURCE")))
                .collect();
            entity
                .nbt
                .insert(JavaString::from("sourcePedestals"), Tag::List(sources));
            if let Some(target) = selection.target {
                entity.nbt.insert(
                    JavaString::from("targetPedestal"),
                    point_tag(target, Some("TARGET")),
                );
            } else {
                entity.nbt.remove(&JavaString::from("targetPedestal"));
            }
            if let Some(result) = selection.result {
                entity.nbt.insert(
                    JavaString::from("resultPedestal"),
                    point_tag(result, Some("RESULT")),
                );
            } else {
                entity.nbt.remove(&JavaString::from("resultPedestal"));
            }
        }
    });
    game.native_block_entity_sync(pos);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_block_pos_codec_round_trip() {
        let tag = point_tag([4, -5, 6], Some("SOURCE"));
        assert_eq!(read_point_tag(&tag), Some([4, -5, 6]));
        let mut nbt = Compound::new();
        nbt.insert(JavaString::from("sourcePedestals"), Tag::List(vec![tag]));
        assert_eq!(read_points(&nbt, "sourcePedestals"), vec![[4, -5, 6]]);
    }

    #[test]
    fn closer_than_matches_block_pos_boundary() {
        assert!(closer_than([0, 0, 0], [15, 0, 0], 16));
        assert!(!closer_than([0, 0, 0], [16, 0, 0], 16));
        assert!(!closer_than([0, 0, 0], [8, 0, 0], 8));
    }

    #[test]
    fn legacy_flat_points_remain_readable() {
        let mut point = Compound::new();
        point.insert(JavaString::from("x"), Tag::Int(1));
        point.insert(JavaString::from("y"), Tag::Int(2));
        point.insert(JavaString::from("z"), Tag::Int(3));
        assert_eq!(read_point_tag(&Tag::Compound(point)), Some([1, 2, 3]));
    }
}
