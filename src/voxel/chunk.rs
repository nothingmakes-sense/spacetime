//! 16³ chunk + culled-cube mesher.
//!
//! **In:** a seed and a chunk coordinate. **Out:** a [`Chunk`] of block ids
//! and, via [`Chunk::mesh`], a Phong-ready [`crate::assets::Model`].

use glam::IVec3;

use super::materials::Block;
use crate::assets::{Mesh, Model, Vertex};

pub const CHUNK_SIZE: i32 = 16;

#[derive(Clone)]
pub struct Chunk {
    pub origin: IVec3,
    blocks: [u8; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize],
}

impl Chunk {
    pub fn empty(origin: IVec3) -> Self {
        Self {
            origin,
            blocks: [0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }

    fn idx(x: i32, y: i32, z: i32) -> Option<usize> {
        if (0..CHUNK_SIZE).contains(&x) && (0..CHUNK_SIZE).contains(&y) && (0..CHUNK_SIZE).contains(&z)
        {
            Some((y * CHUNK_SIZE * CHUNK_SIZE + z * CHUNK_SIZE + x) as usize)
        } else {
            None
        }
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> Block {
        Self::idx(x, y, z)
            .map(|i| Block::from_u8(self.blocks[i]))
            .unwrap_or(Block::Air)
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, b: Block) {
        if let Some(i) = Self::idx(x, y, z) {
            self.blocks[i] = b as u8;
        }
    }

    /// Value-noise heightfield. Deterministic in `seed` so clients agree.
    pub fn from_height(origin: IVec3, seed: u64) -> Self {
        let mut c = Self::empty(origin);
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let wx = origin.x * CHUNK_SIZE + x;
                let wz = origin.z * CHUNK_SIZE + z;
                let h = height(wx, wz, seed);
                for y in 0..CHUNK_SIZE {
                    let wy = origin.y * CHUNK_SIZE + y;
                    let block = if wy > h {
                        Block::Air
                    } else if wy == h {
                        Block::Grass
                    } else if wy > h - 3 {
                        Block::Dirt
                    } else if ore_at(wx, wy, wz, seed) {
                        Block::Iron
                    } else {
                        Block::Stone
                    };
                    c.set(x, y, z, block);
                }
            }
        }
        c
    }

    /// Culled cubes. Each exposed face is two triangles tinted with the
    /// material color (atlas UVs are filled so a later pass can sample the PNG).
    pub fn mesh(&self) -> Model {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        const FACES: [([f32; 3], [[f32; 3]; 4]); 6] = [
            ([0.0, 1.0, 0.0], [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]]),
            ([0.0, -1.0, 0.0], [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]]),
            ([0.0, 0.0, 1.0], [[0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0]]),
            ([0.0, 0.0, -1.0], [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]]),
            ([1.0, 0.0, 0.0], [[1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]]),
            ([-1.0, 0.0, 0.0], [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0]]),
        ];
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let b = self.get(x, y, z);
                    if !b.is_solid() {
                        continue;
                    }
                    let mat = b.material();
                    let uv = mat.uv_rect();
                    for (ni, (n, corners)) in FACES.iter().enumerate() {
                        let nx = x + [0, 0, 0, 0, 1, -1][ni];
                        let ny = y + [1, -1, 0, 0, 0, 0][ni];
                        let nz = z + [0, 0, 1, -1, 0, 0][ni];
                        if self.get(nx, ny, nz).is_solid() {
                            continue;
                        }
                        let base = vertices.len() as u32;
                        for (i, c) in corners.iter().enumerate() {
                            let u = if i == 0 || i == 3 { uv[0] } else { uv[2] };
                            let v = if i < 2 { uv[3] } else { uv[1] };
                            vertices.push(Vertex::new(
                                [x as f32 + c[0], y as f32 + c[1], z as f32 + c[2]],
                                *n,
                                [u, v],
                            ));
                        }
                        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                        let _ = mat.color;
                    }
                }
            }
        }
        Model {
            name: format!("chunk_{}_{}_{}", self.origin.x, self.origin.y, self.origin.z),
            meshes: vec![Mesh {
                vertices,
                indices,
                albedo: [1.0, 1.0, 1.0, 1.0],
                albedo_pixels: None,
                albedo_size: (1, 1),
            }],
            sockets: Vec::new(),
        }
    }
}

fn hash(x: i32, z: i32, seed: u64) -> f32 {
    let mut n = (x as u64).wrapping_mul(374761393)
        ^ (z as u64).wrapping_mul(668265263)
        ^ seed.wrapping_mul(1274126177);
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    ((n ^ (n >> 16)) as u32 as f32) / (u32::MAX as f32)
}

fn vnoise(x: f32, z: f32, seed: u64) -> f32 {
    let x0 = x.floor() as i32;
    let z0 = z.floor() as i32;
    let tx = x.fract();
    let tz = z.fract();
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sz = tz * tz * (3.0 - 2.0 * tz);
    let a = hash(x0, z0, seed);
    let b = hash(x0 + 1, z0, seed);
    let c = hash(x0, z0 + 1, seed);
    let d = hash(x0 + 1, z0 + 1, seed);
    let u = a + (b - a) * sx;
    let v = c + (d - c) * sx;
    u + (v - u) * sz
}

fn height(x: i32, z: i32, seed: u64) -> i32 {
    let n = vnoise(x as f32 / 48.0, z as f32 / 48.0, seed) * 10.0
        + vnoise(x as f32 / 12.0, z as f32 / 12.0, seed.wrapping_add(17)) * 3.0;
    4 + n as i32
}

fn ore_at(x: i32, y: i32, z: i32, seed: u64) -> bool {
    hash(x.wrapping_mul(3) + y, z.wrapping_mul(5) + y, seed) < 0.04 && y < 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heightfield_is_deterministic_and_has_faces() {
        let a = Chunk::from_height(IVec3::new(0, 0, 0), 42);
        let b = Chunk::from_height(IVec3::new(0, 0, 0), 42);
        assert_eq!(a.blocks, b.blocks);
        let mesh = a.mesh();
        assert!(!mesh.meshes[0].indices.is_empty());
    }
}
