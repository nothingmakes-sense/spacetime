#![allow(unused, clippy::all)]
use spacetimedb_sdk::__codegen::{self as __sdk, __lib, __sats, __ws};

#[derive(__lib::ser::Serialize, __lib::de::Deserialize, Clone, PartialEq, Debug)]
#[sats(crate = __lib)]
pub struct Station {
    pub id: u64,
    pub kind: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rot: f32,
    pub slots: String,
    pub fuel: u32,
    pub cook: u32,
    pub last_tick: __sdk::Timestamp,
}

impl __sdk::InModule for Station {
    type Module = super::RemoteModule;
}

pub struct StationCols {
    pub id: __sdk::__query_builder::Col<Station, u64>,
    pub kind: __sdk::__query_builder::Col<Station, u8>,
    pub x: __sdk::__query_builder::Col<Station, f32>,
    pub y: __sdk::__query_builder::Col<Station, f32>,
    pub z: __sdk::__query_builder::Col<Station, f32>,
    pub rot: __sdk::__query_builder::Col<Station, f32>,
    pub slots: __sdk::__query_builder::Col<Station, String>,
    pub fuel: __sdk::__query_builder::Col<Station, u32>,
    pub cook: __sdk::__query_builder::Col<Station, u32>,
    pub last_tick: __sdk::__query_builder::Col<Station, __sdk::Timestamp>,
}

impl __sdk::__query_builder::HasCols for Station {
    type Cols = StationCols;
    fn cols(table_name: &'static str) -> Self::Cols {
        StationCols {
            id: __sdk::__query_builder::Col::new(table_name, "id"),
            kind: __sdk::__query_builder::Col::new(table_name, "kind"),
            x: __sdk::__query_builder::Col::new(table_name, "x"),
            y: __sdk::__query_builder::Col::new(table_name, "y"),
            z: __sdk::__query_builder::Col::new(table_name, "z"),
            rot: __sdk::__query_builder::Col::new(table_name, "rot"),
            slots: __sdk::__query_builder::Col::new(table_name, "slots"),
            fuel: __sdk::__query_builder::Col::new(table_name, "fuel"),
            cook: __sdk::__query_builder::Col::new(table_name, "cook"),
            last_tick: __sdk::__query_builder::Col::new(table_name, "last_tick"),
        }
    }
}

pub struct StationIxCols {
    pub id: __sdk::__query_builder::IxCol<Station, u64>,
}

impl __sdk::__query_builder::HasIxCols for Station {
    type IxCols = StationIxCols;
    fn ix_cols(table_name: &'static str) -> Self::IxCols {
        StationIxCols {
            id: __sdk::__query_builder::IxCol::new(table_name, "id"),
        }
    }
}

impl __sdk::__query_builder::CanBeLookupTable for Station {}
