// SPDX-License-Identifier: MIT
// Read-only native JEI/EMI replacement for Theurgy's data-driven recipes.

use horizon_client::hud::HudBatch;
use horizon_client::items::ItemStack;
use horizon_client::mod_api::NativeScreenView;
use std::sync::Mutex;

const KINDS: &[&str] = &[
    "theurgy:calcination", "theurgy:distillation", "theurgy:liquefaction",
    "theurgy:fermentation", "theurgy:digestion", "theurgy:incubation",
    "theurgy:reformation", "theurgy:catalysation", "theurgy:accumulation",
];

struct RecipeView {
    kind: &'static str,
    id: String,
    result: Option<ItemStack>,
    time: i64,
    inputs: Vec<String>,
    fluid: Option<String>,
}

struct RecipeUi {
    recipes: Vec<RecipeView>,
    selected: Option<usize>,
    kind: usize,
    page: usize,
}

static RECIPE_UI: Mutex<Option<RecipeUi>> = Mutex::new(None);

pub fn close() {
    if let Ok(mut slot) = RECIPE_UI.lock() {
        *slot = None;
    }
}

pub fn open(game: &mut crate::game::Game<'_>) -> bool {
    let mut recipes = Vec::new();
    for kind in KINDS {
        for recipe in game.native_recipes_of_type(kind) {
            let result = game.native_recipe_result(&recipe.json["result"]);
            let time = recipe.json.get("time").and_then(serde_json::Value::as_i64).unwrap_or(100);
            let inputs = theurgy_engine::recipes::ingredient_parts(kind, &recipe.json)
                .into_iter().map(|(ingredient, count)| format!("{count}x {ingredient}")).collect();
            let fluid = theurgy_engine::recipes::fluid_part(kind, &recipe.json)
                .map(|(fluid, amount)| format!("{amount} mB {fluid}"));
            recipes.push(RecipeView { kind, id: recipe.id.to_string(), result, time, inputs, fluid });
        }
    }
    recipes.sort_by(|a, b| a.kind.cmp(b.kind).then(a.id.cmp(&b.id)));
    if recipes.is_empty() || !game.native_mod_screen_open("theurgy") {
        return false;
    }
    if let Ok(mut slot) = RECIPE_UI.lock() {
        *slot = Some(RecipeUi { recipes, selected: None, kind: 0, page: 0 });
        true
    } else {
        game.native_mod_screen_close("theurgy");
        false
    }
}

fn panel(width: f32, height: f32) -> [f32; 4] {
    let w = 360.0f32.min((width - 16.0).max(240.0));
    let h = 240.0f32.min((height - 16.0).max(180.0));
    [((width - w) / 2.0).max(4.0), ((height - h) / 2.0).max(4.0), w, h]
}

fn inside(point: [f32; 2], rect: [f32; 4]) -> bool {
    point[0] >= rect[0] && point[1] >= rect[1]
        && point[0] < rect[0] + rect[2] && point[1] < rect[1] + rect[3]
}

fn filtered(ui: &RecipeUi) -> Vec<usize> {
    ui.recipes.iter().enumerate().filter_map(|(index, recipe)| {
        (ui.kind == 0 || ui.kind == KINDS.iter().position(|kind| *kind == recipe.kind).unwrap_or(usize::MAX) + 1)
            .then_some(index)
    }).collect()
}

