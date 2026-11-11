// SPDX-License-Identifier: MIT
// Native replacement for the Modonomicon GUI of The Hermetica
// (`content/item/book/TheHermeticaItem.use` opens it). Layout constants and
// draw order follow Modonomicon 2.19.0 (MIT): client/gui/book/node/
// BookParentNodeScreen + BookCategoryNodeScreen (node category view),
// entry/EntryConnectionRenderer, entry/BookEntryDoublePageScreen +
// BookPaginatedScreen (double page entry view), render/page/Book*PageRenderer,
// gui/book/markdown/CoreComponentNodeRenderer (text), search/BookSearchScreen,
// bookmarks/BookBookmarksScreen, recentlyunlocked/BookRecentlyUnlockedScreen
// + BookPaginatedListScreen/EntryListButton (lists), button/BookSideButtonRenderer
// + ResearchProgressButton, entry/linkhandler/BookLinkHandler (links),
// bookstate/visibility/DefaultBookVisibilityService (hidden/locked/unlocked),
// and Theurgy's integration/modonomicon/page/calcination/
// BookCalcinationRecipePageRenderer.
// Book data: `data/theurgy/hermetica.json` (tools/build-content.mjs), theme
// sprites: `theurgy:gui/modonomicon/...` (copied from the Modonomicon jar).
// Not ported: the multiblock "visualize" ghost in the world (Horizon has no
// ghost block drawing), https links (vanilla's confirm-link screen) and the
// item-link recipe lookup (no recipe viewer: Modonomicon then does nothing).

use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;

const THEME: &str = "theurgy:gui/modonomicon/";
const GRID: f32 = 30.0;
const ENTRY: f32 = 26.0;
const FULL_W: f32 = 272.0;
const FULL_H: f32 = 180.0;
const BG_H: f32 = 178.0;
const PAGE_X: [f32; 2] = [12.0, 141.0];
const PAGE_TOP: f32 = 15.0;
const PAGE_W: f32 = 124.0;
const PAGE_H: f32 = 155.0;
const LINE: f32 = 9.0;
const TEXT_RGB: u32 = 0x000000;
/// BookTextRenderer link colour; ItemLinkRenderer.ITEM_LINK_COLOR.
const LINK_RGB: u32 = 0x5555FF;
const ITEM_LINK_RGB: u32 = 0x029E5A;
const CYCLE_TICKS: i64 = 20;
/// BookPaginatedListScreen.ENTRIES_IN_FIRST_PAGE / ENTRIES_PER_PAGE.
const LIST_FIRST: usize = 9;
const LIST_PAGE: usize = 11;

#[derive(Debug)]
struct BookData {
    categories: Vec<Value>,
    entries: Vec<Value>,
    translations: HashMap<String, String>,
    /// Research node -> the entries whose first view grants it (every
    /// Hermetica research hook is `modonomicon:entry_viewed_once`).
    research: HashMap<String, Vec<String>>,
}

/// The player's book state (BookUnlockStates + BookVisualStates), kept per
/// world by the server half in client.rs.
#[derive(Debug, Default)]
struct Progress {
    /// Opened entries: no unread indicator.
    read: BTreeSet<String>,
    /// entry_viewed_once research facts.
    viewed: BTreeSet<String>,
    /// BookAddress: an entry and its left page.
    bookmarks: Vec<(String, usize)>,
    /// Entries in the order they unlocked (BookVisibilitySnapshots).
    unlocked: Vec<String>,
    dirty: bool,
}

impl Progress {
    fn parse(bytes: &[u8]) -> Option<Progress> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        let strings = |key: &str| -> Vec<String> {
            v[key].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect()
        };
        let bookmarks = v["bookmarks"].as_array().into_iter().flatten()
            .filter_map(|b| Some((b.get(0)?.as_str()?.to_owned(), b.get(1)?.as_u64()? as usize))).collect();
        Some(Progress {
            read: strings("read").into_iter().collect(),
            viewed: strings("viewed").into_iter().collect(),
            bookmarks,
            unlocked: strings("unlocked"),
            dirty: false,
        })
    }

    fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec(&json!({"read": self.read, "viewed": self.viewed, "bookmarks": self.bookmarks, "unlocked": self.unlocked}))
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy)]
struct View {
    scroll: [f32; 2],
    zoom: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ListKind {
    Search,
    Bookmarks,
    Recent,
}

/// A BookPaginatedListScreen pushed over the node screen.
#[derive(Debug)]
struct List {
    kind: ListKind,
    query: String,
    /// openPagesIndex (one per double page).
    spread: usize,
}

#[derive(Debug)]
struct BookUi {
    data: BookData,
    progress: Progress,
    category: usize,
    views: Vec<View>,
    entry: Option<usize>,
    /// BookEntryDoublePageScreen.openPagesIndex (always even).
    spread: usize,
    /// BookImagePageRenderer.index of the left/right page.
    image: [usize; 2],
    list: Option<List>,
    /// BookGuiManager history (entry, spread), pushed by entry links.
    history: Vec<(usize, usize)>,
    /// Link areas of the last drawn double page.
    links: Vec<([f32; 4], String)>,
    /// BookEntryScreen.ticksInBook (multiblock rotation), held with Shift.
    ticks: f32,
    last_time: i64,
    turn_sound: bool,
    /// False once a recipe/filter screen took the theurgy mod screen.
    shown: bool,
}

static BOOK: Mutex<Option<BookUi>> = Mutex::new(None);
/// Progress the server sent before the book was first opened.
static PENDING: Mutex<Option<Progress>> = Mutex::new(None);

fn tr<'a>(data: &'a BookData, key: &'a str) -> &'a str {
    data.translations.get(key).map(String::as_str).unwrap_or(key)
}

/// A translation with its `%s` filled in order.
fn trf(data: &BookData, key: &str, args: &[&str]) -> String {
    args.iter().fold(tr(data, key).to_owned(), |s, a| s.replacen("%s", a, 1))
}

/// A JSON text component: a string, `{"text"}` or `{"translate", "with"}`.
fn component(data: &BookData, v: &Value) -> String {
    if let Some(s) = v.as_str().or_else(|| v.get("text").and_then(Value::as_str)) {
        return s.to_owned();
    }
    let args: Vec<String> = v.get("with").and_then(Value::as_array).into_iter().flatten().map(|a| component(data, a)).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    v.get("translate").and_then(Value::as_str).map_or_else(String::new, |key| trf(data, key, &args))
}

fn field<'a>(data: &'a BookData, value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).map_or("", |key| tr(data, key))
}

fn id_of(value: &Value) -> &str {
    value.get("id").and_then(Value::as_str).unwrap_or("")
}

fn entry_index(data: &BookData, id: &str) -> Option<usize> {
    data.entries.iter().position(|e| id_of(e) == id)
}

fn num(value: &Value, key: &str) -> f32 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0) as f32
}

fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn inside(p: [f32; 2], r: [f32; 4]) -> bool {
    p[0] >= r[0] && p[1] >= r[1] && p[0] <= r[0] + r[2] && p[1] <= r[1] + r[3]
}

// ---- unlocking (DefaultBookVisibilityService) -----------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum Show {
    Hidden,
    Locked,
    Unlocked,
}

/// BookCondition.test for the conditions the Hermetica uses.
fn test(ui: &BookUi, cond: Option<&Value>) -> bool {
    let Some(cond) = cond else { return true; };
    let children = || cond.get("children").and_then(Value::as_array).into_iter().flatten();
    match cond.get("type").and_then(Value::as_str).unwrap_or("modonomicon:none") {
        "modonomicon:none" | "modonomicon:true" => true,
        "modonomicon:research_node_unlocked" => cond.get("node_id").and_then(Value::as_str)
            .and_then(|node| ui.data.research.get(node))
            .is_some_and(|need| need.iter().all(|e| ui.progress.viewed.contains(e))),
        "modonomicon:or" => children().any(|c| test(ui, Some(c))),
        "modonomicon:and" => children().all(|c| test(ui, Some(c))),
        _ => false,
    }
}

fn unlocked(ui: &BookUi, entry: &Value) -> bool {
    test(ui, entry.get("condition"))
}

/// getEntryDisplayState: locked entries show once their parents unlock.
fn show(ui: &BookUi, entry: &Value) -> Show {
    if unlocked(ui, entry) {
        return Show::Unlocked;
    }
    let parents: Vec<bool> = entry.get("parents").and_then(Value::as_array).into_iter().flatten()
        .filter_map(|p| p.get("entry").and_then(Value::as_str))
        .map(|id| ui.data.entries.iter().find(|e| id_of(e) == id).is_some_and(|p| unlocked(ui, p)))
        .collect();
    let enough = if flag(entry, "show_when_any_parent_unlocked") { parents.contains(&true) } else { !parents.contains(&false) };
    if parents.is_empty() || enough {
        if flag(entry, "hide_while_locked") { Show::Hidden } else { Show::Locked }
    } else {
        Show::Hidden
    }
}

/// BookCondition.getTooltip (BookOrCondition adds its children's).
fn condition_tooltip(data: &BookData, cond: Option<&Value>, out: &mut Vec<String>) {
    let Some(cond) = cond else { return; };
    if let Some(t) = cond.get("tooltip") {
        out.push(component(data, t));
    }
    for child in cond.get("children").and_then(Value::as_array).into_iter().flatten() {
        condition_tooltip(data, Some(child), out);
    }
}

/// Records newly unlocked entries (recently unlocked list).
fn refresh(ui: &mut BookUi) {
    let new: Vec<String> = ui.data.entries.iter().filter(|e| unlocked(ui, e)).map(|e| id_of(e).to_owned())
        .filter(|id| !ui.progress.unlocked.contains(id)).collect();
    if !new.is_empty() {
        ui.progress.unlocked.extend(new);
        ui.progress.dirty = true;
    }
}

/// The entries research hooks watch (ResearchHooks.canProgressEntryViewedOnce
/// while not viewed yet).
fn hook_targets(data: &BookData) -> BTreeSet<&str> {
    data.research.values().flatten().map(String::as_str).collect()
}

/// (hasVisibleResearchProgress, hasAnyResearchProgress).
fn research_progress(ui: &BookUi) -> (bool, bool) {
    let pending: Vec<&str> = hook_targets(&ui.data).into_iter().filter(|t| !ui.progress.viewed.contains(*t)).collect();
    let visible = pending.iter().any(|t| ui.data.entries.iter().find(|e| id_of(e) == *t).is_some_and(|e| unlocked(ui, e)));
    (visible, !pending.is_empty())
}

/// BookGuiManager.openEntry: shows the entry in its category, marks it read
/// and viewed (entry_viewed_once).
fn open_entry(ui: &mut BookUi, index: usize, spread: usize) {
    let entry = &ui.data.entries[index];
    let id = id_of(entry).to_owned();
    let category = entry.get("category").and_then(Value::as_str).unwrap_or("");
    if let Some(c) = ui.data.categories.iter().position(|c| category == format!("theurgy:{}", id_of(c))) {
        ui.category = c;
    }
    ui.entry = Some(index);
    ui.spread = spread & !1;
    ui.image = [0, 0];
    ui.list = None;
    ui.ticks = 0.0;
    ui.turn_sound = true;
    let p = &mut ui.progress;
    if p.read.insert(id.clone()) | p.viewed.insert(id) {
        p.dirty = true;
        refresh(ui);
    }
}

