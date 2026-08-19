//! Inventory, crafting, world loot, and stations.
//!
//! Rules live in [`rules`] (also path-included by the SpacetimeDB module).
//! Single-player uses [`LocalStore`]. Multiplayer implements the same
//! [`ItemStore`] trait on [`crate::multiplayer::Multiplayer`].

pub mod rules;
mod remote;
mod store;
mod ui;
mod world_sync;

pub use remote::{view_from_connection, RemoteUi};
pub use rules::*;
pub use store::{ItemStore, ItemView, LocalStore, LootView, SlotRef, StationView};
pub use ui::ItemUi;
pub use world_sync::WorldSync;

/// Currently selected craftable recipe (for the HUD).
pub fn selected_recipe(view: &ItemView) -> Option<&'static Recipe> {
    let at = view
        .open_station
        .and_then(|id| view.stations.iter().find(|s| s.id == id))
        .and_then(|s| s.kind.craft_station())
        .unwrap_or(CraftStation::Hand);
    let list: Vec<_> = recipes_for(at).collect();
    if list.is_empty() {
        return None;
    }
    list.get(view.recipe_cursor % list.len()).copied()
}
