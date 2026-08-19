//! Procedural voxel terrain (beginnings).
//!
//! Read [`PLAN.md`](PLAN.md) before adding features. Materials come from the
//! KayKit ResourceBits atlas so inventory drops and the world share a palette.

pub mod chunk;
pub mod materials;

pub use chunk::{Chunk, CHUNK_SIZE};
pub use materials::{Block, VoxelMaterial, ATLAS_PATH, MATERIALS};
