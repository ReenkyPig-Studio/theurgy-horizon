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
    crate::game::theurgy_recipe_client::close();
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
        // The atlas (and its mod image slot) may have been rebuilt since.
        if let Ok(mut uploaded) = UPLOADED_IMAGE.lock() {
            *uploaded = None;
        }
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
    if crate::game::theurgy_recipe_client::render(game, batch, screen) {
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
            draw_page(game, batch, &ui.data, page, rect, screen.mouse);
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

/// Ticks each alternative of an ingredient stays on screen, as JEI/Modonomicon
/// cycle tag ingredients.
const CYCLE_TICKS: i64 = 30;
/// Ticks each image of a multi-image page stays (BookImagePageRenderer cycles).
const IMAGE_TICKS: i64 = 40;
const SLOT: f32 = 18.0;
/// horizon-client-render hud.rs MOD_IMAGE_SPRITE (the native_mod_image slot).
const MOD_IMAGE_SPRITE: &str = "horizon/modimage";

/// The image slot currently uploaded through `native_mod_image`.
static UPLOADED_IMAGE: Mutex<Option<(String, [f32; 2])>> = Mutex::new(None);

fn cycled<'a>(items: &'a [serde_json::Value], time: i64, salt: usize) -> Option<&'a str> {
    if items.is_empty() {
        return None;
    }
    let index = ((time / CYCLE_TICKS) as usize).wrapping_add(salt) % items.len();
    items[index].as_str()
}

/// One item slot (`GuiGraphics` slot background + item), returns the item
/// under the mouse for the hover line.
fn draw_slot<'a>(
    game: &crate::game::Game<'_>,
    batch: &mut HudBatch,
    items: Option<&'a [serde_json::Value]>,
    at: [f32; 2],
    time: i64,
    salt: usize,
    mouse: [f32; 2],
    hovered: &mut Option<String>,
) {
    game.native_fill(batch, [at[0], at[1], SLOT, SLOT], 0xFF8B8B8B);
    game.native_fill(batch, [at[0] + 1.0, at[1] + 1.0, SLOT - 2.0, SLOT - 2.0], 0xFF373737);
    if let Some(item) = items.and_then(|items| cycled(items, time, salt)) {
        game.native_item(batch, item, [at[0] + 1.0, at[1] + 1.0], 16.0);
        if inside(mouse, [at[0], at[1], SLOT, SLOT]) {
            *hovered = Some(item.to_owned());
        }
    }
}

fn result_items(result: Option<&serde_json::Value>) -> (Vec<serde_json::Value>, String) {
    let Some(result) = result else { return (Vec::new(), String::new()); };
    if let Some(item) = result.get("item").and_then(serde_json::Value::as_str) {
        let count = result.get("count").and_then(serde_json::Value::as_i64).unwrap_or(1);
        return (vec![serde_json::Value::from(item)], if count > 1 { count.to_string() } else { String::new() });
    }
    if let Some(fluid) = result.get("fluid").and_then(serde_json::Value::as_str) {
        let amount = result.get("amount").and_then(serde_json::Value::as_i64).unwrap_or(0);
        return (Vec::new(), format!("{amount} mB {fluid}"));
    }
    (Vec::new(), String::new())
}

