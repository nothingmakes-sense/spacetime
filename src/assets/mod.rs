mod catalog;
mod gltf_loader;
mod paths;
mod primitives;
mod rig;
mod skeleton;
mod vertex;

pub use catalog::{AdventurerClass, ANIM_GENERAL, ANIM_MOVEMENT, GROUND_HALF_EXTENT};
pub use paths::resolve_asset;
pub use primitives::{
    chest_parts, digit_quad, furnace_parts, ground_plane, item_gem, slot_plate, unit_box,
    workbench_model,
};
pub use rig::{load_rigged, RiggedModel, SkinnedPrim};
pub use skeleton::{Joint, Skeleton};
pub use vertex::Vertex;

use anyhow::Result;
use glam::Mat4;
use std::path::Path;

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub albedo: [f32; 4],
    pub albedo_pixels: Option<Vec<u8>>,
    pub albedo_size: (u32, u32),
}

pub struct Model {
    pub name: String,
    pub meshes: Vec<Mesh>,
    pub sockets: Vec<(String, Mat4)>,
}

impl Model {
    pub fn socket(&self, name: &str) -> Option<Mat4> {
        self.sockets
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, m)| *m)
    }

    pub fn vertex_count(&self) -> usize {
        self.meshes.iter().map(|m| m.vertices.len()).sum()
    }
}

pub struct AssetManager;

impl Default for AssetManager {
    fn default() -> Self {
        Self
    }
}

impl AssetManager {
    pub fn new() -> Self {
        Self
    }

    pub fn load_model(&self, path: impl AsRef<Path>) -> Result<Model> {
        gltf_loader::load_gltf(resolve_asset(path))
    }

    pub fn load_adventurer(&self, class: AdventurerClass) -> Result<Model> {
        self.load_model(class.glb_path())
    }

    pub fn load_rigged(&self, path: impl AsRef<Path>) -> Result<RiggedModel> {
        load_rigged(resolve_asset(path))
    }

    pub fn load_rigged_class(&self, class: AdventurerClass) -> Result<RiggedModel> {
        self.load_rigged(class.glb_path())
    }
}
