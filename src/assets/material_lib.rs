//! KayKit-adjacent PBR pack shipped at `assets/Material-LIB/`.
//!
//! Filenames keep Blender's bracket notation (`Metal[Silver]-B.png`).
//! `-B` is base color, `-N` normal, `-R` roughness, `-M` metallic.
//! The Phong pipeline samples albedo, so we load the `-B` maps (and a few
//! standalone color sheets) and reuse them for voxels, ground, stations,
//! building cubes, and item icons that have no ResourceBits mesh.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use image::imageops::FilterType;

use super::{resolve_asset, Mesh, Model, Vertex};
use crate::voxel::Block;

pub const ROOT: &str = "assets/Material-LIB";
const MAX_DIM: u32 = 512;

/// Named albedo. Paths are relative to the crate so `resolve_asset` works.
#[derive(Clone, Copy, Debug)]
pub struct MatDef {
    pub name: &'static str,
    pub path: &'static str,
}

pub const GRASS: MatDef = MatDef {
    name: "grass",
    path: "assets/Material-LIB/Nature/FoliageGrass/FoliageGrass-B.png",
};
pub const DIRT: MatDef = MatDef {
    name: "dirt",
    path: "assets/Material-LIB/Nature/SurfaceSoil/SurfaceSoil-B.png",
};
pub const STONE: MatDef = MatDef {
    name: "stone",
    path: "assets/Material-LIB/Nature/SurfaceStone/SurfaceStone-B.png",
};
pub const ROCK: MatDef = MatDef {
    name: "rock",
    path: "assets/Material-LIB/Nature/SurfaceRock/SurfaceRock-B.png",
};
pub const WOOD: MatDef = MatDef {
    name: "wood",
    path: "assets/Material-LIB/Nature/Bark/Bark1-B.png",
};
pub const PLANKS: MatDef = MatDef {
    name: "planks",
    path: "assets/Material-LIB/Thatch/Thatch-B.png",
};
pub const IRON: MatDef = MatDef {
    name: "iron",
    path: "assets/Material-LIB/Metal/Metal[Silver]-B.png",
};
pub const COPPER: MatDef = MatDef {
    name: "copper",
    path: "assets/Material-LIB/Metal/Metal[Bronze]-B.png",
};
pub const GOLD: MatDef = MatDef {
    name: "gold",
    path: "assets/Material-LIB/Metal/Metal[Gold]-B.png",
};
pub const WATER: MatDef = MatDef {
    name: "water",
    path: "assets/Material-LIB/Nature/Water/Water-B.png",
};
pub const BRICK: MatDef = MatDef {
    name: "brick",
    path: "assets/Material-LIB/BrickWall/BrickWall[Red]-B.png",
};
pub const COBBLE: MatDef = MatDef {
    name: "cobble",
    path: "assets/Material-LIB/Cobblestone/Cobblestone-B.png",
};
pub const GRAVEL: MatDef = MatDef {
    name: "gravel",
    path: "assets/Material-LIB/Gravel/Gravel-B.png",
};
pub const TILES: MatDef = MatDef {
    name: "tiles",
    path: "assets/Material-LIB/FloorTiles/FloorTiles-B.png",
};
pub const MEDIEVAL: MatDef = MatDef {
    name: "medieval",
    path: "assets/Material-LIB/MedievalTiles/MedievalTiles[Grey]-B.png",
};
pub const FABRIC: MatDef = MatDef {
    name: "fabric",
    path: "assets/Material-LIB/Fabric/Fabric[White]-B.png",
};
pub const MOSS: MatDef = MatDef {
    name: "moss",
    path: "assets/Material-LIB/Nature/Moss/Moss-B.png",
};
pub const SAND: MatDef = MatDef {
    name: "sand",
    path: "assets/Material-LIB/Nature/Sand/Sand-B.png",
};

pub const ALL: &[MatDef] = &[
    GRASS, DIRT, STONE, ROCK, WOOD, PLANKS, IRON, COPPER, GOLD, WATER, BRICK, COBBLE, GRAVEL, TILES,
    MEDIEVAL, FABRIC, MOSS, SAND,
];

pub fn block_material(block: Block) -> Option<MatDef> {
    Some(match block {
        Block::Air => return None,
        Block::Grass => GRASS,
        Block::Dirt => DIRT,
        Block::Stone => STONE,
        Block::Wood => WOOD,
        Block::Iron => IRON,
        Block::Copper => COPPER,
        Block::Gold => GOLD,
        Block::Water => WATER,
        Block::Brick => BRICK,
        Block::Cobble => COBBLE,
        Block::Plank => PLANKS,
        Block::Gravel => GRAVEL,
        Block::Tiles => TILES,
    })
}

