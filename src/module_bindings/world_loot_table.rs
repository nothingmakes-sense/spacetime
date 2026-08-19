#![allow(unused, clippy::all)]
use super::world_loot_type::WorldLoot;
use spacetimedb_sdk::__codegen::{self as __sdk, __lib, __sats, __ws};

crate::impl_spacetime_table! {
    row = WorldLoot,
    table = "world_loot",
    accessor = world_loot,
    handle = WorldLootTableHandle,
    access_trait = WorldLootTableAccess,
    query_trait = world_lootQueryTableAccess,
    pk = id : u64,
    unique = WorldLootIdUnique
}
