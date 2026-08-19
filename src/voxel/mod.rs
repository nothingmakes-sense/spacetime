//! Procedural voxel terrain (beginnings).
//!
//! Read [`PLAN.md`](PLAN.md) before adding features. Albedo now comes from
//! `assets/Material-LIB` so inventory drops, placed blocks, and the ground
//! share one palette.

pub mod chunk;
pub mod materials;

pub use chunk::{surface_at, Chunk, CHUNK_SIZE, WORLD_SEED};
pub use materials::{Block, VoxelMaterial, ATLAS_PATH, MATERIALS};

/// Top face of the surface block at world XZ — the Y a standing character's
/// feet should sit on.
///
/// **Takes:** world XZ in metres (floored to the block column) and the terrain
/// seed that [`Chunk::from_height`] also uses.
/// **Gives:** a world-Y in metres (`surface_block + 1`).
/// **Source:** [`surface_at`] heightfield.
/// **Goes to:** player/NPC spawn, station/loot lift, build placement, void
/// respawn in [`crate::physics::PhysicsWorld::step`].
pub fn stand_y(x: f32, z: f32, seed: u64) -> f32 {
    surface_at(x.floor() as i32, z.floor() as i32, seed) as f32 + 1.0
}

/// Occupancy sample for one integer world cell.
///
/// **Takes:** loaded [`Chunk`]s, the world seed, and the cell `(wx, wy, wz)`.
/// **Gives:** `true` if a walking capsule may not occupy that cell.
/// **Source:** chunk bytes when the cell is inside a loaded chunk; otherwise
/// the heightfield from [`surface_at`] so walking off the meshed ring still
/// has a floor. `wy < 0` is bedrock.
/// **Goes to:** [`crate::physics::PhysicsWorld::step`] when it gathers nearby
/// voxel AABBs.
pub fn solid_at(chunks: &[Chunk], seed: u64, wx: i32, wy: i32, wz: i32) -> bool {
    if wy < 0 {
        return true;
    }
    for c in chunks {
        let ox = c.origin.x * CHUNK_SIZE;
        let oy = c.origin.y * CHUNK_SIZE;
        let oz = c.origin.z * CHUNK_SIZE;
        let lx = wx - ox;
        let ly = wy - oy;
        let lz = wz - oz;
        if (0..CHUNK_SIZE).contains(&lx)
            && (0..CHUNK_SIZE).contains(&ly)
            && (0..CHUNK_SIZE).contains(&lz)
        {
            return c.get(lx, ly, lz).is_solid();
        }
    }
    wy <= surface_at(wx, wz, seed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::IVec3;

    #[test]
    fn stand_y_is_top_of_surface_block() {
        let h = surface_at(0, 0, WORLD_SEED);
        assert_eq!(stand_y(0.4, 0.2, WORLD_SEED), h as f32 + 1.0);
    }

    #[test]
    fn loaded_chunk_agrees_with_heightfield() {
        let c = Chunk::from_height(IVec3::new(0, 0, 0), WORLD_SEED);
        let h = surface_at(3, 5, WORLD_SEED);
        assert!(solid_at(&[c.clone()], WORLD_SEED, 3, h, 5));
        assert!(!solid_at(&[c], WORLD_SEED, 3, h + 1, 5));
    }
}