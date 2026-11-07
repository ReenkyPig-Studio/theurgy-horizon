// SPDX-License-Identifier: MIT
// Native client replacement for TheHermeticaItem's Modonomicon screen.
//
// Java path: `content/item/book/TheHermeticaItem.use`. The complete pinned
// Modonomicon JSON tree is flattened by tools/build-content.mjs into
// `data/theurgy/hermetica.json`; this module only reads that runtime
// resource and draws/navigates it. It has no filesystem, network or unsafe IO.

use horizon_client::hud::HudBatch;
use horizon_client::mod_api::NativeScreenView;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Deserialize)]
struct BookData {
    book: serde_json::Value,
    categories: Vec<serde_json::Value>,
    entries: Vec<serde_json::Value>,
    translations: HashMap<String, String>,
}

#[derive(Debug)]
struct BookUi {
    data: BookData,
    category: usize,
    entry: Option<usize>,
    page: usize,
    list_page: usize,
}

static BOOK_UI: Mutex<Option<BookUi>> = Mutex::new(None);

fn text<'a>(data: &'a BookData, key: &'a str) -> &'a str {
    data.translations.get(key).map(String::as_str).unwrap_or(key)
}

fn value_text<'a>(data: &'a BookData, value: &'a serde_json::Value, key: &str) -> &'a str {
    value.get(key).and_then(serde_json::Value::as_str).map_or("", |key| text(data, key))
}

fn clean_markdown(input: &str) -> String {
    let input = input.replace("\\\n", "\n").replace("\\\r\n", "\n");
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '§' {
            chars.next();
            continue;
        }
        if ch == '[' {
            let mut label = String::new();
            let mut found = false;
            for next in chars.by_ref() {
                if next == ']' {
                    found = true;
                    break;
                }
                label.push(next);
            }
            if found && chars.peek() == Some(&'(') {
                chars.next();
                for next in chars.by_ref() {
                    if next == ')' {
                        break;
                    }
                }
                out.push_str(&label);
                continue;
            }
            out.push('[');
            out.push_str(&label);
            if found {
                out.push(']');
            }
            continue;
        }
        if matches!(ch, '*' | '`') {
            continue;
        }
        out.push(ch);
    }
    out
}

fn panel(width: f32, height: f32) -> [f32; 4] {
    let w = 360.0f32.min((width - 16.0).max(240.0));
    let h = 240.0f32.min((height - 16.0).max(180.0));
    [((width - w) / 2.0).max(4.0), ((height - h) / 2.0).max(4.0), w, h]
}

fn inside(point: [f32; 2], rect: [f32; 4]) -> bool {
    point[0] >= rect[0]
        && point[1] >= rect[1]
        && point[0] < rect[0] + rect[2]
        && point[1] < rect[1] + rect[3]
}

fn entries_in_category(ui: &BookUi) -> Vec<usize> {
    let category = ui.data.categories.get(ui.category)
        .and_then(|value| value.get("id")).and_then(serde_json::Value::as_str).unwrap_or("");
    let full = format!("theurgy:{category}");
    ui.data.entries.iter().enumerate().filter_map(|(index, entry)| {
        let value = entry.get("category")?.as_str()?;
        (value == category || value == full).then_some(index)
    }).collect()
}

