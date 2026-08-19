// Hand-written to match `spacetimedb/src/lib.rs`. Re-run
// `spacetime generate --lang rust --out-dir src/module_bindings --module-path ./spacetimedb`
// after publishing and merge if the CLI rewrites this folder.

#![allow(unused, clippy::all)]
use spacetimedb_sdk::__codegen::{self as __sdk, __lib, __sats, __ws};

#[derive(__lib::ser::Serialize, __lib::de::Deserialize, Clone, PartialEq, Debug)]
#[sats(crate = __lib)]
pub struct Inventory {
    pub owner: __sdk::Identity,
    pub slots: String,
    pub selected: u8,
}

impl __sdk::InModule for Inventory {
    type Module = super::RemoteModule;
}

pub struct InventoryCols {
    pub owner: __sdk::__query_builder::Col<Inventory, __sdk::Identity>,
    pub slots: __sdk::__query_builder::Col<Inventory, String>,
    pub selected: __sdk::__query_builder::Col<Inventory, u8>,
}

impl __sdk::__query_builder::HasCols for Inventory {
    type Cols = InventoryCols;
    fn cols(table_name: &'static str) -> Self::Cols {
        InventoryCols {
            owner: __sdk::__query_builder::Col::new(table_name, "owner"),
            slots: __sdk::__query_builder::Col::new(table_name, "slots"),
            selected: __sdk::__query_builder::Col::new(table_name, "selected"),
        }
    }
}

pub struct InventoryIxCols {
    pub owner: __sdk::__query_builder::IxCol<Inventory, __sdk::Identity>,
}

impl __sdk::__query_builder::HasIxCols for Inventory {
    type IxCols = InventoryIxCols;
    fn ix_cols(table_name: &'static str) -> Self::IxCols {
        InventoryIxCols {
            owner: __sdk::__query_builder::IxCol::new(table_name, "owner"),
        }
    }
}

impl __sdk::__query_builder::CanBeLookupTable for Inventory {}
