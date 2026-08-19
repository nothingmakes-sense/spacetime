//! Authoritative multiplayer module.
//!
//! Item rules are path-included from the client (`src/items/rules`) so a
//! craft / pickup / smelt can never disagree between modes.

#[path = "../../src/items/rules/mod.rs"]
mod items_rules;

use items_rules::{
    decode_slots, encode_slots, insert_stack, recipe_by_id, step_furnace, take_inputs, take_one,
    CraftStation, ItemId, Stack, StationKind, BAG_SLOTS, DEFAULT_LOOT, DEFAULT_STATIONS,
    PICKUP_RANGE, STARTER_KIT, STATION_RANGE,
};
use spacetimedb::{Identity, ReducerContext, Table, Timestamp};

#[spacetimedb::table(accessor = player, public)]
#[derive(Clone, Debug)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rot_y: f32,
    pub last_update: Timestamp,
}

#[spacetimedb::table(accessor = inventory, public)]
#[derive(Clone, Debug)]
pub struct Inventory {
    #[primary_key]
    pub owner: Identity,
    pub slots: String,
    pub selected: u8,
}

#[spacetimedb::table(accessor = world_loot, public)]
#[derive(Clone, Debug)]
pub struct WorldLoot {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub item_id: u16,
    pub count: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[spacetimedb::table(accessor = station, public)]
#[derive(Clone, Debug)]
pub struct Station {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub kind: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rot: f32,
    pub slots: String,
    pub fuel: u32,
    pub cook: u32,
    pub last_tick: Timestamp,
}

fn pack_bag(slots: &[Stack]) -> String {
    encode_slots(slots)
}

fn unpack_bag(s: &str) -> Vec<Stack> {
    decode_slots(s, BAG_SLOTS)
}

fn unpack_station(kind: StationKind, s: &str) -> Vec<Stack> {
    decode_slots(s, kind.slots())
}

fn dist2(ax: f32, ay: f32, az: f32, bx: f32, by: f32, bz: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    let dz = az - bz;
    dx * dx + dy * dy + dz * dz
}

fn seed_world(ctx: &ReducerContext) {
    if ctx.db.station().iter().next().is_some() {
        return;
    }
    for (kind, x, y, z, rot) in DEFAULT_STATIONS {
        let slots = vec![Stack::empty(); kind.slots()];
        ctx.db.station().insert(Station {
            id: 0,
            kind: kind.as_u8(),
            x: *x,
            y: *y,
            z: *z,
            rot: *rot,
            slots: encode_slots(&slots),
            fuel: 0,
            cook: 0,
            last_tick: ctx.timestamp,
        });
    }
    for (item, count, x, y, z) in DEFAULT_LOOT {
        ctx.db.world_loot().insert(WorldLoot {
            id: 0,
            item_id: *item,
            count: *count,
            x: *x,
            y: *y,
            z: *z,
        });
    }
}

fn give_starter(ctx: &ReducerContext, owner: Identity) {
    if ctx.db.inventory().owner().find(&owner).is_some() {
        return;
    }
    let mut bag = items_rules::empty_bag();
    for (id, n) in STARTER_KIT {
        insert_stack(&mut bag, Stack::new(ItemId(*id), *n));
    }
    ctx.db.inventory().insert(Inventory {
        owner,
        slots: pack_bag(&bag),
        selected: 0,
    });
}

fn player_xyz(ctx: &ReducerContext, who: Identity) -> Option<(f32, f32, f32)> {
    ctx.db
        .player()
        .identity()
        .find(&who)
        .map(|p| (p.x, p.y, p.z))
}

fn catch_up_station(ctx: &ReducerContext, mut row: Station) -> Station {
    let kind = StationKind::from_u8(row.kind);
    if kind != StationKind::Furnace {
        return row;
    }
    let now = ctx.timestamp.to_micros_since_unix_epoch();
    let then = row.last_tick.to_micros_since_unix_epoch();
    let ticks = ((now.saturating_sub(then)) / 50_000).clamp(0, 4_000) as u32;
    if ticks == 0 {
        return row;
    }
    let mut slots = unpack_station(kind, &row.slots);
    for _ in 0..ticks {
        step_furnace(&mut slots, &mut row.fuel, &mut row.cook);
    }
    row.slots = encode_slots(&slots);
    row.last_tick = ctx.timestamp;
    row
}

fn catch_up_all(ctx: &ReducerContext) {
    let rows: Vec<_> = ctx.db.station().iter().collect();
    for row in rows {
        let next = catch_up_station(ctx, row);
        ctx.db.station().id().update(next);
    }
}

#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) {
    seed_world(ctx);
}

#[spacetimedb::reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) {
    let sender = ctx.sender();
    seed_world(ctx);
    give_starter(ctx, sender);
    if ctx.db.player().identity().find(&sender).is_none() {
        ctx.db.player().insert(Player {
            identity: sender,
            name: format!("Player-{}", &sender.to_hex()[..8]),
            x: 0.0,
            y: 0.0,
            z: 6.0,
            rot_y: 0.0,
            last_update: ctx.timestamp,
        });
    }
}

#[spacetimedb::reducer(client_disconnected)]
pub fn client_disconnected(_ctx: &ReducerContext) {}

#[spacetimedb::reducer]
pub fn update_transform(ctx: &ReducerContext, x: f32, y: f32, z: f32, rot_y: f32) {
    let sender = ctx.sender();
    if let Some(mut p) = ctx.db.player().identity().find(&sender) {
        p.x = x;
        p.y = y;
        p.z = z;
        p.rot_y = rot_y;
        p.last_update = ctx.timestamp;
        ctx.db.player().identity().update(p);
    }
}