// ---- drawing surface ------------------------------------------------------

/// A styled run of text (Minecraft Style: colour, bold, italic, and the
/// book font (`theurgy:hermetica`, Modonomicon's `modonomicon:default`) or the vanilla one).
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub text: String,
    pub rgb: u32,
    pub bold: bool,
    pub italic: bool,
    pub book_font: bool,
}

/// What the book draws on: Horizon's HUD batch in game (client.rs), a
/// recording canvas in tools/book-preview. Colours/tints are ARGB.
pub trait Canvas {
    /// Size of GUI sprite `name` (`theurgy:gui/...`), zero when missing.
    fn size(&self, name: &str) -> [f32; 2];
    fn sprite(&mut self, name: &str, src: [f32; 4], dst: [f32; 4], tint: u32) -> bool;
    /// An item (with its decorations) `size` px square; false if unknown.
    fn item(&mut self, id: &str, at: [f32; 2], size: f32) -> bool;
    fn fill(&mut self, rect: [f32; 4], argb: u32);
    fn text(&mut self, runs: &[Run], at: [f32; 2], shadow: bool);
    fn width(&self, runs: &[Run]) -> f32;
    /// Draw position for `clip_since`/`scale_since`.
    fn mark(&self) -> usize;
    fn clip_since(&mut self, mark: usize, rect: [f32; 4]);
    fn scale_since(&mut self, mark: usize, origin: [f32; 2], scale: f32);
    fn tooltip(&mut self, lines: &[Vec<Run>], at: [f32; 2]);
    /// Tooltip with the item's display name.
    fn item_tooltip(&mut self, id: &str, at: [f32; 2]);
    /// Client game time in ticks (ingredient cycling, multiblock rotation).
    fn time(&self) -> i64;
    /// Shift is held (Screen.hasShiftDown).
    fn shift(&self) -> bool;
}

const WHITE: u32 = 0xFFFFFFFF;

fn grey(v: u32) -> u32 {
    0xFF000000 | v << 16 | v << 8 | v
}

/// A theme sprite at its own size.
fn theme(c: &mut dyn Canvas, path: &str, at: [f32; 2], tint: u32) -> [f32; 2] {
    let name = format!("{THEME}{path}");
    let [w, h] = c.size(&name);
    c.sprite(&name, [0.0, 0.0, w, h], [at[0], at[1], w, h], tint);
    [w, h]
}

fn whole(c: &mut dyn Canvas, name: &str, dst: [f32; 4], tint: u32) -> bool {
    let [w, h] = c.size(name);
    c.sprite(name, [0.0, 0.0, w, h], dst, tint)
}

/// `src` repeated over `dst`, the last tile cropped (nine-slice `tile`).
fn tile(c: &mut dyn Canvas, name: &str, src: [f32; 4], dst: [f32; 4]) {
    if src[2] <= 0.0 || src[3] <= 0.0 {
        return;
    }
    let mut y = 0.0;
    while y < dst[3] {
        let h = src[3].min(dst[3] - y);
        let mut x = 0.0;
        while x < dst[2] {
            let w = src[2].min(dst[2] - x);
            c.sprite(name, [src[0], src[1], w, h], [dst[0] + x, dst[1] + y, w, h], WHITE);
            x += w;
        }
        y += h;
    }
}

/// GUI sprite `nine_slice` scaling with `stretch_inner: false`.
fn nine_slice(c: &mut dyn Canvas, name: &str, border: f32, dst: [f32; 4]) {
    let [sw, sh] = c.size(name);
    let b = border;
    let (iw, ih) = (sw - 2.0 * b, sh - 2.0 * b);
    let (dw, dh) = (dst[2] - 2.0 * b, dst[3] - 2.0 * b);
    let (x0, x1, x2) = (dst[0], dst[0] + b, dst[0] + dst[2] - b);
    let (y0, y1, y2) = (dst[1], dst[1] + b, dst[1] + dst[3] - b);
    for (sx, dx, w, src_w) in [(0.0, x0, b, b), (b, x1, dw, iw), (sw - b, x2, b, b)] {
        for (sy, dy, h, src_h) in [(0.0, y0, b, b), (b, y1, dh, ih), (sh - b, y2, b, b)] {
            tile(c, name, [sx, sy, src_w, src_h], [dx, dy, w, h]);
        }
    }
}

/// `ns:textures/gui/x.png` -> GUI sprite `ns:gui/x`.
fn texture_sprite(texture: &str) -> String {
    let (ns, path) = texture.split_once(':').unwrap_or(("minecraft", texture));
    format!("{ns}:{}", path.trim_start_matches("textures/").trim_end_matches(".png"))
}

/// BookIcon: an item `{"id"}` or a texture (`"ns:textures/gui/x.png"` or
/// `{"texture"}`) drawn square.
fn icon(c: &mut dyn Canvas, value: &Value, at: [f32; 2], size: f32) {
    if let Some(id) = value.get("id").and_then(Value::as_str) {
        c.item(id, at, size);
    } else if let Some(texture) = value.as_str().or_else(|| value.get("texture").and_then(Value::as_str)) {
        whole(c, &texture_sprite(texture), [at[0], at[1], size, size], WHITE);
    }
}

fn plain(s: &str, rgb: u32) -> Vec<Run> {
    vec![Run { text: s.to_owned(), rgb, bold: false, italic: false, book_font: false }]
}

// ---- markdown (CoreComponentNodeRenderer subset the Hermetica uses) ------

#[derive(Clone, Default, PartialEq)]
struct Fmt {
    bold: bool,
    italic: bool,
    color: Option<u32>,
    /// Link destination (`entry://`, `category://`, `item://`, `https://`).
    link: Option<String>,
}

/// One rendered component: a paragraph line, list item or blank line.
struct Block {
    spans: Vec<(String, Fmt)>,
    indent: bool,
}

fn push(spans: &mut Vec<(String, Fmt)>, s: &str, f: &Fmt) {
    if s.is_empty() {
        return;
    }
    match spans.last_mut() {
        Some((last, lf)) if lf == f => last.push_str(s),
        _ => spans.push((s.to_owned(), f.clone())),
    }
}

fn hex(s: &str) -> Option<u32> {
    (s.len() == 6).then(|| u32::from_str_radix(s, 16).ok()).flatten()
}

/// ChatFormatting colours `§0`..`§f`.
const FORMAT_RGB: [u32; 16] = [
    0x000000, 0x0000AA, 0x00AA00, 0x00AAAA, 0xAA0000, 0xAA00AA, 0xFFAA00, 0xAAAAAA,
    0x555555, 0x5555FF, 0x55FF55, 0x55FFFF, 0xFF5555, 0xFF55FF, 0xFFFF55, 0xFFFFFF,
];

/// Inline markdown: `**`, `*`/`_`, `[#](rrggbb)` colour, `[#]()` reset,
/// `[label](item://..|entry://..|https://..)` links, `\` escapes, `§` codes.
fn inline(src: &str, f: &mut Fmt, out: &mut Vec<(String, Fmt)>) {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut buf = String::new();
    macro_rules! flush { () => { push(out, &buf, f); buf.clear(); } }
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() && chars[i + 1].is_ascii_punctuation() {
            buf.push(chars[i + 1]);
            i += 2;
        } else if c == '§' && i + 1 < chars.len() {
            flush!();
            match chars[i + 1] {
                'o' => f.italic = true,
                'l' => f.bold = true,
                'r' => *f = Fmt { link: f.link.take(), ..Fmt::default() },
                d => if let Some(n) = d.to_digit(16) {
                    *f = Fmt { color: Some(FORMAT_RGB[n as usize]), link: f.link.take(), ..Fmt::default() };
                },
            }
            i += 2;
        } else if c == '*' && chars.get(i + 1) == Some(&'*') {
            flush!();
            f.bold = !f.bold;
            i += 2;
        } else if c == '*' || (c == '_' && (i == 0 || !chars[i - 1].is_alphanumeric())) || (c == '_' && f.italic) {
            flush!();
            f.italic = !f.italic;
            i += 1;
        } else if c == '[' {
            let close = (i + 1..chars.len()).find(|&j| chars[j] == ']');
            let target = close.filter(|&j| chars.get(j + 1) == Some(&'(')).and_then(|j| {
                (j + 2..chars.len()).find(|&k| chars[k] == ')').map(|k| (j, k))
            });
            let Some((j, k)) = target else {
                buf.push(c);
                i += 1;
                continue;
            };
            flush!();
            let label: String = chars[i + 1..j].iter().collect();
            let dest: String = chars[j + 2..k].iter().collect();
            if label == "#" || label.is_empty() {
                // ColorLinkRenderer: `[#](rrggbb)` starts a colour, `[#]()` ends it.
                f.color = hex(dest.trim_start_matches('#'));
            } else {
                let saved = f.clone();
                f.link = Some(dest);
                inline(&label, f, out);
                *f = saved;
            }
            i = k + 1;
        } else {
            buf.push(c);
            i += 1;
        }
    }
    flush!();
}

fn markdown(src: &str) -> Vec<Block> {
    let src = src.replace("\r\n", "\n");
    let mut blocks: Vec<Block> = Vec::new();
    let mut f = Fmt::default();
    for (index, para) in src.split("\n\n").filter(|p| !p.trim().is_empty()).enumerate() {
        let mut lines: Vec<&str> = para.lines().collect();
        while lines.first().is_some_and(|l| l.trim().is_empty()) {
            lines.remove(0);
        }
        let is_item = |l: &str| {
            let t = l.trim_start();
            t.starts_with("- ") || t.starts_with("* ") || t.split_once(". ").is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        };
        let list = lines.first().is_some_and(|l| is_item(l));
        // Paragraph: a blank line before every one but the first.
        if index > 0 && !list {
            blocks.push(Block { spans: Vec::new(), indent: false });
        }
        let mut current: Option<Block> = None;
        for line in lines {
            let hard = line.ends_with("  ") || line.ends_with('\\');
            let body = line.trim_end_matches('\\').trim_end();
            if list && is_item(line) {
                if let Some(done) = current.take() {
                    blocks.push(done);
                }
                let t = body.trim_start();
                let (marker, rest) = t.split_once(' ').unwrap_or((t, ""));
                let mut spans = Vec::new();
                push(&mut spans, &format!("{marker} "), &Fmt::default());
                inline(rest, &mut f, &mut spans);
                current = Some(Block { spans, indent: true });
                continue;
            }
            let block = current.get_or_insert_with(|| Block { spans: Vec::new(), indent: list });
            if !block.spans.is_empty() {
                let f2 = f.clone();
                push(&mut block.spans, " ", &f2);
            }
            inline(body.trim_start(), &mut f, &mut block.spans);
            if hard {
                blocks.push(current.take().unwrap_or(Block { spans: Vec::new(), indent: false }));
            }
        }
        if let Some(done) = current.take() {
            blocks.push(done);
        }
    }
    blocks
}

