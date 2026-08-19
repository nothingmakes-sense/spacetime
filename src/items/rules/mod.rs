//! Item rules. Lives inside the client crate so `cargo build` needs no
//! extra workspace member. The SpacetimeDB module path-includes this
//! same folder so both modes stay in lockstep.

mod catalog;
mod logic;
mod recipe;
mod stack;
mod station;

pub use catalog::{
    ItemDef, ItemId, BAG_SLOTS, CATALOG, CHEST_SLOTS, FURNACE_SLOTS, HOTBAR, PICKUP_RANGE,
    STATION_RANGE,
};
pub use logic::{
    count_item, first_compatible, first_nonempty, insert_stack, step_furnace, take_inputs, take_one,
};
pub use recipe::{
    recipe_by_id, recipes_for, smelt_of, CraftStation, Ingredient, Recipe, RECIPES,
};
pub use stack::{decode_slots, empty_bag, encode_slots, Stack};
pub use station::StationKind;

/// Default yard layout — identical on the server `init` reducer and in `LocalStore`.
pub const DEFAULT_STATIONS: &[(StationKind, f32, f32, f32, f32)] = &[
    (StationKind::Chest, 2.2, 0.0, 3.5, 0.0),
    (StationKind::Furnace, -2.2, 0.0, 3.5, 0.4),
    (StationKind::Workbench, 0.0, 0.0, 4.8, 0.0),
];

pub const DEFAULT_LOOT: &[(u16, u16, f32, f32, f32)] = &[
    (ItemId::WOOD.0, 6, 1.2, 0.0, 1.6),
    (ItemId::STONE.0, 4, -1.4, 0.0, 1.4),
    (ItemId::ORE.0, 3, -3.2, 0.0, 2.2),
    (ItemId::COAL.0, 4, 3.4, 0.0, 2.0),
    (ItemId::RAW_MEAT.0, 2, 0.6, 0.0, 0.4),
];

pub const STARTER_KIT: &[(u16, u16)] = &[
    (ItemId::WOOD.0, 12),
    (ItemId::STONE.0, 8),
    (ItemId::COAL.0, 6),
    (ItemId::ORE.0, 4),
    (ItemId::RAW_MEAT.0, 2),
    (ItemId::STICK.0, 4),
];
