// SPDX-License-Identifier: MIT
// Theurgy client glue: Hermetica/recipe/filter screen dispatch, derivative
// item decorations, key bindings and the item HUD. The Hermetica itself
// (TheHermeticaItem.use -> Modonomicon screen) lives in book.rs.

use horizon_client::hud::HudBatch;
use horizon_client::mod_api::NativeScreenView;

/// Opens the native Hermetica (native/theurgy/book.rs).
pub fn open_book(game: &mut crate::game::Game<'_>) -> bool {
    crate::game::theurgy_recipe_client::close();
    crate::game::theurgy_filter_client::close();
    let Some(bytes) = game.native_data("data/theurgy/hermetica.json") else { return false; };
    if !crate::game::theurgy_book::open(&bytes) {
        return false;
    }
    if !game.native_mod_screen_open("theurgy") {
        crate::game::theurgy_book::close();
        return false;
    }
    true
}

/// book.rs drawing surface over Horizon's HUD batch.
struct BookCanvas<'a, 'g> {
    game: &'a crate::game::Game<'g>,
    batch: &'a mut HudBatch,
}

fn book_text(runs: &[crate::game::theurgy_book::Run]) -> horizon_client::text::Text {
    let mut text = horizon_client::text::Text::default();
    for run in runs {
        text.spans.push(horizon_client::text::Span {
            text: run.text.clone(),
            style: horizon_client::text::Style {
                color: Some(run.rgb),
                bold: Some(run.bold),
                italic: Some(run.italic),
                font: run.book_font.then(|| "theurgy:hermetica".to_owned()),
                ..Default::default()
            },
        });
    }
    text
}

/// ARGB -> the batch's packed RGBA.
fn book_rgba(argb: u32) -> u32 {
    let [a, r, g, b] = argb.to_be_bytes();
    horizon_client::hud::rgba(r, g, b, a)
}

impl crate::game::theurgy_book::Canvas for BookCanvas<'_, '_> {
    fn size(&self, name: &str) -> [f32; 2] {
        self.game.native_sprite_size(name).unwrap_or([0.0, 0.0])
    }
    fn sprite(&mut self, name: &str, src: [f32; 4], dst: [f32; 4], tint: u32) -> bool {
        let Some(mut r) = self.game.gui.atlas.sprites.get(name).copied() else { return false; };
        let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
        r.u0 += src[0] * du;
        r.v0 += src[1] * dv;
        r.u1 = r.u0 + src[2] * du;
        r.v1 = r.v0 + src[3] * dv;
        self.batch.rect_uv(dst[0], dst[1], dst[2], dst[3], r, book_rgba(tint));
        true
    }
    fn item(&mut self, id: &str, at: [f32; 2], size: f32) -> bool {
        let stack = horizon_client::items::ItemStack::new(id, 1);
        self.game.native_item_stack(self.batch, &stack, at, size)
            || self.game.native_item_model(self.batch, "theurgy:the_hermetica", id, at, size)
    }
    fn fill(&mut self, rect: [f32; 4], argb: u32) {
        self.game.native_fill(self.batch, rect, argb);
    }
    fn text(&mut self, runs: &[crate::game::theurgy_book::Run], at: [f32; 2], shadow: bool) {
        self.batch.text_styled(&self.game.gui.atlas, &book_text(runs), at[0], at[1], u32::MAX, shadow);
    }
    fn width(&self, runs: &[crate::game::theurgy_book::Run]) -> f32 {
        self.game.gui.atlas.font.width_text(&book_text(runs))
    }
    fn mark(&self) -> usize {
        self.batch.vertices.len()
    }
    fn clip_since(&mut self, mark: usize, r: [f32; 4]) {
        self.batch.clip_since(mark, r[0], r[1], r[0] + r[2], r[1] + r[3]);
    }
    fn scale_since(&mut self, mark: usize, origin: [f32; 2], scale: f32) {
        self.batch.scale_since(mark, origin[0], origin[1], scale, scale);
    }
    fn tooltip(&mut self, lines: &[Vec<crate::game::theurgy_book::Run>], at: [f32; 2]) {
        let lines: Vec<_> = lines.iter().map(|l| (book_text(l), u32::MAX)).collect();
        self.game.draw_tooltip_lines(self.batch, &lines, at[0], at[1]);
    }
    fn item_tooltip(&mut self, id: &str, at: [f32; 2]) {
        let name = self.game.stack_display_name(&horizon_client::items::ItemStack::new(id, 1));
        self.game.draw_tooltip_lines(self.batch, &[(name, u32::MAX)], at[0], at[1]);
    }
    fn time(&self) -> i64 {
        self.game.world.game_time
    }
    fn shift(&self) -> bool {
        self.game.has_shift_down()
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
    let (w, h) = (batch.width, batch.height);
    crate::game::theurgy_book::render(&mut BookCanvas { game, batch }, w, h, screen.mouse);
}

/// Plays the page turn a click queued and sends changed book progress
/// (clicks only get `&Game`).
pub fn frame(game: &mut crate::game::Game<'_>) {
    if crate::game::theurgy_book::take_turn_sound()
        && let Some(p) = game.native_player()
    {
        game.native_play_local_sound("minecraft:item.book.page_turn", [p.x, p.y, p.z], 1.0, 1.0, 0);
    }
    if let Some(bytes) = crate::game::theurgy_book::take_progress() {
        // Without the server half (a vanilla server) it lasts this session.
        let _ = game.native_net_send_to_server(BOOK_CHANNEL, &bytes);
    }
}

// ---- Hermetica progress, server half: Modonomicon keeps read entries,
// research and bookmarks per player and world (BookUnlockStates). ---------

const BOOK_CHANNEL: &str = "theurgy:book";
static BOOK_PROGRESS: std::sync::Mutex<std::collections::BTreeMap<u128, Vec<u8>>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

/// `players/data/<uuid>.theurgy_book.json`.
fn book_file(directory: &std::path::Path, player: u128) -> std::path::PathBuf {
    let h = format!("{player:032x}");
    directory.join(format!("{}-{}-{}-{}-{}.theurgy_book.json", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..]))
}