fn draw_lines(
    game: &crate::game::Game<'_>,
    batch: &mut HudBatch,
    value: &str,
    mut at: [f32; 2],
    max_width: f32,
    max_lines: usize,
    rgb: u32,
) -> f32 {
    let cleaned = clean_markdown(value);
    let mut lines = 0usize;
    for source in cleaned.lines() {
        let mut line = String::new();
        for word in source.split_whitespace() {
            let candidate = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
            if !line.is_empty() && game.native_text_width(&candidate) > max_width {
                if lines >= max_lines {
                    return at[1];
                }
                game.native_text(batch, &line, at, rgb, false);
                at[1] += 10.0;
                lines += 1;
                line.clear();
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if lines >= max_lines {
            return at[1];
        }
        game.native_text(batch, &line, at, rgb, false);
        at[1] += 10.0;
        lines += 1;
    }
    at[1]
}

/// Opens the native Hermetica. False means the current platform cannot open a
/// client screen (headless/server/already covered by another screen).
pub fn open_book(game: &mut crate::game::Game<'_>) -> bool {
    crate::game::theurgy_filter_client::close();
    if !game.native_mod_screen_open("theurgy") {
        return false;
    }
    let Some(bytes) = game.native_data("data/theurgy/hermetica.json") else {
        game.native_mod_screen_close("theurgy");
        return false;
    };
    let Ok(data) = serde_json::from_slice::<BookData>(&bytes) else {
        game.native_mod_screen_close("theurgy");
        return false;
    };
    if data.categories.is_empty() || data.entries.is_empty() {
        game.native_mod_screen_close("theurgy");
        return false;
    }
    let book_title = data.book.get("name").and_then(serde_json::Value::as_str).unwrap_or("");
    let _ = book_title;
    if let Ok(mut slot) = BOOK_UI.lock() {
        *slot = Some(BookUi { data, category: 0, entry: None, page: 0, list_page: 0 });
        true
    } else {
        game.native_mod_screen_close("theurgy");
        false
    }
}

pub fn render_post(
    game: &crate::game::Game<'_>,
    batch: &mut HudBatch,
    screen: &NativeScreenView<'_>,
) {
    if screen.id != "mod" || game.native_mod_screen() != Some("theurgy") {
        return;
    }
    if crate::game::theurgy_filter_client::render(game, batch, screen) {
        return;
    }
    let Ok(guard) = BOOK_UI.lock() else { return; };
    let Some(ui) = guard.as_ref() else { return; };
    let rect = panel(batch.width, batch.height);
    game.native_fill(batch, [0.0, 0.0, batch.width, batch.height], 0xA0000000);
    game.native_fill(batch, rect, 0xFF18251F);
    game.native_fill(batch, [rect[0] + 3.0, rect[1] + 3.0, rect[2] - 6.0, rect[3] - 6.0], 0xFF263A30);
    game.native_text(batch, "The Hermetica", [rect[0] + 9.0, rect[1] + 8.0], 0xE7D59A, false);
    for (index, category) in ui.data.categories.iter().take(3).enumerate() {
        let x = rect[0] + 8.0 + index as f32 * 112.0;
        let selected = index == ui.category;
        game.native_fill(batch, [x, rect[1] + 23.0, 106.0, 15.0], if selected { 0xFF657B48 } else { 0xFF34483A });
        let name = value_text(&ui.data, category, "name");
        game.native_text(batch, name, [x + 4.0, rect[1] + 27.0], 0xFFFFFF, false);
    }
    if let Some(entry_index) = ui.entry {
        let Some(entry) = ui.data.entries.get(entry_index) else { return; };
        game.native_fill(batch, [rect[0] + 8.0, rect[1] + 42.0, 42.0, 14.0], 0xFF4A5C50);
        game.native_text(batch, "< Back", [rect[0] + 12.0, rect[1] + 46.0], 0xFFFFFF, false);
        let pages = entry.get("pages").and_then(serde_json::Value::as_array);
        let page = pages.and_then(|pages| pages.get(ui.page));
        let title = page.map_or_else(|| value_text(&ui.data, entry, "name"), |page| {
            let title = value_text(&ui.data, page, "title");
            if title.is_empty() { value_text(&ui.data, entry, "name") } else { title }
        });
        game.native_text(batch, title, [rect[0] + 57.0, rect[1] + 46.0], 0xE7D59A, false);
        if let Some(page) = page {
            let kind = page.get("type").and_then(serde_json::Value::as_str).unwrap_or("");
            let mut y = rect[1] + 64.0;
            if let Some(item) = page.get("item").and_then(serde_json::Value::as_str) {
                game.native_item(batch, item, [rect[0] + 12.0, y - 3.0], 24.0);
                y += 27.0;
            }
            if let Some(images) = page.get("images").and_then(serde_json::Value::as_array) {
                if let Some(image) = images.first().and_then(serde_json::Value::as_str) {
                    let sprite = image.replace("textures/", "").trim_end_matches(".png").to_owned();
                    if let Some([w, h]) = game.native_sprite_size(&sprite) {
                        let width = 150.0f32.min(rect[2] - 24.0);
                        let height = (width * h / w.max(1.0)).min(82.0);
                        game.native_sprite(batch, &sprite, [0.0, 0.0, w, h], [rect[0] + 12.0, y, width, height]);
                        y += height + 5.0;
                    }
                }
            }
            let body = value_text(&ui.data, page, "text");
            y = draw_lines(game, batch, body, [rect[0] + 12.0, y], rect[2] - 24.0, 9, 0xE3E9DF);
            for key in ["recipe_id_1", "recipe_id_2", "multiblock_id"] {
                if let Some(id) = page.get(key).and_then(serde_json::Value::as_str) {
                    game.native_text(batch, id, [rect[0] + 12.0, y], 0xA9C9B0, false);
                    y += 10.0;
                }
            }
            if body.is_empty() && !kind.is_empty() {
                game.native_text(batch, kind, [rect[0] + 12.0, y], 0xA9C9B0, false);
            }
        }
        let count = pages.map_or(0, Vec::len);
        game.native_fill(batch, [rect[0] + 8.0, rect[1] + rect[3] - 22.0, 24.0, 14.0], 0xFF4A5C50);
        game.native_text(batch, "<", [rect[0] + 17.0, rect[1] + rect[3] - 18.0], 0xFFFFFF, false);
        game.native_text(batch, &format!("{} / {}", ui.page + 1, count.max(1)), [rect[0] + rect[2] / 2.0 - 14.0, rect[1] + rect[3] - 18.0], 0xFFFFFF, false);
        game.native_fill(batch, [rect[0] + rect[2] - 32.0, rect[1] + rect[3] - 22.0, 24.0, 14.0], 0xFF4A5C50);
        game.native_text(batch, ">", [rect[0] + rect[2] - 23.0, rect[1] + rect[3] - 18.0], 0xFFFFFF, false);
        return;
    }

    let entries = entries_in_category(ui);
    let per_page = 9usize;
    for (row, index) in entries.iter().skip(ui.list_page * per_page).take(per_page).enumerate() {
        let Some(entry) = ui.data.entries.get(*index) else { continue; };
        let y = rect[1] + 43.0 + row as f32 * 17.0;
        game.native_fill(batch, [rect[0] + 8.0, y, rect[2] - 16.0, 15.0], 0xFF34483A);
        if let Some(icon) = entry.get("icon").and_then(|icon| icon.get("id")).and_then(serde_json::Value::as_str) {
            game.native_item(batch, icon, [rect[0] + 10.0, y - 1.0], 16.0);
        }
        game.native_text(batch, value_text(&ui.data, entry, "name"), [rect[0] + 30.0, y + 4.0], 0xFFFFFF, false);
    }
    let pages = entries.len().div_ceil(per_page).max(1);
    game.native_text(batch, &format!("Entries: {}   Page {}/{}", entries.len(), ui.list_page + 1, pages), [rect[0] + 10.0, rect[1] + rect[3] - 19.0], 0xE7D59A, false);
}

pub fn mouse_pressed(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    if screen.id != "mod" || game.native_mod_screen() != Some("theurgy") || button != 0 {
        return;
    }
    if crate::game::theurgy_filter_client::click(game, screen, button, canceled) {
        return;
    }
    let Ok(mut guard) = BOOK_UI.lock() else { return; };
    let Some(ui) = guard.as_mut() else { return; };
    let rect = panel(screen.rect[2], screen.rect[3]);
    let point = screen.mouse;
    *canceled = true;
    for index in 0..ui.data.categories.len().min(3) {
        let tab = [rect[0] + 8.0 + index as f32 * 112.0, rect[1] + 23.0, 106.0, 15.0];
        if inside(point, tab) {
            ui.category = index;
            ui.entry = None;
            ui.list_page = 0;
            ui.page = 0;
            return;
        }
    }
    if let Some(entry_index) = ui.entry {
        if inside(point, [rect[0] + 8.0, rect[1] + 42.0, 42.0, 14.0]) {
            ui.entry = None;
            ui.page = 0;
            return;
        }
        let count = ui.data.entries.get(entry_index).and_then(|entry| entry.get("pages"))
            .and_then(serde_json::Value::as_array).map_or(0, Vec::len);
        if inside(point, [rect[0] + 8.0, rect[1] + rect[3] - 22.0, 24.0, 14.0]) {
            ui.page = ui.page.saturating_sub(1);
        } else if inside(point, [rect[0] + rect[2] - 32.0, rect[1] + rect[3] - 22.0, 24.0, 14.0]) {
            ui.page = (ui.page + 1).min(count.saturating_sub(1));
        }
        return;
    }
    let entries = entries_in_category(ui);
    for (row, index) in entries.iter().skip(ui.list_page * 9).take(9).enumerate() {
        let row_rect = [rect[0] + 8.0, rect[1] + 43.0 + row as f32 * 17.0, rect[2] - 16.0, 15.0];
        if inside(point, row_rect) {
            ui.entry = Some(*index);
            ui.page = 0;
            return;
        }
    }
    let pages = entries.len().div_ceil(9).max(1);
    if point[1] >= rect[1] + rect[3] - 25.0 {
        if point[0] < rect[0] + rect[2] / 2.0 {
            ui.list_page = ui.list_page.saturating_sub(1);
        } else {
            ui.list_page = (ui.list_page + 1).min(pages - 1);
        }
    }
}

pub fn mouse_scrolled(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    scroll: [f64; 2],
    canceled: &mut bool,
) {
    if screen.id != "mod" || game.native_mod_screen() != Some("theurgy") {
        return;
    }
    if crate::game::theurgy_filter_client::scroll(screen, game, scroll[1], canceled) {
        return;
    }
    let Ok(mut guard) = BOOK_UI.lock() else { return; };
    let Some(ui) = guard.as_mut() else { return; };
    *canceled = true;
    if let Some(index) = ui.entry {
        let count = ui.data.entries.get(index).and_then(|entry| entry.get("pages"))
            .and_then(serde_json::Value::as_array).map_or(0, Vec::len);
        ui.page = if scroll[1] > 0.0 { ui.page.saturating_sub(1) } else { (ui.page + 1).min(count.saturating_sub(1)) };
    } else {
        let pages = entries_in_category(ui).len().div_ceil(9).max(1);
        ui.list_page = if scroll[1] > 0.0 { ui.list_page.saturating_sub(1) } else { (ui.list_page + 1).min(pages - 1) };
    }
}

fn derivative_source(game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack) -> Option<String> {
    if let Some(item) = crate::game::theurgy_items::component_string(game, stack, "theurgy:source_item")
        .filter(|item| game.res.items().get(item).is_some())
    {
        return Some(item);
    }
    let tag = crate::game::theurgy_items::component_string(game, stack, "theurgy:source_tag")?;
    let tag = tag.trim_start_matches('#');
    let mut items: Vec<_> = game.res.items().item_tags.get(tag)?.iter().cloned().collect();
    items.sort();
    items.into_iter().next()
}

fn derivative_tier(game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack) -> Option<&'static str> {
    if stack.item.starts_with("theurgy:alchemical_niter_") {
        return ["abundant", "common", "commond", "rare", "precious"].into_iter()
            .find(|tier| stack.item.ends_with(&format!("_{tier}")))
            .map(|tier| if tier == "commond" { "common" } else { tier });
    }
    ["abundant", "common", "rare", "precious"].into_iter().find(|tier| {
        game.res.items().item_in_tag(&format!("theurgy:alchemical_sulfurs/{tier}"), &stack.item)
    })
}

/// AlchemicalDerivativeRenderer/AlchemicalSaltRenderer's GUI portion: tier
/// frame, label and small source over the visible jar base. Holding Shift
/// shows the source full-size instead, as the Java special renderers do.
pub fn item_decorations(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    x: f32,
    y: f32,
    batch: &mut HudBatch,
) {
    if !stack.item.starts_with("theurgy:alchemical_sulfur_")
        && !stack.item.starts_with("theurgy:alchemical_niter_")
    {
        return;
    }
    let source = derivative_source(game, stack);
    if game.has_shift_down() {
        if let Some(source) = source {
            game.native_item(batch, &source, [x, y], 16.0);
        }
        return;
    }
    if let Some(tier) = derivative_tier(game, stack) {
        game.native_item(batch, &format!("theurgy:jar_label_frame_{tier}_icon"), [x, y], 16.0);
    }
    game.native_item(batch, "theurgy:jar_label_icon", [x, y], 16.0);
    if let Some(source) = source {
        game.native_item(batch, &source, [x + 5.0, y + 7.0], 6.0);
    }
}

/// Client half of TheHermeticaItem.use. `use_item` is authoritative/server
/// side in Horizon, so the actual window must be opened from InteractionKey.
pub fn interaction_key(
    game: &mut crate::game::Game<'_>,
    button: i32,
    hand: horizon_client::first_person::Hand,
    canceled: &mut bool,
) {
    if *canceled || button != 1 {
        return;
    }
    let slot = if hand == horizon_client::first_person::Hand::Off { -106 } else { game.selected as i32 };
    let Some(stack) = game.native_carried_stack(slot).cloned() else { return; };
    if stack.item.as_ref() == "theurgy:the_hermetica" && open_book(game) {
        *canceled = true;
        return;
    }
    let filter_hand = if hand == horizon_client::first_person::Hand::Off { "off" } else { "main" };
    let close_block = game.native_raycast(6.0).is_some();
    if !close_block
        && matches!(stack.item.as_ref(), "theurgy:list_filter" | "theurgy:attribute_filter")
        && crate::game::theurgy_filter_client::open(game, &stack, filter_hand)
    {
        *canceled = true;
    }
}

/// Register the two keys from KeyMappingsRegistry. Both are intentionally
/// unbound by default, matching InputConstants.UNKNOWN upstream.
pub fn mods(mods: &mut Vec<horizon_client::mod_api::NativeMod>) {
    mods.push(horizon_client::mod_api::NativeMod {
        id: "theurgy".to_owned(),
        title: "Theurgy 1.120.1".to_owned(),
        description: "Classical alchemy: replicate and transmute matter".to_owned(),
        keys: vec![
            ["change_item_mode".to_owned(), "key.theurgy.change_item_mode".to_owned(), String::new()],
            ["show_held_stack_fit_outline".to_owned(), "key.theurgy.show_held_stack_fit_outline".to_owned(), String::new()],
        ],
        ..Default::default()
    });
}

pub fn key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_playing() {
        return;
    }
    let binding = game.native_mod_key("theurgy", "change_item_mode", "");
    if !binding.is_empty() && binding == key {
        crate::game::theurgy_items::key_change_mode(game);
    }
}

