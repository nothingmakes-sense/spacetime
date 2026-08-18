//! glTF / GLB import with node-transform baking and CPU linear-blend skinning.

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use glam::{Mat4, Vec3};
use gltf::mesh::Mode;

use super::vertex::Vertex;
use super::{Mesh, Model};

pub fn load_gltf(path: impl AsRef<Path>) -> Result<Model> {
    let path = path.as_ref();
    let (doc, buffers, images) = gltf::import(path)
        .with_context(|| format!("failed to import glTF {}", path.display()))?;

    let node_locals = collect_node_locals(&doc);
    let node_worlds = compute_world_matrices(&doc, &node_locals);

    let mut meshes = Vec::new();
    let mut sockets = Vec::new();

    for node in doc.nodes() {
        if let Some(name) = node.name() {
            if name.to_ascii_lowercase().contains("slot") {
                sockets.push((name.to_string(), node_worlds[node.index()]));
            }
        }

        let Some(mesh) = node.mesh() else { continue };
        let node_world = node_worlds[node.index()];

        let joint_palette = node
            .skin()
            .map(|s| build_joint_palette(&s, &node_worlds, &buffers))
            .transpose()?;

        for primitive in mesh.primitives() {
            if primitive.mode() != Mode::Triangles {
                log::warn!(
                    "{}: skipping non-triangle primitive {:?}",
                    path.display(),
                    primitive.mode()
                );
                continue;
            }
            if let Some(m) = load_primitive(
                &primitive,
                &buffers,
                &images,
                node_world,
                joint_palette.as_deref(),
            )? {
                meshes.push(m);
            }
        }
    }

    if meshes.is_empty() {
        return Err(anyhow!("{} contained no triangle meshes", path.display()));
    }

    Ok(Model {
        name: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("model")
            .to_string(),
        meshes,
        sockets,
    })
}

fn collect_node_locals(doc: &gltf::Document) -> Vec<Mat4> {
    let count = doc.nodes().len();
    let mut locals = vec![Mat4::IDENTITY; count];
    for node in doc.nodes() {
        locals[node.index()] = Mat4::from_cols_array_2d(&node.transform().matrix());
    }
    locals
}

fn compute_world_matrices(doc: &gltf::Document, locals: &[Mat4]) -> Vec<Mat4> {
    let mut worlds = vec![Mat4::IDENTITY; locals.len()];
    let mut visited = vec![false; locals.len()];

    fn walk(
        node: gltf::Node<'_>,
        parent: Mat4,
        locals: &[Mat4],
        worlds: &mut [Mat4],
        visited: &mut [bool],
    ) {
        let world = parent * locals[node.index()];
        worlds[node.index()] = world;
        visited[node.index()] = true;
        for child in node.children() {
            walk(child, world, locals, worlds, visited);
        }
    }

    if let Some(scene) = doc.default_scene().or_else(|| doc.scenes().next()) {
        for root in scene.nodes() {
            walk(root, Mat4::IDENTITY, locals, &mut worlds, &mut visited);
        }
    }

    for (i, was) in visited.iter().enumerate() {
        if !was {
            worlds[i] = locals[i];
        }
    }
    worlds
}

fn build_joint_palette(
    skin: &gltf::Skin<'_>,
    node_worlds: &[Mat4],
    buffers: &[gltf::buffer::Data],
) -> Result<Vec<Mat4>> {
    let joints: Vec<usize> = skin.joints().map(|n| n.index()).collect();
    let reader = skin.reader(|b| Some(&buffers[b.index()]));
    let ibms: Vec<Mat4> = if let Some(iter) = reader.read_inverse_bind_matrices() {
        iter.map(|m| Mat4::from_cols_array_2d(&m)).collect()
    } else {
        vec![Mat4::IDENTITY; joints.len()]
    };
    if ibms.len() != joints.len() {
        return Err(anyhow!(
            "skin inverse-bind count {} != joint count {}",
            ibms.len(),
            joints.len()
        ));
    }
    Ok(joints
        .iter()
        .zip(ibms.iter())
        .map(|(&joint, ibm)| node_worlds[joint] * *ibm)
        .collect())
}

