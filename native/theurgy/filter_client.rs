// SPDX-License-Identifier: MIT
// Native list/attribute filter screen. Rendering is client-local; every edit
// is queued as a validated server command and never written directly here.

use horizon_client::hud::HudBatch;
use horizon_client::items::ItemStack;
use horizon_client::mod_api::NativeScreenView;
use horizon_world::nbt::{Compound, JavaString, Tag};
use std::sync::Mutex;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind { List, Attribute }

struct RuleView {
    kind: String,
    label: String,
    inverted: bool,
}

struct FilterUi {
    kind: Kind,
    hand: &'static str,
    mode: String,
    respect: bool,
    entries: Vec<ItemStack>,
    rules: Vec<RuleView>,
    page: usize,
    refresh_ticks: u32,
    editing: bool,
    input: String,
}

static FILTER_UI: Mutex<Option<FilterUi>> = Mutex::new(None);
static UI_ACTIONS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn component(game: &crate::game::Game<'_>, stack: &ItemStack, id: &str) -> Option<Tag> {
    game.native_component(stack, id).ok().flatten().cloned()
}

fn map(tag: Option<Tag>) -> Compound {
    match tag { Some(Tag::Compound(map)) => map, _ => Compound::new() }
}

fn text(map: &Compound, key: &str, fallback: &str) -> String {
    match map.get(&JavaString::from(key)) {
        Some(Tag::String(value)) => value.to_string_lossy(),
        _ => fallback.to_owned(),
    }
}

fn bool_tag(map: &Compound, key: &str) -> bool {
    match map.get(&JavaString::from(key)) {
        Some(Tag::Byte(value)) => *value != 0,
        Some(Tag::Int(value)) => *value != 0,
        _ => false,
    }
}

fn load(game: &crate::game::Game<'_>, stack: &ItemStack, hand: &'static str, page: usize) -> Option<FilterUi> {
    let kind = match stack.item.as_ref() {
        "theurgy:list_filter" => Kind::List,
        "theurgy:attribute_filter" => Kind::Attribute,
        _ => return None,
    };
    let (mode, respect, entries, rules) = if kind == Kind::List {
        let config = map(component(game, stack, "codedefinedgui:list_filter_config"));
        let mut entries = Vec::new();
        if let Some(Tag::List(values)) = component(game, stack, "codedefinedgui:list_filter_contents") {
            for value in &values {
                if let Some(stack) = crate::game::theurgy_logistics::template_stack(&value) {
                    entries.push(stack);
                }
            }
        }
        (text(&config, "mode", "allow"), bool_tag(&config, "respect_data_components"), entries, Vec::new())
    } else {
        let config = map(component(game, stack, "codedefinedgui:attribute_filter_config"));
        let mut rules = Vec::new();
        if let Some(Tag::List(values)) = config.get(&JavaString::from("rules")) {
            for value in values {
                let Tag::Compound(rule) = value else { continue; };
                let kind = text(rule, "type", "unknown");
                let payload = match rule.get(&JavaString::from("payload")) {
                    Some(Tag::Compound(payload)) => payload.clone(),
                    _ => Compound::new(),
                };
                let label = if kind == "codedefinedgui:in_tag" {
                    text(&payload, "id", "tag")
                } else {
                    text(&payload, "value", &kind)
                };
                rules.push(RuleView { kind, label, inverted: bool_tag(rule, "inverted") });
            }
        }
        (text(&config, "mode", "match_any"), false, Vec::new(), rules)
    };
    Some(FilterUi { kind, hand, mode, respect, entries, rules, page, refresh_ticks: 0, editing: false, input: String::new() })
}

pub fn open(game: &mut crate::game::Game<'_>, stack: &ItemStack, hand: &'static str) -> bool {
    if load(game, stack, hand, 0).is_none() || !game.native_mod_screen_open("theurgy") {
        return false;
    }
    if let Ok(mut slot) = FILTER_UI.lock() {
        *slot = load(game, stack, hand, 0);
        true
    } else {
        game.native_mod_screen_close("theurgy");
        false
    }
}

