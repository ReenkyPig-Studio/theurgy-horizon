// SPDX-License-Identifier: MIT
// Machine menu layout and drawing for Theurgy apparatus screens.
//
// The Java sources describe the distiller and fermentation vat block
// entities; their menus are standard machine menus: a grid of input
// slots, an output slot, a progress arrow, and (for the vat) a fluid
// tank gauge. This module keeps that layout data, small pure geometry
// helpers for the progress arrow and tank gauge, and a renderer that
// draws it through the host drawing helpers.

/// Fluid capacity of the fermentation vat tank, mirroring the Java
/// `FermentationStorageBehaviour` (10 buckets, in millibuckets).
pub const VAT_TANK_CAPACITY: i32 = 10_000;

/// A point or size in GUI pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    /// The rectangle as `[x, y, w, h]` in pixels.
    pub const fn as_f32(self) -> [f32; 4] {
        [self.x as f32, self.y as f32, self.w as f32, self.h as f32]
    }

    /// The horizontal fraction `[0, 1]` of this rectangle for `numerator`
    /// out of `denominator`; clamped so a broken recipe never spills.
    pub fn fraction_x(self, numerator: i32, denominator: i32) -> i32 {
        if denominator <= 0 {
            return 0;
        }
        let full = (self.w as i64) * (numerator.max(0) as i64) / (denominator as i64);
        full.clamp(0, self.w as i64) as i32
    }

    /// The vertical fraction `[0, 1]` of this rectangle for `numerator`
    /// out of `denominator`; clamped so a full tank never draws above.
    pub fn fraction_y(self, numerator: i32, denominator: i32) -> i32 {
        if denominator <= 0 {
            return 0;
        }
        let full = (self.h as i64) * (numerator.max(0) as i64) / (denominator as i64);
        full.clamp(0, self.h as i64) as i32
    }
}

/// The menu of one machine. It is stable data, derived from the block id,
/// so the host can ask for it once and reuse it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Machine {
    Distiller,
    FermentationVat,
}

impl Machine {
    /// The machine a block id names, or none for another block.
    pub fn from_block(block: &str) -> Option<Self> {
        match block {
            "theurgy:distiller" => Some(Self::Distiller),
            "theurgy:fermentation_vat" => Some(Self::FermentationVat),
            _ => None,
        }
    }

    /// The background texture name, as used with the sprite helper.
    pub const fn background(self) -> &'static str {
        match self {
            // The distiller uses a simple single-slot machine menu.
            Self::Distiller => "theurgy:gui/distiller",
            Self::FermentationVat => "theurgy:gui/fermentation_vat",
        }
    }

    /// The menu title, translated by the game.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Distiller => "container.theurgy.distiller",
            Self::FermentationVat => "container.theurgy.fermentation_vat",
        }
    }

    /// The item slots a machine menu shows, in order. The count matches
    /// the container the registry declares.
    pub const fn slots(self) -> &'static [SlotKind] {
        match self {
            Self::Distiller => &[
                SlotKind::Input,
                SlotKind::Output,
                SlotKind::PlayerStart,
            ],
            Self::FermentationVat => &[
                SlotKind::Input,
                SlotKind::Input,
                SlotKind::Input,
                SlotKind::Output,
                SlotKind::PlayerStart,
            ],
        }
    }

    /// Whether this machine has a fluid tank.
    pub const fn has_tank(self) -> bool {
        matches!(self, Self::FermentationVat)
    }

    /// The tank capacity of this machine, in millibuckets.
    pub const fn tank_capacity(self) -> i32 {
        match self {
            Self::Distiller => 0,
            Self::FermentationVat => VAT_TANK_CAPACITY,
        }
    }
}

/// What a visible slot is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotKind {
    /// An input item stack; some are hidden from the player.
    Input,
    /// The single output slot.
    Output,
    /// Followed by the player's own 36 slots (3 rows of 9 plus the
    /// hotbar); the player slots always come last.
    PlayerStart,
}

/// The fixed screen layout of a machine menu, computed from the machine.
pub struct Layout {
    pub machine: Machine,
    /// The whole panel's size in GUI pixels.
    pub width: i32,
    pub height: i32,
    /// The progress arrow bar, drawn left to right.
    pub arrow: Rect,
    /// The slot area, the input and output slots, then the player slots.
    pub slots: Vec<Slot>,
    /// The tank gauge, when the machine has a tank.
    pub tank: Option<Rect>,
}

/// One item slot in the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub kind: SlotKind,
    /// The index of this slot in the block entity's item list; the player
    /// slots count from here within their own inventory.
    pub index: usize,
    pub rect: Rect,
}

