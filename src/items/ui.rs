//! Client-only HUD state.
//!
//! Authoritative stacks live in [`crate::items::ItemStore`]. This struct
//! tracks which panel is up, the mouse-held stack (click-to-move), and the
//! F3 debug flag. It does not persist — closing the bag returns a held
//! stack to its origin.

use super::{ItemStore, ItemView, SlotRef, Stack, BAG_SLOTS, HOTBAR};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InvTab {
    #[default]
    Bag,
    Stats,
    Skills,
    Craft,
    Build,
}

impl InvTab {
    pub const ALL: [InvTab; 5] = [Self::Bag, Self::Stats, Self::Skills, Self::Craft, Self::Build];

    pub fn label(self) -> &'static str {
        match self {
            Self::Bag => "BAG",
            Self::Stats => "STATS",
            Self::Skills => "SKILLS",
            Self::Craft => "CRAFT",
            Self::Build => "BUILD",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Bag => Self::Stats,
            Self::Stats => Self::Skills,
            Self::Skills => Self::Craft,
            Self::Craft => Self::Build,
            Self::Build => Self::Bag,
        }
    }
}

/// Pointer + panel state. Built on the client; never written to SpacetimeDB.
#[derive(Clone, Debug, Default)]
pub struct ItemUi {
    /// Full bag / station panel is visible.
    pub bag_open: bool,
    /// Arrow keys walk the station grid instead of the bag.
    pub focus_station: bool,
    pub station_cursor: usize,
    /// Stack currently riding the cursor (picked up with LMB).
    pub held: Stack,
    /// Slot the held stack came from — used to put it back on cancel.
    pub origin: Option<SlotRef>,
    /// Slot to hide while the stack is on the cursor (source still full on the server).
    pub hide: Option<SlotRef>,
    /// Last mouse position in HUD NDC (x −1..1 left→right, y −1..1 bottom→top).
    pub mouse_ndc: (f32, f32),
    /// F3 debug overlay.
    pub debug: bool,
    pub tab: InvTab,
    pub hover: Option<SlotRef>,
}

impl ItemUi {
    pub fn toggle_bag(&mut self) {
        self.bag_open = !self.bag_open;
        if !self.bag_open {
            self.focus_station = false;
        }
    }

    pub fn on_station_opened(&mut self) {
        self.bag_open = true;
        self.focus_station = false;
    }

    pub fn on_station_closed(&mut self) {
        self.focus_station = false;
        self.station_cursor = 0;
    }

    /// Convert a window-pixel cursor into HUD NDC. `y` is flipped so +Y is up
    /// (matches the overlay projection).
    pub fn set_mouse_pixels(&mut self, x: f32, y: f32, width: f32, height: f32) {
        if width <= 1.0 || height <= 1.0 {
            return;
        }
        self.mouse_ndc = ((x / width) * 2.0 - 1.0, 1.0 - (y / height) * 2.0);
    }

    /// LMB (`one = false`) picks up / places a whole stack. RMB places one.
    ///
    /// Takes the clicked [`SlotRef`] from HUD hit-testing, reads the live
    /// store, and either lifts a stack onto `held` or commits [`ItemStore::move_between`].
    pub fn click_slot(&mut self, store: &mut dyn ItemStore, slot: SlotRef, one: bool) {
        if self.held.is_empty() {
            let src = store.peek_slot(slot);
            if src.is_empty() {
                return;
            }
            self.held = if one {
                Stack::new(src.item, 1)
            } else {
                src
            };
            self.origin = Some(slot);
            self.hide = Some(slot);
            store.select(match slot {
                SlotRef::Bag(i) => i,
                SlotRef::Station(_) | SlotRef::Equip(_) => store.view().selected,
            });
            return;
        }

        let from = self.origin.unwrap_or(slot);
        if store.move_between(from, slot, one) {
            if one {
                self.held.count = self.held.count.saturating_sub(1);
                if self.held.count == 0 {
                    self.clear_cursor();
                }
            } else {
                self.clear_cursor();
            }
        }
    }

    pub fn clear_cursor(&mut self) {
        self.held = Stack::empty();
        self.origin = None;
        self.hide = None;
    }

    /// Cancel a drag — the source slot is shown again.
    pub fn cancel_drag(&mut self) {
        self.clear_cursor();
    }

    pub fn hides(&self, slot: SlotRef) -> bool {
        self.hide == Some(slot)
    }

    pub fn move_cursor(&mut self, view: &ItemView, dx: i32, dy: i32) {
        if !self.bag_open {
            return;
        }
        if self.focus_station {
            let n = view
                .open_station_view()
                .map(|s| s.slots.len())
                .unwrap_or(0)
                .max(1);
            let cols = 6usize;
            let cur = self.station_cursor as i32;
            let x = cur % cols as i32 + dx;
            let y = cur / cols as i32 + dy;
            let next = (y * cols as i32 + x).rem_euclid(n as i32);
            self.station_cursor = next as usize;
        }
    }

    pub fn next_hotbar(selected: usize, dir: i32) -> usize {
        ((selected as i32 + dir).rem_euclid(HOTBAR as i32)) as usize
    }

    pub fn bag_index(row: usize, col: usize) -> usize {
        (row * HOTBAR + col).min(BAG_SLOTS - 1)
    }
}