/// ItemHUDProvider's wand text. Kept small and above the hotbar so it does
/// not hide block-target tooltips or the minimap.
pub fn hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    let Some(stack) = game.native_carried_stack(game.selected as i32).cloned() else { return; };
    if stack.item.as_ref() != "theurgy:mercurial_wand" {
        return;
    }
    let mode = crate::game::theurgy_items::wand_mode(game, &stack);
    let mode_name = match mode {
        0 => "Cycle selected direction",
        1 => "Enable/Disable",
        2 => "Select frequency",
        3 => "Set selected frequency",
        4 => "Cycle capacitor side",
        _ => "Unknown mode",
    };
    let frequency = crate::game::theurgy_items::component_i32(game, &stack, "theurgy:selected_frequency").unwrap_or(0);
    let (x, y) = corners.place(horizon_client::extras::Corner::TopLeft, 166.0, 27.0);
    game.native_fill(batch, [x, y, 166.0, 27.0], 0x7800_0000);
    game.native_text(batch, mode_name, [x + 4.0, y + 4.0], 0xE7D59A, false);
    game.native_text(batch, &format!("Frequency: {frequency}"), [x + 4.0, y + 16.0], 0xFFFFFF, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_cleanup_keeps_labels_and_removes_formatting() {
        assert_eq!(clean_markdown("§a[Logistics](category://theurgy:the_hermetica/logistics)§r"), "Logistics");
        assert_eq!(clean_markdown("first\\\nsecond"), "first\nsecond");
    }

    #[test]
    fn hit_test_excludes_far_edge() {
        assert!(inside([10.0, 10.0], [10.0, 10.0, 20.0, 20.0]));
        assert!(!inside([30.0, 10.0], [10.0, 10.0, 20.0, 20.0]));
    }
}
