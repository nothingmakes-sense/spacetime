//! Procedural voxel terrain (beginnings).
//!
//! Read [`PLAN.md`](PLAN.md) before adding features. Albedo now comes from
//! `assets/Material-LIB` so inventory drops, placed blocks, and the ground
//! share one palette.

pub mod chunk;
pub mod materials;

pub use chunk::{surface_at, Chunk, CHUNK_SIZE, WORLD_SEED};
pub use materials::{Block, VoxelMaterial, ATLAS_PATH, MATERIALS};