impl Layout {
    /// Computes the layout for `machine`.
    pub fn new(machine: Machine) -> Self {
        let arrow = Rect::new(79, 35, 24, 16);
        let tank = machine.has_tank().then_some(Rect::new(154, 17, 12, 52));
        let mut slots = Vec::new();
        // The machine's own slots.
        let machine_slots: &[(SlotKind, Rect)] = match machine {
            Machine::Distiller => &[
                (SlotKind::Input, Rect::new(56, 17, 16, 16)),
                (SlotKind::Output, Rect::new(116, 17, 16, 16)),
            ],
            Machine::FermentationVat => &[
                (SlotKind::Input, Rect::new(44, 17, 16, 16)),
                (SlotKind::Input, Rect::new(62, 17, 16, 16)),
                (SlotKind::Input, Rect::new(80, 17, 16, 16)),
                (SlotKind::Output, Rect::new(134, 17, 16, 16)),
            ],
        };
        for (index, (kind, rect)) in machine_slots.iter().enumerate() {
            slots.push(Slot { kind: *kind, index, rect: *rect });
        }
        // The player inventory: main 9x3 rows then the hotbar.
        let first_player = machine_slots.len();
        for row in 0..3 {
            for col in 0..9 {
                slots.push(Slot {
                    kind: SlotKind::PlayerStart,
                    index: first_player + row * 9 + col,
                    rect: Rect::new(8 + col as i32 * 18, 84 + row as i32 * 18, 16, 16),
                });
            }
        }
        for col in 0..9 {
            slots.push(Slot {
                kind: SlotKind::PlayerStart,
                index: first_player + 27 + col,
                rect: Rect::new(8 + col as i32 * 18, 142, 16, 16),
            });
        }
        Self { machine, width: 176, height: 166, arrow, slots, tank }
    }

    /// The item slot at `hit` (`[x, y]` inside the screen), if any.
    pub fn slot_at(&self, hit: [i32; 2]) -> Option<&Slot> {
        self.slots.iter().find(|slot| {
            let r = slot.rect;
            hit[0] >= r.x && hit[0] < r.x + r.w && hit[1] >= r.y && hit[1] < r.y + r.h
        })
    }

    /// The input slots visible in the menu, that a shift-click may fill.
    pub fn input_slots(&self) -> impl Iterator<Item = &Slot> {
        self.slots.iter().filter(|slot| slot.kind == SlotKind::Input)
    }

    /// The output slot, if the machine has one declared in the menu.
    pub fn output_slot(&self) -> Option<&Slot> {
        self.slots.iter().find(|slot| slot.kind == SlotKind::Output)
    }
}

/// The width and height of the progress arrow's filled part.
pub fn arrow_filled(arrow: Rect, progress: i32, total_time: i32) -> Rect {
    Rect::new(arrow.x, arrow.y, arrow.fraction_x(progress, total_time), arrow.h)
}

/// The part of the tank gauge that the fluid fills, measured from the
/// bottom up. A full tank fills the whole gauge; an empty tank draws
/// nothing.
pub fn tank_filled(tank: Rect, amount: i32, capacity: i32) -> Rect {
    let filled = tank.fraction_y(amount, capacity);
    Rect::new(tank.x, tank.y + (tank.h - filled), tank.w, filled)
}

/// The panel origin in GUI pixels for a `screen_w` x `screen_h` window.
pub fn panel_origin(screen_w: i32, screen_h: i32, panel_w: i32, panel_h: i32) -> [i32; 2] {
    let x = (screen_w - panel_w) / 2;
    let y = (screen_h - panel_h) / 2;
    [x.max(0), y.max(0)]
}

