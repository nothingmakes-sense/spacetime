//! Concrete assets. To add a new one:
//!
//! 1. Embed [`crate::scene::Object`] as `base`.
//! 2. `impl GameObject` (tick / interact / draws).
//! 3. `scene.spawn(Box::new(YourObject::new(...)))`.
//!
//! Stations and loot are inventory-aware: they call [`crate::items::ItemStore`]
//! from `interact`, so the same object works in single-player and multiplayer.

mod character;
mod chest;
mod loot;
mod prop;
mod station;

pub use character::{AttachedItem, CharacterObject};
pub use chest::ChestObject;
pub use loot::LootObject;
pub use prop::PropObject;
pub use station::{StationMeshes, StationObject};
