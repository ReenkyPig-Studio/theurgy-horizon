// SPDX-License-Identifier: MIT
// Records native/theurgy/book.rs draw calls for every category screen and
// every double page of the Hermetica, without Horizon.
// Usage: book-preview <hermetica.json> <font.json> <sprites.json> <out dir> [entry filter]
#[path = "../../../native/theurgy/book.rs"]
#[allow(dead_code)]
mod book;

use book::{Canvas, Run};
use serde_json::{Value, json};
use std::collections::HashMap;

struct Recorder {
    cmds: Vec<Value>,
    /// Advances: vanilla font, then `modonomicon:default`.
    font: [HashMap<char, f32>; 2],
    shift: bool,
    sprites: HashMap<String, [f32; 2]>,
}

fn runs(r: &[Run]) -> Value {
    Value::Array(r.iter().map(|r| json!({"text": r.text, "rgb": r.rgb, "bold": r.bold, "italic": r.italic, "book": r.book_font})).collect())
}

impl Canvas for Recorder {
    fn size(&self, name: &str) -> [f32; 2] {
        self.sprites.get(name).copied().unwrap_or([0.0, 0.0])
    }
    fn sprite(&mut self, name: &str, src: [f32; 4], dst: [f32; 4], tint: u32) -> bool {
        let ok = self.sprites.contains_key(name);
        self.cmds.push(json!({"k": "sprite", "name": name, "src": src, "dst": dst, "tint": tint, "missing": !ok}));
        ok
    }
    fn item(&mut self, id: &str, at: [f32; 2], size: f32) -> bool {
        self.cmds.push(json!({"k": "item", "id": id, "dst": [at[0], at[1], size, size]}));
        true
    }
    fn fill(&mut self, rect: [f32; 4], argb: u32) {
        self.cmds.push(json!({"k": "fill", "dst": rect, "argb": argb}));
    }
    fn text(&mut self, r: &[Run], at: [f32; 2], shadow: bool) {
        self.cmds.push(json!({"k": "text", "runs": runs(r), "at": at, "shadow": shadow}));
    }
    fn width(&self, r: &[Run]) -> f32 {
        r.iter().map(|r| {
            let font = &self.font[usize::from(r.book_font)];
            r.text.chars().map(|c| font.get(&c).copied().unwrap_or(6.0) + if r.bold { 1.0 } else { 0.0 }).sum::<f32>()
        }).sum()
    }
    fn mark(&self) -> usize {
        self.cmds.len()
    }
    fn clip_since(&mut self, mark: usize, rect: [f32; 4]) {
        for c in &mut self.cmds[mark..] {
            c["clip"] = json!(rect);
        }
    }
    fn scale_since(&mut self, mark: usize, origin: [f32; 2], scale: f32) {
        for c in &mut self.cmds[mark..] {
            c["scale"] = json!([origin[0], origin[1], scale]);
        }
    }
    fn tooltip(&mut self, lines: &[Vec<Run>], at: [f32; 2]) {
        self.cmds.push(json!({"k": "tooltip", "lines": lines.iter().map(|l| runs(l)).collect::<Vec<_>>(), "at": at}));
    }
    fn item_tooltip(&mut self, id: &str, at: [f32; 2]) {
        self.cmds.push(json!({"k": "item_tooltip", "id": id, "at": at}));
    }
    fn time(&self) -> i64 {
        0
    }
    fn shift(&self) -> bool {
        self.shift
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let read = |i: usize| std::fs::read(&args[i]).unwrap_or_else(|e| panic!("{}: {e}", args[i]));
    let data = read(1);
    let fonts: [HashMap<String, f32>; 2] = serde_json::from_slice(&read(2)).expect("font.json");
    let sprites: HashMap<String, [f32; 2]> = serde_json::from_slice(&read(3)).expect("sprites.json");
    let out = std::path::PathBuf::from(&args[4]);
    let filter = args.get(5).cloned().unwrap_or_default();
    std::fs::create_dir_all(&out).expect("out dir");
    let font = fonts.map(|f| f.into_iter().filter_map(|(k, v)| Some((k.chars().next()?, v))).collect());
    let mut canvas = Recorder { cmds: Vec::new(), font, sprites, shift: false };
    assert!(book::open(&data), "hermetica.json did not load");
    let json: Value = serde_json::from_slice(&data).unwrap();
    let (w, h) = (427.0, 240.0);
    let mut scenes = Vec::new();
    let mut shoot = |canvas: &mut Recorder, name: String, mouse: [f32; 2]| {
        canvas.cmds.clear();
        book::render(canvas, w, h, mouse);
        std::fs::write(out.join(format!("{name}.json")), serde_json::to_vec(&json!({"w": w, "h": h, "cmds": canvas.cmds})).unwrap()).unwrap();
        scenes.push(name);
    };
    let categories = json["categories"].as_array().unwrap();
    // A new player: locked entries and unread indicators.
    for (index, category) in categories.iter().enumerate() {
        book::goto(index, None, 0, None);
        shoot(&mut canvas, format!("fresh-{}", category["id"].as_str().unwrap()), [-1.0, -1.0]);
    }
    for (name, kind, query) in [("search", "search", ""), ("search-calc", "search", "calc"), ("search-none", "search", "xyzzy"), ("recent", "recent", "")] {
        book::goto(0, None, 0, Some((kind, query)));
        shoot(&mut canvas, format!("list-{name}"), [-1.0, -1.0]);
    }
    // Everything read and viewed, one bookmark.
    let ids: Vec<&str> = json["entries"].as_array().unwrap().iter().filter_map(|e| e["id"].as_str()).collect();
    book::set_progress(&serde_json::to_vec(&json!({"read": ids, "viewed": ids, "unlocked": ids, "bookmarks": [[ids[0], 0]]})).unwrap());
    for (name, kind) in [("bookmarks", "bookmarks"), ("recent-all", "recent")] {
        book::goto(0, None, 0, Some((kind, "")));
        shoot(&mut canvas, format!("list-{name}"), [-1.0, -1.0]);
    }
    book::goto(0, None, 1, Some(("search", "")));
    shoot(&mut canvas, "list-search-2".to_owned(), [-1.0, -1.0]);
    for (index, category) in categories.iter().enumerate() {
        let id = category["id"].as_str().unwrap();
        book::goto(index, None, 0, None);
        shoot(&mut canvas, format!("category-{id}"), [-1.0, -1.0]);
        for entry in json["entries"].as_array().unwrap() {
            let entry_id = entry["id"].as_str().unwrap();
            if entry["category"].as_str() != Some(&format!("theurgy:{id}")) || !entry_id.contains(&filter) {
                continue;
            }
            let pages = entry["pages"].as_array().map_or(0, Vec::len);
            for spread in (0..pages.max(1)).step_by(2) {
                book::goto(index, Some(entry_id), spread, None);
                shoot(&mut canvas, format!("{}-{}", entry_id.replace([':', '/'], "_"), spread / 2), [-1.0, -1.0]);
            }
        }
    }
    std::fs::write(out.join("scenes.json"), serde_json::to_vec(&scenes).unwrap()).unwrap();
    println!("{} scenes", scenes.len());
}
