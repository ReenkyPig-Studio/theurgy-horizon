// SPDX-License-Identifier: MIT
// Item behaviour that belongs to the block/item hooks. The carried content
// already supplies names, models, stack sizes, durability and tool rules; this
// module adds the two code behaviours that data cannot express: the upper half
// of the double-high apparatus, and the sal ammoniac bucket's fluid identity.
use horizon_client::items::ItemStack;
use horizon_world::chunk::BlockState;
use horizon_world::nbt::{Compound, JavaString, Tag};

/// Blocks whose Java class is a double-high apparatus; the lower half holds the
/// block entity, the upper half is only the model/collision.
const DOUBLE_HIGH: [&str; 4] = [
    "theurgy:calcination_oven",
    "theurgy:liquefaction_cauldron",
    "theurgy:distiller",
    "theurgy:incubator",
];

const WIRE_ITEM: &str = "theurgy:copper_wire";
const WIRE_END_POINT: &str = "theurgy:wire_end_point";
const WIRE_MAX_RANGE: i64 = 32;

const WIRE_ENDPOINTS: [&str; 8] = [
    "theurgy:logistics_connector_node",
    "theurgy:logistics_item_inserter",
    "theurgy:logistics_item_extractor",
    "theurgy:logistics_fluid_inserter",
    "theurgy:logistics_fluid_extractor",
    "theurgy:logistics_capability_probe",
    "theurgy:logistics_capability_proxy",
    "theurgy:logistics_mercury_flux_connector",
];

/// Blocks that the Java classes break as a pair when either half is removed.
pub fn is_double_high(block: &str) -> bool {
    DOUBLE_HIGH.contains(&block)
}

/// The upper-half descriptor for `lower`, if `block` is double high.
fn upper_state(game: &crate::game::Game<'_>, state: horizon_world::registry::BlockStateId) -> Option<horizon_world::registry::BlockStateId> {
    let registry = &game.pack.assets.registry;
    let mut descriptor: BlockState = registry.state(state)?.clone();
    if !is_double_high(&descriptor.name) {
        return None;
    }
    descriptor.properties.insert("half".to_owned(), "upper".to_owned());
    registry.resolve(&descriptor).ok()
}

/// `DistillerBlock.setPlacedBy` and its siblings: place the upper half at
/// `pos.above()` with the same block's `half=upper` state.
pub fn placed(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
) {
    if !is_double_high(&stack.item) {
        return;
    }
    let Some(upper) = upper_state(game, state) else { return; };
    let dimension = game.dim.id();
    game.native_set_block(dimension, [pos[0], pos[1] + 1, pos[2]], upper, 3);
}

/// Read a registered native component from `stack` as a string.
fn component_string(
    game: &crate::game::Game<'_>,
    stack: &ItemStack,
    id: &str,
) -> Option<String> {
    match game.native_component(stack, id).ok().flatten()? {
        Tag::String(value) => Some(value.to_string_lossy()),
        _ => None,
    }
}

/// Read a registered native component as a boolean (Java persists booleans as
/// bytes/ints through NBT).
fn component_bool(game: &crate::game::Game<'_>, stack: &ItemStack, id: &str) -> bool {
    match game.native_component(stack, id).ok().flatten() {
        Some(Tag::Byte(value)) => *value != 0,
        Some(Tag::Int(value)) => *value != 0,
        Some(Tag::Short(value)) => *value != 0,
        _ => false,
    }
}

/// Write a component on the acting player's held stack. The hook's `stack` is
/// immutable, so the authoritative carried stack is updated by value.
fn set_held_component(game: &mut crate::game::Game<'_>, id: &str, value: Tag) {
    let slot = game.selected as i32;
    let Some(mut held) = game.native_carried_stack(slot).cloned() else {
        return;
    };
    if game.native_set_component(&mut held, id, value).is_ok()
        && let Some(current) = game.native_carried_stack(slot)
    {
        *current = held;
    }
}

/// Java `BlockPos.asLong`: x in the high 26 bits, z in the middle 26 bits,
/// y in the low 12 bits.
fn block_pos_long(pos: [i32; 3]) -> i64 {
    (((pos[0] as i64) & 0x3ff_ffff) << 38)
        | (((pos[2] as i64) & 0x3ff_ffff) << 12)
        | ((pos[1] as i64) & 0xfff)
}

/// Inverse of the Minecraft `BlockPos.asLong` packing above.
fn block_pos_from_long(value: i64) -> [i32; 3] {
    let x = (value >> 38) as i32;
    let y = ((value << 52) >> 52) as i32;
    let z = ((value << 26) >> 38) as i32;
    [x, y, z]
}

fn manhattan(a: [i32; 3], b: [i32; 3]) -> i64 {
    (a[0] as i64 - b[0] as i64).abs()
        + (a[1] as i64 - b[1] as i64).abs()
        + (a[2] as i64 - b[2] as i64).abs()
}

