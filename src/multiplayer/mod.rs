use anyhow::Result;
use glam::Vec3;
use log::{error, info};

use spacetimedb_sdk::{DbContext, Table, TableWithPrimaryKey};

use crate::items::{
    first_compatible, first_nonempty, recipe_by_id, recipes_for, view_from_connection, CraftStation,
    ItemId, ItemStore, ItemView, RemoteUi, BAG_SLOTS,
};
use crate::module_bindings::{
    craft, drop_selected, pickup_loot, select_slot, swap_slots, transfer_station, update_transform,
    DbConnection, InventoryTableAccess, PlayerTableAccess, StationTableAccess,
    WorldLootTableAccess,
};

pub struct Multiplayer {
    pub conn: DbConnection,
    pub local_identity: Option<spacetimedb_sdk::Identity>,
    pub ui: RemoteUi,
    pending_pickup: Option<u64>,
    pending_age: u32,
}

impl Multiplayer {
    pub fn connect(uri: &str, db_name: &str) -> Result<Self> {
        let conn = DbConnection::builder()
            .with_uri(uri)
            .with_database_name(db_name)
            .on_connect(|_conn, identity, _token| {
                info!("Connected as {}", identity.to_hex());
            })
            .on_connect_error(|_, err| {
                error!("Connect error: {err}");
            })
            .build()?;

        Ok(Self {
            conn,
            local_identity: None,
            ui: RemoteUi::default(),
            pending_pickup: None,
            pending_age: 0,
        })
    }

    pub fn is_connected(&self) -> bool {
        self.conn.is_active()
    }

    pub fn subscribe_players(&self) {
        self.conn
            .subscription_builder()
            .on_applied(|ctx| {
                info!(
                    "subscription applied  players={} inv={} loot={} stations={}",
                    ctx.db.player().count(),
                    ctx.db.inventory().count(),
                    ctx.db.world_loot().count(),
                    ctx.db.station().count()
                );
            })
            .subscribe_to_all_tables();
    }

    pub fn register_callbacks(&self) {
        self.conn.db.player().on_insert(|_ctx, player| {
            info!(
                "Player joined: {} at ({}, {}, {})",
                player.name, player.x, player.y, player.z
            );
        });
        self.conn.db.player().on_update(|_ctx, _old, new| {
            log::debug!(
                "Player updated: {} → ({}, {}, {})",
                new.name,
                new.x,
                new.y,
                new.z
            );
        });
    }

    pub fn send_transform(&self, x: f32, y: f32, z: f32, rot_y: f32) {
        let _ = self.conn.reducers.update_transform(x, y, z, rot_y);
    }

    pub fn frame_tick(&mut self) -> Result<()> {
        self.conn.frame_tick()?;
        if self.local_identity.is_none() {
            self.local_identity = self.conn.try_identity();
        }
        Ok(())
    }

    fn log(&mut self, msg: impl Into<String>) {
        self.ui.last_log = msg.into();
        info!("items: {}", self.ui.last_log);
    }

    fn call_err(&mut self, label: &str, r: spacetimedb_sdk::Result<()>) -> bool {
        match r {
            Ok(()) => true,
            Err(e) => {
                self.log(format!("{label}: {e}"));
                false
            }
        }
    }
}

impl ItemStore for Multiplayer {
    fn view(&self) -> ItemView {
        view_from_connection(&self.conn, &self.ui)
    }

    fn tick(&mut self, _dt: f32) {
        if let Some(id) = self.pending_pickup {
            self.pending_age += 1;
            let gone = !self.view().loot.iter().any(|l| l.id == id);
            if gone || self.pending_age > 20 {
                self.pending_pickup = None;
                self.pending_age = 0;
            }
        }
    }

    fn pickup(&mut self, loot_id: u64) -> bool {
        if self.pending_pickup == Some(loot_id) {
            return false;
        }
        let ok = self.call_err("pickup", self.conn.reducers.pickup_loot(loot_id));
        if ok {
            self.pending_pickup = Some(loot_id);
            self.pending_age = 0;
        }
        ok
    }

    fn pickup_nearest(&mut self, pos: Vec3) -> bool {
        let view = self.view();
        let Some(loot) = view
            .loot
            .iter()
            .filter(|l| l.pos.distance(pos) <= crate::items::PICKUP_RANGE)
            .min_by(|a, b| {
                a.pos
                    .distance(pos)
                    .partial_cmp(&b.pos.distance(pos))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        else {
            return false;
        };
        self.pickup(loot.id)
    }

    fn drop_selected(&mut self, _pos: Vec3, _yaw: f32) -> bool {
        self.call_err("drop", self.conn.reducers.drop_selected())
    }

    fn select(&mut self, slot: usize) {
        if slot < BAG_SLOTS {
            let _ = self.conn.reducers.select_slot(slot as u8);
        }
    }

    fn swap(&mut self, a: usize, b: usize) {
        let _ = self.conn.reducers.swap_slots(a as u8, b as u8);
    }

    fn transfer_station(&mut self, bag_slot: usize, st_slot: usize, to_station: bool) -> bool {
        let Some(id) = self.ui.open_station else {
            return false;
        };
        self.call_err(
            "transfer",
            self.conn
                .reducers
                .transfer_station(id, bag_slot as u8, st_slot as u8, to_station),
        )
    }

    fn transfer_selected(&mut self, to_station: bool) -> bool {
        let view = self.view();
        let Some(st) = view.open_station_view() else {
            self.log("open a chest or furnace first");
            return false;
        };
        if st.slots.is_empty() {
            self.log("this station has no slots");
            return false;
        }
        let bag_slot = view.selected.min(BAG_SLOTS - 1);
        let st_slot = if to_station {
            first_compatible(&st.slots, view.bag[bag_slot]).unwrap_or(0)
        } else {
            first_nonempty(&st.slots).unwrap_or(0)
        };
        self.transfer_station(bag_slot, st_slot, to_station)
    }

    fn craft(&mut self, recipe_id: u16) -> bool {
        if let Some(r) = recipe_by_id(recipe_id) {
            if r.station == CraftStation::Furnace {
                self.log("put ore in the furnace instead");
                return false;
            }
        }
        self.call_err("craft", self.conn.reducers.craft(recipe_id))
    }

    fn cycle_recipe(&mut self, dir: i32) {
        let view = self.view();
        let at = view
            .open_station
            .and_then(|id| view.stations.iter().find(|s| s.id == id))
            .and_then(|s| s.kind.craft_station())
            .unwrap_or(CraftStation::Hand);
        let n = recipes_for(at).count();
        if n == 0 {
            return;
        }
        let cur = self.ui.recipe_cursor as i32 + dir;
        self.ui.recipe_cursor = cur.rem_euclid(n as i32) as usize;
    }

    fn toggle_station(&mut self, id: u64) -> bool {
        if self.ui.open_station == Some(id) {
            self.ui.open_station = None;
            return true;
        }
        if self.view().stations.iter().any(|s| s.id == id) {
            self.ui.open_station = Some(id);
            if let Some(st) = self.view().stations.iter().find(|s| s.id == id) {
                self.log(format!("opened {}", st.kind.name()));
            }
            true
        } else {
            false
        }
    }

    fn close_station(&mut self) {
        self.ui.open_station = None;
    }

    fn give(&mut self, _item: ItemId, _count: u16) -> u16 {
        0
    }
}
