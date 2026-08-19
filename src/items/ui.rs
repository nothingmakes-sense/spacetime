use super::{ItemView, BAG_SLOTS, HOTBAR};

/// Client-only HUD state (which panel is up, bag cursor). Authoritative
/// contents always come from [`super::ItemStore::view`].
#[derive(Clone, Debug, Default)]
pub struct ItemUi {
    pub bag_open: bool,
    /// When true, arrow keys move through the open station instead of the bag.
    pub focus_station: bool,
    pub station_cursor: usize,
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