/// BookCraftingRecipePageRenderer / Theurgy's Book*RecipePage: ingredient
/// slots with cycling alternatives, an arrow and the result. Returns the
/// height used.
fn draw_recipe(
    game: &crate::game::Game<'_>,
    batch: &mut HudBatch,
    recipe: &serde_json::Value,
    at: [f32; 2],
    width: f32,
    time: i64,
    mouse: [f32; 2],
    hovered: &mut Option<String>,
) -> f32 {
    let kind = recipe.get("kind").and_then(serde_json::Value::as_str).unwrap_or("");
    let (result, result_note) = result_items(recipe.get("result"));
    if recipe.get("missing").is_some() {
        game.native_text(batch, "missing recipe", at, 0xFF8080, false);
        return 12.0;
    }
    if kind == "crafting" {
        let grid = recipe.get("grid").and_then(serde_json::Value::as_array);
        for index in 0..9 {
            let cell = grid.and_then(|grid| grid.get(index)).and_then(serde_json::Value::as_array);
            let x = at[0] + (index % 3) as f32 * SLOT;
            let y = at[1] + (index / 3) as f32 * SLOT;
            draw_slot(game, batch, cell.map(Vec::as_slice), [x, y], time, index, mouse, hovered);
        }
        game.native_text(batch, "=>", [at[0] + 3.0 * SLOT + 8.0, at[1] + SLOT + 5.0], 0xE7D59A, false);
        draw_slot(game, batch, Some(&result), [at[0] + 3.0 * SLOT + 28.0, at[1] + SLOT], time, 0, mouse, hovered);
        if !result_note.is_empty() {
            game.native_text(batch, &result_note, [at[0] + 3.0 * SLOT + 48.0, at[1] + SLOT + 5.0], 0xFFFFFF, false);
        }
        if recipe.get("shapeless").and_then(serde_json::Value::as_bool) == Some(true) {
            game.native_text(batch, "shapeless", [at[0] + 3.0 * SLOT + 8.0, at[1] + 3.0], 0xA9C9B0, false);
        }
        return 3.0 * SLOT + 4.0;
    }
    // Machine recipes: labelled input slots in a row, solvent/fluid, arrow, result.
    let inputs = recipe.get("inputs").and_then(serde_json::Value::as_array);
    let mut x = at[0];
    for (index, input) in inputs.into_iter().flatten().enumerate() {
        if x + SLOT > at[0] + width - 70.0 {
            break;
        }
        let items = input.get("items").and_then(serde_json::Value::as_array);
        draw_slot(game, batch, items.map(Vec::as_slice), [x, at[1]], time, index, mouse, hovered);
        let count = input.get("count").and_then(serde_json::Value::as_i64).unwrap_or(1);
        if count > 1 {
            game.native_text(batch, &count.to_string(), [x + 11.0, at[1] + 10.0], 0xFFFFFF, true);
        }
        x += SLOT + 2.0;
    }
    let mut note_y = at[1] + SLOT + 3.0;
    if let Some(fluid) = recipe.get("fluid").filter(|fluid| !fluid.is_null()) {
        let id = fluid.get("fluid").and_then(serde_json::Value::as_str).unwrap_or("");
        let amount = fluid.get("amount").and_then(serde_json::Value::as_i64).unwrap_or(0);
        game.native_text(batch, &format!("+ {amount} mB {id}"), [at[0], note_y], 0x9FD3FF, false);
        note_y += 10.0;
    }
    game.native_text(batch, "=>", [x + 4.0, at[1] + 5.0], 0xE7D59A, false);
    if result.is_empty() {
        game.native_text(batch, &result_note, [x + 20.0, at[1] + 5.0], 0x9FD3FF, false);
    } else {
        draw_slot(game, batch, Some(&result), [x + 20.0, at[1]], time, 0, mouse, hovered);
        if !result_note.is_empty() {
            game.native_text(batch, &result_note, [x + 40.0, at[1] + 5.0], 0xFFFFFF, false);
        }
    }
    let mut details = Vec::new();
    if let Some(time) = recipe.get("time").and_then(serde_json::Value::as_i64) {
        details.push(format!("{time} ticks"));
    }
    if let Some(extra) = recipe.get("extra").and_then(serde_json::Value::as_object) {
        for (key, value) in extra {
            details.push(format!("{key}: {value}"));
        }
    }
    if !details.is_empty() {
        game.native_text(batch, &details.join("  "), [at[0], note_y], 0xA9C9B0, false);
        note_y += 10.0;
    }
    note_y - at[1]
}

/// BookMultiblockPageRenderer replacement: every layer (top first, as the
/// Modonomicon pattern) drawn as a slice of block-item icons.
fn draw_multiblock(
    game: &crate::game::Game<'_>,
    batch: &mut HudBatch,
    multiblock: &serde_json::Value,
    at: [f32; 2],
    width: f32,
    mouse: [f32; 2],
    hovered: &mut Option<String>,
) -> f32 {
    let Some(layers) = multiblock.get("layers").and_then(serde_json::Value::as_array) else { return 0.0; };
    let mapping = multiblock.get("mapping").and_then(serde_json::Value::as_object);
    let cell = 14.0f32;
    let mut x = at[0];
    let mut height = 0.0f32;
    for (layer_index, layer) in layers.iter().enumerate() {
        let Some(rows) = layer.as_array() else { continue; };
        let columns = rows.iter().filter_map(serde_json::Value::as_str).map(|row| row.chars().count()).max().unwrap_or(0);
        let layer_width = columns as f32 * cell;
        if x + layer_width > at[0] + width {
            break;
        }
        game.native_text(batch, &format!("Layer {}", layer_index + 1), [x, at[1]], 0xA9C9B0, false);
        for (row_index, row) in rows.iter().filter_map(serde_json::Value::as_str).enumerate() {
            for (column, symbol) in row.chars().enumerate() {
                let cell_at = [x + column as f32 * cell, at[1] + 11.0 + row_index as f32 * cell];
                game.native_fill(batch, [cell_at[0], cell_at[1], cell - 1.0, cell - 1.0], 0xFF34483A);
                let block = mapping.and_then(|mapping| mapping.get(&symbol.to_string())).and_then(serde_json::Value::as_str);
                if let Some(block) = block {
                    game.native_item(batch, block, [cell_at[0], cell_at[1]], cell - 1.0);
                    if inside(mouse, [cell_at[0], cell_at[1], cell, cell]) {
                        *hovered = Some(block.to_owned());
                    }
                }
            }
            height = height.max(11.0 + (row_index + 1) as f32 * cell);
        }
        x += layer_width + 10.0;
    }
    height + 4.0
}