/// Load a base-color PNG, capped so voxel/prop uploads stay VRAM-friendly.
pub fn load_albedo(path: impl AsRef<Path>) -> Result<(u32, u32, Vec<u8>)> {
    let path = resolve_asset(path);
    let img = image::open(&path)
        .with_context(|| format!("material-lib {}", path.display()))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    if w <= MAX_DIM && h <= MAX_DIM {
        return Ok((w, h, img.into_raw()));
    }
    let scale = (MAX_DIM as f32 / w.max(h) as f32).min(1.0);
    let nw = ((w as f32) * scale).round().max(1.0) as u32;
    let nh = ((h as f32) * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(&img, nw, nh, FilterType::Triangle);
    Ok((nw, nh, resized.into_raw()))
}

pub fn load_def(def: MatDef) -> Result<(u32, u32, Vec<u8>)> {
    load_albedo(def.path)
}

/// GPU-ready cache keyed by [`Block`] discriminant (and a few named extras).
pub struct MatCache {
    by_block: HashMap<u8, (u32, u32, Vec<u8>)>,
    by_name: HashMap<&'static str, (u32, u32, Vec<u8>)>,
}

impl MatCache {
    pub fn load() -> Self {
        let mut by_block = HashMap::new();
        let mut by_name = HashMap::new();
        for def in ALL {
            match load_def(*def) {
                Ok(px) => {
                    log::info!("material-lib {} {}x{}", def.name, px.0, px.1);
                    by_name.insert(def.name, px);
                }
                Err(e) => log::warn!("material-lib {}: {e:#}", def.name),
            }
        }
        for b in Block::ALL {
            if let Some(def) = block_material(b) {
                if let Some(px) = by_name.get(def.name) {
                    by_block.insert(b as u8, px.clone());
                }
            }
        }
        Self { by_block, by_name }
    }

    pub fn block(&self, b: Block) -> Option<&(u32, u32, Vec<u8>)> {
        self.by_block.get(&(b as u8))
    }

    pub fn named(&self, name: &str) -> Option<&(u32, u32, Vec<u8>)> {
        self.by_name.get(name)
    }

    pub fn or_solid(&self, name: &str, color: [u8; 4]) -> (u32, u32, Vec<u8>) {
        self.named(name)
            .cloned()
            .unwrap_or_else(|| (1, 1, color.to_vec()))
    }
}

/// Box with repeating Material-LIB albedo. Origin at bottom-center (same as `unit_box`).
pub fn textured_box(sx: f32, sy: f32, sz: f32, pixels: (u32, u32, Vec<u8>), name: &str) -> Model {
    let mut m = super::primitives::unit_box([1.0, 1.0, 1.0, 1.0]);
    for mesh in &mut m.meshes {
        for v in &mut mesh.vertices {
            v.position[0] *= sx;
            v.position[1] *= sy;
            v.position[2] *= sz;
        }
        mesh.albedo = [1.0, 1.0, 1.0, 1.0];
        mesh.albedo_pixels = Some(pixels.2.clone());
        mesh.albedo_size = (pixels.0.max(1), pixels.1.max(1));
    }
    m.name = name.into();
    m
}

pub fn textured_ground(half_extent: f32, uv_repeat: f32, pixels: (u32, u32, Vec<u8>)) -> Model {
    let n = [0.0, 1.0, 0.0];
    let h = half_extent;
    let u = uv_repeat;
    Model {
        meshes: vec![Mesh {
            vertices: vec![
                Vertex::new([-h, 0.0, -h], n, [0.0, 0.0]),
                Vertex::new([h, 0.0, -h], n, [u, 0.0]),
                Vertex::new([h, 0.0, h], n, [u, u]),
                Vertex::new([-h, 0.0, h], n, [0.0, u]),
            ],
            indices: vec![0, 2, 1, 0, 3, 2],
            albedo: [1.0, 1.0, 1.0, 1.0],
            albedo_pixels: Some(pixels.2),
            albedo_size: (pixels.0.max(1), pixels.1.max(1)),
        }],
        name: "ground".into(),
        sockets: Vec::new(),
    }
}

/// Small HUD icon: a 1×1 billboard using the material (or a solid tint).
pub fn item_icon(pixels: (u32, u32, Vec<u8>), name: &str) -> Model {
    let n = [0.0, 0.0, 1.0];
    Model {
        meshes: vec![Mesh {
            vertices: vec![
                Vertex::new([-0.5, -0.5, 0.0], n, [0.0, 1.0]),
                Vertex::new([0.5, -0.5, 0.0], n, [1.0, 1.0]),
                Vertex::new([0.5, 0.5, 0.0], n, [1.0, 0.0]),
                Vertex::new([-0.5, 0.5, 0.0], n, [0.0, 0.0]),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            albedo: [1.0, 1.0, 1.0, 1.0],
            albedo_pixels: Some(pixels.2),
            albedo_size: (pixels.0.max(1), pixels.1.max(1)),
        }],
        name: name.into(),
        sockets: Vec::new(),
    }
}

/// Verify the pack is on disk (used by unit tests, no GPU).
pub fn pack_present() -> bool {
    resolve_asset(GRASS.path).exists() && resolve_asset(BRICK.path).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_is_on_disk_and_loads() {
        assert!(pack_present(), "assets/Material-LIB missing");
        let cache = MatCache::load();
        assert!(cache.named("grass").is_some());
        assert!(cache.named("brick").is_some());
        assert!(cache.block(Block::Stone).is_some());
    }
}