pub fn book_load(player: u128, directory: &std::path::Path) {
    let bytes = std::fs::read(book_file(directory, player)).unwrap_or_else(|_| b"{}".to_vec());
    if let Ok(mut all) = BOOK_PROGRESS.lock() {
        all.insert(player, bytes);
    }
}

pub fn book_save(player: u128, directory: &std::path::Path) {
    let bytes = BOOK_PROGRESS.lock().ok().and_then(|all| all.get(&player).cloned());
    if let Some(bytes) = bytes {
        let _ = std::fs::write(book_file(directory, player), bytes);
    }
}

pub fn book_logged_in(game: &mut crate::game::Game<'_>, player: u128) {
    let bytes = BOOK_PROGRESS.lock().ok().and_then(|all| all.get(&player).cloned()).unwrap_or_else(|| b"{}".to_vec());
    let _ = game.native_net_send_to_player(player, BOOK_CHANNEL, &bytes);
}

/// The player's own bytes: kept only when they are a JSON object.
pub fn book_from_client(player: u128, channel: &str, data: &[u8]) {
    if channel == BOOK_CHANNEL
        && serde_json::from_slice::<serde_json::Map<String, serde_json::Value>>(data).is_ok()
        && let Ok(mut all) = BOOK_PROGRESS.lock()
    {
        all.insert(player, data.to_vec());
    }
}

pub fn book_from_server(channel: &str, data: &[u8]) {
    if channel == BOOK_CHANNEL {
        crate::game::theurgy_book::set_progress(data);
    }
}

pub fn screen_char(game: &crate::game::Game<'_>, screen: &NativeScreenView<'_>, c: char, canceled: &mut bool) {
    if book_screen(game, screen) && crate::game::theurgy_book::char_typed(c) {
        *canceled = true;
    }
}

/// The book's screen hooks only act while the theurgy mod screen is open.
fn book_screen(game: &crate::game::Game<'_>, screen: &NativeScreenView<'_>) -> bool {
    screen.id == "mod" && game.native_mod_screen() == Some("theurgy")
}

pub fn mouse_dragged(game: &crate::game::Game<'_>, screen: &NativeScreenView<'_>, button: i32, delta: [f32; 2], canceled: &mut bool) {
    if book_screen(game, screen) && crate::game::theurgy_book::drag(button, delta) {
        *canceled = true;
    }
}

pub fn screen_key(game: &crate::game::Game<'_>, screen: &NativeScreenView<'_>, key: &str, canceled: &mut bool) {
    if book_screen(game, screen) && crate::game::theurgy_book::key(key) {
        *canceled = true;
    }
}

pub fn mouse_pressed(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    if screen.id != "mod" || game.native_mod_screen() != Some("theurgy") {
        return;
    }
    if button == 0 && crate::game::theurgy_recipe_client::click(game, screen, button, canceled) {
        return;
    }
    if button == 0 && crate::game::theurgy_filter_client::click(game, screen, button, canceled) {
        return;
    }
    if crate::game::theurgy_book::click(screen.rect[2], screen.rect[3], screen.mouse, button, game.has_shift_down()) {
        *canceled = true;
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
    if crate::game::theurgy_book::scroll(scroll[1]) {
        *canceled = true;
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
        && { crate::game::theurgy_book::close(); crate::game::theurgy_filter_client::open(game, &stack, filter_hand) }
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
        crate::game::theurgy_book::close();
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