/// Spans as runs; `base` is the colour of unstyled text.
fn styled(spans: &[(String, Fmt)], base: u32, book_font: bool) -> Vec<Run> {
    spans.iter().map(|(text, f)| Run {
        text: text.clone(),
        rgb: f.color.unwrap_or(match &f.link {
            Some(l) if l.starts_with("item://") => ITEM_LINK_RGB,
            Some(_) => LINK_RGB,
            None => base,
        }),
        bold: f.bold,
        italic: f.italic,
        book_font,
    }).collect()
}

/// A tooltip line with `§` formatting, vanilla font.
fn rich(s: &str, rgb: u32) -> Vec<Run> {
    let mut spans = Vec::new();
    inline(s, &mut Fmt::default(), &mut spans);
    styled(&spans, rgb, false)
}

/// Word wrap of one block; continuation lines of list items are indented
/// (MarkdownComponentRenderUtils.wrapComponents(width, width - 10)).
fn wrap(c: &dyn Canvas, block: &Block, max: f32) -> Vec<(Vec<(String, Fmt)>, f32)> {
    let mut words: Vec<Vec<(String, Fmt)>> = vec![Vec::new()];
    for (s, f) in &block.spans {
        for (n, piece) in s.split(' ').enumerate() {
            if n > 0 {
                words.push(Vec::new());
            }
            if let Some(w) = words.last_mut() {
                push(w, piece, f);
            }
        }
    }
    let mut lines = Vec::new();
    let mut line: Vec<(String, Fmt)> = Vec::new();
    let mut indent = 0.0;
    for word in words {
        let mut candidate = line.clone();
        if !candidate.is_empty() {
            let f = word.first().map(|w| w.1.clone()).unwrap_or_default();
            push(&mut candidate, " ", &f);
        }
        for (s, f) in &word {
            push(&mut candidate, s, f);
        }
        if !line.is_empty() && c.width(&styled(&candidate, TEXT_RGB, true)) > max - indent {
            lines.push((line, indent));
            if block.indent {
                indent = 10.0;
            }
            line = word;
        } else {
            line = candidate;
        }
    }
    lines.push((line, indent));
    lines
}

/// BookPageRenderer.renderBookTextHolder: markdown wrapped to the page,
/// scaled down until it fits (`default_auto_scale`); link areas go to `links`.
fn body(c: &mut dyn Canvas, src: &str, at: [f32; 2], w: f32, h: f32, links: &mut Vec<([f32; 4], String)>) {
    if src.is_empty() {
        return;
    }
    let blocks = markdown(src);
    let mut scale = 1.0f32;
    let mut lines;
    loop {
        lines = blocks.iter().flat_map(|b| wrap(&*c, b, w / scale)).collect::<Vec<_>>();
        if lines.len() as f32 * LINE * scale <= h || scale <= 0.5 {
            break;
        }
        scale -= 0.05;
    }
    let start = c.mark();
    for (n, (line, indent)) in lines.iter().enumerate() {
        let y = at[1] + n as f32 * LINE;
        let mut x = at[0] + indent;
        for (s, f) in line {
            let run_w = c.width(&styled(&[(s.clone(), f.clone())], TEXT_RGB, true));
            if let Some(link) = &f.link {
                let r = [at[0] + (x - at[0]) * scale, at[1] + (y - at[1]) * scale, run_w * scale, LINE * scale];
                links.push((r, link.clone()));
            }
            x += run_w;
        }
        c.text(&styled(line, TEXT_RGB, true), [at[0] + indent, y], false);
    }
    if scale < 1.0 {
        c.scale_since(start, [at[0], at[1]], scale);
    }
}

/// BookPageRenderer.renderTitle: centred, shrunk past 120 px, optional
/// separator 12 px below.
fn title(c: &mut dyn Canvas, s: &str, at: [f32; 2], separator: bool) {
    if !s.is_empty() {
        let mut spans = Vec::new();
        inline(s, &mut Fmt::default(), &mut spans);
        let t = styled(&spans, TEXT_RGB, true);
        let w = c.width(&t);
        let scale = (120.0 / w.max(1.0)).min(1.0);
        let start = c.mark();
        let y = at[1] + LINE * (1.0 - scale);
        c.text(&t, [at[0] - w * scale / 2.0, y], false);
        if scale < 1.0 {
            let x = at[0] - w * scale / 2.0;
            c.scale_since(start, [x, y], scale);
        }
    }
    if separator {
        separator_at(c, [at[0], at[1] + 12.0]);
    }
}

/// BookContentRenderer.drawTitleSeparator: centred on `at`, 80% alpha.
fn separator_at(c: &mut dyn Canvas, at: [f32; 2]) {
    theme(c, "content/decorations/book/title_separator", [at[0] - 55.0, at[1]], 0xCCFFFFFF);
}

/// drawCenteredStringNoShadow, vanilla font.
fn centered(c: &mut dyn Canvas, s: &str, at: [f32; 2], rgb: u32) {
    let t = plain(s, rgb);
    let w = c.width(&t);
    c.text(&t, [at[0] - w / 2.0, at[1]], false);
}

// ---- layout shared by drawing and clicks ---------------------------------

struct Frame {
    outer: [f32; 4],
    inner: [f32; 4],
}

fn frame(w: f32, h: f32) -> Frame {
    let (fw, fh) = ((w - 60.0).floor(), (h - 20.0).floor());
    let (x, y) = (((w - fw) / 2.0).floor(), ((h - fh) / 2.0).floor());
    Frame { outer: [x, y, fw, fh], inner: [x + 7.0, y + 7.0, fw - 14.0, fh - 14.0] }
}

/// BookParentNodeScreen.updateCategoryButtons.
fn category_button(w: f32, h: f32, index: usize) -> [f32; 4] {
    let f = frame(w, h);
    let x = f.outer[0] - 14.0 - 11.0;
    let y = f.outer[1] - 14.0 + 30.0 + 2.0;
    let bw = f.outer[0] - 11.0 + 6.0;
    [x, y + 22.0 * index as f32, bw, 20.0]
}

fn offset(f: &Frame, view: &View) -> [f32; 2] {
    [f.inner[2] / 2.0 / view.zoom - view.scroll[0] / 2.0, f.inner[3] / 2.0 / view.zoom - view.scroll[1] / 2.0]
}

fn category_id(ui: &BookUi) -> String {
    let id = ui.data.categories.get(ui.category).and_then(|c| c.get("id")).and_then(Value::as_str).unwrap_or("");
    format!("theurgy:{id}")
}

fn entries(ui: &BookUi) -> Vec<usize> {
    let full = category_id(ui);
    ui.data.entries.iter().enumerate()
        .filter(|(_, e)| e.get("category").and_then(Value::as_str) == Some(full.as_str()))
        .map(|(i, _)| i).collect()
}

fn entry_rect(ui: &BookUi, f: &Frame, entry: &Value) -> [f32; 4] {
    let view = ui.views[ui.category];
    let o = offset(f, &view);
    let z = view.zoom;
    [(num(entry, "x") * GRID + o[0] + 2.0) * z, (num(entry, "y") * GRID + o[1] + 2.0) * z, ENTRY * z, ENTRY * z]
}

fn book_origin(w: f32, h: f32) -> [f32; 2] {
    [((w - FULL_W) / 2.0).floor(), ((h - BG_H) / 2.0).floor()]
}