fn current_image(page: &serde_json::Value, time: i64) -> Option<(String, [f32; 2])> {
    let images = page.get("imageData")?.as_array()?;
    if images.is_empty() {
        return None;
    }
    let image = &images[((time / IMAGE_TICKS) as usize) % images.len()];
    let path = image.get("path")?.as_str()?.to_owned();
    let size = image.get("size")?.as_array()?;
    Some((path, [size.first()?.as_f64()? as f32, size.get(1)?.as_f64()? as f32]))
}

/// Uploads the shown page's image (once per change) into Horizon's mod image
/// slot; drawing reads it back with `native_sprite`.
pub fn frame(game: &mut crate::game::Game<'_>) {
    if game.native_mod_screen() != Some("theurgy") {
        return;
    }
    let time = game.world.game_time;
    let wanted = BOOK_UI.lock().ok().and_then(|guard| {
        let ui = guard.as_ref()?;
        let entry = ui.data.entries.get(ui.entry?)?;
        let page = entry.get("pages")?.as_array()?.get(ui.page)?;
        current_image(page, time)
    });
    let Some((path, size)) = wanted else { return; };
    let Ok(mut uploaded) = UPLOADED_IMAGE.lock() else { return; };
    if uploaded.as_ref().is_some_and(|(current, _)| *current == path) {
        return;
    }
    let Some(bytes) = game.native_data(&path) else { return; };
    if bytes.len() < 8 {
        return;
    }
    let width = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let height = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if game.native_mod_image(width, height, &bytes[8..]) {
        *uploaded = Some((path, size));
    }
}

