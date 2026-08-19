//! Shared block materials for the voxel world.
//!
//! Albedo now comes from `assets/Material-LIB` (see [`crate::assets::material_lib`]).
//! Inventory items map onto these ids so a mined iron vein drops `ItemId::ORE`.

use crate::items::ItemId;

pub const ATLAS_PATH: &str = "assets/kaykit/resource_bits/resource_bits_texture.png";
pub const ATLAS_SIZE: u32 = 1024;
pub const ATLAS_CELLS: u32 = 8;
pub const CELL_PX: u32 = ATLAS_SIZE / ATLAS_CELLS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Block {
    Air = 0,
    Grass = 1,
    Dirt = 2,
    Stone = 3,
    Wood = 4,
    Iron = 5,
    Copper = 6,
    Gold = 7,
    Water = 8,
    Brick = 9,
    Cobble = 10,
    Plank = 11,
    Gravel = 12,
    Tiles = 13,
}

impl Block {
    pub const ALL: [Block; 13] = [
        Self::Grass,
        Self::Dirt,
        Self::Stone,
        Self::Wood,
        Self::Iron,
        Self::Copper,
        Self::Gold,
        Self::Water,
        Self::Brick,
        Self::Cobble,
        Self::Plank,
        Self::Gravel,
        Self::Tiles,
    ];

    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Grass,
            2 => Self::Dirt,
            3 => Self::Stone,
            4 => Self::Wood,
            5 => Self::Iron,
            6 => Self::Copper,
            7 => Self::Gold,
            8 => Self::Water,
            9 => Self::Brick,
            10 => Self::Cobble,
            11 => Self::Plank,
            12 => Self::Gravel,
            13 => Self::Tiles,
            _ => Self::Air,
        }
    }

    pub fn is_solid(self) -> bool {
        !matches!(self, Self::Air | Self::Water)
    }

    pub fn material(self) -> VoxelMaterial {
        MATERIALS.get(self as usize).copied().unwrap_or(MATERIALS[0])
    }

    /// What the player receives when this block is mined. `None` = unbreakable / air.
    pub fn drops(self) -> Option<(ItemId, u16)> {
        match self {
            Self::Air | Self::Water => None,
            Self::Grass | Self::Dirt => Some((ItemId::STONE, 1)),
            Self::Stone | Self::Cobble | Self::Gravel => Some((ItemId::STONE, 1)),
            Self::Wood => Some((ItemId::WOOD, 1)),
            Self::Plank => Some((ItemId::WOOD_PLANK, 1)),
            Self::Iron => Some((ItemId::ORE, 1)),
            Self::Copper => Some((ItemId::COPPER_NUGGET, 1)),
            Self::Gold => Some((ItemId::GOLD_NUGGET, 1)),
            Self::Brick | Self::Tiles => Some((ItemId::STONE_BRICK, 1)),
        }
    }

    pub fn from_place_id(id: u8) -> Option<Self> {
        if id == 0 {
            None
        } else {
            let b = Self::from_u8(id);
            (!matches!(b, Self::Air)).then_some(b)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_ids_map_to_blocks() {
        assert_eq!(Block::from_place_id(4), Some(Block::Wood));
        assert_eq!(Block::from_place_id(9), Some(Block::Brick));
        assert_eq!(Block::from_place_id(11), Some(Block::Plank));
        assert_eq!(Block::from_place_id(0), None);
        assert!(Block::Stone.is_solid());
        assert!(!Block::Water.is_solid());
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VoxelMaterial {
    pub id: u8,
    pub name: &'static str,
    /// Atlas cell (col, row) in `[0, ATLAS_CELLS)` — fallback if Material-LIB is missing.
    pub cell: (u32, u32),
    pub color: [f32; 4],
    pub solid: bool,
}

impl VoxelMaterial {
    pub fn uv_rect(self) -> [f32; 4] {
        let s = ATLAS_CELLS as f32;
        let (c, r) = self.cell;
        [c as f32 / s, r as f32 / s, (c + 1) as f32 / s, (r + 1) as f32 / s]
    }
}

/// One row per [`Block`] discriminant.
pub const MATERIALS: &[VoxelMaterial] = &[
    VoxelMaterial { id: 0, name: "air", cell: (0, 0), color: [0.0, 0.0, 0.0, 0.0], solid: false },
    VoxelMaterial { id: 1, name: "grass", cell: (1, 5), color: [0.35, 0.52, 0.22, 1.0], solid: true },
    VoxelMaterial { id: 2, name: "dirt", cell: (2, 3), color: [0.45, 0.30, 0.16, 1.0], solid: true },
    VoxelMaterial { id: 3, name: "stone", cell: (0, 0), color: [0.38, 0.42, 0.46, 1.0], solid: true },
    VoxelMaterial { id: 4, name: "wood", cell: (5, 2), color: [0.55, 0.34, 0.16, 1.0], solid: true },
    VoxelMaterial { id: 5, name: "iron", cell: (6, 1), color: [0.62, 0.64, 0.68, 1.0], solid: true },
    VoxelMaterial { id: 6, name: "copper", cell: (4, 2), color: [0.80, 0.45, 0.22, 1.0], solid: true },
    VoxelMaterial { id: 7, name: "gold", cell: (3, 1), color: [0.90, 0.74, 0.20, 1.0], solid: true },
    VoxelMaterial { id: 8, name: "water", cell: (0, 6), color: [0.15, 0.32, 0.55, 0.55], solid: false },
    VoxelMaterial { id: 9, name: "brick", cell: (2, 1), color: [0.62, 0.28, 0.22, 1.0], solid: true },
    VoxelMaterial { id: 10, name: "cobble", cell: (0, 1), color: [0.48, 0.48, 0.46, 1.0], solid: true },
    VoxelMaterial { id: 11, name: "plank", cell: (5, 3), color: [0.62, 0.42, 0.22, 1.0], solid: true },
    VoxelMaterial { id: 12, name: "gravel", cell: (1, 2), color: [0.50, 0.48, 0.42, 1.0], solid: true },
    VoxelMaterial { id: 13, name: "tiles", cell: (3, 4), color: [0.42, 0.40, 0.38, 1.0], solid: true },
];
