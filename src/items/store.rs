//! Authoritative item rules for single-player. The SpacetimeDB reducers
//! call the same `rules` helpers so both modes stay in lockstep.

use glam::Vec3;

use super::{
    decode_slots, empty_bag, encode_slots, first_compatible, first_nonempty, insert_stack,
    recipe_by_id, recipes_for, step_furnace, take_inputs, take_one, CraftStation, ItemId, Recipe,
    SlotRole, Stack, StationKind, BAG_SLOTS, DEFAULT_LOOT, DEFAULT_STATIONS, PICKUP_RANGE,
    STACK_MERGE_RANGE, STARTER_KIT, STATION_RANGE,
};

#[derive(Clone, Debug)]
pub struct LootView {
    pub id: u64,
    pub stack: Stack,
    pub pos: Vec3,
}

#[derive(Clone, Debug)]
pub struct StationView {
    pub id: u64,
    pub kind: StationKind,
    pub pos: Vec3,
    pub rot: f32,
    pub slots: Vec<Stack>,
    pub fuel: u32,
    pub cook: u32,
}

#[derive(Clone, Debug)]
pub struct ItemView {
    pub bag: Vec<Stack>,
    pub selected: usize,
    pub loot: Vec<LootView>,
    pub stations: Vec<StationView>,
    pub open_station: Option<u64>,
    pub recipe_cursor: usize,
    pub last_log: String,
}

impl ItemView {
    pub fn selected_stack(&self) -> Stack {
        self.bag.get(self.selected).copied().unwrap_or_else(Stack::empty)
    }

    pub fn open_station_view(&self) -> Option<&StationView> {
        let id = self.open_station?;
        self.stations.iter().find(|s| s.id == id)
    }

    pub fn packed_bag(&self) -> String {
        encode_slots(&self.bag)
    }
}

/// Address of one inventory cell. HUD hit-testing and click-to-move use this
/// so bag and station slots share one code path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotRef {
    /// Player bag / hotbar index `0..BAG_SLOTS`.
    Bag(usize),
    /// Open station slot (chest or furnace).
    Station(usize),
}

impl SlotRef {
    pub fn role(self, station: Option<StationKind>) -> SlotRole {
        match self {
            Self::Bag(_) => SlotRole::Any,
            Self::Station(i) => station.map(|k| k.slot_role(i)).unwrap_or(SlotRole::Any),
        }
    }
}

pub trait ItemStore {
    fn view(&self) -> ItemView;
    fn tick(&mut self, dt: f32);
    fn pickup(&mut self, loot_id: u64) -> bool;
    fn pickup_nearest(&mut self, pos: Vec3) -> bool;
    fn drop_selected(&mut self, pos: Vec3, yaw: f32) -> bool;
    fn select(&mut self, slot: usize);
    fn swap(&mut self, a: usize, b: usize);
    fn transfer_station(&mut self, bag_slot: usize, st_slot: usize, to_station: bool) -> bool;
    fn transfer_selected(&mut self, to_station: bool) -> bool;
    fn craft(&mut self, recipe_id: u16) -> bool;
    fn cycle_recipe(&mut self, dir: i32);
    fn toggle_station(&mut self, id: u64) -> bool;
    fn close_station(&mut self);
    fn give(&mut self, item: ItemId, count: u16) -> u16;

    /// Read a slot from the current view (server cache or local bag).
    fn peek_slot(&self, slot: SlotRef) -> Stack {
        let v = self.view();
        match slot {
            SlotRef::Bag(i) => v.bag.get(i).copied().unwrap_or_else(Stack::empty),
            SlotRef::Station(i) => v
                .open_station_view()
                .and_then(|s| s.slots.get(i).copied())
                .unwrap_or_else(Stack::empty),
        }
    }

    /// Move `from` → `to`. Whole stack, or a single item when `one` is set.
    /// Rejects the write when the destination [`SlotRole`] does not accept the
    /// payload (e.g. cobble in the furnace fuel slot).
    fn move_between(&mut self, from: SlotRef, to: SlotRef, one: bool) -> bool;
}

pub struct LocalStore {
    next_id: u64,
    bag: Vec<Stack>,
    selected: usize,
    loot: Vec<LootView>,
    stations: Vec<StationView>,
    open_station: Option<u64>,
    recipe_cursor: usize,
    last_log: String,
    furnace_acc: f32,
}