fn pages(ui: &BookUi) -> &[Value] {
    ui.entry.and_then(|e| ui.data.entries.get(e)).and_then(|e| e.get("pages"))
        .and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn arrows(o: [f32; 2]) -> [[f32; 4]; 2] {
    [[o[0] - 4.0, o[1] + FULL_H - 6.0, 18.0, 10.0], [o[0] + FULL_W - 14.0, o[1] + FULL_H - 6.0, 18.0, 10.0]]
}

fn exit_button(o: [f32; 2]) -> [f32; 4] {
    [o[0] + FULL_W - 10.0, o[1] - 2.0, 12.0, 12.0]
}

/// BookImagePageRenderer button position (DEFAULT display mode), page-local.
fn image_buttons(left: [f32; 2]) -> [[f32; 4]; 2] {
    let (x, y) = (left[0] + 9.0 + 106.0 - 21.0, left[1] + 9.0 + 106.0 - 12.0);
    [[x, y, 5.0, 7.0], [x + 10.0, y, 5.0, 7.0]]
}

fn hovered_entry(ui: &BookUi, f: &Frame, mouse: [f32; 2]) -> Option<usize> {
    if !inside(mouse, f.inner) {
        return None;
    }
    entries(ui).into_iter().find(|&i| {
        let e = &ui.data.entries[i];
        show(ui, e) != Show::Hidden && inside(mouse, entry_rect(ui, f, e))
    })
}

/// BookParentNodeScreen.init: search, bookmarks and recently unlocked above
/// the frame's bottom right, the research progress button at its top right.
fn side_buttons(w: f32, h: f32) -> [[f32; 4]; 4] {
    let f = frame(w, h);
    let x = f.outer[0] + f.outer[2];
    let y = f.outer[3] - 26.0;
    [[x, y, 44.0, 20.0], [x, y - 22.0, 44.0, 20.0], [x, y - 44.0, 44.0, 20.0], [x, f.outer[1] + 18.0, 44.0, 20.0]]
}

/// BookEntryScreen.updateBookmarksButton.
fn bookmark_button(o: [f32; 2]) -> [f32; 4] {
    [o[0] + FULL_W, o[1] + FULL_H - 30.0, 44.0, 20.0]
}

/// BookEntryScreen.initNavigationButtons: BackButton.
fn back_button(w: f32, o: [f32; 2]) -> [f32; 4] {
    [(w / 2.0).floor() - 9.0, o[1] + FULL_H - 4.0, 18.0, 9.0]
}

/// BookSideButtonRenderer.renderSlidingButton + renderRightIcon (scale 2/3):
/// a 44 px button sliding out of the edge at `r[0]`.
fn side_button(c: &mut dyn Canvas, r: [f32; 4], hovered: bool, background: &str, icon: &str, icon_size: [f32; 2], dx: f32) {
    let x = r[0] - 16.0 + if hovered { 1.0 } else { 0.0 };
    let start = c.mark();
    theme(c, background, [x, r[1]], WHITE);
    let y = r[1] + 2.0 + ((16.0 - icon_size[1]) / 2.0).floor().max(0.0);
    whole(c, &format!("{THEME}{icon}"), [x + 44.0 - 11.0 - icon_size[0] + dx, y, icon_size[0], icon_size[1]], WHITE);
    c.clip_since(start, [r[0], r[1], r[2] - if hovered { 1.0 } else { 0.0 }, r[3]]);
}

fn images(page: &Value) -> Vec<String> {
    page.get("images").and_then(Value::as_array).map_or_else(Vec::new, |list| {
        list.iter().filter_map(Value::as_str).map(|id| {
            let (ns, path) = id.split_once(':').unwrap_or(("minecraft", id));
            format!("{ns}:{}", path.trim_start_matches("textures/").trim_end_matches(".png"))
        }).collect()
    })
}

// ---- category (node) screen ----------------------------------------------

/// Splits `length` GUI pixels of a texture `size` wide that repeats, starting
/// at texel `start`: `[texel, offset, run]` per piece.
fn tiles(start: f32, size: f32, length: f32) -> Vec<[f32; 3]> {
    let (mut texel, mut offset, mut out) = (start, 0.0, Vec::new());
    while offset < length && size >= 1.0 {
        let run = (size - texel).min(length - offset);
        out.push([texel, offset, run]);
        offset += run;
        texel = 0.0;
    }
    out
}

fn draw_category(c: &mut dyn Canvas, ui: &BookUi, w: f32, h: f32, mouse: [f32; 2]) {
    let f = frame(w, h);
    let [ix, iy, iw, ih] = f.inner;
    let category = &ui.data.categories[ui.category];
    let view = ui.views[ui.category];
    // renderBackground (no parallax layers): one texel per GUI pixel, scrolled.
    // GuiGraphics.blit repeats the texture past its edge; stretching it over a
    // frame wider than the texture smeared the texels instead.
    if let Some(bg) = category.get("background").and_then(Value::as_str) {
        let (ns, path) = bg.split_once(':').unwrap_or(("minecraft", bg));
        let name = format!("{ns}:{}", path.trim_start_matches("textures/").trim_end_matches(".png"));
        let (bw, bh) = (num(category, "background_width").max(1.0), num(category, "background_height").max(1.0));
        let max = bw.max(bh);
        let x_scale = max * 2.0 / (max + 14.0 - f.outer[2]).max(1.0);
        let y_scale = max * 2.0 / (max + 14.0 - f.outer[3]).max(1.0);
        let scale = x_scale.max(y_scale);
        let x_off = if x_scale == scale { 0.0 } else { (max - (iw + max * 2.0 / scale)) / 2.0 };
        let y_off = if y_scale == scale { 0.0 } else { (max - (ih + max * 2.0 / scale)) / 2.0 };
        let u0 = ((view.scroll[0] + max) / scale + x_off).floor().rem_euclid(bw);
        let v0 = ((view.scroll[1] + max) / scale + y_off).floor().rem_euclid(bh);
        let [tw, th] = c.size(&name);
        let (kx, ky) = (tw / bw, th / bh);
        let mut drawn = true;
        for [u, x, w] in tiles(u0, bw, iw) {
            for [v, y, h] in tiles(v0, bh, ih) {
                drawn &= c.sprite(&name, [u * kx, v * ky, w * kx, h * ky], [ix + x, iy + y, w, h], WHITE);
            }
        }
        if !drawn {
            c.fill(f.inner, 0xFF0B0B1E);
        }
    }
    // renderEntries: connections first, then backgrounds and icons, clipped;
    // hidden entries and their connections are left out.
    let start = c.mark();
    let o = offset(&f, &view);
    let z = view.zoom;
    let mine = entries(ui);
    let states: HashMap<usize, Show> = mine.iter().map(|&i| (i, show(ui, &ui.data.entries[i]))).collect();
    let by_id: HashMap<&str, usize> = mine.iter().map(|&i| (id_of(&ui.data.entries[i]), i)).collect();
    let mut blit = |c: &mut dyn Canvas, path: &str, gx: f32, gy: f32, dx: f32, dy: f32| {
        let name = format!("{THEME}node/connections/{path}");
        let [sw, sh] = c.size(&name);
        let at = [(gx * GRID + dx + o[0]) * z, (gy * GRID + dy + o[1]) * z];
        c.sprite(&name, [0.0, 0.0, sw, sh], [at[0], at[1], sw * z, sh * z], WHITE);
    };
    for &i in &mine {
        if states[&i] == Show::Hidden {
            continue;
        }
        let entry = &ui.data.entries[i];
        for parent in entry.get("parents").and_then(Value::as_array).into_iter().flatten() {
            let Some(&p) = parent.get("entry").and_then(Value::as_str).and_then(|id| by_id.get(id)) else { continue; };
            if states[&p] == Show::Hidden {
                continue;
            }
            let p = &ui.data.entries[p];
            let arrow = parent.get("draw_arrow").and_then(Value::as_bool).unwrap_or(true);
            let reversed = parent.get("line_reversed").and_then(Value::as_bool).unwrap_or(false);
            connection(c, &mut blit, [num(entry, "x"), num(entry, "y")], [num(p, "x"), num(p, "y")], arrow, reversed);
        }
    }
    let hovered = hovered_entry(ui, &f, mouse);
    for &i in &mine {
        let state = states[&i];
        if state == Show::Hidden {
            continue;
        }
        let entry = &ui.data.entries[i];
        let r = entry_rect(ui, &f, entry);
        let tint = if state == Show::Locked { grey(51) } else if hovered == Some(i) { grey(204) } else { WHITE };
        if let Some(bg) = entry.get("background").and_then(|b| b.get("sprite")).and_then(Value::as_str) {
            let path = bg.trim_start_matches("modonomicon:modonomicon/themes/default/");
            whole(c, &format!("{THEME}{path}"), r, tint);
        }
        if let Some(value) = entry.get("icon") {
            icon(c, value, [r[0] + 5.0 * z, r[1] + 5.0 * z], 16.0 * z);
        }
        if state == Show::Unlocked && !ui.progress.read.contains(id_of(entry)) {
            // BookContentRenderer.drawUnreadIndicator at the top right.
            let s = if hovered == Some(i) { "hover" } else { "normal" };
            let name = format!("{THEME}content/indicators/book/unread_indicator_{s}");
            whole(c, &name, [r[0] + 18.0 * z, r[1] - 2.0 * z, 11.0 * z, 11.0 * z], WHITE);
        }
    }
    c.clip_since(start, [ix, iy, iw - 1.0, ih - 1.0]);
    // renderFrame + overlays.
    let [fx, fy, fw, fh] = f.outer;
    nine_slice(c, &format!("{THEME}frame/frame"), 24.0, f.outer);
    for (path, cx, cy, dx, dy) in [
        ("frame/top_overlay", fx + (fw / 2.0).floor(), fy, 0.0, 8.0),
        ("frame/bottom_overlay", fx + (fw / 2.0).floor(), fy + fh, 0.0, -10.0),
        ("frame/left_overlay", fx, fy + (fh / 2.0).floor(), 8.0, 0.0),
        ("frame/right_overlay", fx + fw, fy + (fh / 2.0).floor(), -8.0, 0.0),
    ] {
        let [sw, sh] = c.size(&format!("{THEME}{path}"));
        theme(c, path, [cx - (sw / 2.0).floor() + dx, cy - (sh / 2.0).floor() + dy], WHITE);
    }
    // CategoryButton: selected shifts out 3 px, hovered 1 px, others dimmed.
    let mut tooltip: Option<Vec<Vec<Run>>> = None;
    for (index, category) in ui.data.categories.iter().enumerate().take(13) {
        let mut r = category_button(w, h, index);
        let hover = inside(mouse, r);
        let tint = if index == ui.category {
            r[0] -= 3.0;
            r[2] += 3.0;
            WHITE
        } else if hover {
            r[0] -= 1.0;
            r[2] += 1.0;
            WHITE
        } else {
            grey(204)
        };
        whole(c, &format!("{THEME}content/buttons/category/category_button_normal"), r, tint);
        if let Some(value) = category.get("icon") {
            icon(c, value, [r[0] + 8.0, r[1] + 2.0], 16.0);
        }
        if hover {
            tooltip = Some(vec![plain(field(&ui.data, category, "name"), 0xFFFFFF)]);
        }
    }
    // SearchButton, ShowBookmarksButton, ShowRecentlyUnlockedButton.
    let [search, bookmarks, recent, research] = side_buttons(w, h);
    for (r, icon, dx, tip) in [
        (search, "search_icon", -4.0, "modonomicon.gui.open_search"),
        (bookmarks, "show_bookmarks_icon", -4.0, "modonomicon.gui.open_bookmarks"),
        (recent, "unlocked_icon", -5.0, "modonomicon.gui.open_recently_unlocked"),
    ] {
        let hover = inside(mouse, r);
        side_button(c, r, hover, "content/buttons/collection/collection_button_normal", &format!("content/buttons/collection/{icon}"), [11.0, 11.0], dx);
        if hover {
            tooltip = Some(vec![plain(tr(&ui.data, tip), 0xFFFFFF)]);
        }
    }
    // ResearchProgressButton: only while some viewed-once research is left.
    let (visible, any) = research_progress(ui);
    if visible || any {
        let hover = inside(mouse, research);
        let kind = if c.shift() { "all" } else if visible { "unlocked" } else { "none" };
        let s = if hover { "hover" } else { "normal" };
        let icon = format!("content/buttons/read/read_{kind}_button_{s}");
        side_button(c, research, hover, "content/buttons/collection/collection_button_golden", &icon, [11.0, 9.0], -4.0);
        if hover {
            let [a, b] = if c.shift() { ["all", "shift_warning"] } else if visible { ["visible", "shift"] } else { ["none", "shift"] };
            let line = |k: &str| rich(tr(&ui.data, &format!("modonomicon.gui.button.viewed_once_research.tooltip.{k}")), 0xFFFFFF);
            tooltip = Some(vec![line(a), Vec::new(), line(b)]);
        }
    }
    // renderEntryTooltips: the condition while locked, else bold name and
    // the description.
    if let Some(i) = hovered {
        let entry = &ui.data.entries[i];
        if states[&i] == Show::Locked {
            let mut lines = Vec::new();
            condition_tooltip(&ui.data, entry.get("condition"), &mut lines);
            tooltip = Some(lines.iter().map(|l| rich(l, 0xFFFFFF)).collect());
        } else {
            let mut name = plain(field(&ui.data, entry, "name"), 0xFFFFFF);
            name[0].bold = true;
            let mut lines = vec![name];
            let description = field(&ui.data, entry, "description");
            if !description.is_empty() {
                lines.push(plain(description, 0xFFFFFF));
            }
            tooltip = Some(lines);
        }
    }
    if let Some(lines) = tooltip {
        c.tooltip(&lines, mouse);
    }
}

/// EntryConnectionRenderer.drawVertical/HorizontalLine(Shortened): tiles
/// strictly between `a` and `b`; `short` leaves out the one next to `b`.
fn line(c: &mut dyn Canvas, blit: &mut impl FnMut(&mut dyn Canvas, &str, f32, f32, f32, f32), vertical: bool, fixed: f32, a: f32, b: f32, short: bool) {
    let (mut lo, mut hi) = (a.min(b), a.max(b));
    if short {
        if a > b { lo += 1.0; } else { hi -= 1.0; }
    }
    let mut j = lo + 1.0;
    while j < hi {
        if vertical {
            blit(c, "vertical_line", fixed, j, 0.0, 0.0);
        } else {
            blit(c, "horizontal_line", j, fixed, 0.0, 0.0);
        }
        j += 1.0;
    }
}

/// EntryConnectionRenderer.render in grid units.
fn connection(c: &mut dyn Canvas, blit: &mut impl FnMut(&mut dyn Canvas, &str, f32, f32, f32, f32), e: [f32; 2], p: [f32; 2], arrow: bool, reversed: bool) {
    let [ex, ey] = e;
    let [px, py] = p;
    let (dx, dy) = ((ex - px).abs(), (ey - py).abs());
    let vline = |c: &mut dyn Canvas, blit: &mut _, x: f32, a: f32, b: f32, short: bool| line(c, blit, true, x, a, b, short);
    let hline = |c: &mut dyn Canvas, blit: &mut _, y: f32, a: f32, b: f32, short: bool| line(c, blit, false, y, a, b, short);
    // Arrows: (sprite, grid x, grid y, pixel dx, pixel dy).
    let up = ("up_arrow", ex, ey + 1.0, 0.0, -1.0);
    let down = ("down_arrow", ex, ey - 1.0, 0.0, 1.0);
    let left = ("left_arrow", ex + 1.0, ey, -1.0, 0.0);
    let right = ("right_arrow", ex - 1.0, ey, 1.0, 0.0);
    let mut arrows: Vec<(&str, f32, f32, f32, f32)> = Vec::new();
    let mut curve: Option<(&str, f32, f32)> = None;
    if dx == 0.0 {
        vline(c, blit, px, ey, py, false);
        arrows.push(if py > ey { up } else { down });
    } else if dy == 0.0 {
        hline(c, blit, py, ex, px, false);
        arrows.push(if px > ex { left } else { right });
    } else if dx >= 2.0 && dy >= 2.0 {
        if !reversed {
            hline(c, blit, py, px, ex, true);
            vline(c, blit, ex, ey, py, true);
            curve = Some(match (ex > px, ey > py) {
                (true, true) => ("large_curve_left_down", ex - 1.0, py),
                (true, false) => ("large_curve_left_up", ex - 1.0, py - 1.0),
                (false, true) => ("large_curve_right_down", ex, py),
                (false, false) => ("large_curve_right_up", ex, py - 1.0),
            });
            arrows.push(if ey > py { down } else { up });
        } else {
            hline(c, blit, ey, ex, px, true);
            vline(c, blit, px, py, ey, true);
            curve = Some(match (ex > px, ey > py) {
                (true, true) => ("large_curve_right_up", px, ey - 1.0),
                (true, false) => ("large_curve_right_down", px, ey),
                (false, true) => ("large_curve_left_up", px - 1.0, py + 1.0),
                (false, false) => ("large_curve_left_down", px - 1.0, ey),
            });
            arrows.push(if ex > px { right } else { left });
        }
    } else if !reversed {
        vline(c, blit, ex, py, ey, false);
        hline(c, blit, py, px, ex, false);
        curve = Some(match (ex > px, ey > py) {
            (true, true) => ("small_curve_left_down", ex, py),
            (true, false) => ("small_curve_left_up", ex, py),
            (false, true) => ("small_curve_right_down", ex, py),
            (false, false) => ("small_curve_right_up", ex, py),
        });
        arrows.push(if ey > py { down } else { up });
    } else {
        hline(c, blit, ey, ex, px, false);
        vline(c, blit, px, py, ey, false);
        curve = Some(match (ex < px, ey > py) {
            (true, true) => ("small_curve_left_up", px, ey),
            (true, false) => ("small_curve_left_down", px, ey),
            (false, true) => ("small_curve_right_up", px, ey),
            (false, false) => ("small_curve_right_down", px, ey),
        });
        arrows.push(if ex < px { left } else { right });
    }
    if let Some((name, x, y)) = curve {
        blit(c, name, x, y, 0.0, 0.0);
    }
    if arrow {
        for (name, x, y, ddx, ddy) in arrows {
            blit(c, name, x, y, ddx, ddy);
        }
    }
}

// ---- entry (double page) screen ------------------------------------------

struct Hover {
    item: Option<String>,
    text: Option<String>,
    links: Vec<([f32; 4], String)>,
}

fn slot_item(c: &mut dyn Canvas, items: &[Value], at: [f32; 2], count: i64, salt: usize, mouse: [f32; 2], hover: &mut Hover) {
    if items.is_empty() {
        return;
    }
    let index = ((c.time() / CYCLE_TICKS) as usize).wrapping_add(salt) % items.len();
    let Some(id) = items[index].as_str() else { return; };
    c.item(id, at, 16.0);
    if count > 1 {
        let t = plain(&count.to_string(), 0xFFFFFF);
        let w = c.width(&t);
        c.text(&t, [at[0] + 17.0 - w, at[1] + 9.0], true);
    }
    if inside(mouse, [at[0], at[1], 16.0, 16.0]) {
        hover.item = Some(id.to_owned());
    }
}

fn result_of(recipe: &Value) -> (Vec<Value>, i64) {
    let result = recipe.get("result");
    let id = result.and_then(|r| r.get("item")).and_then(Value::as_str);
    let count = result.and_then(|r| r.get("count")).and_then(Value::as_i64).unwrap_or(1);
    (id.map(|id| vec![Value::from(id)]).unwrap_or_default(), count)
}

fn draw_page(c: &mut dyn Canvas, ui: &BookUi, side: usize, page: &Value, o: [f32; 2], mouse: [f32; 2], hover: &mut Hover) {
    let data = &ui.data;
    let left = [o[0] + PAGE_X[side], o[1] + PAGE_TOP];
    let at = |x: f32, y: f32| [left[0] + x, left[1] + y];
    let kind = page.get("type").and_then(Value::as_str).unwrap_or("");
    let page_title = field(data, page, "title");
    let text_y = match kind {
        "modonomicon:spotlight" => {
            title(c, page_title, at(62.0, 0.0), false);
            theme(c, "content/pages/recipes/spotlight_slot", at(62.0 - 33.0, 10.0), WHITE);
            let id = page.get("item").and_then(|i| i.as_str().or_else(|| i.get("id").and_then(Value::as_str)));
            if let Some(id) = id {
                slot_item(c, &[Value::from(id)], at(54.0, 15.0), 1, 0, mouse, hover);
            }
            40.0
        }
        "modonomicon:image" => {
            title(c, page_title, at(62.0, 0.0), false);
            let list = images(page);
            if let Some(name) = list.get(ui.image[side].min(list.len().saturating_sub(1))) {
                let [sw, sh] = c.size(name);
                c.sprite(name, [0.0, 0.0, sw, sh], [left[0] + 12.0, left[1] + 12.0, 100.0, 100.0], WHITE);
            }
            if page.get("border").and_then(Value::as_bool).unwrap_or(true) {
                nine_slice(c, &format!("{THEME}content/pages/media/frame"), 3.0, [left[0] + 9.0, left[1] + 9.0, 106.0, 106.0]);
                if list.len() > 1 {
                    let [prev, next] = image_buttons(left);
                    c.fill([prev[0] - 3.0, prev[1] - 4.0, 21.0, 12.0], 0x44000000);
                    if ui.image[side] > 0 {
                        let s = if inside(mouse, prev) { "hover" } else { "normal" };
                        theme(c, &format!("content/buttons/navigation/small_previous_page_button_{s}"), [prev[0], prev[1]], WHITE);
                    }
                    if ui.image[side] + 1 < list.len() {
                        let s = if inside(mouse, next) { "hover" } else { "normal" };
                        theme(c, &format!("content/buttons/navigation/small_next_page_button_{s}"), [next[0], next[1]], WHITE);
                    }
                }
            }
            9.0 + 106.0 + 2.0
        }
        "modonomicon:multiblock" => {
            theme(c, "content/pages/media/frame", at(9.0, 7.0), WHITE);
            title(c, field(data, page, "multiblock_name"), at(62.0, 0.0), false);
            if let Some(mb) = page.get("multiblock") {
                multiblock(c, mb, [left[0] + 9.0, left[1] + 7.0, 106.0, 106.0], ui.ticks, mouse, hover);
            }
            115.0
        }
        "modonomicon:crafting_recipe" | "theurgy:calcination_recipe" => {
            let recipes = page.get("recipes").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
            let crafting = kind == "modonomicon:crafting_recipe";
            let height = if crafting { 75.0 } else { 60.0 };
            for (n, recipe) in recipes.iter().take(2).enumerate() {
                // BookRecipePageRenderer: X 13, Y 4; the second sits a recipe lower, 10 up (no title2).
                let (rx, ry) = (13.0, 4.0 + n as f32 * (height - 10.0));
                if recipe.get("missing").is_some() {
                    let t = plain(&format!("Recipe not found: {}", recipe.get("id").and_then(Value::as_str).unwrap_or("")), 0xAA0000);
                    c.text(&t, at(0.0, ry), false);
                    continue;
                }
                let (result, count) = result_of(recipe);
                if crafting {
                    theme(c, "content/pages/recipes/crafting_recipe_background", at(rx - 2.0, ry - 2.0), WHITE);
                    if recipe.get("shapeless").and_then(Value::as_bool) == Some(true) {
                        let icon = at(rx + 62.0, ry + 2.0);
                        theme(c, "content/pages/recipes/shapeless_icon", icon, WHITE);
                        if inside(mouse, [icon[0], icon[1], 11.0, 11.0]) {
                            hover.text = Some("Shapeless Recipe".to_owned());
                        }
                    }
                    slot_item(c, &result, at(rx + 79.0, ry + 22.0), count, 0, mouse, hover);
                    let grid = recipe.get("grid").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
                    for (cell, items) in grid.iter().enumerate().take(9) {
                        let items = items.as_array().map_or(&[][..], Vec::as_slice);
                        let (x, y) = ((cell % 3) as f32, (cell / 3) as f32);
                        slot_item(c, items, at(rx + x * 19.0 + 3.0, ry + y * 19.0 + 3.0), 1, cell, mouse, hover);
                    }
                    slot_item(c, &[Value::from("minecraft:crafting_table")], at(rx + 79.0, ry + 41.0), 1, 0, mouse, hover);
                } else {
                    // BookCalcinationRecipePageRenderer.
                    let ry = ry + 10.0;
                    let input = recipe.get("inputs").and_then(Value::as_array).and_then(|i| i.first());
                    let items = input.and_then(|i| i.get("items")).and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
                    let input_count = input.and_then(|i| i.get("count")).and_then(Value::as_i64).unwrap_or(1);
                    theme(c, "content/pages/recipes/crafting_slot", at(rx, ry), WHITE);
                    slot_item(c, items, at(rx + 3.0, ry + 3.0), input_count, 0, mouse, hover);
                    theme(c, "content/pages/recipes/crafting_slot", at(rx + 61.0, ry), WHITE);
                    slot_item(c, &result, at(rx + 64.0, ry + 3.0), count, 0, mouse, hover);
                    theme(c, "content/pages/recipes/crafting_arrow", at(rx + 40.0, ry + 7.0), WHITE);
                    slot_item(c, &[Value::from("theurgy:calcination_oven")], at(rx + 36.0, ry + 20.0), 1, 0, mouse, hover);
                }
            }
            let shown = recipes.len().clamp(1, 2) as f32;
            4.0 + height * shown - 10.0 - if shown > 1.0 { 10.0 } else { 0.0 }
        }
        _ => {
            // BookTextPageRenderer.
            let separator = page.get("show_title_separator").and_then(Value::as_bool).unwrap_or(true);
            if page_title.is_empty() {
                -4.0
            } else {
                title(c, page_title, at(62.0, 0.0), separator);
                if separator { 17.0 } else { 7.0 }
            }
        }
    };
    body(c, field(data, page, "text"), at(0.0, text_y), PAGE_W, PAGE_H - text_y, &mut hover.links);
}

/// BookMultiblockPageRenderer replacement: the structure as isometric block
/// icons, fitted to 90 px and turning half a degree a tick
/// (GuiMultiblockRenderer: 30 degrees down, 45 + ticksInBook / 2 around).
fn multiblock(c: &mut dyn Canvas, mb: &Value, area: [f32; 4], ticks: f32, mouse: [f32; 2], hover: &mut Hover) {
    let Some(layers) = mb.get("layers").and_then(Value::as_array) else { return; };
    let mapping = mb.get("mapping").and_then(Value::as_object);
    let mut blocks = Vec::new();
    let height = layers.len();
    let (mut size_x, mut size_z) = (0.0f32, 0.0f32);
    for (layer, rows) in layers.iter().enumerate() {
        for (z, row) in rows.as_array().into_iter().flatten().filter_map(Value::as_str).enumerate() {
            size_z = size_z.max(z as f32 + 1.0);
            for (x, symbol) in row.chars().enumerate() {
                size_x = size_x.max(x as f32 + 1.0);
                let id = mapping.and_then(|m| m.get(&symbol.to_string())).and_then(Value::as_str);
                if let Some(id) = id.filter(|id| *id != "minecraft:air") {
                    blocks.push(((height - 1 - layer) as f32, x as f32, z as f32, id));
                }
            }
        }
    }
    if blocks.is_empty() {
        return;
    }
    let size_y = height as f32;
    // A GUI block icon is 1.6 cube edges wide; `l` is one block's edge.
    let diag = (size_x * size_x + size_z * size_z).sqrt();
    let l = (90.0 / (diag + 1.6)).min(90.0 / (size_y * 0.866 + diag * 0.5 + 1.6)).min(20.0);
    let size = 1.6 * l;
    let (sin, cos) = (45.0 + ticks * 0.5f32).to_radians().sin_cos();
    let mut placed: Vec<(f32, f32, [f32; 2], &str)> = blocks.iter().map(|&(y, x, z, id)| {
        let (dx, dz) = (x + 0.5 - size_x / 2.0, z + 0.5 - size_z / 2.0);
        let across = dx * cos - dz * sin;
        let depth = dx * sin + dz * cos;
        let screen = [across * l, depth * 0.5 * l - (y + 0.5 - size_y / 2.0) * 0.866 * l];
        (y, depth, screen, id)
    }).collect();
    // Back to front, bottom to top.
    placed.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).unwrap_or(std::cmp::Ordering::Equal));
    let centre = [area[0] + area[2] / 2.0, area[1] + area[3] / 2.0];
    for (_, _, p, id) in placed {
        let at = [centre[0] + p[0] - size / 2.0, centre[1] + p[1] - size / 2.0];
        c.item(id, at, size);
        if inside(mouse, [at[0] + size * 0.2, at[1] + size * 0.2, size * 0.6, size * 0.6]) {
            hover.item = Some(id.to_owned());
        }
    }
}

