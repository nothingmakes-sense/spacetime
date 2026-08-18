use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use std::path::Path;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
}

pub struct AssetManager;

impl AssetManager {
    pub fn new() -> Self {
        Self
    }

    pub fn load_model(&self, path: impl AsRef<Path>) -> Result<Model> {
        // Temporary stub – replace with real loader later
        // (russimp-ng / asset-importer / gltf)
        log::warn!("Asset loading is currently stubbed: {}", path.as_ref().display());

        // Return a simple placeholder triangle so the rest of the code compiles
        let vertices = vec![
            Vertex {
                position: [0.0, 0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [0.5, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0, 1.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [1.0, 1.0],
            },
        ];

        Ok(Model {
            meshes: vec![Mesh {
                vertices,
                indices: vec![0, 1, 2],
            }],
        })
    }
}