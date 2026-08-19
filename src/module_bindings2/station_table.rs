#![allow(unused, clippy::all)]
use super::station_type::Station;
use spacetimedb_sdk::__codegen::{self as __sdk, __lib, __sats, __ws};

crate::impl_spacetime_table! {
    row = Station,
    table = "station",
    accessor = station,
    handle = StationTableHandle,
    access_trait = StationTableAccess,
    query_trait = stationQueryTableAccess,
    pk = id : u64,
    unique = StationIdUnique
}