fn load_primitive(
    primitive: &gltf::Primitive<'_>,
    buffers: &[gltf::buffer::Data],
    images: &[gltf::image::Data],
    node_world: Mat4,
    joint_palette: Option<&[Mat4]>,
) -> Result<Option<Mesh>> {
    let reader = primitive.reader(|b| Some(&buffers[b.index()]));

    let positions: Vec<[f32; 3]> = match reader.read_positions() {
        Some(iter) => iter.collect(),
        None => return Ok(None),
    };
    let normals: Vec<[f32; 3]> = reader
        .read_normals()
        .map(|i| i.collect())
        .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
    let uvs: Vec<[f32; 2]> = reader
        .read_tex_coords(0)
        .map(|t| t.into_f32().collect())
        .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

    let joints: Option<Vec<[u16; 4]>> = reader.read_joints(0).map(|j| j.into_u16().collect());
    let weights: Option<Vec<[f32; 4]>> = reader.read_weights(0).map(|w| w.into_f32().collect());

    let indices: Vec<u32> = if let Some(idx) = reader.read_indices() {
        idx.into_u32().collect()
    } else {
        (0..positions.len() as u32).collect()
    };

    let mut vertices = Vec::with_capacity(positions.len());
    for i in 0..positions.len() {
        let mut p = Vec3::from_array(positions[i]);
        let mut n = Vec3::from_array(normals[i]);

        if let (Some(palette), Some(js), Some(ws)) =
            (joint_palette, joints.as_ref(), weights.as_ref())
        {
            // glTF ignores the mesh-node transform for skinned primitives.
            let (sp, sn) = skin_vertex(p, n, js[i], ws[i], palette);
            p = sp;
            n = sn;
        } else {
            p = node_world.transform_point3(p);
            n = node_world.transform_vector3(n).normalize_or_zero();
        }

        vertices.push(Vertex::new(
            p.to_array(),
            n.to_array(),
            uvs.get(i).copied().unwrap_or([0.0, 0.0]),
        ));
    }

    let (albedo, albedo_pixels, albedo_size) = extract_albedo(primitive, images);

    Ok(Some(Mesh {
        vertices,
        indices,
        albedo,
        albedo_pixels,
        albedo_size,
    }))
}

fn skin_vertex(
    position: Vec3,
    normal: Vec3,
    joints: [u16; 4],
    weights: [f32; 4],
    palette: &[Mat4],
) -> (Vec3, Vec3) {
    let mut skinned_p = Vec3::ZERO;
    let mut skinned_n = Vec3::ZERO;
    let mut wsum = 0.0_f32;
    for i in 0..4 {
        let w = weights[i];
        if w <= 0.0 {
            continue;
        }
        let ji = joints[i] as usize;
        if ji >= palette.len() {
            continue;
        }
        let m = palette[ji];
        skinned_p += w * m.transform_point3(position);
        skinned_n += w * m.transform_vector3(normal);
        wsum += w;
    }
    if wsum <= 0.0 {
        return (position, normal);
    }
    (skinned_p / wsum, skinned_n.normalize_or_zero())
}

fn extract_albedo(
    primitive: &gltf::Primitive<'_>,
    images: &[gltf::image::Data],
) -> ([f32; 4], Option<Vec<u8>>, (u32, u32)) {
    let mat = primitive.material();
    let pbr = mat.pbr_metallic_roughness();
    let factor = pbr.base_color_factor();

    if let Some(tex) = pbr.base_color_texture() {
        let image_index = tex.texture().source().index();
        if let Some(img) = images.get(image_index) {
            let rgba = to_rgba8(img);
            return (factor, Some(rgba), (img.width, img.height));
        }
    }

    (factor, None, (1, 1))
}

fn to_rgba8(img: &gltf::image::Data) -> Vec<u8> {
    let n = (img.width as usize) * (img.height as usize);
    match img.format {
        gltf::image::Format::R8G8B8A8 => img.pixels.clone(),
        gltf::image::Format::R8G8B8 => {
            let mut out = Vec::with_capacity(n * 4);
            for c in img.pixels.chunks(3) {
                out.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
            out
        }
        gltf::image::Format::R8 => {
            let mut out = Vec::with_capacity(n * 4);
            for &p in &img.pixels {
                out.extend_from_slice(&[p, p, p, 255]);
            }
            out
        }
        gltf::image::Format::R8G8 => {
            let mut out = Vec::with_capacity(n * 4);
            for c in img.pixels.chunks(2) {
                out.extend_from_slice(&[c[0], c[0], c[0], c[1]]);
            }
            out
        }
        other => {
            log::warn!("unsupported glTF image format {other:?}, using white");
            vec![255, 255, 255, 255]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn pack(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
    }

    #[test]
    fn loads_knight_skin_and_hand_sockets() {
        let model = load_gltf(pack("assets/kaykit/characters/Knight.glb")).unwrap();
        assert!(model.meshes.len() >= 8, "expected body parts, got {}", model.meshes.len());
        assert!(model.vertex_count() > 500);
        assert!(
            model.socket("handslot.r").is_some(),
            "KayKit handslot.r missing"
        );
    }

    #[test]
    fn loads_sword() {
        let model = load_gltf(pack("assets/kaykit/weapons/sword_1handed.gltf")).unwrap();
        assert_eq!(model.meshes.len(), 1);
        assert!(model.meshes[0].albedo_pixels.is_some());
    }
}