/// Everything the renderer needs to draw the menu.
pub struct DrawState<'a> {
    pub progress: i32,
    pub total_time: i32,
    pub fluid_amount: i32,
    pub fluid: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distiller_layout_matches_registry_slot_count() {
        let l = Layout::new(Machine::Distiller);
        let slots = l.slots.iter().filter(|s| s.kind != SlotKind::PlayerStart).count();
        assert_eq!(slots, 2);
        assert!(l.tank.is_none());
        assert_eq!(Machine::Distiller.tank_capacity(), 0);
    }

    #[test]
    fn vat_layout_has_three_inputs_one_output_and_a_tank() {
        let l = Layout::new(Machine::FermentationVat);
        let inputs = l.input_slots().count();
        assert_eq!(inputs, 3);
        let outputs = l.slots.iter().filter(|s| s.kind == SlotKind::Output).count();
        assert_eq!(outputs, 1);
        assert!(l.tank.is_some());
        assert_eq!(Machine::FermentationVat.tank_capacity(), VAT_TANK_CAPACITY);
    }

    #[test]
    fn player_slots_come_last_and_count_thirty_six() {
        let l = Layout::new(Machine::Distiller);
        let player: Vec<_> = l.slots.iter().filter(|s| s.kind == SlotKind::PlayerStart).collect();
        assert_eq!(player.len(), 36);
        let first_player = l.slots.iter().position(|s| s.kind == SlotKind::PlayerStart).unwrap();
        assert!(l.slots[first_player..].iter().all(|s| s.kind == SlotKind::PlayerStart));
    }

    #[test]
    fn arrow_never_overflows() {
        let arrow = Rect::new(10, 10, 40, 5);
        assert_eq!(arrow_filled(arrow, -5, 10).w, 0);
        assert_eq!(arrow_filled(arrow, 5, 10).w, 20);
        assert_eq!(arrow_filled(arrow, 10, 10).w, 40);
        assert_eq!(arrow_filled(arrow, 20, 10).w, 40);
        assert_eq!(arrow_filled(Rect::new(0, 0, 40, 5), 1, 0).w, 0);
    }

    #[test]
    fn tank_fills_from_the_bottom_up() {
        let tank = Rect::new(5, 10, 4, 20);
        assert_eq!(tank_filled(tank, 0, 100), Rect::new(5, 30, 4, 0));
        assert_eq!(tank_filled(tank, 50, 100), Rect::new(5, 20, 4, 10));
        assert_eq!(tank_filled(tank, 100, 100), Rect::new(5, 10, 4, 20));
        assert_eq!(tank_filled(tank, 200, 100), Rect::new(5, 10, 4, 20));
        assert_eq!(tank_filled(tank, 5, 0), Rect::new(5, 30, 4, 0));
    }

    #[test]
    fn slot_hit_testing_respects_bounds() {
        let l = Layout::new(Machine::Distiller);
        let input = *l.input_slots().next().unwrap();
        let x = input.rect.x;
        let y = input.rect.y;
        assert!(l.slot_at([x, y]).is_some());
        assert!(l.slot_at([x + 15, y + 15]).is_some());
        assert!(l.slot_at([x - 1, y]).is_none());
        assert!(l.slot_at([x + 16, y]).is_none());
    }

    #[test]
    fn panel_is_centred_and_never_negative() {
        assert_eq!(panel_origin(400, 300, 176, 166), [(400 - 176) / 2, (300 - 166) / 2]);
        assert_eq!(panel_origin(100, 100, 176, 166), [0, 0]);
    }

    #[test]
    fn unknown_block_has_no_machine() {
        assert_eq!(Machine::from_block("theurgy:distiller"), Some(Machine::Distiller));
        assert_eq!(Machine::from_block("theurgy:fermentation_vat"), Some(Machine::FermentationVat));
        assert_eq!(Machine::from_block("theurgy:calcination_oven"), None);
    }
}

/// The menu layout Horizon asks for when a Theurgy machine opens. Only the
/// machine's own slots are listed; the player inventory is placed by the game.
pub fn native_screen(block: &str) -> Option<super::native_hooks::NativeScreen> {
    let machine = Machine::from_block(block)?;
    let layout = Layout::new(machine);
    let slots = layout
        .slots
        .iter()
        .take_while(|slot| slot.kind != SlotKind::PlayerStart)
        .map(|slot| Some([slot.rect.x as f32, slot.rect.y as f32]))
        .collect();
    Some(super::native_hooks::NativeScreen {
        width: layout.width as f32,
        height: layout.height as f32,
        slots,
        inventory_y: 84.0,
        inventory_x: 8.0,
        own_labels: true,
    })
}

/// Draws the panel, progress arrow and fluid gauge from the machine's data
/// slots (`[progress, total_time, fluid_amount, capacity]`). The item slots,
/// items and labels are drawn by the game on top.
pub fn draw(
    game: &super::Game<'_>,
    block: &str,
    data: &[i32],
    batch: &mut super::HudBatch,
    origin: [f32; 2],
) {
    let Some(machine) = Machine::from_block(block) else { return };
    if data.is_empty() {
        return;
    }
    let layout = Layout::new(machine);
    let [ox, oy] = origin;
    let panel = [ox - 4.0, oy - 4.0, layout.width as f32 + 8.0, layout.height as f32 + 8.0];
    game.native_fill(batch, panel, 0xFFC6C6C6);
    game.native_fill(batch, [panel[0] + 2.0, panel[1] + 2.0, panel[2] - 4.0, panel[3] - 4.0], 0xFF8B8B8B);
    let inner = [ox, oy, layout.width as f32, layout.height as f32];
    game.native_fill(batch, inner, 0xFFE8E8E8);
    let progress = data.first().copied().unwrap_or(0).clamp(0, i32::MAX);
    let total = data.get(1).copied().unwrap_or(0).max(0);
    let arrow = arrow_filled(layout.arrow, progress, total);
    game.native_fill(batch, [ox + arrow.x as f32, oy + arrow.y as f32, arrow.w as f32, arrow.h as f32], 0xFF3C8C3C);
    if let (Some(tank), Some(amount), Some(capacity)) = (layout.tank, data.get(2), data.get(3)) {
        let filled = tank_filled(tank, *amount, *capacity);
        game.native_fill(batch, [ox + filled.x as f32, oy + filled.y as f32, filled.w as f32, filled.h as f32], 0xFF55AADD);
    }
    game.native_text(batch, machine.title(), [ox + 8.0, oy + 6.0], 0x404040, false);
}