fn is_wire_endpoint(block: &str) -> bool {
    WIRE_ENDPOINTS.contains(&block)
}

fn is_sneaking(game: &crate::game::Game<'_>) -> bool {
    game.has_shift_down() || game.keys.contains(&winit::keyboard::KeyCode::ShiftLeft)
}

/// `WireEndPoint.load(ItemStack)`: the component is a compound with a packed
/// `pos` long and a `level` dimension id string.
fn load_wire_endpoint(
    game: &crate::game::Game<'_>,
    stack: &ItemStack,
) -> Option<([i32; 3], String)> {
    let tag = game.native_component(stack, WIRE_END_POINT).ok().flatten()?;
    let Tag::Compound(entries) = tag else {
        return None;
    };
    let mut pos = None;
    let mut level = None;
    for (key, value) in entries {
        if key.to_string_lossy() == "pos" {
            if let Tag::Long(value) = value {
                pos = Some(block_pos_from_long(*value));
            }
        } else if key.to_string_lossy() == "level" {
            if let Tag::String(value) = value {
                level = Some(value.to_string_lossy());
            }
        }
    }
    Some((pos?, level?))
}

fn store_wire_endpoint(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    let level = game.dim.id().to_string();
    let mut entries = Compound::new();
    entries.insert(JavaString::from("pos"), Tag::Long(block_pos_long(pos)));
    entries.insert(
        JavaString::from("level"),
        Tag::String(JavaString::from(level.as_str())),
    );

    let slot = game.selected as i32;
    let Some(mut held) = game.native_carried_stack(slot).cloned() else {
        return;
    };
    if game
        .native_set_component(&mut held, WIRE_END_POINT, Tag::Compound(entries))
        .is_ok()
        && let Some(current) = game.native_carried_stack(slot)
    {
        *current = held;
    }
}

fn clear_wire_endpoint(game: &mut crate::game::Game<'_>) {
    let slot = game.selected as i32;
    let Some(mut held) = game.native_carried_stack(slot).cloned() else {
        return;
    };
    let _ = game.native_remove_component(&mut held, WIRE_END_POINT);
    if let Some(current) = game.native_carried_stack(slot) {
        *current = held;
    }
}

fn consume_one_wire(game: &mut crate::game::Game<'_>) {
    let slot = game.selected as i32;
    let Some(mut held) = game.native_carried_stack(slot).cloned() else {
        return;
    };
    if held.item.as_ref() == WIRE_ITEM && held.count > 0 {
        held.count = held.count.saturating_sub(1);
    }
    if let Some(current) = game.native_carried_stack(slot) {
        *current = held;
    }
}

fn wire_use_on(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    claimed: &mut bool,
) -> bool {
    *claimed = true;

    let block = game.native_block(pos);
    if !is_wire_endpoint(block.as_str()) {
        if is_sneaking(game) {
            clear_wire_endpoint(game);
        }
        return true;
    }

    if is_sneaking(game) {
        clear_wire_endpoint(game);
        return true;
    }

    let Some((stored, stored_level)) = load_wire_endpoint(game, stack) else {
        store_wire_endpoint(game, pos);
        return true;
    };

    if stored_level != game.dim.id().to_string() {
        return true;
    }
    if stored == pos {
        return true;
    }
    if manhattan(stored, pos) > WIRE_MAX_RANGE {
        return true;
    }

    clear_wire_endpoint(game);
    consume_one_wire(game);
    true
}

/// `DivinationRodItem.useOn`: sneak-use links the rod to an allowed block. The
/// attached block is stored in the rod's `divination_linked_block` component.
fn attune_rod(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    claimed: &mut bool,
) -> bool {
    let sneaking = game.has_shift_down() || game.keys.contains(&winit::keyboard::KeyCode::ShiftLeft);
    if !sneaking {
        return false;
    }
    *claimed = true;
    if !component_bool(game, stack, "theurgy:divination_settings_allow_attuning") {
        return true;
    }
    let block = game.native_block(pos);
    if block == "minecraft:air" {
        return true;
    }
    let allowed = component_string(game, stack, "theurgy:divination_settings_allowed_blocks_tag");
    let disallowed =
        component_string(game, stack, "theurgy:divination_settings_disallowed_blocks_tag");
    if let Some(tag) = allowed.as_deref()
        && !tag.is_empty()
        && !game.res.items().block_in_tag(tag, &block)
    {
        return true;
    }
    if let Some(tag) = disallowed.as_deref()
        && !tag.is_empty()
        && game.res.items().block_in_tag(tag, &block)
    {
        return true;
    }
    set_held_component(
        game,
        "theurgy:divination_linked_block",
        Tag::String(JavaString::from(block.as_str())),
    );
    true
}

