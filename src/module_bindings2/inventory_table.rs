#![allow(unused, clippy::all)]
use super::inventory_type::Inventory;
use spacetimedb_sdk::__codegen::{self as __sdk, __lib, __sats, __ws};

crate::impl_spacetime_table! {
    row = Inventory,
    table = "inventory",
    accessor = inventory,
    handle = InventoryTableHandle,
    access_trait = InventoryTableAccess,
    query_trait = inventoryQueryTableAccess,
    pk = owner : __sdk::Identity,
    unique = InventoryOwnerUnique
}