fn draw_page(
    game: &crate::game::Game<'_>,
    batch: &mut HudBatch,
    data: &BookData,
    page: &serde_json::Value,
    rect: [f32; 4],
    mouse: [f32; 2],
) {
    let time = game.world.game_time;
    let left = rect[0] + 12.0;
    let width = rect[2] - 24.0;
    let bottom = rect[1] + rect[3] - 26.0;
    let mut y = rect[1] + 62.0;
    let mut hovered = None;
    let kind = page.get("type").and_then(serde_json::Value::as_str).unwrap_or("");
    // BookSpotlightPageRenderer: the item large and centred above the text.
    if let Some(item) = page.get("item").and_then(|item| item.as_str().or_else(|| item.get("id").and_then(serde_json::Value::as_str))) {
        let at = [rect[0] + rect[2] / 2.0 - 16.0, y];
        game.native_fill(batch, [at[0] - 2.0, at[1] - 2.0, 36.0, 36.0], 0xFF34483A);
        game.native_item(batch, item, at, 32.0);
        if inside(mouse, [at[0], at[1], 32.0, 32.0]) {
            hovered = Some(item.to_owned());
        }
        y += 38.0;
    }
    if let Some(recipes) = page.get("recipes").and_then(serde_json::Value::as_array) {
        for recipe in recipes {
            y += draw_recipe(game, batch, recipe, [left, y], width, time, mouse, &mut hovered) + 4.0;
        }
    }
    if let Some(multiblock) = page.get("multiblock") {
        y += draw_multiblock(game, batch, multiblock, [left, y], width, mouse, &mut hovered);
    }
    if let Some((path, [w, h])) = current_image(page, time) {
        let uploaded = UPLOADED_IMAGE.lock().ok().and_then(|slot| slot.clone());
        if uploaded.as_ref().is_some_and(|(current, _)| *current == path) {
            let max_h = (bottom - y - 30.0).clamp(40.0, 110.0);
            let scale = (max_h / h).min(width / w);
            let (dw, dh) = (w * scale, h * scale);
            game.native_fill(batch, [rect[0] + rect[2] / 2.0 - dw / 2.0 - 1.0, y - 1.0, dw + 2.0, dh + 2.0], 0xFF0E1511);
            game.native_sprite(batch, MOD_IMAGE_SPRITE, [0.0, 0.0, w, h],
                [rect[0] + rect[2] / 2.0 - dw / 2.0, y, dw, dh]);
            y += dh + 5.0;
        }
    }
    let body = value_text(data, page, "text");
    let lines = (((bottom - y) / 10.0).floor().max(1.0)) as usize;
    let end = draw_lines(game, batch, body, [left, y], width, lines, 0xE3E9DF);
    if body.is_empty() && kind == "modonomicon:text" {
        game.native_text(batch, kind, [left, end], 0xA9C9B0, false);
    }
    if let Some(item) = hovered {
        let text_width = game.native_text_width(&item);
        let at = [(mouse[0] + 8.0).min(rect[0] + rect[2] - text_width - 6.0), mouse[1] - 12.0];
        game.native_fill(batch, [at[0] - 3.0, at[1] - 2.0, text_width + 6.0, 12.0], 0xF0100010);
        game.native_text(batch, &item, at, 0xFFFFFF, false);
    }
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
    if crate::game::theurgy_recipe_client::click(game, screen, button, canceled) {
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
    if crate::game::theurgy_recipe_client::scroll(game, screen, scroll[1], canceled) {
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
    if let Some(source) = source
        && crate::game::theurgy_config::get().render_sulfur_source_item
    {
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
            ["open_recipes".to_owned(), "Open Theurgy Recipes".to_owned(), "key.keyboard.r".to_owned()],
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
    let recipe_binding = game.native_mod_key("theurgy", "open_recipes", "key.keyboard.r");
    if recipe_binding == key {
        crate::game::theurgy_recipe_client::open(game);
    }
}

/// ItemHUDProvider's wand text plus the native Jade-style target panel.
pub fn hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    let settings = crate::game::theurgy_config::get();
    if !settings.enable_item_hud {
        return;
    }
    let mut lines = Vec::new();
    if let Some(stack) = game.native_carried_stack(game.selected as i32).cloned()
        && stack.item.as_ref() == "theurgy:mercurial_wand"
    {
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
        lines.push(mode_name.to_owned());
        lines.push(format!("Frequency: {frequency}"));
    }
    if settings.enable_in_world_hud
        && let Some(hit) = game.native_raycast(6.0)
        && hit.block.starts_with("theurgy:")
    {
        lines.push(hit.block.clone());
        let entity = game.native_with_mod_block_entity([hit.block_x, hit.block_y, hit.block_z], |entity| entity.nbt.clone());
        if let Some(nbt) = entity {
            let progress = crate::game::theurgy_host::int(&nbt, "progress", 0);
            let total = crate::game::theurgy_host::int(&nbt, "totalTime", 0);
            if total > 0 {
                lines.push(format!("Progress: {progress} / {total}"));
            }
            if let Some(horizon_world::nbt::Tag::Compound(handler)) = nbt.get(&horizon_world::nbt::JavaString::from("mercuryFluxHandler")) {
                let energy = crate::game::theurgy_host::int(handler, "energy", 0);
                lines.push(format!("Mercury flux: {energy}"));
            }
            if let Some(horizon_world::nbt::Tag::List(fluids)) = nbt.get(&horizon_world::nbt::JavaString::from("fluids")) {
                if let Some(horizon_world::nbt::Tag::Compound(tank)) = fluids.first() {
                    let amount = crate::game::theurgy_host::int(tank, "Amount", 0);
                    if amount > 0 {
                        let fluid = match tank.get(&horizon_world::nbt::JavaString::from("Fluid")) {
                            Some(horizon_world::nbt::Tag::String(id)) => id.to_string_lossy(),
                            _ => "fluid".to_owned(),
                        };
                        lines.push(format!("{fluid}: {amount} mB"));
                    }
                }
            }
        }
    }
    if lines.is_empty() {
        return;
    }
    let scale = settings.hud_scale.clamp(0.25, 3.0);
    let height = (7.0 + lines.len() as f32 * 10.0) * scale;
    let width = 196.0 * scale;
    let (x, y) = corners.place(horizon_client::extras::Corner::TopLeft, width, height);
    game.native_fill(batch, [x, y, width, height], 0x7800_0000);
    for (index, line) in lines.iter().enumerate() {
        game.native_text(batch, line, [x + 4.0 * scale, y + 4.0 * scale + index as f32 * 10.0 * scale], if index == 0 { 0xE7D59A } else { 0xFFFFFF }, false);
    }
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