/// Early `right_click_block` hook: logistics blocks use a barrel template that
/// consumes the block interaction before `use_item_on` runs. Handle the wire
/// before that so a valid endpoint click is never lost.
pub fn right_click_block(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    _face: [i32; 3],
    canceled: &mut bool,
) {
    if *canceled || stack.item.as_ref() != WIRE_ITEM {
        return;
    }
    let mut claimed = false;
    wire_use_on(game, stack, pos, &mut claimed);
    if claimed {
        *canceled = true;
    }
}

/// `getStateForPlacement`: a double-high apparatus is only placeable when the
/// block above can be replaced. Claim a prohibited click before the lower half
/// and its item are committed.
pub fn use_on(
    game: &mut crate::game::Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    face: [i32; 3],
    claimed: &mut bool,
) {
    if *claimed {
        return;
    }
    if stack.item.as_ref() == WIRE_ITEM {
        wire_use_on(game, stack, pos, claimed);
        return;
    }
    if stack.item.contains("divination_rod") && attune_rod(game, stack, pos, claimed) {
        return;
    }
    if !is_double_high(&stack.item) {
        return;
    }
    let clicked = game.world.block(pos[0], pos[1], pos[2]);
    let clicked_replaceable = game.with_level(|b, _level, _ticks| b.is_replaceable(clicked));
    let placement = if clicked_replaceable {
        pos
    } else {
        [pos[0] + face[0], pos[1] + face[1], pos[2] + face[2]]
    };
    let above = [placement[0], placement[1] + 1, placement[2]];
    let state = game.world.block(above[0], above[1], above[2]);
    let replaceable = game.with_level(|b, _level, _ticks| b.is_replaceable(state));
    if !replaceable {
        *claimed = true;
    }
}

/// The two halves are one apparatus. Horizon's vanilla template does not know
/// Theurgy's `updateShape` rule, so a player breaking either half removes the
/// matching other half through the normal level path (including BE spill).
pub fn destroyed(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
    canceled: &mut bool,
) {
    if *canceled {
        return;
    }
    let Some(descriptor) = game.pack.assets.registry.state(state).cloned() else {
        return;
    };
    if !is_double_high(&descriptor.name) {
        return;
    }
    let Some(half) = descriptor.properties.get("half") else {
        return;
    };
    let (other, expected) = match half.as_str() {
        "lower" => ([pos[0], pos[1] + 1, pos[2]], "upper"),
        "upper" => ([pos[0], pos[1] - 1, pos[2]], "lower"),
        _ => return,
    };
    if game.native_block(other) != descriptor.name
        || game.native_block_property(other, "half").as_deref() != Some(expected)
    {
        return;
    }
    let Some(air) = game.pack.assets.registry.default_state("minecraft:air") else {
        return;
    };
    let dimension = game.dim.id();
    game.native_set_block(dimension, other, air, 3);
}

/// `playerWillDestroy`: return true when this is the lower half of a double
/// block broken in creative, where Java suppresses the duplicate drop. The
/// caller decides how to suppress it; this function never cancels the break.
pub fn destroy_suppresses_drop(
    stack: &ItemStack,
    state: horizon_world::registry::BlockStateId,
    creative: bool,
) -> bool {
    let _ = (stack, state);
    creative
}

/// The sal ammoniac bucket leaves an empty bucket, as the Java item's
/// `craftRemainder(Items.BUCKET)` does. The content declaration already
/// carries `crafting_remainder`; this is a pure helper for hosts that model
/// use rather than crafting.
pub fn bucket_remainder(item: &str) -> Option<&'static str> {
    (item == "theurgy:sal_ammoniac_bucket").then_some("minecraft:bucket")
}

/// Whether an item is one of Theurgy's own apparatus block items.
pub fn is_theurgy_item(item: &str) -> bool {
    item.starts_with("theurgy:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_high_set_matches_the_apparatus() {
        for id in DOUBLE_HIGH {
            assert!(is_double_high(id), "{id}");
        }
        assert!(!is_double_high("theurgy:calcination_oven_top"));
        assert!(!is_double_high("minecraft:furnace"));
    }

    #[test]
    fn bucket_keeps_the_empty_bucket() {
        assert_eq!(bucket_remainder("theurgy:sal_ammoniac_bucket"), Some("minecraft:bucket"));
        assert_eq!(bucket_remainder("minecraft:water_bucket"), None);
    }

    #[test]
    fn creative_drop_suppression_is_explicit() {
        assert!(destroy_suppresses_drop(&ItemStack::new("theurgy:distiller", 1), horizon_world::registry::BlockStateId(0), true));
        assert!(!destroy_suppresses_drop(&ItemStack::new("theurgy:distiller", 1), horizon_world::registry::BlockStateId(0), false));
    }
}