/// A `category://` or `entry://` link of this book (BookLink.from).
enum Link {
    Entry(usize),
    Category(usize),
}

fn book_link(data: &BookData, target: &str) -> Option<Link> {
    let (kind, rest) = target.split_once("://")?;
    let (ns, path) = rest.split_once(':')?;
    let mut parts = path.split('/').skip(1);
    let category = parts.next()?;
    match kind {
        "category" => data.categories.iter().position(|c| id_of(c) == category).map(Link::Category),
        "entry" => entry_index(data, &format!("{ns}:{category}/{}", parts.collect::<Vec<_>>().join("/"))).map(Link::Entry),
        _ => None,
    }
}

/// BookLinkRenderer hover "Go to", or BookTextInteraction.renderHover's
/// locked-entry text.
fn link_tooltip(ui: &BookUi, link: &Link) -> Vec<Vec<Run>> {
    let d = &ui.data;
    let (name, entry) = match link {
        Link::Category(i) => (field(d, &d.categories[*i], "name"), None),
        Link::Entry(i) => (field(d, &d.entries[*i], "name"), Some(&d.entries[*i])),
    };
    let go = trf(d, "modonomicon.gui.hover.book_link", &[name]);
    let Some(entry) = entry.filter(|e| !unlocked(ui, e)) else { return vec![plain(&go, 0xFFFFFF)]; };
    let category = entry.get("category").and_then(Value::as_str).unwrap_or("");
    let category = d.categories.iter().find(|c| category == format!("theurgy:{}", id_of(c))).map_or("", |c| field(d, c, "name"));
    let mut info = plain(tr(d, "modonomicon.gui.hover.book_entry_link_locked_info"), 0xFF0015);
    info[0].bold = true;
    let hint = tr(d, "modonomicon.gui.hover.book_entry_link_locked_info.hint");
    let (before, after) = hint.split_once("%s").unwrap_or((hint, ""));
    let mut hint = plain(before, 0xFFFFFF);
    hint.push(Run { text: category.to_owned(), rgb: 0xAAAAAA, bold: false, italic: true, book_font: false });
    hint.extend(plain(after, 0xFFFFFF));
    vec![plain(&format!("{go}."), 0xFFFFFF), info, hint]
}