impl LocalStore {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            bag: empty_bag(),
            selected: 0,
            loot: Vec::new(),
            stations: Vec::new(),
            open_station: None,
            recipe_cursor: 0,
            last_log: String::new(),
            furnace_acc: 0.0,
        }
    }

    /// Starter kit + the default yard (chest, furnace, workbench, ground loot).
    pub fn with_default_world() -> Self {
        let mut s = Self::new();
        for (id, count) in STARTER_KIT {
            s.give(ItemId(*id), *count);
        }
        for (kind, x, y, z, rot) in DEFAULT_STATIONS {
            s.spawn_station(*kind, Vec3::new(*x, *y, *z), *rot);
        }
        for (item, count, x, y, z) in DEFAULT_LOOT {
            s.spawn_loot(Stack::new(ItemId(*item), *count), Vec3::new(*x, *y, *z));
        }
        s
    }

    fn alloc(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn spawn_station(&mut self, kind: StationKind, pos: Vec3, rot: f32) -> u64 {
        let id = self.alloc();
        self.stations.push(StationView {
            id,
            kind,
            pos,
            rot,
            slots: vec![Stack::empty(); kind.slots()],
            fuel: 0,
            cook: 0,
        });
        id
    }

    pub fn spawn_loot(&mut self, stack: Stack, pos: Vec3) -> u64 {
        let id = self.alloc();
        self.loot.push(LootView { id, stack, pos });
        id
    }

    fn log(&mut self, msg: impl Into<String>) {
        self.last_log = msg.into();
        log::info!("items: {}", self.last_log);
    }

    fn available_recipes(&self) -> Vec<&'static Recipe> {
        let at = self
            .open_station
            .and_then(|id| self.stations.iter().find(|s| s.id == id))
            .and_then(|s| s.kind.craft_station())
            .unwrap_or(CraftStation::Hand);
        recipes_for(at).collect()
    }

    fn step_furnaces(&mut self) {
        for st in &mut self.stations {
            if st.kind != StationKind::Furnace {
                continue;
            }
            step_furnace(&mut st.slots, &mut st.fuel, &mut st.cook);
        }
    }

    /// Collapse same-item world drops that sit next to each other.
    /// The surviving pile grows; [`ItemId::visual_mesh`] then picks a bigger
    /// ResourceBits model (1 log → log stack, 1 bar → bar crate, …).
    fn merge_nearby_loot(&mut self) {
        let mut i = 0;
        while i < self.loot.len() {
            let mut j = i + 1;
            while j < self.loot.len() {
                let same = self.loot[i].stack.item == self.loot[j].stack.item
                    && !self.loot[i].stack.is_empty();
                let close = self.loot[i].pos.distance(self.loot[j].pos) <= STACK_MERGE_RANGE;
                if same && close {
                    let extra = self.loot[j].stack;
                    let leftover = self.loot[i].stack.absorb(extra);
                    if leftover.is_empty() {
                        self.loot.remove(j);
                        continue;
                    }
                    self.loot[j].stack = leftover;
                }
                j += 1;
            }
            i += 1;
        }
    }

    fn station_in_range(st: &StationView, pos: Vec3) -> bool {
        st.pos.distance(pos) <= STATION_RANGE
    }

    fn slot_get(&self, slot: SlotRef) -> Stack {
        match slot {
            SlotRef::Bag(i) => self.bag.get(i).copied().unwrap_or_else(Stack::empty),
            SlotRef::Station(i) => self
                .stations
                .iter()
                .find(|s| Some(s.id) == self.open_station)
                .and_then(|s| s.slots.get(i).copied())
                .unwrap_or_else(Stack::empty),
        }
    }

    fn slot_set(&mut self, slot: SlotRef, stack: Stack) {
        match slot {
            SlotRef::Bag(i) => {
                if let Some(s) = self.bag.get_mut(i) {
                    *s = stack;
                }
            }
            SlotRef::Station(i) => {
                if let Some(st) = self
                    .stations
                    .iter_mut()
                    .find(|s| Some(s.id) == self.open_station)
                {
                    if let Some(s) = st.slots.get_mut(i) {
                        *s = stack;
                    }
                }
            }
        }
    }

    fn open_kind(&self) -> Option<StationKind> {
        self.open_station
            .and_then(|id| self.stations.iter().find(|s| s.id == id))
            .map(|s| s.kind)
    }
}

impl ItemStore for LocalStore {
    fn view(&self) -> ItemView {
        ItemView {
            bag: self.bag.clone(),
            selected: self.selected,
            loot: self.loot.clone(),
            stations: self.stations.clone(),
            open_station: self.open_station,
            recipe_cursor: self.recipe_cursor,
            last_log: self.last_log.clone(),
        }
    }

