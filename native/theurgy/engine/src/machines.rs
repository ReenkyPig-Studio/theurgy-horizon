// SPDX-License-Identifier: MIT
use crate::crafting::Crafting;
use crate::recipes::{Recipe, Tank, can_fit, find_recipe};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MachineLayout {
    pub inputs: &'static [usize],
    pub output: usize,
    pub tank: Option<usize>,
}

pub fn layout(kind: &str) -> Option<MachineLayout> {
    Some(match kind {
        "theurgy:calcination" | "theurgy:distillation" | "theurgy:liquefaction" =>
            MachineLayout { inputs: &[0], output: 1, tank: (kind == "theurgy:liquefaction").then_some(0) },
        "theurgy:fermentation" | "theurgy:digestion" =>
            MachineLayout { inputs: &[0, 1, 2], output: 3, tank: Some(0) },
        _ => return None,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessResult {
    pub selected_recipe: Option<String>,
    pub crafted: bool,
    pub state_changed: bool,
}

/// Atomic item/fluid machine tick, following CraftingBehaviour.tickServer.
/// No item or fluid is committed if any extraction/output check fails.
pub fn tick_machine<F>(
    kind: &str,
    state: &mut Crafting,
    items: &mut [Option<crate::recipes::Item>],
    tanks: &mut [Tank],
    recipes: &[Recipe],
    can_process: bool,
    matches: F,
    plan_for: impl Fn(&Recipe) -> Option<CraftPlan>,
) -> Option<ProcessResult>
where F: FnMut(&Value, &crate::recipes::Item) -> bool {
    let layout = layout(kind)?;
    let old = state.clone();
    let recipe = find_recipe(kind, recipes, items, layout.tank.and_then(|i| tanks.get(i)), matches);
    let Some(recipe) = recipe else {
        state.stop();
        return Some(ProcessResult { selected_recipe: None, crafted: false, state_changed: old != *state });
    };
    let time = recipe.time.max(0);
    if state.total_time != time {
        state.input_changed(time);
    }
    let plan = plan_for(recipe);
    let effects = state.tick(can_process, true, plan.is_some(), time);
    let crafted = effects.craft_attempt && plan.is_some_and(|plan| commit_plan(items, tanks, &plan));
    Some(ProcessResult { selected_recipe: Some(recipe.id.into()), crafted,
        state_changed: old != *state || effects.block_updates != 0 })
}

/// A transactional recipe plan produced by the host after tag/component matching.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CraftPlan {
    pub inputs: Vec<(usize, u32)>,
    pub output: Option<(usize, crate::recipes::Item)>,
    pub fluid: Option<(usize, i32)>,
}

pub fn commit_plan(items: &mut [Option<crate::recipes::Item>], tanks: &mut [Tank], plan: &CraftPlan) -> bool {
    let mut new_items = items.to_vec();
    let mut new_tanks = tanks.to_vec();
    for &(slot, count) in &plan.inputs {
        let Some(stack) = new_items.get_mut(slot).and_then(Option::as_mut) else { return false };
        if stack.count < count { return false; }
        stack.count -= count;
        if stack.count == 0 { new_items[slot] = None; }
    }
    if let Some((slot, result)) = &plan.output {
        if !can_fit(items.get(*slot).and_then(Option::as_ref), result) { return false; }
        match &mut new_items[*slot] {
            Some(old) => old.count += result.count,
            empty => *empty = Some(result.clone()),
        }
    }
    if let Some((slot, amount)) = plan.fluid {
        let Some(tank) = new_tanks.get_mut(slot) else { return false; };
        if tank.amount < amount { return false; }
        tank.amount -= amount;
        if tank.amount == 0 { tank.fluid = None; }
    }
    items.clone_from_slice(&new_items);
    tanks.clone_from_slice(&new_tanks);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipes::Item;
    #[test]
    fn plan_is_atomic_on_short_input_and_full_output() {
        let mut items = vec![Some(Item { id: "a".into(), count: 1, max_stack: 64 }), None];
        let original = items.clone();
        let plan = CraftPlan { inputs: vec![(0, 2)],
            output: Some((1, Item { id: "b".into(), count: 1, max_stack: 64 })), fluid: None };
        assert!(!commit_plan(&mut items, &mut [], &plan));
        assert_eq!(items, original);
    }
    #[test]
    fn successful_plan_conserves_count_minus_consumption() {
        let mut items = vec![Some(Item { id: "a".into(), count: 2, max_stack: 64 }), None];
        let plan = CraftPlan { inputs: vec![(0, 2)],
            output: Some((1, Item { id: "b".into(), count: 1, max_stack: 64 })), fluid: None };
        assert!(commit_plan(&mut items, &mut [], &plan));
        assert_eq!(items[0], None);
        assert_eq!(items[1].as_ref().unwrap().count, 1);
    }
}