fn bookmarked(ui: &BookUi) -> bool {
    ui.entry.is_some_and(|e| ui.progress.bookmarks.iter().any(|(id, _)| id == id_of(&ui.data.entries[e])))
}

/// The double page; returns its link areas.
fn draw_entry(c: &mut dyn Canvas, ui: &BookUi, w: f32, h: f32, mouse: [f32; 2]) -> Vec<([f32; 4], String)> {
    let o = book_origin(w, h);
    theme(c, "content/backgrounds/book/double_page_background", o, WHITE);
    let all = pages(ui);
    let mut hover = Hover { item: None, text: None, links: Vec::new() };
    for side in 0..2 {
        if let Some(page) = all.get(ui.spread + side) {
            draw_page(c, ui, side, page, o, mouse, &mut hover);
        }
    }
    let [prev, next] = arrows(o);
    let state = |r: [f32; 4]| if inside(mouse, r) { "hover" } else { "normal" };
    if ui.spread > 0 {
        theme(c, &format!("content/buttons/navigation/previous_page_button_{}", state(prev)), [prev[0], prev[1]], WHITE);
    }
    if ui.spread + 2 < all.len() {
        theme(c, &format!("content/buttons/navigation/next_page_button_{}", state(next)), [next[0], next[1]], WHITE);
    }
    let exit = exit_button(o);
    theme(c, &format!("content/buttons/navigation/exit_button_{}", state(exit)), [exit[0], exit[1]], WHITE);
    let back = back_button(w, o);
    let can_back = !ui.history.is_empty();
    if can_back {
        theme(c, &format!("content/buttons/navigation/back_button_{}", state(back)), [back[0], back[1]], WHITE);
    }
    let mark = bookmark_button(o);
    let marked = bookmarked(ui);
    let icon = if marked { "remove_bookmarks_icon" } else { "add_bookmarks_icon" };
    side_button(c, mark, inside(mouse, mark), "content/buttons/collection/collection_button_normal", &format!("content/buttons/collection/{icon}"), [11.0, 11.0], -4.0);
    let d = &ui.data;
    let link = hover.links.iter().find(|(r, _)| inside(mouse, *r)).map(|(_, t)| t.as_str());
    if let Some(id) = hover.item.as_deref().or_else(|| link.and_then(|t| t.strip_prefix("item://"))) {
        c.item_tooltip(id, mouse);
    } else if let Some(link) = link.and_then(|t| book_link(d, t)) {
        c.tooltip(&link_tooltip(ui, &link), mouse);
    } else if let Some(s) = hover.text {
        c.tooltip(&[plain(&s, 0xFFFFFF)], mouse);
    } else if inside(mouse, exit) {
        c.tooltip(&[plain(tr(d, "modonomicon.gui.button.exit"), 0xFFFFFF)], mouse);
    } else if can_back && inside(mouse, back) {
        c.tooltip(&[plain(tr(d, "modonomicon.gui.button.back.tooltip"), 0xFFFFFF)], mouse);
    } else if inside(mouse, mark) {
        let key = if marked { "modonomicon.gui.remove_bookmark" } else { "modonomicon.gui.add_bookmark" };
        c.tooltip(&[plain(tr(d, key), 0xFFFFFF)], mouse);
    }
    hover.links
}

// ---- search, bookmarks, recently unlocked (BookPaginatedListScreen) -------

/// Screen title, list title, info text, empty text.
fn list_keys(kind: ListKind) -> [&'static str; 4] {
    match kind {
        ListKind::Search => ["modonomicon.gui.search.screen.title", "modonomicon.gui.search.entry_list_title",
            "modonomicon.gui.search.info", "modonomicon.gui.search.no_results"],
        ListKind::Bookmarks => ["modonomicon.gui.bookmarks.screen.title", "modonomicon.gui.bookmarks.entry_list_title",
            "modonomicon.gui.bookmarks.info", "modonomicon.gui.bookmarks.no_results"],
        ListKind::Recent => ["modonomicon.gui.recently_unlocked.screen.title", "modonomicon.gui.recently_unlocked.entry_list_title",
            "modonomicon.gui.recently_unlocked.info", "modonomicon.gui.recently_unlocked.no_results"],
    }
}

/// BookEntry.matchesQuery: the name, page titles and texts.
fn matches(d: &BookData, entry: &Value, query: &str) -> bool {
    field(d, entry, "name").to_lowercase().contains(query)
        || entry.get("pages").and_then(Value::as_array).into_iter().flatten().any(|p| {
            ["title", "text", "multiblock_name"].iter().any(|k| field(d, p, k).to_lowercase().contains(query))
        })
}

/// computeAllEntries + filterEntries: (entry, page to open).
fn list_entries(ui: &BookUi, list: &List) -> Vec<(usize, usize)> {
    let d = &ui.data;
    match list.kind {
        ListKind::Search => {
            let query = list.query.to_lowercase();
            let mut found: Vec<usize> = (0..d.entries.len())
                .filter(|&i| unlocked(ui, &d.entries[i]) && matches(d, &d.entries[i], &query)).collect();
            found.sort_by_cached_key(|&i| field(d, &d.entries[i], "name").to_owned());
            found.into_iter().map(|i| (i, 0)).collect()
        }
        ListKind::Bookmarks => ui.progress.bookmarks.iter().filter_map(|(id, page)| Some((entry_index(d, id)?, *page))).collect(),
        ListKind::Recent => {
            // Unread first, then the most recent.
            let mut recent: Vec<(bool, std::cmp::Reverse<usize>, usize)> = ui.progress.unlocked.iter().enumerate()
                .filter_map(|(n, id)| Some((ui.progress.read.contains(id), std::cmp::Reverse(n), entry_index(d, id)?))).collect();
            recent.sort();
            recent.into_iter().map(|(_, _, i)| (i, 0)).collect()
        }
    }
}

/// maxOpenPagesIndex.
fn list_pages(count: usize) -> usize {
    1 + count.saturating_sub(LIST_FIRST).div_ceil(2 * LIST_PAGE)
}

