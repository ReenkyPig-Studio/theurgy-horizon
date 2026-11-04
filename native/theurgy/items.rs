// SPDX-License-Identifier: MIT
// Item behaviour that belongs to the block/item hooks. The carried content
// already supplies names, models, stack sizes, durability and tool rules; this
// module adds the two code behaviours that data cannot express: the upper half
// of the double-high apparatus, and the sal ammoniac bucket's fluid identity.
use horizon_client::items::ItemStack;
use horizon_world::chunk::BlockState;
use horizon_world::nbt::{JavaString, Tag};

/// Blocks whose Java class is a double-high apparatus; the lower half holds the
/// block entity, the upper half is only the model/collision.
const DOUBLE_HIGH: [&str; 4] = [
    "theurgy:calcination_oven",
    "theurgy:liquefaction_cauldron",
    "theurgy:distiller",
    "theurgy:incubator",
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
    let Some(upper) = upper_state(game, state) else { return };
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
