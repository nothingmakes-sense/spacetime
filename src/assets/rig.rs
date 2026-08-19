//! Skinned GLB import: bind-pose verts + skeleton (no rest-pose bake).

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use glam::{Mat4, Vec3};
use gltf::mesh::Mode;

use super::skeleton::{Joint, Skeleton};
use super::vertex::Vertex;
use super::{Mesh, Model};

#[derive(Clone, Debug)]
pub struct SkinnedPrim {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub albedo: [f32; 4],
    pub albedo_pixels: Option<Vec<u8>>,
    pub albedo_size: (u32, u32),
}

impl SkinnedPrim {
    pub fn bind_mesh(&self) -> Mesh {
        let vertices = self
            .positions
            .iter()
            .enumerate()
            .map(|(i, p)| {
                Vertex::new(
                    p.to_array(),
                    self.normals.get(i).map(|n| n.to_array()).unwrap_or([0.0, 1.0, 0.0]),
                    self.uvs.get(i).copied().unwrap_or([0.0, 0.0]),
                )
            })
            .collect();
        Mesh {
            vertices,
            indices: self.indices.clone(),
            albedo: self.albedo,
            albedo_pixels: self.albedo_pixels.clone(),
            albedo_size: self.albedo_size,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RiggedModel {
    pub name: String,
    pub skeleton: Skeleton,
    pub primitives: Vec<SkinnedPrim>,
}

impl RiggedModel {
    pub fn as_model(&self) -> Model {
        Model {
            name: self.name.clone(),
            meshes: self.primitives.iter().map(|p| p.bind_mesh()).collect(),
            sockets: Vec::new(),
        }
    }
}

pub fn load_rigged(path: impl AsRef<Path>) -> Result<RiggedModel> {
    let path = path.as_ref();
    let (doc, buffers, images) =
        gltf::import(path).with_context(|| format!("import rigged {}", path.display()))?;

    let node_locals = collect_locals(&doc);
    let node_worlds = compute_worlds(&doc, &node_locals);

    let skin = doc
        .skins()
        .next()
        .ok_or_else(|| anyhow!("{} has no skin", path.display()))?;
    let skeleton = load_skeleton(&skin, &doc, &node_locals, &buffers)?;

    let mut primitives = Vec::new();
    for node in doc.nodes() {
        let Some(mesh) = node.mesh() else { continue };
        if node.skin().is_none() {
            continue;
        }
        for primitive in mesh.primitives() {
            if primitive.mode() != Mode::Triangles {
                continue;
            }
            if let Some(p) = load_skinned_prim(&primitive, &buffers, &images)? {
                primitives.push(p);
            }
        }
    }

    if primitives.is_empty() {
        return Err(anyhow!("{} has no skinned triangles", path.display()));
    }

    let _ = node_worlds;
    Ok(RiggedModel {
        name: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("rig")
            .to_string(),
        skeleton,
        primitives,
    })
}

fn load_skeleton(
    skin: &gltf::Skin<'_>,
    doc: &gltf::Document,
    locals: &[Mat4],
    buffers: &[gltf::buffer::Data],
) -> Result<Skeleton> {
    let joint_nodes: Vec<gltf::Node<'_>> = skin.joints().collect();
    let mut parent_of = vec![None; joint_nodes.len()];
    let index_of = |node_idx: usize| -> Option<usize> {
        joint_nodes.iter().position(|n| n.index() == node_idx)
    };
    for (ji, node) in joint_nodes.iter().enumerate() {
        for child in node.children() {
            if let Some(ci) = index_of(child.index()) {
                parent_of[ci] = Some(ji);
            }
        }
    }

    let reader = skin.reader(|b| Some(buffers[b.index()].as_ref()));
    let ibms: Vec<Mat4> = if let Some(iter) = reader.read_inverse_bind_matrices() {
        iter.map(|m| Mat4::from_cols_array_2d(&m)).collect()
    } else {
        vec![Mat4::IDENTITY; joint_nodes.len()]
    };

    let joints = joint_nodes
        .iter()
        .enumerate()
        .map(|(i, n)| Joint {
            name: n.name().unwrap_or("joint").to_string(),
            parent: parent_of[i],
            rest_local: locals[n.index()],
        })
        .collect();

    let _ = doc;
    Ok(Skeleton::new(joints, ibms))
}

fn load_skinned_prim(
    primitive: &gltf::Primitive<'_>,
    buffers: &[gltf::buffer::Data],
    images: &[gltf::image::Data],
) -> Result<Option<SkinnedPrim>> {
    let reader = primitive.reader(|b| Some(buffers[b.index()].as_ref()));
    let Some(pos_it) = reader.read_positions() else {
        return Ok(None);
    };
    let positions: Vec<Vec3> = pos_it.map(Vec3::from_array).collect();
    let normals: Vec<Vec3> = reader
        .read_normals()
        .map(|i| i.map(Vec3::from_array).collect())
        .unwrap_or_else(|| vec![Vec3::Y; positions.len()]);
    let uvs: Vec<[f32; 2]> = reader
        .read_tex_coords(0)
        .map(|t| t.into_f32().collect())
        .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
    let joints = reader
        .read_joints(0)
        .map(|j| j.into_u16().collect())
        .unwrap_or_else(|| vec![[0; 4]; positions.len()]);
    let weights = reader
        .read_weights(0)
        .map(|w| w.into_f32().collect())
        .unwrap_or_else(|| vec![[1.0, 0.0, 0.0, 0.0]; positions.len()]);
    let indices: Vec<u32> = if let Some(idx) = reader.read_indices() {
        idx.into_u32().collect()
    } else {
        (0..positions.len() as u32).collect()
    };

    let (albedo, albedo_pixels, albedo_size) = super::gltf_loader::extract_albedo_pub(primitive, images);

    Ok(Some(SkinnedPrim {
        positions,
        normals,
        uvs,
        joints,
        weights,
        indices,
        albedo,
        albedo_pixels,
        albedo_size,
    }))
}

fn collect_locals(doc: &gltf::Document) -> Vec<Mat4> {
    let mut locals = vec![Mat4::IDENTITY; doc.nodes().len()];
    for node in doc.nodes() {
        locals[node.index()] = Mat4::from_cols_array_2d(&node.transform().matrix());
    }
    locals
}

fn compute_worlds(doc: &gltf::Document, locals: &[Mat4]) -> Vec<Mat4> {
    let mut worlds = vec![Mat4::IDENTITY; locals.len()];
    fn walk(node: gltf::Node<'_>, parent: Mat4, locals: &[Mat4], worlds: &mut [Mat4]) {
        let world = parent * locals[node.index()];
        worlds[node.index()] = world;
        for child in node.children() {
            walk(child, world, locals, worlds);
        }
    }
    if let Some(scene) = doc.default_scene().or_else(|| doc.scenes().next()) {
        for root in scene.nodes() {
            walk(root, Mat4::IDENTITY, locals, &mut worlds);
        }
    }
    worlds
}