/// createEntryList: EntryListButtons (124 x 10, 13 apart) and their list index.
fn list_buttons(o: [f32; 2], spread: usize, count: usize) -> Vec<([f32; 4], usize)> {
    let mut out = Vec::new();
    let mut add = |x: f32, y: f32, start: usize, n: usize| {
        for k in (0..n).take_while(|k| start + k < count) {
            out.push(([o[0] + x, o[1] + y + 13.0 * k as f32, 124.0, 10.0], start + k));
        }
    };
    if spread == 0 {
        add(138.0, 35.0, 0, LIST_FIRST);
    } else {
        let start = LIST_FIRST + 2 * LIST_PAGE * (spread - 1);
        add(12.0, 15.0, start, LIST_PAGE);
        add(138.0, 15.0, start + LIST_PAGE, LIST_PAGE);
    }
    out
}

fn draw_list(c: &mut dyn Canvas, ui: &BookUi, list: &List, w: f32, h: f32, mouse: [f32; 2]) {
    let d = &ui.data;
    let o = book_origin(w, h);
    let at = |x: f32, y: f32| [o[0] + x, o[1] + y];
    theme(c, "content/backgrounds/book/double_page_background", o, WHITE);
    let [screen_title, list_title, info, none] = list_keys(list.kind);
    if list.spread == 0 {
        centered(c, tr(d, screen_title), at(74.0, 15.0), TEXT_RGB);
        centered(c, tr(d, list_title), at(203.0, 15.0), TEXT_RGB);
        separator_at(c, at(74.0, 27.0));
        separator_at(c, at(203.0, 27.0));
        body(c, tr(d, info), at(12.0, 37.0), 124.0, 118.0, &mut Vec::new());
    }
    let searching = list.kind == ListKind::Search && !list.query.is_empty();
    if searching {
        theme(c, "content/fields/search/background", at(152.0, 170.0), WHITE);
        c.text(&plain(&list.query, TEXT_RGB), at(167.0, 171.0), false);
    }
    let all = list_entries(ui, list);
    if all.is_empty() {
        centered(c, tr(d, none), at(203.0, 80.0), 0x333333);
        if searching {
            let start = c.mark();
            centered(c, tr(d, "modonomicon.gui.search.sad"), at(101.0, 47.0), 0x999999);
            c.scale_since(start, o, 2.0);
        }
    }
    for (r, k) in list_buttons(o, list.spread, all.len()) {
        let entry = &d.entries[all[k].0];
        if inside(mouse, r) {
            c.fill([r[0], r[1] - 1.0, r[2], r[3] + 2.0], 0x22000000);
        }
        let locked = !unlocked(ui, entry);
        if locked {
            whole(c, &format!("{THEME}content/icons/book/lock_icon"), [r[0] + 1.0, r[1] + 1.0, 8.0, 8.0], 0xB3FFFFFF);
        } else if let Some(value) = entry.get("icon") {
            icon(c, value, [r[0] + 1.0, r[1] + 1.0], 8.0);
        }
        let name = if locked { tr(d, "modonomicon.gui.search.entry.locked") } else { field(d, entry, "name") };
        let mut t = plain(name, TEXT_RGB);
        t[0].book_font = true;
        let scale = (112.0 / c.width(&t).max(1.0)).min(1.0);
        let text_at = [r[0] + 12.0, r[1] + 1.0 + (9.0 - 9.0 * scale) / 2.0];
        let start = c.mark();
        c.text(&t, text_at, false);
        if scale < 1.0 {
            c.scale_since(start, text_at, scale);
        }
    }
    let [prev, next] = arrows(o);
    let state = |r: [f32; 4]| if inside(mouse, r) { "hover" } else { "normal" };
    if list.spread > 0 {
        theme(c, &format!("content/buttons/navigation/previous_page_button_{}", state(prev)), [prev[0], prev[1]], WHITE);
    }
    if list.spread + 1 < list_pages(all.len()) {
        theme(c, &format!("content/buttons/navigation/next_page_button_{}", state(next)), [next[0], next[1]], WHITE);
    }
    let exit = exit_button(o);
    theme(c, &format!("content/buttons/navigation/exit_button_{}", state(exit)), [exit[0], exit[1]], WHITE);
    if inside(mouse, exit) {
        c.tooltip(&[plain(tr(d, "modonomicon.gui.button.exit"), 0xFFFFFF)], mouse);
    }
}

// ---- public API (glue: client.rs, tools/book-preview) -----------------------

/// Parses `data/theurgy/hermetica.json` and shows the book. Reopening keeps
/// the last category, entry, scroll and zoom (CategoryVisualState).
pub fn open(bytes: &[u8]) -> bool {
    let Ok(json) = serde_json::from_slice::<Value>(bytes) else { return false; };
    let list = |key: &str| json.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
    let translations = json.get("translations").and_then(Value::as_object).map(|t| {
        t.iter().filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned()))).collect()
    }).unwrap_or_default();
    let research = json.get("research").and_then(Value::as_object).map(|r| {
        r.iter().map(|(node, entries)| {
            (node.clone(), entries.as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect())
        }).collect()
    }).unwrap_or_default();
    let data = BookData { categories: list("categories"), entries: list("entries"), translations, research };
    if data.categories.is_empty() || data.entries.is_empty() {
        return false;
    }
    let Ok(mut slot) = BOOK.lock() else { return false; };
    let pending = PENDING.lock().ok().and_then(|mut p| p.take());
    let ui = match slot.take() {
        Some(mut old) if old.views.len() == data.categories.len() && old.entry.is_none_or(|e| e < data.entries.len()) => {
            old.data = data;
            old.progress = pending.unwrap_or(old.progress);
            old.list = None;
            old.shown = true;
            old
        }
        _ => BookUi {
            views: vec![View { scroll: [0.0, 0.0], zoom: 0.7 }; data.categories.len()],
            data,
            progress: pending.unwrap_or_default(),
            category: 0,
            entry: None,
            spread: 0,
            image: [0, 0],
            list: None,
            history: Vec::new(),
            links: Vec::new(),
            ticks: 0.0,
            last_time: 0,
            turn_sound: false,
            shown: true,
        },
    };
    let ui = slot.insert(ui);
    refresh(ui);
    true
}

/// Another theurgy screen takes the mod screen (or it failed to open).
pub fn close() {
    if let Ok(mut slot) = BOOK.lock()
        && let Some(ui) = slot.as_mut()
    {
        ui.shown = false;
        ui.turn_sound = false;
    }
}

fn with<R>(f: impl FnOnce(&mut BookUi) -> R) -> Option<R> {
    let mut guard = BOOK.lock().ok()?;
    guard.as_mut().filter(|ui| ui.shown).map(f)
}

/// This player's progress in this world, from the server half (client.rs).
pub fn set_progress(bytes: &[u8]) {
    let Some(progress) = Progress::parse(bytes) else { return; };
    let Ok(mut slot) = BOOK.lock() else { return; };
    if let Some(ui) = slot.as_mut() {
        ui.progress = progress;
        ui.history.clear();
        refresh(ui);
    } else if let Ok(mut pending) = PENDING.lock() {
        *pending = Some(progress);
    }
}

/// The progress to keep, when it changed since the last call.
pub fn take_progress() -> Option<Vec<u8>> {
    let mut slot = BOOK.lock().ok()?;
    let p = &mut slot.as_mut()?.progress;
    std::mem::take(&mut p.dirty).then(|| p.to_json())
}

/// Draws the open screen into a `w` x `h` GUI. False when the book is not shown.
pub fn render(c: &mut dyn Canvas, w: f32, h: f32, mouse: [f32; 2]) -> bool {
    with(|ui| {
        let now = c.time();
        if !c.shift() {
            ui.ticks += (now - ui.last_time).clamp(0, 20) as f32;
        }
        ui.last_time = now;
        ui.links.clear();
        if let Some(list) = &ui.list {
            draw_list(c, ui, list, w, h, mouse);
        } else if ui.entry.is_some() {
            ui.links = draw_entry(c, ui, w, h, mouse);
        } else {
            draw_category(c, ui, w, h, mouse);
        }
    }).is_some()
}

fn flip(ui: &mut BookUi, back: bool) {
    if let Some(list) = &ui.list {
        let count = list_pages(list_entries(ui, list).len());
        let Some(list) = ui.list.as_mut() else { return; };
        if back && list.spread > 0 {
            list.spread -= 1;
        } else if !back && list.spread + 1 < count {
            list.spread += 1;
        } else {
            return;
        }
        ui.turn_sound = true;
        return;
    }
    let count = pages(ui).len();
    if back && ui.spread > 0 {
        ui.spread -= 2;
    } else if !back && ui.spread + 2 < count {
        ui.spread += 2;
    } else {
        return;
    }
    ui.image = [0, 0];
    ui.turn_sound = true;
}

/// BookPaginatedScreen.back: the last linked-from page, else close.
fn back(ui: &mut BookUi) {
    if let Some((entry, spread)) = ui.history.pop() {
        open_entry(ui, entry, spread);
    } else if ui.list.is_some() {
        ui.list = None;
    } else {
        ui.entry = None;
    }
}

/// BookLinkHandler: true when the click was a book link (taken even when
/// the entry is locked, as Modonomicon's FAILURE is).
fn follow(ui: &mut BookUi, target: &str) -> bool {
    match book_link(&ui.data, target) {
        Some(Link::Entry(index)) => {
            if unlocked(ui, &ui.data.entries[index]) {
                if let Some(current) = ui.entry {
                    ui.history.push((current, ui.spread));
                }
                open_entry(ui, index, 0);
            }
            true
        }
        Some(Link::Category(index)) => {
            ui.entry = None;
            ui.category = index;
            true
        }
        // ItemLinkHandler without a recipe viewer: FAILURE, still taken.
        None => target.starts_with("item://"),
    }
}

fn click_entry(ui: &mut BookUi, w: f32, h: f32, mouse: [f32; 2], button: i32) {
    let o = book_origin(w, h);
    if button == 0 || button == 1 {
        let link = ui.links.iter().find(|(r, _)| inside(mouse, *r)).map(|(_, t)| t.clone());
        if link.is_some_and(|t| follow(ui, &t)) {
            return;
        }
    }
    if button == 0 && inside(mouse, bookmark_button(o)) {
        // Add/RemoveBookmarkButton: the entry at its left page.
        let id = ui.entry.map(|e| id_of(&ui.data.entries[e]).to_owned()).unwrap_or_default();
        let marks = &mut ui.progress.bookmarks;
        match marks.iter().position(|(e, _)| *e == id) {
            Some(k) => drop(marks.remove(k)),
            None => marks.push((id, ui.spread)),
        }
        ui.progress.dirty = true;
        return;
    }
    if button == 0 && !ui.history.is_empty() && inside(mouse, back_button(w, o)) {
        back(ui);
        return;
    }
    let [prev, next] = arrows(o);
    let outside = mouse[0] < o[0] - 20.0 || mouse[0] > o[0] + FULL_W + 20.0 || mouse[1] < o[1] - 20.0 || mouse[1] > o[1] + FULL_H + 20.0;
    if outside || inside(mouse, exit_button(o)) {
        ui.entry = None;
    } else if button == 3 {
        back(ui);
    } else if button == 0 && inside(mouse, prev) {
        flip(ui, true);
    } else if button == 0 && inside(mouse, next) {
        flip(ui, false);
    } else if button == 0 {
        for side in 0..2 {
            let count = pages(ui).get(ui.spread + side).map_or(0, |p| images(p).len());
            let [bp, bn] = image_buttons([o[0] + PAGE_X[side], o[1] + PAGE_TOP]);
            if count > 1 && inside(mouse, bp) && ui.image[side] > 0 {
                ui.image[side] -= 1;
            } else if count > 1 && inside(mouse, bn) && ui.image[side] + 1 < count {
                ui.image[side] += 1;
            }
        }
    }
}