    fn tick(&mut self, dt: f32) {
        self.furnace_acc += dt;
        while self.furnace_acc >= 0.05 {
            self.furnace_acc -= 0.05;
            self.step_furnaces();
        }
        self.merge_nearby_loot();
    }

    fn pickup(&mut self, loot_id: u64) -> bool {
        let Some(idx) = self.loot.iter().position(|l| l.id == loot_id) else {
            return false;
        };
        let loot = self.loot.remove(idx);
        let left = insert_stack(&mut self.bag, loot.stack);
        if !left.is_empty() {
            self.loot.push(LootView {
                id: loot.id,
                stack: left,
                pos: loot.pos,
            });
            self.log("inventory full");
            return false;
        }
        self.log(format!(
            "picked up {} x{}",
            loot.stack.item.def().name,
            loot.stack.count
        ));
        true
    }

    fn pickup_nearest(&mut self, pos: Vec3) -> bool {
        let Some((idx, _)) = self
            .loot
            .iter()
            .enumerate()
            .filter(|(_, l)| l.pos.distance(pos) <= PICKUP_RANGE)
            .min_by(|a, b| {
                a.1.pos
                    .distance(pos)
                    .partial_cmp(&b.1.pos.distance(pos))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        else {
            return false;
        };
        let id = self.loot[idx].id;
        self.pickup(id)
    }

    fn drop_selected(&mut self, pos: Vec3, yaw: f32) -> bool {
        let slot = self.selected.min(BAG_SLOTS - 1);
        let Some(drop) = take_one(&mut self.bag, slot) else {
            return false;
        };
        let dir = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
        let p = pos + dir * 1.2 + Vec3::Y * 0.35;
        self.spawn_loot(drop, p);
        self.log(format!("dropped {}", drop.item.def().name));
        true
    }

    fn select(&mut self, slot: usize) {
        if slot < BAG_SLOTS {
            self.selected = slot;
        }
    }

    fn swap(&mut self, a: usize, b: usize) {
        if a < self.bag.len() && b < self.bag.len() {
            self.bag.swap(a, b);
        }
    }

    fn transfer_station(&mut self, bag_slot: usize, st_slot: usize, to_station: bool) -> bool {
        let Some(id) = self.open_station else {
            return false;
        };
        let Some(st) = self.stations.iter_mut().find(|s| s.id == id) else {
            return false;
        };
        if bag_slot >= self.bag.len() || st_slot >= st.slots.len() {
            return false;
        }
        if to_station {
            let leftover = st.slots[st_slot].absorb(self.bag[bag_slot]);
            self.bag[bag_slot] = leftover;
        } else {
            let leftover = self.bag[bag_slot].absorb(st.slots[st_slot]);
            st.slots[st_slot] = leftover;
        }
        true
    }

    fn transfer_selected(&mut self, to_station: bool) -> bool {
        let Some(id) = self.open_station else {
            self.log("open a chest or furnace first");
            return false;
        };
        let Some(st) = self.stations.iter().find(|s| s.id == id) else {
            return false;
        };
        if st.slots.is_empty() {
            self.log("this station has no slots");
            return false;
        }
        let bag_slot = self.selected.min(BAG_SLOTS - 1);
        let st_slot = if to_station {
            first_compatible(&st.slots, self.bag[bag_slot]).unwrap_or(0)
        } else {
            first_nonempty(&st.slots).unwrap_or(0)
        };
        self.transfer_station(bag_slot, st_slot, to_station)
    }

    fn craft(&mut self, recipe_id: u16) -> bool {
        let Some(recipe) = recipe_by_id(recipe_id) else {
            return false;
        };
        let station = self
            .open_station
            .and_then(|id| self.stations.iter().find(|s| s.id == id));
        let at = station
            .and_then(|s| s.kind.craft_station())
            .unwrap_or(CraftStation::Hand);
        if recipe.station == CraftStation::Furnace {
            self.log("put ore in the furnace instead");
            return false;
        }
        if recipe.station != CraftStation::Hand && recipe.station != at {
            self.log("wrong station for that recipe");
            return false;
        }
        if recipe.station == CraftStation::Workbench {
            if station.is_none_or(|s| s.kind != StationKind::Workbench) {
                self.log("stand at a workbench");
                return false;
            }
        }
        if !take_inputs(&mut self.bag, recipe) {
            self.log("missing ingredients");
            return false;
        }
        let left = insert_stack(&mut self.bag, recipe.output.into());
        if !left.is_empty() {
            // refund is awkward after take; drop leftover in the world at origin of first station
            self.log("no space for craft result");
            return false;
        }
        self.log(format!("crafted {}", recipe.name));
        true
    }

    fn cycle_recipe(&mut self, dir: i32) {
        let n = self.available_recipes().len();
        if n == 0 {
            return;
        }
        let cur = self.recipe_cursor as i32 + dir;
        self.recipe_cursor = cur.rem_euclid(n as i32) as usize;
    }

    fn toggle_station(&mut self, id: u64) -> bool {
        if self.open_station == Some(id) {
            self.open_station = None;
            return true;
        }
        if let Some(st) = self.stations.iter().find(|s| s.id == id) {
            self.open_station = Some(id);
            self.log(format!("opened {}", st.kind.name()));
            true
        } else {
            false
        }
    }

    fn close_station(&mut self) {
        self.open_station = None;
    }

    fn give(&mut self, item: ItemId, count: u16) -> u16 {
        insert_stack(&mut self.bag, Stack::new(item, count)).count
    }

    fn move_between(&mut self, from: SlotRef, to: SlotRef, one: bool) -> bool {
        if from == to {
            return true;
        }
        let mut src = self.slot_get(from);
        if src.is_empty() {
            return false;
        }
        let moving = if one {
            Stack::new(src.item, 1)
        } else {
            src
        };
        let kind = self.open_kind();
        if !to.role(kind).accepts(moving) {
            self.log("that slot does not accept this item");
            return false;
        }
        let mut dest = self.slot_get(to);
        if dest.is_empty() || dest.item == moving.item {
            let leftover = dest.absorb(moving);
            if one {
                src.count = src.count.saturating_sub(1);
                if src.count == 0 {
                    src = Stack::empty();
                }
                if !leftover.is_empty() {
                    src.count += leftover.count;
                }
            } else if leftover.is_empty() {
                src = Stack::empty();
            } else {
                src = leftover;
            }
            self.slot_set(to, dest);
            self.slot_set(from, src);
            true
        } else if !one && from.role(kind).accepts(dest) {
            self.slot_set(from, dest);
            self.slot_set(to, moving);
            true
        } else {
            self.log("cannot swap into that slot");
            false
        }
    }
}

impl Default for LocalStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Used by the server-side catch-up math (mirrored) and tests.
#[allow(dead_code)]
pub fn pack_station(st: &StationView) -> String {
    encode_slots(&st.slots)
}

#[allow(dead_code)]
pub fn unpack_station(kind: StationKind, packed: &str) -> Vec<Stack> {
    decode_slots(packed, kind.slots())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn craft_sticks_and_pickup() {
        let mut s = LocalStore::new();
        s.give(ItemId::WOOD, 2);
        assert!(s.craft(1));
        assert_eq!(
            s.bag.iter().filter(|x| x.item == ItemId::STICK).map(|x| x.count).sum::<u16>(),
            2
        );
        s.spawn_loot(Stack::new(ItemId::COAL, 3), Vec3::ZERO);
        assert!(s.pickup_nearest(Vec3::ZERO));
        assert_eq!(
            s.bag.iter().filter(|x| x.item == ItemId::COAL).map(|x| x.count).sum::<u16>(),
            3
        );
    }

    #[test]
    fn drop_and_repickup() {
        let mut s = LocalStore::new();
        s.give(ItemId::STONE, 2);
        s.select(0);
        assert!(s.drop_selected(Vec3::ZERO, 0.0));
        assert_eq!(s.loot.len(), 1);
        assert!(s.pickup_nearest(Vec3::new(0.0, 0.0, -1.2)));
        assert!(s.loot.is_empty());
    }

    #[test]
    fn chest_transfer() {
        let mut s = LocalStore::new();
        s.give(ItemId::WOOD, 4);
        let id = s.spawn_station(StationKind::Chest, Vec3::ZERO, 0.0);
        s.toggle_station(id);
        assert!(s.transfer_selected(true));
        let st = s.stations.iter().find(|x| x.id == id).unwrap();
        assert_eq!(st.slots[0].item, ItemId::WOOD);
        assert_eq!(st.slots[0].count, 4);
    }

    #[test]
    fn nearby_same_item_merges_and_grows() {
        let mut s = LocalStore::new();
        s.spawn_loot(Stack::new(ItemId::WOOD, 3), Vec3::ZERO);
        s.spawn_loot(Stack::new(ItemId::WOOD, 5), Vec3::new(0.4, 0.0, 0.2));
        s.tick(0.05);
        assert_eq!(s.view().loot.len(), 1);
        assert_eq!(s.view().loot[0].stack.count, 8);
        assert_eq!(ItemId::WOOD.visual_mesh(8), "Wood_Log_B");
        assert_eq!(ItemId::WOOD.visual_mesh(1), "Wood_Log_A");
    }
}