pub fn render(
    game: &crate::game::Game<'_>,
    batch: &mut HudBatch,
    screen: &NativeScreenView<'_>,
) -> bool {
    if screen.id != "mod" || game.native_mod_screen() != Some("theurgy") {
        return false;
    }
    let Ok(guard) = RECIPE_UI.lock() else { return false; };
    let Some(ui) = guard.as_ref() else { return false; };
    let rect = panel(batch.width, batch.height);
    game.native_fill(batch, [0.0, 0.0, batch.width, batch.height], 0xA0000000);
    game.native_fill(batch, rect, 0xFF18251F);
    game.native_fill(batch, [rect[0] + 3.0, rect[1] + 3.0, rect[2] - 6.0, rect[3] - 6.0], 0xFF263A30);
    game.native_text(batch, "Theurgy Recipes", [rect[0] + 9.0, rect[1] + 8.0], 0xE7D59A, false);
    game.native_text(batch, if ui.kind == 0 { "All types" } else { KINDS[ui.kind - 1] },
        [rect[0] + 9.0, rect[1] + 25.0], 0xA9C9B0, false);
    if let Some(selected) = ui.selected {
        let Some(recipe) = ui.recipes.get(selected) else { return false; };
        game.native_text(batch, "< Back", [rect[0] + 9.0, rect[1] + 44.0], 0xFFFFFF, false);
        game.native_text(batch, &recipe.id, [rect[0] + 60.0, rect[1] + 44.0], 0xE7D59A, false);
        if let Some(result) = &recipe.result {
            game.native_item(batch, &result.item, [rect[0] + 10.0, rect[1] + 64.0], 24.0);
            game.native_text(batch, &format!("{}x {}", result.count, result.item), [rect[0] + 42.0, rect[1] + 72.0], 0xFFFFFF, false);
        }
        game.native_text(batch, &format!("{} ticks", recipe.time), [rect[0] + 10.0, rect[1] + 96.0], 0xA9C9B0, false);
        for (index, input) in recipe.inputs.iter().take(8).enumerate() {
            game.native_text(batch, input, [rect[0] + 10.0, rect[1] + 114.0 + index as f32 * 10.0], 0xFFFFFF, false);
        }
        if let Some(fluid) = &recipe.fluid {
            game.native_text(batch, fluid, [rect[0] + 10.0, rect[1] + rect[3] - 38.0], 0xA9C9B0, false);
        }
        return true;
    }
    let recipes = filtered(ui);
    for (row, index) in recipes.iter().skip(ui.page * 8).take(8).enumerate() {
        let Some(recipe) = ui.recipes.get(*index) else { continue; };
        let y = rect[1] + 40.0 + row as f32 * 20.0;
        game.native_fill(batch, [rect[0] + 8.0, y, rect[2] - 16.0, 18.0], 0xFF34483A);
        if let Some(result) = &recipe.result {
            game.native_item(batch, &result.item, [rect[0] + 10.0, y + 1.0], 16.0);
        }
        game.native_text(batch, &recipe.id, [rect[0] + 32.0, y + 5.0], 0xFFFFFF, false);
    }
    let pages = recipes.len().div_ceil(8).max(1);
    game.native_text(batch, &format!("Page {}/{}   (click type to cycle)", ui.page + 1, pages),
        [rect[0] + 9.0, rect[1] + rect[3] - 19.0], 0xE7D59A, false);
    true
}

pub fn click(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) -> bool {
    if screen.id != "mod" || game.native_mod_screen() != Some("theurgy") || button != 0 {
        return false;
    }
    let Ok(mut guard) = RECIPE_UI.lock() else { return false; };
    let Some(ui) = guard.as_mut() else { return false; };
    *canceled = true;
    let rect = panel(screen.rect[2], screen.rect[3]);
    if inside(screen.mouse, [rect[0] + 8.0, rect[1] + 20.0, 180.0, 17.0]) {
        ui.kind = (ui.kind + 1) % (KINDS.len() + 1);
        ui.selected = None;
        ui.page = 0;
        return true;
    }
    if ui.selected.is_some() && inside(screen.mouse, [rect[0] + 8.0, rect[1] + 40.0, 44.0, 16.0]) {
        ui.selected = None;
        return true;
    }
    if ui.selected.is_none() {
        let recipes = filtered(ui);
        for (row, index) in recipes.iter().skip(ui.page * 8).take(8).enumerate() {
            let y = rect[1] + 40.0 + row as f32 * 20.0;
            if inside(screen.mouse, [rect[0] + 8.0, y, rect[2] - 16.0, 18.0]) {
                ui.selected = Some(*index);
                return true;
            }
        }
    }
    if screen.mouse[1] >= rect[1] + rect[3] - 24.0 {
        let pages = filtered(ui).len().div_ceil(8).max(1);
        if screen.mouse[0] < rect[0] + rect[2] / 2.0 {
            ui.page = ui.page.saturating_sub(1);
        } else {
            ui.page = (ui.page + 1).min(pages - 1);
        }
    }
    true
}

pub fn scroll(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    delta: f64,
    canceled: &mut bool,
) -> bool {
    if screen.id != "mod" || game.native_mod_screen() != Some("theurgy") {
        return false;
    }
    let Ok(mut guard) = RECIPE_UI.lock() else { return false; };
    let Some(ui) = guard.as_mut() else { return false; };
    *canceled = true;
    if ui.selected.is_some() {
        return true;
    }
    let pages = filtered(ui).len().div_ceil(8).max(1);
    ui.page = if delta > 0.0 { ui.page.saturating_sub(1) } else { (ui.page + 1).min(pages - 1) };
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_test_has_strict_far_edge() {
        assert!(inside([2.0, 2.0], [2.0, 2.0, 5.0, 5.0]));
        assert!(!inside([7.0, 2.0], [2.0, 2.0, 5.0, 5.0]));
    }
}