fn click_list(ui: &mut BookUi, w: f32, h: f32, mouse: [f32; 2], button: i32) {
    let o = book_origin(w, h);
    let outside = mouse[0] < o[0] - 20.0 || mouse[0] > o[0] + FULL_W + 20.0 || mouse[1] < o[1] - 20.0 || mouse[1] > o[1] + FULL_H + 20.0;
    if outside || (button == 0 && inside(mouse, exit_button(o))) {
        ui.list = None;
    } else if button == 3 {
        back(ui);
    } else if button == 0 {
        let [prev, next] = arrows(o);
        if inside(mouse, prev) {
            flip(ui, true);
        } else if inside(mouse, next) {
            flip(ui, false);
        } else if let Some(list) = &ui.list {
            let all = list_entries(ui, list);
            let hit = list_buttons(o, list.spread, all.len()).into_iter().find(|(r, _)| inside(mouse, *r));
            if let Some((index, page)) = hit.map(|(_, k)| all[k]).filter(|&(i, _)| unlocked(ui, &ui.data.entries[i])) {
                open_entry(ui, index, page);
            }
        }
    }
}

fn click_category(ui: &mut BookUi, w: f32, h: f32, mouse: [f32; 2], shift: bool) {
    for index in 0..ui.data.categories.len().min(13) {
        if inside(mouse, category_button(w, h, index)) {
            ui.category = index;
            return;
        }
    }
    let [search, bookmarks, recent, research] = side_buttons(w, h);
    for (r, kind) in [(search, ListKind::Search), (bookmarks, ListKind::Bookmarks), (recent, ListKind::Recent)] {
        if inside(mouse, r) {
            ui.list = Some(List { kind, query: String::new(), spread: 0 });
            return;
        }
    }
    if inside(mouse, research) {
        // ResearchProgressButton: visible entries, or every one with Shift.
        let (visible, any) = research_progress(ui);
        if (visible && !shift) || (any && shift) {
            let targets: Vec<String> = hook_targets(&ui.data).into_iter()
                .filter(|t| shift || entry_index(&ui.data, t).is_some_and(|i| unlocked(ui, &ui.data.entries[i])))
                .map(str::to_owned).collect();
            ui.progress.viewed.extend(targets);
            ui.progress.dirty = true;
            refresh(ui);
        }
        return;
    }
    if let Some(index) = hovered_entry(ui, &frame(w, h), mouse)
        && show(ui, &ui.data.entries[index]) == Show::Unlocked
    {
        open_entry(ui, index, 0);
    }
}

/// A mouse button went down; true when the book took it.
pub fn click(w: f32, h: f32, mouse: [f32; 2], button: i32, shift: bool) -> bool {
    with(|ui| {
        if ui.list.is_some() {
            click_list(ui, w, h, mouse, button);
        } else if ui.entry.is_some() {
            click_entry(ui, w, h, mouse, button);
        } else if button == 0 {
            click_category(ui, w, h, mouse, shift);
        }
    }).is_some()
}

/// Wheel: flips pages in an entry or list, zooms the category map
/// (BookCategoryNodeScreen.zoom).
pub fn scroll(delta: f64) -> bool {
    with(|ui| {
        if ui.entry.is_some() || ui.list.is_some() {
            if delta != 0.0 {
                flip(ui, delta > 0.0);
            }
            return;
        }
        let view = &mut ui.views[ui.category];
        if (delta < 0.0 && view.zoom > 0.5) || (delta > 0.0 && view.zoom < 1.0) {
            view.zoom = (view.zoom * if delta > 0.0 { 1.2 } else { 1.0 / 1.2 }).min(1.0);
        }
    }).is_some()
}

/// Dragging the category map: BookCategoryNodeScreen.scroll, 1.5x the drag.
pub fn drag(button: i32, delta: [f32; 2]) -> bool {
    with(|ui| {
        if ui.entry.is_some() || ui.list.is_some() || !(button == 0 || button == 1) {
            return;
        }
        let category = &ui.data.categories[ui.category];
        let max = [num(category, "max_scroll_x"), num(category, "max_scroll_y")];
        let view = &mut ui.views[ui.category];
        for k in 0..2 {
            view.scroll[k] = (view.scroll[k] - delta[k] * 1.5).clamp(-max[k], max[k]);
        }
    }).is_some()
}

/// Keys: arrows flip; Escape closes a list (an entry closes the whole book,
/// BookEntryScreen.keyPressed, and reopens there); the search field takes
/// typing keys, Backspace and Enter (opens the only result).
pub fn key(key: &str) -> bool {
    with(|ui| {
        match key {
            "key.keyboard.left" if ui.entry.is_some() || ui.list.is_some() => flip(ui, true),
            "key.keyboard.right" if ui.entry.is_some() || ui.list.is_some() => flip(ui, false),
            _ => {
                let Some(list) = &mut ui.list else { return false; };
                let searching = list.kind == ListKind::Search;
                match key {
                    "key.keyboard.escape" => ui.list = None,
                    "key.keyboard.backspace" if searching => {
                        list.query.pop();
                    }
                    "key.keyboard.enter" | "key.keyboard.keypad.enter" => {
                        let all = list_entries(ui, ui.list.as_ref().expect("list"));
                        if let [(index, page)] = all[..] {
                            open_entry(ui, index, page);
                        }
                    }
                    _ => return searching,
                }
                clamp_list(ui);
            }
        }
        true
    }).unwrap_or(false)
}

/// createEntryList keeps the page while it still has entries.
fn clamp_list(ui: &mut BookUi) {
    if let Some(list) = &ui.list {
        let count = list_pages(list_entries(ui, list).len());
        if let Some(list) = ui.list.as_mut() {
            list.spread = list.spread.min(count - 1);
        }
    }
}

/// A typed character: the search field (EditBox, 32 characters).
pub fn char_typed(ch: char) -> bool {
    with(|ui| {
        let Some(list) = ui.list.as_mut().filter(|l| l.kind == ListKind::Search) else { return false; };
        if !ch.is_control() && ch != '§' && list.query.chars().count() < 32 {
            list.query.push(ch);
        }
        clamp_list(ui);
        true
    }).unwrap_or(false)
}

/// Jumps to a category (and an entry id's `spread`-th double page), or a
/// list (`search`, `bookmarks`, `recent`) with a query; tools/book-preview
/// uses it to render every page.
pub fn goto(category: usize, entry: Option<&str>, spread: usize, list: Option<(&str, &str)>) -> bool {
    with(|ui| {
        ui.category = category.min(ui.data.categories.len() - 1);
        ui.entry = entry.and_then(|id| entry_index(&ui.data, id));
        ui.spread = spread & !1;
        ui.image = [0, 0];
        ui.list = list.and_then(|(kind, query)| {
            let kind = match kind { "search" => ListKind::Search, "bookmarks" => ListKind::Bookmarks, "recent" => ListKind::Recent, _ => return None };
            Some(List { kind, query: query.to_owned(), spread })
        });
        ui.entry.is_some() || entry.is_none()
    }).unwrap_or(false)
}

/// A click turned a page since the last call (played by the frame hook).
pub fn take_turn_sound() -> bool {
    with(|ui| std::mem::take(&mut ui.turn_sound)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(blocks: &[Block]) -> Vec<String> {
        blocks.iter().map(|b| b.spans.iter().map(|(s, _)| s.as_str()).collect()).collect()
    }

    // A 512 texture behind a 900 px frame repeats, never stretches.
    #[test]
    fn background_tiles_wrap_at_the_texture_edge() {
        assert_eq!(tiles(500.0, 512.0, 900.0), [[500.0, 0.0, 12.0], [0.0, 12.0, 512.0], [0.0, 524.0, 376.0]]);
        assert_eq!(tiles(10.0, 512.0, 100.0), [[10.0, 0.0, 100.0]]);
        assert!(tiles(0.0, 0.0, 100.0).is_empty());
    }

    #[test]
    fn markdown_paragraphs_soft_breaks_and_lists() {
        let blocks = markdown("One\ntwo\n\nThree\n\n- a\n- b\n");
        assert_eq!(flat(&blocks), ["One two", "", "Three", "- a", "- b"]);
    }

    #[test]
    fn markdown_colour_links_and_emphasis() {
        let blocks = markdown("A [#](ad03fc)Salt[#]() and [Oven](item://theurgy:calcination_oven) **b** *i*");
        let spans = &blocks[0].spans;
        assert_eq!(flat(&blocks), ["A Salt and Oven b i"]);
        assert!(spans.iter().any(|(s, f)| s == "Salt" && f.color == Some(0xAD03FC)));
        assert!(spans.iter().any(|(s, f)| s == "Oven" && f.link.as_deref() == Some("item://theurgy:calcination_oven")));
        assert!(spans.iter().any(|(s, f)| s == "b" && f.bold));
        assert!(spans.iter().any(|(s, f)| s == "i" && f.italic));
    }

    #[test]
    fn book_links_and_list_pages() {
        let entry = |id: &str| json!({"id": id});
        let data = BookData {
            categories: vec![json!({"id": "apparatus"})],
            entries: vec![entry("theurgy:apparatus/intro"), entry("theurgy:apparatus/incubator")],
            translations: HashMap::new(),
            research: HashMap::new(),
        };
        assert!(matches!(book_link(&data, "entry://theurgy:the_hermetica/apparatus/incubator"), Some(Link::Entry(1))));
        assert!(matches!(book_link(&data, "category://theurgy:the_hermetica/apparatus"), Some(Link::Category(0))));
        assert!(book_link(&data, "item://theurgy:incubator").is_none());
        // BookPaginatedListScreen: 9 on the first double page, then 22 each.
        assert_eq!([list_pages(0), list_pages(9), list_pages(10), list_pages(31), list_pages(32)], [1, 1, 2, 2, 3]);
    }

    #[test]
    fn progress_round_trips() {
        let p = Progress::parse(br#"{"read":["a"],"viewed":["a","b"],"bookmarks":[["b",2]],"unlocked":["a","b"]}"#).unwrap();
        let q = Progress::parse(&p.to_json()).unwrap();
        assert_eq!((q.read.len(), q.viewed.len(), q.bookmarks, q.unlocked), (1, 2, vec![("b".to_owned(), 2)], vec!["a".to_owned(), "b".to_owned()]));
        assert!(Progress::parse(b"{}").is_some_and(|p| p.read.is_empty()));
    }
}