#[spacetimedb::reducer]
pub fn set_name(ctx: &ReducerContext, name: String) {
    let sender = ctx.sender();
    if let Some(mut p) = ctx.db.player().identity().find(&sender) {
        p.name = name;
        ctx.db.player().identity().update(p);
    }
}

#[spacetimedb::reducer]
pub fn pickup_loot(ctx: &ReducerContext, loot_id: u64) {
    let sender = ctx.sender();
    let Some((px, py, pz)) = player_xyz(ctx, sender) else {
        return;
    };
    let Some(loot) = ctx.db.world_loot().id().find(&loot_id) else {
        return;
    };
    if dist2(px, py, pz, loot.x, loot.y, loot.z) > PICKUP_RANGE * PICKUP_RANGE {
        return;
    }
    let Some(mut inv) = ctx.db.inventory().owner().find(&sender) else {
        return;
    };
    let mut bag = unpack_bag(&inv.slots);
    let left = insert_stack(&mut bag, Stack::new(ItemId(loot.item_id), loot.count));
    inv.slots = pack_bag(&bag);
    ctx.db.inventory().owner().update(inv);
    if left.is_empty() {
        ctx.db.world_loot().id().delete(&loot_id);
    } else {
        let mut rest = loot;
        rest.count = left.count;
        ctx.db.world_loot().id().update(rest);
    }
}

#[spacetimedb::reducer]
pub fn drop_selected(ctx: &ReducerContext) {
    let sender = ctx.sender();
    let Some((px, py, pz)) = player_xyz(ctx, sender) else {
        return;
    };
    let Some(mut inv) = ctx.db.inventory().owner().find(&sender) else {
        return;
    };
    let mut bag = unpack_bag(&inv.slots);
    let i = (inv.selected as usize).min(BAG_SLOTS - 1);
    let Some(one) = take_one(&mut bag, i) else {
        return;
    };
    inv.slots = pack_bag(&bag);
    ctx.db.inventory().owner().update(inv);
    ctx.db.world_loot().insert(WorldLoot {
        id: 0,
        item_id: one.item.0,
        count: one.count,
        x: px,
        y: py,
        z: pz + 0.6,
    });
}

#[spacetimedb::reducer]
pub fn select_slot(ctx: &ReducerContext, slot: u8) {
    let sender = ctx.sender();
    if let Some(mut inv) = ctx.db.inventory().owner().find(&sender) {
        inv.selected = slot.min((BAG_SLOTS - 1) as u8);
        ctx.db.inventory().owner().update(inv);
    }
}

#[spacetimedb::reducer]
pub fn swap_slots(ctx: &ReducerContext, a: u8, b: u8) {
    let sender = ctx.sender();
    let Some(mut inv) = ctx.db.inventory().owner().find(&sender) else {
        return;
    };
    let mut bag = unpack_bag(&inv.slots);
    let a = a as usize;
    let b = b as usize;
    if a >= bag.len() || b >= bag.len() {
        return;
    }
    bag.swap(a, b);
    inv.slots = pack_bag(&bag);
    ctx.db.inventory().owner().update(inv);
}

#[spacetimedb::reducer]
pub fn transfer_station(ctx: &ReducerContext, station_id: u64, bag_slot: u8, st_slot: u8, to_station: bool) {
    let sender = ctx.sender();
    catch_up_all(ctx);
    let Some((px, py, pz)) = player_xyz(ctx, sender) else {
        return;
    };
    let Some(mut inv) = ctx.db.inventory().owner().find(&sender) else {
        return;
    };
    let Some(mut st) = ctx.db.station().id().find(&station_id) else {
        return;
    };
    if dist2(px, py, pz, st.x, st.y, st.z) > STATION_RANGE * STATION_RANGE {
        return;
    }
    let kind = StationKind::from_u8(st.kind);
    let mut bag = unpack_bag(&inv.slots);
    let mut slots = unpack_station(kind, &st.slots);
    let bi = bag_slot as usize;
    let si = st_slot as usize;
    if bi >= bag.len() || si >= slots.len() {
        return;
    }
    if to_station {
        if bag[bi].is_empty() {
            return;
        }
        let left = {
            let src = bag[bi];
            insert_stack(std::slice::from_mut(&mut slots[si]), src)
        };
        bag[bi] = left;
    } else {
        if slots[si].is_empty() {
            return;
        }
        let left = insert_stack(&mut bag, slots[si]);
        slots[si] = left;
    }
    inv.slots = pack_bag(&bag);
    st.slots = encode_slots(&slots);
    ctx.db.inventory().owner().update(inv);
    ctx.db.station().id().update(st);
}

#[spacetimedb::reducer]
pub fn craft(ctx: &ReducerContext, recipe_id: u16) {
    let sender = ctx.sender();
    catch_up_all(ctx);
    let Some(recipe) = recipe_by_id(recipe_id) else {
        return;
    };
    let Some((px, py, pz)) = player_xyz(ctx, sender) else {
        return;
    };
    let Some(mut inv) = ctx.db.inventory().owner().find(&sender) else {
        return;
    };
    if recipe.station != CraftStation::Hand {
        let nearby = ctx.db.station().iter().any(|s| {
            StationKind::from_u8(s.kind).craft_station() == Some(recipe.station)
                && dist2(px, py, pz, s.x, s.y, s.z) <= STATION_RANGE * STATION_RANGE
        });
        if !nearby {
            return;
        }
    }
    let mut bag = unpack_bag(&inv.slots);
    if !take_inputs(&mut bag, recipe) {
        return;
    }
    insert_stack(&mut bag, recipe.output.into());
    inv.slots = pack_bag(&bag);
    ctx.db.inventory().owner().update(inv);
}