pub fn close() {
    if let Ok(mut slot) = FILTER_UI.lock() {
        *slot = None;
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

fn button(game: &crate::game::Game<'_>, batch: &mut HudBatch, rect: [f32; 4], label: &str, active: bool) {
    game.native_fill(batch, rect, if active { 0xFF657B48 } else { 0xFF34483A });
    game.native_text(batch, label, [rect[0] + 4.0, rect[1] + 4.0], 0xFFFFFF, false);
}

pub fn render(
    game: &crate::game::Game<'_>,
    batch: &mut HudBatch,
    screen: &NativeScreenView<'_>,
) -> bool {
    if screen.id != "mod" || game.native_mod_screen() != Some("theurgy") {
        return false;
    }
    let Ok(guard) = FILTER_UI.lock() else { return false; };
    let Some(ui) = guard.as_ref() else { return false; };
    let rect = panel(batch.width, batch.height);
    if ui.editing {
        game.native_fill(batch, [0.0, 0.0, batch.width, batch.height], 0xA0000000);
        game.native_fill(batch, rect, 0xFF18251F);
        game.native_text(batch, "Advanced rule: <kind> <JSON payload>", [rect[0] + 10.0, rect[1] + 12.0], 0xE7D59A, false);
        game.native_text(batch, "Example: codedefinedgui:standard {\"value\":\"renamed\"}", [rect[0] + 10.0, rect[1] + 28.0], 0xA9C9B0, false);
        game.native_fill(batch, [rect[0] + 10.0, rect[1] + 52.0, rect[2] - 20.0, 22.0], 0xFF101010);
        game.native_text(batch, &ui.input, [rect[0] + 14.0, rect[1] + 58.0], 0xFFFFFF, false);
        game.native_text(batch, "Enter: apply   Escape: cancel", [rect[0] + 10.0, rect[1] + rect[3] - 24.0], 0xFFFFFF, false);
        return true;
    }
    game.native_fill(batch, [0.0, 0.0, batch.width, batch.height], 0xA0000000);
    game.native_fill(batch, rect, 0xFF18251F);
    game.native_fill(batch, [rect[0] + 3.0, rect[1] + 3.0, rect[2] - 6.0, rect[3] - 6.0], 0xFF263A30);
    let title = if ui.kind == Kind::List { "List Filter" } else { "Attribute Filter" };
    game.native_text(batch, title, [rect[0] + 9.0, rect[1] + 8.0], 0xE7D59A, false);
    if ui.kind == Kind::List {
        for (index, (mode, label)) in [("allow", "Allow"), ("deny", "Deny")].into_iter().enumerate() {
            button(game, batch, [rect[0] + 8.0 + index as f32 * 60.0, rect[1] + 23.0, 56.0, 15.0], label, ui.mode == mode);
        }
        button(game, batch, [rect[0] + 134.0, rect[1] + 23.0, 138.0, 15.0], "Match components", ui.respect);
        for (index, stack) in ui.entries.iter().skip(ui.page * 18).take(18).enumerate() {
            let x = rect[0] + 10.0 + (index % 9) as f32 * 20.0;
            let y = rect[1] + 48.0 + (index / 9) as f32 * 20.0;
            game.native_fill(batch, [x - 1.0, y - 1.0, 18.0, 18.0], 0xFF4A5C50);
            game.native_item(batch, &stack.item, [x, y], 16.0);
        }
        button(game, batch, [rect[0] + 8.0, rect[1] + rect[3] - 42.0, 106.0, 15.0], "Add held item", false);
        button(game, batch, [rect[0] + 120.0, rect[1] + rect[3] - 42.0, 64.0, 15.0], "Clear", false);
    } else {
        for (index, (mode, label)) in [("match_any", "Any"), ("match_all", "All"), ("deny", "Deny")].into_iter().enumerate() {
            button(game, batch, [rect[0] + 8.0 + index as f32 * 60.0, rect[1] + 23.0, 56.0, 15.0], label, ui.mode == mode);
        }
        for (row, rule) in ui.rules.iter().skip(ui.page * 7).take(7).enumerate() {
            let y = rect[1] + 45.0 + row as f32 * 18.0;
            game.native_fill(batch, [rect[0] + 8.0, y, rect[2] - 16.0, 16.0], 0xFF34483A);
            let prefix = if rule.inverted { "! " } else { "" };
            game.native_text(batch, &format!("{prefix}{}", rule.label), [rect[0] + 12.0, y + 4.0], 0xFFFFFF, false);
            game.native_text(batch, "invert", [rect[0] + rect[2] - 105.0, y + 4.0], 0xE7D59A, false);
            game.native_text(batch, "remove", [rect[0] + rect[2] - 53.0, y + 4.0], 0xE7D59A, false);
            let _ = &rule.kind;
        }
        button(game, batch, [rect[0] + 8.0, rect[1] + rect[3] - 42.0, 92.0, 15.0], "Add renamed", false);
        button(game, batch, [rect[0] + 106.0, rect[1] + rect[3] - 42.0, 88.0, 15.0], "Add item tag", false);
        button(game, batch, [rect[0] + 200.0, rect[1] + rect[3] - 42.0, 64.0, 15.0], "Clear", false);
        button(game, batch, [rect[0] + 270.0, rect[1] + rect[3] - 42.0, 78.0, 15.0], "Advanced", false);
    }
    let count = if ui.kind == Kind::List { ui.entries.len() } else { ui.rules.len() };
    let page_size = if ui.kind == Kind::List { 18 } else { 7 };
    game.native_text(batch, &format!("{} rules   Page {}/{}", count, ui.page + 1, count.div_ceil(page_size).max(1)),
        [rect[0] + 9.0, rect[1] + rect[3] - 19.0], 0xE7D59A, false);
    true
}

fn queue(ui: &mut FilterUi, action: String) {
    if let Ok(mut actions) = UI_ACTIONS.lock() {
        actions.push(format!("{} {action}", ui.hand));
        ui.refresh_ticks = 20;
    }
}

pub fn click(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    button_index: i32,
    canceled: &mut bool,
) -> bool {
    if screen.id != "mod" || game.native_mod_screen() != Some("theurgy") || button_index != 0 {
        return false;
    }
    let Ok(mut guard) = FILTER_UI.lock() else { return false; };
    let Some(ui) = guard.as_mut() else { return false; };
    *canceled = true;
    let rect = panel(screen.rect[2], screen.rect[3]);
    let point = screen.mouse;
    if ui.kind == Kind::List {
        for (index, mode) in ["allow", "deny"].into_iter().enumerate() {
            if inside(point, [rect[0] + 8.0 + index as f32 * 60.0, rect[1] + 23.0, 56.0, 15.0]) {
                queue(ui, format!("mode {mode}"));
                return true;
            }
        }
        if inside(point, [rect[0] + 134.0, rect[1] + 23.0, 138.0, 15.0]) {
            queue(ui, format!("components {}", !ui.respect));
            return true;
        }
        for (index, _) in ui.entries.iter().skip(ui.page * 18).take(18).enumerate() {
            let x = rect[0] + 10.0 + (index % 9) as f32 * 20.0;
            let y = rect[1] + 48.0 + (index / 9) as f32 * 20.0;
            if inside(point, [x - 1.0, y - 1.0, 18.0, 18.0]) {
                queue(ui, format!("remove {}", ui.page * 18 + index));
                return true;
            }
        }
        if inside(point, [rect[0] + 8.0, rect[1] + rect[3] - 42.0, 106.0, 15.0]) {
            queue(ui, "add auto".to_owned());
        } else if inside(point, [rect[0] + 120.0, rect[1] + rect[3] - 42.0, 64.0, 15.0]) {
            queue(ui, "clear".to_owned());
        }
    } else {
        for (index, mode) in ["match_any", "match_all", "deny"].into_iter().enumerate() {
            if inside(point, [rect[0] + 8.0 + index as f32 * 60.0, rect[1] + 23.0, 56.0, 15.0]) {
                queue(ui, format!("mode {mode}"));
                return true;
            }
        }
        for (row, _) in ui.rules.iter().skip(ui.page * 7).take(7).enumerate() {
            let y = rect[1] + 45.0 + row as f32 * 18.0;
            if !inside(point, [rect[0] + 8.0, y, rect[2] - 16.0, 16.0]) {
                continue;
            }
            let index = ui.page * 7 + row;
            if point[0] >= rect[0] + rect[2] - 60.0 {
                queue(ui, format!("rule_remove {index}"));
            } else if point[0] >= rect[0] + rect[2] - 112.0 {
                queue(ui, format!("rule_invert {index}"));
            }
            return true;
        }
        if inside(point, [rect[0] + 8.0, rect[1] + rect[3] - 42.0, 92.0, 15.0]) {
            queue(ui, "rule_renamed".to_owned());
        } else if inside(point, [rect[0] + 106.0, rect[1] + rect[3] - 42.0, 88.0, 15.0]) {
            queue(ui, "rule_tag auto".to_owned());
        } else if inside(point, [rect[0] + 200.0, rect[1] + rect[3] - 42.0, 64.0, 15.0]) {
            queue(ui, "clear".to_owned());
        } else if inside(point, [rect[0] + 270.0, rect[1] + rect[3] - 42.0, 78.0, 15.0]) {
            ui.editing = true;
            ui.input.clear();
        }
    }
    true
}

pub fn scroll(screen: &NativeScreenView<'_>, game: &crate::game::Game<'_>, delta: f64, canceled: &mut bool) -> bool {
    if screen.id != "mod" || game.native_mod_screen() != Some("theurgy") {
        return false;
    }
    let Ok(mut guard) = FILTER_UI.lock() else { return false; };
    let Some(ui) = guard.as_mut() else { return false; };
    *canceled = true;
    let (count, size) = if ui.kind == Kind::List { (ui.entries.len(), 18) } else { (ui.rules.len(), 7) };
    let pages = count.div_ceil(size).max(1);
    ui.page = if delta > 0.0 { ui.page.saturating_sub(1) } else { (ui.page + 1).min(pages - 1) };
    true
}

pub fn char_typed(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    c: char,
    canceled: &mut bool,
) -> bool {
    if screen.id != "mod" || game.native_mod_screen() != Some("theurgy") {
        return false;
    }
    let Ok(mut guard) = FILTER_UI.lock() else { return false; };
    let Some(ui) = guard.as_mut() else { return false; };
    if !ui.editing {
        return false;
    }
    *canceled = true;
    if ui.input.len() < 256 && !c.is_control() {
        ui.input.push(c);
    }
    true
}

pub fn key_pressed(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    key: &str,
    canceled: &mut bool,
) -> bool {
    if screen.id != "mod" || game.native_mod_screen() != Some("theurgy") {
        return false;
    }
    let Ok(mut guard) = FILTER_UI.lock() else { return false; };
    let Some(ui) = guard.as_mut() else { return false; };
    if !ui.editing {
        return false;
    }
    *canceled = true;
    match key {
        "key.keyboard.escape" => {
            ui.editing = false;
            ui.input.clear();
        }
        "key.keyboard.backspace" => {
            ui.input.pop();
        }
        "key.keyboard.enter" | "key.keyboard.keypad_enter" => {
            if ui.input.split_once(' ').is_some() {
                let action = ui.input.trim().to_owned();
                queue(ui, format!("rule_add {action}"));
                ui.editing = false;
                ui.input.clear();
            }
        }
        _ => {}
    }
    true
}

/// Executes queued edits through the normal local/remote command path, then
/// refreshes from the authoritative stack after the server has answered.
pub fn frame(game: &mut crate::game::Game<'_>) {
    let actions = UI_ACTIONS.lock().map(|mut actions| std::mem::take(&mut *actions)).unwrap_or_default();
    for action in actions {
        game.click_command(&format!("/theurgy_filter {action}"));
    }
    let refresh = FILTER_UI.lock().ok().and_then(|mut slot| {
        let ui = slot.as_mut()?;
        if ui.refresh_ticks == 0 {
            return None;
        }
        ui.refresh_ticks -= 1;
        (ui.refresh_ticks == 0).then_some(ui.hand)
    });
    if let Some(hand) = refresh {
        let slot = if hand == "off" { -106 } else { game.selected as i32 };
        let page = FILTER_UI.lock().ok().and_then(|slot| slot.as_ref().map(|ui| ui.page)).unwrap_or(0);
        let held = game.native_carried_stack(slot).cloned();
        let updated = held.and_then(|stack| load(game, &stack, hand, page));
        if let Ok(mut slot) = FILTER_UI.lock() {
            *slot = updated;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_boxes_exclude_the_far_edge() {
        assert!(inside([4.0, 4.0], [4.0, 4.0, 10.0, 10.0]));
        assert!(!inside([14.0, 4.0], [4.0, 4.0, 10.0, 10.0]));
    }
}
