//! Rebuild an [`ItemView`] from the SpacetimeDB client cache.

use glam::Vec3;
use spacetimedb_sdk::{DbContext, Table};

use super::{decode_slots, empty_bag, ItemId, ItemView, LootView, Stack, StationKind, StationView, BAG_SLOTS};
use crate::rpg::{BuildPiece, Hero, EQUIP_SLOTS};
use crate::module_bindings::{
    DbConnection, InventoryTableAccess, StationTableAccess, WorldLootTableAccess,
};

/// UI-only fields that live on the client even in multiplayer
/// (recipe cursor, which station panel is open).
#[derive(Clone, Debug, Default)]
pub struct RemoteUi {
    pub open_station: Option<u64>,
    pub recipe_cursor: usize,
    pub last_log: String,
    pub hero: Hero,
    pub equip: [Stack; EQUIP_SLOTS],
    pub builds: Vec<BuildPiece>,
}

pub fn view_from_connection(conn: &DbConnection, ui: &RemoteUi) -> ItemView {
    let identity = conn.try_identity();
    let bag = identity
        .and_then(|id| conn.db.inventory().owner().find(&id))
        .map(|row| decode_slots(&row.slots, BAG_SLOTS))
        .unwrap_or_else(empty_bag);
    let selected = identity
        .and_then(|id| conn.db.inventory().owner().find(&id))
        .map(|row| row.selected as usize)
        .unwrap_or(0)
        .min(BAG_SLOTS - 1);

    let loot = conn
        .db
        .world_loot()
        .iter()
        .map(|row| LootView {
            id: row.id,
            stack: Stack::new(ItemId(row.item_id), row.count),
            pos: Vec3::new(row.x, row.y, row.z),
        })
        .collect();

    let stations = conn
        .db
        .station()
        .iter()
        .map(|row| {
            let kind = StationKind::from_u8(row.kind);
            StationView {
                id: row.id,
                kind,
                pos: Vec3::new(row.x, row.y, row.z),
                rot: row.rot,
                slots: decode_slots(&row.slots, kind.slots()),
                fuel: row.fuel,
                cook: row.cook,
            }
        })
        .collect();

    ItemView {
        bag,
        selected,
        loot,
        stations,
        open_station: ui.open_station,
        recipe_cursor: ui.recipe_cursor,
        last_log: ui.last_log.clone(),
        equip: ui.equip,
        hero: ui.hero.clone(),
        builds: ui.builds.clone(),
    }
}
